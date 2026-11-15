//! How the displayer takes the panel (docs/face.md, 「How it takes the panel」). In a logind session on
//! a VT of its own it takes control of the session, has logind hand it the card with display mastery
//! while its VT is in front and take it back when it is not, brings its VT to the front as it starts,
//! and brings it back when a greeter started since takes the front. Outside a session it opens the card
//! itself and takes mastery once nothing holds it.

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::StreamExt;
use tokio::sync::mpsc::UnboundedSender;
use zbus::{Connection, OwnedFd as BusFd, OwnedObjectPath, Proxy};

use crate::listener::Backoff;
use crate::panel::{Card, has_connected};

const LOGIND: &str = "org.freedesktop.login1";
const MANAGER_PATH: &str = "/org/freedesktop/login1";
const MANAGER: &str = "org.freedesktop.login1.Manager";
const SESSION: &str = "org.freedesktop.login1.Session";
const SEAT: &str = "org.freedesktop.login1.Seat";
const GREETER: &str = "greeter";
/// The first wait before bringing the VT back from a greeter that started, so it is up before it is
/// moved to the background; each bringing back after waits twice as long, up to a minute.
const GREETER_SETTLES: Duration = Duration::from_secs(3);
/// The kernel's VT calls, from its `vt.h`.
const VT_GETSTATE: libc::c_ulong = 0x5603;
const VT_ACTIVATE: libc::c_ulong = 0x5606;
/// The DRM cards, by their device names.
const CARDS: &str = "/dev/dri";
const CARD_PREFIX: &str = "card";

/// What the seat tells the displayer.
pub enum SeatEvent {
  /// The card is the displayer's to draw on, as mastery is had again.
  Card(Card),
  /// The card is not the displayer's to draw on until it comes again.
  Lost,
}

/// The kernel's `struct vt_stat`.
#[repr(C)]
#[derive(Default)]
struct VtState {
  active: libc::c_ushort,
  signal: libc::c_ushort,
  state: libc::c_ushort,
}

/// The VT this process's standard input is, as far as the kernel's VT calls go.
#[derive(Clone, Copy, Debug)]
struct Terminal;

impl Terminal {
  /// The VT in front, where standard input is a VT.
  fn active(self) -> Option<u32> {
    let mut state = VtState::default();
    // SAFETY: VT_GETSTATE fills a `struct vt_stat`, which `state` is laid out as.
    let got = unsafe { libc::ioctl(std::io::stdin().as_raw_fd(), VT_GETSTATE, &mut state) };
    (got == 0).then_some(u32::from(state.active))
  }

  /// Brings a VT to the front, as this VT's controlling process may.
  fn activate(self, vt: u32) -> Result<(), String> {
    // SAFETY: VT_ACTIVATE takes the VT's number by value.
    let done = unsafe { libc::ioctl(std::io::stdin().as_raw_fd(), VT_ACTIVATE, libc::c_ulong::from(vt)) };
    if done == 0 {
      Ok(())
    } else {
      Err(format!("VT {vt} was not brought to the front: {}", std::io::Error::last_os_error()))
    }
  }
}

/// The hand the displayer keeps on its seat: what to give back as it stops.
#[derive(Clone, Default)]
pub struct Seat {
  /// The displayer's VT and the one that stood in front when it first took it.
  vts: Arc<Mutex<Option<(u32, u32)>>>,
}

impl Seat {
  /// Brings back the VT that stood in front before the displayer took it, where the displayer's own VT
  /// is still in front.
  pub fn give_back(&self) {
    let vts = *self.vts.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((ours, before)) = vts
      && Terminal.active() == Some(ours)
      && before != ours
    {
      let _ = Terminal.activate(before);
    }
  }

  /// Takes the panel for as long as the displayer runs, telling it each time the card comes and goes.
  pub async fn hold(self, card: Option<PathBuf>, first_retry: Duration, tell: UnboundedSender<SeatEvent>) {
    let mut backoff = Backoff::new(first_retry);
    loop {
      let fault = match seat_session() {
        Some(session) => self.through_logind(&session, card.as_deref(), &tell).await,
        None => return opened_by_hand(card.as_deref(), &mut backoff, &tell).await,
      };
      let _ = tell.send(SeatEvent::Lost);
      eprintln!("bmo-face-displayer: the seat: {fault}; trying again");
      tokio::time::sleep(backoff.next_wait()).await;
    }
  }

  /// The card through logind, for as long as the bus holds; what broke it, when it does.
  async fn through_logind(
    &self,
    session: &str,
    named: Option<&Path>,
    tell: &UnboundedSender<SeatEvent>,
  ) -> String {
    match self.logind(session, named, tell).await {
      Ok(()) => "logind's signals ended".to_owned(),
      Err(fault) => fault.to_string(),
    }
  }

  async fn logind(
    &self,
    session_id: &str,
    named: Option<&Path>,
    tell: &UnboundedSender<SeatEvent>,
  ) -> zbus::Result<()> {
    let bus = Connection::system().await?;
    let manager = Proxy::new(&bus, LOGIND, MANAGER_PATH, MANAGER).await?;
    let path: OwnedObjectPath = manager.call("GetSession", &(session_id,)).await?;
    let session = Proxy::new(&bus, LOGIND, path.clone(), SESSION).await?;
    session.call::<_, _, ()>("TakeControl", &(false,)).await?;
    let mut pauses = session.receive_signal("PauseDevice").await?;
    let mut resumes = session.receive_signal("ResumeDevice").await?;
    let (device, card) = take_card(&session, named).await?;
    if let Some(card) = card {
      let _ = tell.send(SeatEvent::Card(card));
    }

    let (_, seat_path): (String, OwnedObjectPath) = session.get_property("Seat").await?;
    let seat = Proxy::new(&bus, LOGIND, seat_path, SEAT).await?;
    let mut fronts = seat.receive_property_changed::<(String, OwnedObjectPath)>("ActiveSession").await;
    let ours: u32 = session.get_property("VTNr").await?;
    // The sessions standing as the displayer takes the front: a greeter among them was chosen by someone.
    let mut known: HashSet<String> = sessions(&manager).await?;
    let mut reclaims = Backoff::new(GREETER_SETTLES);
    if ours > 0
      && let Some(before) = Terminal.active()
    {
      self
        .vts
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get_or_insert((ours, before));
      if before != ours {
        bring_forward(ours);
      }
    }

    loop {
      tokio::select! {
        Some(pause) = pauses.next() => {
          let (major, minor, kind): (u32, u32, String) = pause.body().deserialize()?;
          if (major, minor) == device {
            let _ = tell.send(SeatEvent::Lost);
            if kind == "pause" {
              session.call::<_, _, ()>("PauseDeviceComplete", &(major, minor)).await?;
            }
          }
        }
        Some(resume) = resumes.next() => {
          let (major, minor, fd): (u32, u32, BusFd) = resume.body().deserialize()?;
          if (major, minor) == device {
            let _ = tell.send(SeatEvent::Card(Card::new(OwnedFd::from(fd))));
          }
        }
        Some(front) = fronts.next() => {
          let (id, front_path) = front.get().await?;
          if front_path == path {
            known = sessions(&manager).await?;
          } else if ours > 0 && !id.is_empty() && !known.contains(&id) && is_greeter(&bus, front_path.clone()).await {
            // A greeter that started since the displayer last stood in front took the VT by starting.
            tokio::time::sleep(reclaims.next_wait()).await;
            let still: (String, OwnedObjectPath) = seat.get_property("ActiveSession").await?;
            if still.1 == front_path {
              bring_forward(ours);
            }
          }
        }
        else => return Ok(()),
      }
    }
  }
}

/// The logind session this process runs in, where it stands on a seat; none for a session with no
/// seat, as over ssh, or none at all.
fn seat_session() -> Option<String> {
  let named = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
  named("XDG_SEAT").and(named("XDG_SESSION_ID"))
}

/// Brings the displayer's VT to the front, saying so where it cannot.
fn bring_forward(vt: u32) {
  if let Err(fault) = Terminal.activate(vt) {
    eprintln!("bmo-face-displayer: {fault}");
  }
}

/// The ids of every session logind holds now.
async fn sessions(manager: &Proxy<'_>) -> zbus::Result<HashSet<String>> {
  let listed: Vec<(String, u32, String, String, OwnedObjectPath)> =
    manager.call("ListSessions", &()).await?;
  Ok(listed.into_iter().map(|(id, ..)| id).collect())
}

/// Whether a session is a greeter's; a session gone already is none.
async fn is_greeter(bus: &Connection, path: OwnedObjectPath) -> bool {
  let class = async { Proxy::new(bus, LOGIND, path, SESSION).await?.get_property::<String>("Class").await };
  class.await.is_ok_and(|class| class == GREETER)
}

/// The cards to look at: the one named, else every card in order.
fn candidates(named: Option<&Path>) -> Vec<PathBuf> {
  if let Some(named) = named {
    return vec![named.to_owned()];
  }
  let mut cards: Vec<PathBuf> = fs::read_dir(CARDS)
    .map(|entries| {
      entries
        .filter_map(Result::ok)
        .filter(|entry| {
          entry
            .file_name()
            .to_string_lossy()
            .strip_prefix(CARD_PREFIX)
            .is_some_and(|n| n.parse::<u32>().is_ok())
        })
        .map(|entry| entry.path())
        .collect()
    })
    .unwrap_or_default();
  cards.sort_by_key(|card| {
    card
      .file_name()
      .and_then(|name| name.to_str())
      .and_then(|name| name[CARD_PREFIX.len()..].parse::<u32>().ok())
  });
  cards
}

/// The card logind hands over: the one named, else the first with a connected connector. The card
/// comes with it where the session is in front.
async fn take_card(session: &Proxy<'_>, named: Option<&Path>) -> zbus::Result<((u32, u32), Option<Card>)> {
  for path in candidates(named) {
    let Ok(rdev) = fs::metadata(&path).map(|meta| meta.rdev()) else { continue };
    let device = (libc::major(rdev), libc::minor(rdev));
    let (fd, inactive): (BusFd, bool) = session.call("TakeDevice", &device).await?;
    let card = Card::new(OwnedFd::from(fd));
    if named.is_some() || has_connected(&card) {
      return Ok((device, (!inactive).then_some(card)));
    }
    drop(card);
    session.call::<_, _, ()>("ReleaseDevice", &device).await?;
  }
  Err(zbus::Error::Failure("no card with a connected connector".to_owned()))
}

/// The card opened by hand, outside any session: once something stops holding its mastery.
async fn opened_by_hand(named: Option<&Path>, backoff: &mut Backoff, tell: &UnboundedSender<SeatEvent>) {
  loop {
    let mut faults = Vec::new();
    for path in candidates(named) {
      match OpenOptions::new().read(true).write(true).open(&path) {
        Ok(file) => {
          let card = Card::new(OwnedFd::from(file));
          if named.is_none() && !has_connected(&card) {
            continue;
          }
          match drm::Device::acquire_master_lock(&card) {
            Ok(()) => {
              let _ = tell.send(SeatEvent::Card(card));
              return;
            }
            Err(fault) => faults.push(format!("{}: mastery not had: {fault}", path.display())),
          }
        }
        Err(fault) => faults.push(format!("{}: {fault}", path.display())),
      }
    }
    let why = if faults.is_empty() {
      "no card with a connected connector".to_owned()
    } else {
      faults.join("; ")
    };
    eprintln!("bmo-face-displayer: the panel: {why}; trying again");
    tokio::time::sleep(backoff.next_wait()).await;
  }
}
