//! How the displayer takes the panel (docs/face.md, 「How it takes the panel」). In a logind session on
//! a VT of its own it takes control of the session, has logind hand it the card with display mastery
//! while its VT is in front and take it back when it is not, brings its VT to the front as it starts,
//! and brings it back when a greeter started since takes the front. Outside a session it opens the card
//! itself and takes mastery once nothing holds it. Either way the cards it tries are the one named, else
//! those the kernel reads a connected connector on, and a card refused is passed over for the next.

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

use crate::journal::{say, seconds};
use crate::listener::Backoff;
use crate::panel::Card;

const LOGIND: &str = "org.freedesktop.login1";
const MANAGER_PATH: &str = "/org/freedesktop/login1";
const MANAGER: &str = "org.freedesktop.login1.Manager";
const SESSION: &str = "org.freedesktop.login1.Session";
const SEAT: &str = "org.freedesktop.login1.Seat";
const GREETER: &str = "greeter";
/// The kinds of logind's `PauseDevice`: one that waits for its completion, and the device's removal.
const PAUSE: &str = "pause";
const GONE: &str = "gone";
/// The first wait before bringing the VT back from a greeter that started, so it is up before it is
/// moved to the background; each bringing back after waits twice as long, up to a minute.
const GREETER_SETTLES: Duration = Duration::from_secs(3);
/// The kernel's VT calls, from its `vt.h`.
const VT_GETSTATE: libc::c_ulong = 0x5603;
const VT_ACTIVATE: libc::c_ulong = 0x5606;
/// The DRM cards, by their device names.
const CARDS: &str = "/dev/dri";
/// The kernel's DRM class, where each card's connectors stand as `<card>-<connector>` with their status.
const CLASSES: &str = "/sys/class/drm";
const CARD_PREFIX: &str = "card";
const CONNECTED: &str = "connected";
const NO_CARD: &str = "no card has a connected connector";

/// What the seat tells the displayer.
pub enum SeatEvent {
  /// The card is the displayer's to draw on, as mastery is had again.
  Card(Card),
  /// The card is not the displayer's to draw on until it comes again.
  Lost,
}

/// The card logind lent: the device, and the card itself where the session stands in front.
struct Taken {
  device: (u32, u32),
  path: PathBuf,
  card: Option<Card>,
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
    let Some(session) = seat_session() else {
      say("no logind session on a seat: opening the card by hand");
      return opened_by_hand(card.as_deref(), &mut backoff, &tell).await;
    };
    loop {
      let fault = match self.logind(&session, card.as_deref(), &mut backoff, &tell).await {
        Ok(()) => "logind's signals ended".to_owned(),
        Err(fault) => fault,
      };
      let _ = tell.send(SeatEvent::Lost);
      let wait = backoff.next_wait();
      say(&format!("the seat: {fault}; nothing is drawn; trying again in {}", seconds(wait)));
      tokio::time::sleep(wait).await;
    }
  }

  /// The card through logind, for as long as the bus holds; what broke it, when it does.
  async fn logind(
    &self,
    session_id: &str,
    named: Option<&Path>,
    backoff: &mut Backoff,
    tell: &UnboundedSender<SeatEvent>,
  ) -> Result<(), String> {
    let bus = Connection::system().await.map_err(on("the system bus"))?;
    let manager = Proxy::new(&bus, LOGIND, MANAGER_PATH, MANAGER).await.map_err(on("logind"))?;
    let path: OwnedObjectPath =
      manager.call("GetSession", &(session_id,)).await.map_err(on("the session"))?;
    let session = Proxy::new(&bus, LOGIND, path.clone(), SESSION).await.map_err(on("the session"))?;
    session
      .call::<_, _, ()>("TakeControl", &(false,))
      .await
      .map_err(on("taking control of the session"))?;
    let mut pauses = session.receive_signal("PauseDevice").await.map_err(on("logind's pauses"))?;
    let mut resumes = session.receive_signal("ResumeDevice").await.map_err(on("logind's resumes"))?;
    let ours: u32 = session.get_property("VTNr").await.map_err(on("the session's VT"))?;
    let Taken {
      device,
      path: card_path,
      card,
    } = take_card(&session, named).await?;
    backoff.reset();
    match card {
      Some(card) => {
        say(&format!("took {} from logind", card_path.display()));
        let _ = tell.send(SeatEvent::Card(card));
      }
      None => say(&format!(
        "took {} from logind while the session is not in front: nothing is drawn until logind resumes it",
        card_path.display()
      )),
    }

    let (_, seat_path): (String, OwnedObjectPath) =
      session.get_property("Seat").await.map_err(on("the session's seat"))?;
    let seat = Proxy::new(&bus, LOGIND, seat_path, SEAT).await.map_err(on("the seat"))?;
    let mut fronts = seat.receive_property_changed::<(String, OwnedObjectPath)>("ActiveSession").await;
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
        say(&format!("bringing VT {ours} to the front from VT {before}"));
        bring_forward(ours);
      }
    }

    loop {
      tokio::select! {
        Some(pause) = pauses.next() => {
          let (major, minor, kind): (u32, u32, String) = pause.body().deserialize().map_err(on("a pause"))?;
          if (major, minor) == device {
            let _ = tell.send(SeatEvent::Lost);
            if kind == GONE {
              return Err(format!("{} is gone", card_path.display()));
            }
            say(&format!(
              "logind paused {} ({kind}): the session left the front; nothing is drawn until it comes back",
              card_path.display()
            ));
            if kind == PAUSE {
              session
                .call::<_, _, ()>("PauseDeviceComplete", &(major, minor))
                .await
                .map_err(on("completing a pause"))?;
            }
          }
        }
        Some(resume) = resumes.next() => {
          let (major, minor, fd): (u32, u32, BusFd) = resume.body().deserialize().map_err(on("a resume"))?;
          if (major, minor) == device {
            say(&format!("logind resumed {}", card_path.display()));
            let _ = tell.send(SeatEvent::Card(Card::new(OwnedFd::from(fd), card_path.clone())));
          }
        }
        Some(front) = fronts.next() => {
          let (id, front_path) = front.get().await.map_err(on("the seat's front"))?;
          if front_path == path {
            known = sessions(&manager).await?;
          } else if ours > 0 && !id.is_empty() && !known.contains(&id) && is_greeter(&bus, front_path.clone()).await {
            // A greeter that started since the displayer last stood in front took the VT by starting.
            let wait = reclaims.next_wait();
            say(&format!(
              "greeter session {id} started and took the front; bringing VT {ours} back in {}",
              seconds(wait)
            ));
            tokio::time::sleep(wait).await;
            let still: (String, OwnedObjectPath) =
              seat.get_property("ActiveSession").await.map_err(on("the seat's front"))?;
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

/// A bus fault said with what was being done.
fn on(what: &str) -> impl FnOnce(zbus::Error) -> String + '_ {
  move |fault| format!("{what}: {fault}")
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
    say(&fault);
  }
}

/// The ids of every session logind holds now.
async fn sessions(manager: &Proxy<'_>) -> Result<HashSet<String>, String> {
  let listed: Vec<(String, u32, String, String, OwnedObjectPath)> =
    manager.call("ListSessions", &()).await.map_err(on("logind's sessions"))?;
  Ok(listed.into_iter().map(|(id, ..)| id).collect())
}

/// Whether a session is a greeter's; a session gone already is none.
async fn is_greeter(bus: &Connection, path: OwnedObjectPath) -> bool {
  let class = async { Proxy::new(bus, LOGIND, path, SESSION).await?.get_property::<String>("Class").await };
  class.await.is_ok_and(|class| class == GREETER)
}

/// The cards to try: the one named, else those with a connected connector.
fn candidates(named: Option<&Path>) -> Vec<PathBuf> {
  match named {
    Some(named) => vec![named.to_owned()],
    None => cards_with_connected(Path::new(CARDS), Path::new(CLASSES)),
  }
}

/// The cards among the devices in `devices`, in their numbers' order, that have a connector the
/// kernel's DRM class in `classes` reads as connected. Read before any card is opened, so a card with
/// nothing on it is never asked for, whoever holds it.
pub fn cards_with_connected(devices: &Path, classes: &Path) -> Vec<PathBuf> {
  let mut cards: Vec<(u32, String)> = names_in(devices)
    .into_iter()
    .filter_map(|name| Some((name.strip_prefix(CARD_PREFIX)?.parse().ok()?, name)))
    .collect();
  cards.sort();
  let connectors = names_in(classes);
  cards
    .into_iter()
    .filter(|(_, card)| {
      connectors
        .iter()
        .filter(|connector| connector.strip_prefix(card.as_str()).is_some_and(|rest| rest.starts_with('-')))
        .any(|connector| {
          fs::read_to_string(classes.join(connector).join("status"))
            .is_ok_and(|status| status.trim() == CONNECTED)
        })
    })
    .map(|(_, card)| devices.join(card))
    .collect()
}

/// The names of what stands in a folder; none where it cannot be read.
fn names_in(folder: &Path) -> Vec<String> {
  fs::read_dir(folder)
    .map(|entries| {
      entries
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
    })
    .unwrap_or_default()
}

/// The first card logind hands over, each refusal said and passed over for the next card. The card
/// comes with it where the session is in front.
async fn take_card(session: &Proxy<'_>, named: Option<&Path>) -> Result<Taken, String> {
  let candidates = candidates(named);
  if candidates.is_empty() {
    return Err(NO_CARD.to_owned());
  }
  for path in candidates {
    let rdev = match fs::metadata(&path) {
      Ok(meta) => meta.rdev(),
      Err(fault) => {
        say(&format!("{}: {fault}", path.display()));
        continue;
      }
    };
    let device = (libc::major(rdev), libc::minor(rdev));
    match session.call::<_, _, (BusFd, bool)>("TakeDevice", &device).await {
      Ok((fd, inactive)) => {
        let card = (!inactive).then(|| Card::new(OwnedFd::from(fd), path.clone()));
        return Ok(Taken { device, path, card });
      }
      Err(fault) => say(&format!("logind does not hand over {}: {fault}", path.display())),
    }
  }
  Err("logind handed over none of the cards with a connected connector".to_owned())
}

/// The card opened by hand, outside any session: once something stops holding its mastery.
async fn opened_by_hand(named: Option<&Path>, backoff: &mut Backoff, tell: &UnboundedSender<SeatEvent>) {
  loop {
    let mut faults = Vec::new();
    for path in candidates(named) {
      match OpenOptions::new().read(true).write(true).open(&path) {
        Ok(file) => {
          let card = Card::new(OwnedFd::from(file), path.clone());
          match drm::Device::acquire_master_lock(&card) {
            Ok(()) => {
              say(&format!("took {} by hand, with display mastery", path.display()));
              let _ = tell.send(SeatEvent::Card(card));
              return;
            }
            Err(fault) => faults.push(format!("{}: mastery not had: {fault}", path.display())),
          }
        }
        Err(fault) => faults.push(format!("{}: {fault}", path.display())),
      }
    }
    let why = if faults.is_empty() { NO_CARD.to_owned() } else { faults.join("; ") };
    let wait = backoff.next_wait();
    say(&format!("the panel: {why}; nothing is drawn; trying again in {}", seconds(wait)));
    tokio::time::sleep(wait).await;
  }
}
