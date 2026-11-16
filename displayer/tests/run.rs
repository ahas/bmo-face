//! The whole displayer against a stand-in face server and a stand-in voice engine (docs/face.md, 「What it
//! hears」): run in-process with its drawings written as frames into a scratch folder, its waits
//! shortened, over one stand-in that serves the face's document and its saves under `/bmo-face`, and the
//! engine's event stream as the test feeds it and a briefing's sound under `/engine`, and writes down what
//! it was asked. The engine's events and briefings take the shapes its `docs/openapi.json` states, and its
//! stream opens with a comment line as the engine's keep beat writes one. Every wait is a bounded wait on
//! what it waits for.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::body::Body;
use axum::body::Bytes;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use bmo_face_displayer::config::Config;
use bmo_face_displayer::gaze;
use bmo_face_displayer::run::{Output, display};
use futures_util::Stream;
use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, UnixStream};
use tokio::sync::{broadcast, mpsc};

const SOUND: &[u8] = include_bytes!("fixtures/tone-then-silence.ogg");
const TOKEN: &str = "engine-token";
const REVOKED: &str = "revoked-token";
const STREAM: &str = "/engine/events";
const FIRST_QUERY: &str = "?kinds=briefing.ended,briefings.stopped";
/// The first of the growing waits, shortened.
const RETRY_MS: u64 = 20;
/// The longest any condition is waited for.
const PATIENCE: Duration = Duration::from_secs(20);

/// What the stand-in was asked: the path with its query, the bearer token, and when.
#[derive(Clone, Debug)]
struct Asked {
  path: String,
  token: Option<String>,
  at: Instant,
}

#[derive(Clone)]
struct StandIn {
  document: Arc<Mutex<Value>>,
  saves: broadcast::Sender<f64>,
  /// Raw frames for every open engine stream; none ends the streams open.
  heard: broadcast::Sender<Option<String>>,
  /// Whether a stream resumed from a cursor is answered 410, the events past it no longer kept.
  gone: Arc<AtomicBool>,
  asked: Arc<Mutex<Vec<Asked>>>,
}

/// An event stream's body: its first chunk at once, then every frame fed to it until it is ended.
fn frames(
  first: String,
  later: broadcast::Receiver<Option<String>>,
) -> impl Stream<Item = Result<Bytes, std::io::Error>> {
  futures_util::stream::unfold((Some(first), later), |(first, mut later)| async move {
    if let Some(first) = first {
      return Some((Ok(Bytes::from(first)), (None, later)));
    }
    loop {
      match later.recv().await {
        Ok(Some(frame)) => return Some((Ok(Bytes::from(frame)), (None, later))),
        Ok(None) | Err(broadcast::error::RecvError::Closed) => return None,
        Err(broadcast::error::RecvError::Lagged(_)) => continue,
      }
    }
  })
}

fn bearer(headers: &HeaderMap) -> Option<String> {
  headers.get("authorization")?.to_str().ok()?.strip_prefix("Bearer ").map(str::to_owned)
}

fn event_stream(body: impl Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static) -> Response {
  ([("content-type", "text/event-stream")], Body::from_stream(body)).into_response()
}

/// The engine's refusal of a token, in its shape: a code and a sentence.
fn unauthorized() -> Response {
  (
    StatusCode::UNAUTHORIZED,
    axum::Json(json!({ "code": "unauthorized", "error": "the token is wrong" })),
  )
    .into_response()
}

async fn answer(State(stand_in): State<StandIn>, request: Request) -> Response {
  let path = request.uri().path_and_query().map_or(String::new(), |path| path.as_str().to_owned());
  let token = bearer(request.headers());
  stand_in.asked.lock().expect("the log").push(Asked {
    path: path.clone(),
    token: token.clone(),
    at: Instant::now(),
  });
  let route = path.split('?').next().unwrap_or_default().to_owned();
  match route.as_str() {
    "/bmo-face/api/faces" => {
      axum::Json(stand_in.document.lock().expect("the document").clone()).into_response()
    }
    "/bmo-face/api/faces/events" => {
      let first = stand_in.document.lock().expect("the document")["savedAt"].clone();
      let mut saves = stand_in.saves.subscribe();
      let (tx, rx) = broadcast::channel(16);
      tokio::spawn(async move {
        while let Ok(version) = saves.recv().await {
          let frame = format!("event: faces\ndata: {}\n\n", json!({ "savedAt": version }));
          if tx.send(Some(frame)).is_err() {
            return;
          }
        }
      });
      event_stream(frames(format!("event: faces\ndata: {}\n\n", json!({ "savedAt": first })), rx))
    }
    STREAM => {
      if token.as_deref() != Some(TOKEN) {
        return unauthorized();
      }
      if path.contains("after=") && stand_in.gone.swap(false, Ordering::Relaxed) {
        let refusal = json!({ "code": "gone", "error": "the events are no longer kept" });
        return (StatusCode::GONE, axum::Json(refusal)).into_response();
      }
      event_stream(frames(":\n\n".to_owned(), stand_in.heard.subscribe()))
    }
    _ if route.starts_with("/engine/briefings/") && route.ends_with("/sound") => {
      if token.as_deref() != Some(TOKEN) {
        return unauthorized();
      }
      ([("content-type", "audio/ogg")], SOUND).into_response()
    }
    _ => (StatusCode::NOT_FOUND, axum::Json(json!({ "code": "no-route", "error": "no route" })))
      .into_response(),
  }
}

/// How the displayer is set up on a bench.
struct Setup {
  /// Whether the engine's address is named.
  voice: bool,
  /// What the token file holds, or no file named.
  token: Option<&'static str>,
  rooms: Option<&'static str>,
}

const HEARING: Setup = Setup {
  voice: true,
  token: Some(TOKEN),
  rooms: None,
};

struct Bench {
  stand_in: StandIn,
  frames: PathBuf,
  token_file: PathBuf,
  _scratch: tempfile::TempDir,
  displayer: tokio::task::JoinHandle<()>,
  _server: tokio::task::JoinHandle<()>,
}

impl Drop for Bench {
  fn drop(&mut self) {
    self.displayer.abort();
  }
}

fn document(saved_at: f64, screen: &str) -> Value {
  let mut document = bmo_face_core::default::document().to_json();
  document["savedAt"] = json!(saved_at);
  document["colors"]["screen"] = json!(screen);
  document
}

fn epoch_ms() -> i64 {
  SystemTime::now().duration_since(UNIX_EPOCH).expect("after 1970").as_millis() as i64
}

/// A summary the engine spoke for a room and ended done at `finished_at`, as `GET /briefings/{briefingId}`
/// answers it.
fn summary(id: i64, chat_id: i64, finished_at: i64) -> Value {
  json!({
    "id": id, "chatId": chat_id, "messageId": 900 + id, "kind": "summary", "title": "a title",
    "voiceName": "voice-a", "text": "a line", "emotion": null, "synthesisId": 70 + id, "state": "done",
    "durationMs": 1_000, "plays": true, "awaitsLine": false, "voiceMessageId": null,
    "startedAt": finished_at - 5_000, "finishedAt": finished_at
  })
}

impl Bench {
  async fn start(setup: Setup) -> Bench {
    let (saves, _) = broadcast::channel(16);
    let (heard, _) = broadcast::channel(64);
    let stand_in = StandIn {
      document: Arc::new(Mutex::new(document(1.0, "#102030"))),
      saves,
      heard,
      gone: Arc::new(AtomicBool::new(false)),
      asked: Arc::new(Mutex::new(Vec::new())),
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
    let address = listener.local_addr().expect("its address");
    let app = Router::new().fallback(answer).with_state(stand_in.clone());
    let server = tokio::spawn(async move {
      let _ = axum::serve(listener, app).await;
    });
    let scratch = tempfile::tempdir().expect("a scratch folder");
    let frames = scratch.path().join("frames");
    std::fs::create_dir_all(&frames).expect("the frames' folder");
    let token_file = scratch.path().join("voice.token");
    if let Some(token) = setup.token {
      std::fs::write(&token_file, format!("{token}\n")).expect("the token written");
    }
    let mut settings = vec![
      ("BMO_FACE_URL", format!("http://{address}/bmo-face")),
      ("BMO_FACE_DISPLAYER_DATA", scratch.path().join("data").display().to_string()),
      ("BMO_FACE_DISPLAYER_SOUND", "off".to_owned()),
      ("BMO_FACE_DISPLAYER_GAZE", scratch.path().join("gaze.sock").display().to_string()),
      ("BMO_FACE_DISPLAYER_RETRY_MS", RETRY_MS.to_string()),
    ];
    if setup.voice {
      settings.push(("BMO_FACE_DISPLAYER_VOICE", format!("http://{address}/engine/")));
      settings.push(("BMO_FACE_DISPLAYER_VOICE_TOKEN_FILE", token_file.display().to_string()));
    }
    if let Some(rooms) = setup.rooms {
      settings.push(("BMO_FACE_DISPLAYER_ROOMS", rooms.to_owned()));
    }
    let config = Config::from_lookup(|name| {
      settings.iter().find(|(key, _)| *key == name).map(|(_, value)| value.clone())
    })
    .expect("settings");
    let output = Output::Frames(frames.clone());
    let displayer = tokio::spawn(async move {
      let _ = display(config, output, std::future::pending()).await;
    });
    Bench {
      stand_in,
      frames,
      token_file,
      _scratch: scratch,
      displayer,
      _server: server,
    }
  }

  fn asked(&self, starting: &str) -> Vec<Asked> {
    self
      .stand_in
      .asked
      .lock()
      .expect("the log")
      .iter()
      .filter(|asked| asked.path.starts_with(starting))
      .cloned()
      .collect()
  }

  fn sound_asked(&self, briefing: i64) -> Vec<Asked> {
    self.asked(&format!("/engine/briefings/{briefing}/sound"))
  }

  /// One of the engine's events, numbered `id`, fed to every open stream.
  fn hear(&self, id: i64, kind: &str, data: Value) {
    let event = json!({ "id": id, "kind": kind, "at": epoch_ms(), "data": data });
    let _ = self.stand_in.heard.send(Some(format!("id: {id}\nevent: {kind}\ndata: {event}\n\n")));
  }

  /// Ends every open engine stream after what was fed to it.
  fn cut(&self) {
    let _ = self.stand_in.heard.send(None);
  }

  async fn stream_opened(&self) {
    until("the engine's stream opened", || self.stand_in.heard.receiver_count() > 0).await;
  }

  /// The frames written so far, in order.
  fn frames(&self) -> Vec<PathBuf> {
    let mut frames: Vec<PathBuf> = std::fs::read_dir(&self.frames)
      .expect("the frames' folder")
      .filter_map(Result::ok)
      .map(|entry| entry.path())
      .filter(|path| path.extension().is_some_and(|extension| extension == "png"))
      .collect();
    frames.sort();
    frames
  }

  fn screen_drawn(&self) -> Option<String> {
    self.frames().last().and_then(|frame| screen_of(frame))
  }
}

/// Waits until a condition holds, and fails the test if it does not within the patience.
async fn until(what: &str, mut holds: impl FnMut() -> bool) {
  let began = Instant::now();
  while !holds() {
    assert!(began.elapsed() < PATIENCE, "never: {what}");
    tokio::time::sleep(Duration::from_millis(10)).await;
  }
}

/// The colour at a frame's top left corner: the screen's, which fills the panel whole.
fn screen_of(frame: &Path) -> Option<String> {
  let pixmap = tiny_skia::Pixmap::load_png(frame).ok()?;
  let pixel = pixmap.pixel(0, 0)?;
  Some(format!("#{:02x}{:02x}{:02x}", pixel.red(), pixel.green(), pixel.blue()))
}

#[tokio::test]
async fn the_document_is_drawn_and_a_save_heard_is_drawn_at_once() {
  let bench = Bench::start(HEARING).await;
  until("the served document drawn", || bench.screen_drawn().as_deref() == Some("#102030")).await;
  *bench.stand_in.document.lock().expect("the document") = document(2.0, "#405060");
  let _ = bench.stand_in.saves.send(2.0);
  until("the saved document drawn", || bench.screen_drawn().as_deref() == Some("#405060")).await;
}

#[tokio::test]
async fn a_fresh_summary_is_fetched_and_spoken_and_a_late_one_is_left() {
  let bench = Bench::start(HEARING).await;
  bench.stream_opened().await;
  until("a frame drawn", || !bench.frames().is_empty()).await;
  bench.hear(1, "briefing.ended", summary(1, 3, epoch_ms() - 10 * 60_000));
  bench.hear(2, "briefing.ended", summary(2, 3, epoch_ms()));
  until("the fresh briefing's sound fetched", || !bench.sound_asked(2).is_empty()).await;
  assert!(bench.sound_asked(1).is_empty(), "the late briefing is not played");
  assert_eq!(bench.sound_asked(2)[0].token.as_deref(), Some(TOKEN));
  assert_eq!(bench.asked(STREAM)[0].token.as_deref(), Some(TOKEN));
  // Spoken: the mouth moves frame after frame over the second of sound, far faster than a blink.
  let before = bench.frames().len();
  until("the mouth moving", || bench.frames().len() >= before + 4).await;
  // Heard again, it is not played twice.
  bench.hear(3, "briefing.ended", summary(2, 3, epoch_ms()));
  bench.hear(4, "briefing.ended", summary(4, 3, epoch_ms()));
  until("the next briefing's sound fetched", || !bench.sound_asked(4).is_empty()).await;
  assert_eq!(bench.sound_asked(2).len(), 1);
}

#[tokio::test]
async fn a_stop_silences_the_room_speaking_and_what_waits_behind() {
  let bench = Bench::start(HEARING).await;
  bench.stream_opened().await;
  let now = epoch_ms();
  bench.hear(1, "briefing.ended", summary(5, 3, now));
  bench.hear(2, "briefing.ended", summary(6, 3, now));
  bench.hear(3, "briefings.stopped", json!({ "chatId": 3, "briefingIds": [] }));
  // A briefing of the room that ends after the stop is spoken; the one that waited never is.
  bench.hear(4, "briefing.ended", summary(7, 3, now + 60_000));
  until("the briefing after the stop fetched", || !bench.sound_asked(7).is_empty()).await;
  assert!(bench.sound_asked(6).is_empty(), "the one waiting behind went with the stop");
}

#[tokio::test]
async fn a_cut_stream_is_opened_again_after_its_last_event_and_from_the_live_ones_once_gone() {
  let bench = Bench::start(HEARING).await;
  bench.stream_opened().await;
  bench.hear(7, "briefings.stopped", json!({ "chatId": 99, "briefingIds": [] }));
  bench.cut();
  until("the stream opened again", || bench.asked(STREAM).len() >= 2).await;
  let opened = bench.asked(STREAM);
  assert_eq!(opened[0].path, format!("{STREAM}{FIRST_QUERY}"));
  assert_eq!(opened[1].path, format!("{STREAM}{FIRST_QUERY}&after=7"));

  // The engine no longer keeping the events past the cursor, the stream reads on from the live ones.
  bench.stream_opened().await;
  bench.hear(9, "briefings.stopped", json!({ "chatId": 99, "briefingIds": [] }));
  bench.stand_in.gone.store(true, Ordering::Relaxed);
  bench.cut();
  until("the stream opened from the live ones", || bench.asked(STREAM).len() >= 4).await;
  let opened = bench.asked(STREAM);
  assert_eq!(opened[2].path, format!("{STREAM}{FIRST_QUERY}&after=9"), "answered 410");
  assert_eq!(opened[3].path, format!("{STREAM}{FIRST_QUERY}"));
}

#[tokio::test]
async fn a_refused_token_is_tried_again_with_growing_waits_and_the_displayer_runs_on() {
  let bench = Bench::start(Setup {
    token: Some(REVOKED),
    ..HEARING
  })
  .await;
  until("the stream asked five times", || bench.asked(STREAM).len() >= 5).await;
  let tries: Vec<Instant> = bench.asked(STREAM).iter().map(|asked| asked.at).collect();
  for (step, pair) in tries.windows(2).take(4).enumerate() {
    let waited = pair[1] - pair[0];
    assert!(waited >= Duration::from_millis(RETRY_MS << step), "try {step} waited {waited:?}");
  }
  assert!(!bench.displayer.is_finished(), "a refusal never ends the displayer");
  assert_eq!(
    std::fs::read_to_string(&bench.token_file).expect("the token file").trim(),
    REVOKED,
    "the engine's token file is never touched"
  );
  // The token file holding the engine's token again, the stream opens with it.
  std::fs::write(&bench.token_file, TOKEN).expect("the token written");
  until("the stream opened with the token", || {
    bench.asked(STREAM).iter().any(|asked| asked.token.as_deref() == Some(TOKEN))
  })
  .await;
}

#[tokio::test]
async fn the_rooms_named_alone_are_spoken() {
  let bench = Bench::start(Setup {
    rooms: Some("3, 5"),
    ..HEARING
  })
  .await;
  bench.stream_opened().await;
  bench.hear(1, "briefing.ended", summary(8, 4, epoch_ms()));
  bench.hear(2, "briefing.ended", summary(9, 3, epoch_ms()));
  until("the named room's briefing fetched", || !bench.sound_asked(9).is_empty()).await;
  assert!(bench.sound_asked(8).is_empty(), "another room's briefing is not spoken");
}

#[tokio::test]
async fn nothing_is_asked_of_the_engine_while_none_is_named() {
  let bench = Bench::start(Setup {
    voice: false,
    ..HEARING
  })
  .await;
  until("the served document drawn", || bench.screen_drawn().as_deref() == Some("#102030")).await;
  tokio::time::sleep(Duration::from_millis(20 * RETRY_MS)).await;
  assert!(bench.asked("/engine").is_empty());
  assert!(!bench.displayer.is_finished());
}

#[tokio::test]
async fn the_gaze_socket_takes_lines_from_its_own_user_alone() {
  let folder = tempfile::tempdir().expect("a scratch folder");
  let path = folder.path().join("gaze.sock");
  let listener = gaze::listen(&path).expect("listening");
  {
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(std::fs::metadata(&path).expect("the socket").permissions().mode() & 0o777, 0o600);
  }
  let (tell, mut points) = mpsc::unbounded_channel();
  let serving = tokio::spawn(gaze::serve(listener, tell));
  let mut sender = UnixStream::connect(&path).await.expect("connected");
  sender
    .write_all(b"{\"x\": 0.25, \"y\": -1}\nnot a direction\n{\"x\": 2, \"y\": 0}\n")
    .await
    .expect("written");
  let first = tokio::time::timeout(PATIENCE, points.recv()).await.expect("heard in time");
  let second = tokio::time::timeout(PATIENCE, points.recv()).await.expect("heard in time");
  assert_eq!((first, second), (Some([0.25, -1.0]), Some([2.0, 0.0])));
  serving.abort();
  // A socket left by an earlier run is taken away and listened on again.
  drop(gaze::listen(&path).expect("listening again"));
}
