//! The displayer running (docs/face.md, 「The native displayer」): the roads it hears on — the face
//! server's saves and the voice engine's events — each held open by a task of its own and opened again
//! with growing waits, and the one loop that tells the face what they say, draws when the drawing may
//! change and sleeps in between.

use std::os::fd::{AsFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bmo_face_core::drawing::Drawing;
use bmo_face_core::geometry::Point;
use serde_json::Value;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::time::{sleep, sleep_until};

use crate::alsa::Pcm;
use crate::config::{Config, Sound as SoundSetting};
use crate::engine::{Unasked, VoiceEngine};
use crate::face::Face;
use crate::gaze;
use crate::http::Fault;
use crate::listener::{Backoff, Cursor, ENGINE_SILENCE, Heard, SILENCE, heard};
use crate::panel::{Card, Panel};
use crate::playback;
use crate::render::Canvas;
use crate::seat::{Seat, SeatEvent};
use crate::server::FaceServer;
use crate::sound::{self, Sound};

/// The size the frames are written at, the panel the displayer is first made for.
pub const FRAMES_SIZE: (u32, u32) = (1024, 600);
/// The breath between one briefing ending and the next one waiting behind it starting, as every listener
/// takes it.
const BREATH: Duration = Duration::from_millis(600);

/// Where the drawings go.
pub enum Output {
  /// The panel, taken through the seat.
  Panel,
  /// PNG files in a folder, one a drawing shown.
  Frames(PathBuf),
}

/// What the roads tell the loop.
enum Event {
  Document(Value),
  /// The saves stream opened again after its first opening.
  SavesAgain,
  Heard(Heard),
  Ready {
    id: u64,
    sound: Sound,
    pcm: Option<Pcm>,
  },
  Ended {
    id: u64,
    fault: Option<String>,
  },
  Gaze(Point),
  Seat(SeatEvent),
  Stop,
}

fn say(line: &str) {
  eprintln!("bmo-face-displayer: {line}");
}

/// Milliseconds on this machine's wall clock, as the engine's moments are written.
fn epoch_ms() -> f64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map_or(0.0, |since| since.as_secs_f64() * 1_000.0)
}

/// Whether the deploy link the program was started through now leads to another executable.
pub fn release_moved(started_as: &Path, running: &Path) -> bool {
  std::fs::canonicalize(started_as).is_ok_and(|now| now != running)
}

/// The expression document, read whenever the saves stream names a version not read yet.
async fn saves(server: FaceServer, mut backoff: Backoff, tell: UnboundedSender<Event>) {
  let mut read: Option<f64> = None;
  let mut opened = false;
  loop {
    match server.saves().await {
      Ok(mut events) => {
        if opened {
          let _ = tell.send(Event::SavesAgain);
        }
        opened = true;
        backoff.reset();
        while let Some(event) = events.next_within(SILENCE).await {
          if event.event.as_deref() != Some("faces") {
            continue;
          }
          let named = event.data.as_deref().and_then(|data| serde_json::from_str::<Value>(data).ok());
          let Some(version) = named.and_then(|named| named["savedAt"].as_f64()) else { continue };
          if read == Some(version) {
            continue;
          }
          match server.document().await {
            Ok(document) => {
              read = document["savedAt"].as_f64();
              let _ = tell.send(Event::Document(document));
            }
            Err(fault) => say(&format!("the document: {fault}")),
          }
        }
      }
      Err(fault) => say(&format!("the saves stream: {fault}")),
    }
    sleep(backoff.next_wait()).await;
  }
}

/// The voice engine's event stream, resumed from its cursor: the briefings of the rooms the settings name,
/// or of every room where they name none, and every stop.
async fn listen(engine: VoiceEngine, rooms: Vec<i64>, mut backoff: Backoff, tell: UnboundedSender<Event>) {
  let mut cursor = Cursor::default();
  let mut told: Option<String> = None;
  loop {
    match engine.events(&cursor.query()).await {
      Ok(mut events) => {
        told = None;
        backoff.reset();
        while let Some(event) = events.next_within(ENGINE_SILENCE).await {
          cursor.on_id(event.id.as_deref());
          let news = heard(event.event.as_deref(), event.data.as_deref());
          if let Heard::Briefing(briefing) = &news
            && !rooms.is_empty()
            && !rooms.contains(&briefing.chat_id)
          {
            continue;
          }
          let _ = tell.send(Event::Heard(news));
        }
      }
      Err(Unasked::Fault(Fault::Status(hyper::StatusCode::GONE))) => {
        say("the voice engine keeps the events past the cursor no longer; reading on from the live ones");
        cursor.gone();
      }
      // The same fault said once while it lasts, however often it is tried again.
      Err(fault) => {
        let line = format!("the voice engine's events: {fault}");
        if told.as_deref() != Some(line.as_str()) {
          say(&line);
          told = Some(line);
        }
      }
    }
    sleep(backoff.next_wait()).await;
  }
}

/// A briefing's sound fetched, decoded and its device opened, off the loop, after `delay`.
async fn ready(
  engine: VoiceEngine,
  setting: SoundSetting,
  id: u64,
  briefing: i64,
  delay: Duration,
  tell: UnboundedSender<Event>,
) {
  sleep(delay).await;
  let made = async {
    let bytes = engine.sound(briefing).await.map_err(|fault| fault.to_string())?;
    tokio::task::spawn_blocking(move || {
      let sound = sound::decode(bytes.to_vec())?;
      let pcm = match &setting {
        SoundSetting::Off => None,
        SoundSetting::Device(name) => Pcm::open(name, sound.channels, sound.rate)
          .inspect_err(|fault| say(&format!("{fault}; the briefing is spoken in silence")))
          .ok(),
      };
      Ok::<_, String>((sound, pcm))
    })
    .await
    .map_err(|fault| fault.to_string())?
  };
  let _ = tell.send(match made.await {
    Ok((sound, pcm)) => Event::Ready { id, sound, pcm },
    Err(fault) => Event::Ended {
      id,
      fault: Some(format!("the briefing's sound: {fault}")),
    },
  });
}

/// The panel as it stands: the card while the seat lends it, and the panel set up on it.
struct Shown {
  card: Option<OwnedFd>,
  panel: Option<Panel>,
  retry: Option<Instant>,
  backoff: Backoff,
}

impl Shown {
  /// Sets the panel up on the card held, or says why not and when to try again.
  fn set_up(&mut self, config: &Config) {
    let Some(card) = &self.card else { return };
    let copy = match card.as_fd().try_clone_to_owned() {
      Ok(copy) => copy,
      Err(fault) => return say(&format!("the card: {fault}")),
    };
    match Panel::new(Card::new(copy), config.connector.as_deref(), config.mode) {
      Ok(panel) => {
        say(&format!("showing on {}", panel.name()));
        self.panel = Some(panel);
        self.retry = None;
        self.backoff.reset();
      }
      Err(fault) => {
        say(&format!("the panel: {fault}; trying again"));
        self.retry = Some(Instant::now() + self.backoff.next_wait());
      }
    }
  }
}

/// Runs the displayer for as long as the machine does, until SIGTERM or an interrupt.
pub async fn run(config: Config, output: Output) -> Result<(), String> {
  let mut terminate = signal(SignalKind::terminate()).map_err(|fault| fault.to_string())?;
  let mut interrupt = signal(SignalKind::interrupt()).map_err(|fault| fault.to_string())?;
  let stop = async move {
    tokio::select! {
      _ = terminate.recv() => {}
      _ = interrupt.recv() => {}
    }
  };
  display(config, output, stop).await
}

/// Runs the displayer until `stop` ends, when it gives the seat back and ends the process.
pub async fn display(
  config: Config,
  output: Output,
  stop: impl Future<Output = ()> + Send + 'static,
) -> Result<(), String> {
  let started_as = std::env::args_os().next().map(PathBuf::from).ok_or("no program path")?;
  let running = std::env::current_exe().map_err(|fault| format!("the program's own path: {fault}"))?;
  let server = FaceServer::new(&config.face_url);
  let engine = config
    .voice
    .as_deref()
    .map(|voice| VoiceEngine::new(voice, config.voice_token.as_deref()));
  let (tell, mut heard_rx) = mpsc::unbounded_channel::<Event>();

  tokio::spawn(saves(server, Backoff::new(config.first_retry), tell.clone()));
  match &engine {
    Some(engine) => {
      tokio::spawn(listen(
        engine.clone(),
        config.rooms.clone(),
        Backoff::new(config.first_retry),
        tell.clone(),
      ));
    }
    None => say("no voice engine is named: no briefing is heard"),
  }
  // The sound of the play the face answers, made ready once the breath before it is taken.
  let start = |play: Option<u64>, face: &Face, delay: Duration| {
    let (Some(engine), Some(id)) = (&engine, play) else { return };
    let Some(briefing) = face.briefing_of(id) else { return };
    tokio::spawn(ready(engine.clone(), config.sound.clone(), id, briefing.id, delay, tell.clone()));
  };
  match gaze::listen(&config.gaze) {
    Ok(listener) => {
      let (points, mut point_rx) = mpsc::unbounded_channel();
      tokio::spawn(gaze::serve(listener, points));
      let tell = tell.clone();
      tokio::spawn(async move {
        while let Some(point) = point_rx.recv().await {
          let _ = tell.send(Event::Gaze(point));
        }
      });
    }
    Err(fault) => say(&format!("the gaze socket {}: {fault}", config.gaze.display())),
  }
  let seat = Seat::default();
  if let Output::Panel = output {
    let (cards, mut card_rx) = mpsc::unbounded_channel();
    tokio::spawn(seat.clone().hold(config.card.clone(), config.first_retry, cards));
    let tell = tell.clone();
    tokio::spawn(async move {
      while let Some(event) = card_rx.recv().await {
        let _ = tell.send(Event::Seat(event));
      }
    });
  }
  {
    let tell = tell.clone();
    tokio::spawn(async move {
      stop.await;
      let _ = tell.send(Event::Stop);
    });
  }

  let seed = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos() as u64);
  let mut face = Face::new(seed, Instant::now());
  let mut shown = Shown {
    card: None,
    panel: None,
    retry: None,
    backoff: Backoff::new(config.first_retry),
  };
  let (width, height) = FRAMES_SIZE;
  let mut canvas = Canvas::new(width, height).ok_or("no picture of the panel's size")?;
  let mut drawn: Option<Drawing> = None;
  let mut frames_written = 0_u64;

  loop {
    let wake = [face.next_change(Instant::now()), shown.retry].into_iter().flatten().min();
    let event = tokio::select! {
      event = heard_rx.recv() => event,
      () = async { if let Some(wake) = wake { sleep_until(wake.into()).await } else { std::future::pending().await } } => None,
    };
    let now = Instant::now();
    match event {
      Some(Event::Document(document)) => {
        if let Err(fault) = face.document(&document) {
          say(&format!("a document the check refuses is left: {fault}"));
        }
      }
      Some(Event::SavesAgain) => {
        if release_moved(&started_as, &running) {
          say("a new release stands; ending so the service starts it");
          std::process::exit(0);
        }
      }
      Some(Event::Heard(Heard::Briefing(briefing))) => {
        let play = face.briefing(briefing, epoch_ms());
        start(play, &face, Duration::ZERO);
      }
      Some(Event::Heard(Heard::Stopped { chat_id, at })) => {
        let play = face.silence(chat_id, at, epoch_ms(), now);
        start(play, &face, BREATH);
      }
      Some(Event::Heard(Heard::Other)) => {}
      Some(Event::Ready { id, sound, pcm }) => {
        let latency = pcm.as_ref().map_or(0.0, |pcm| pcm.buffer() as f64 / f64::from(pcm.rate()));
        if let Some(target) = face.speak(id, sound.loudness.clone(), latency, now) {
          match pcm {
            Some(pcm) => {
              let tell = tell.clone();
              let playing = playback::play(pcm, sound, target, move |result| {
                let _ = tell.send(Event::Ended {
                  id,
                  fault: result.err(),
                });
              });
              face.playing(id, playing);
            }
            None => {
              let tell = tell.clone();
              let until = target + std::time::Duration::from_secs_f64(sound.seconds());
              tokio::spawn(async move {
                sleep_until(until.into()).await;
                let _ = tell.send(Event::Ended { id, fault: None });
              });
            }
          }
        }
      }
      Some(Event::Ended { id, fault }) => {
        if let Some(fault) = fault {
          say(&format!("the sound: {fault}"));
        }
        let play = face.ended(id, now);
        start(play, &face, BREATH);
      }
      Some(Event::Gaze(point)) => face.gaze(point, now),
      Some(Event::Seat(SeatEvent::Card(card))) => {
        shown.panel = None;
        shown.card = Some(OwnedFd::from(card));
        shown.set_up(&config);
        drawn = None;
      }
      Some(Event::Seat(SeatEvent::Lost)) => {
        shown.panel = None;
        shown.card = None;
        shown.retry = None;
        drawn = None;
      }
      Some(Event::Stop) => {
        say("stopping");
        seat.give_back();
        std::process::exit(0);
      }
      None => {
        if shown.retry.is_some_and(|retry| retry <= now) {
          shown.retry = None;
          shown.set_up(&config);
        }
      }
    }

    let drawing = face.drawing(Instant::now());
    if drawn.as_ref() == Some(&drawing) {
      continue;
    }
    match &output {
      Output::Frames(folder) => {
        canvas.draw(&drawing);
        frames_written += 1;
        let path = folder.join(format!("frame-{frames_written:06}.png"));
        if let Err(fault) = canvas.pixmap().save_png(&path) {
          say(&format!("{}: {fault}", path.display()));
        }
        drawn = Some(drawing);
      }
      Output::Panel => {
        let Some(panel) = shown.panel.as_mut() else { continue };
        let (width, height) = panel.size();
        if canvas.pixmap().width() != width || canvas.pixmap().height() != height {
          canvas = Canvas::new(width, height).ok_or("no picture of the panel's size")?;
        }
        canvas.draw(&drawing);
        match panel.show(canvas.pixmap()).await {
          Ok(()) => drawn = Some(drawing),
          Err(fault) => {
            say(&format!("the panel: {fault}; setting it up again"));
            shown.panel = None;
            shown.retry = Some(Instant::now() + shown.backoff.next_wait());
          }
        }
      }
    }
  }
}
