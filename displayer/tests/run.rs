//! The whole displayer against a stand-in face server (docs/face.md, 「What it hears」): run in-process
//! with its drawings written as frames into a scratch folder, its waits shortened, over a stand-in that
//! serves the document and its saves, BMO's listener stream as the test feeds it, and a briefing's
//! sound, and writes down what it was asked. Every wait is a bounded wait on what it waits for.

use std::path::{Path, PathBuf};
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
use bmo_face_displayer::server::{FaceServer, Fault};
use bmo_face_displayer::token::TokenFile;
use futures_util::Stream;
use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, UnixStream};
use tokio::sync::{broadcast, mpsc};

const SOUND: &[u8] = include_bytes!("fixtures/tone-then-silence.ogg");
const TOKEN: &str = "device-token";
const REVOKED: &str = "revoked-token";
/// The longest any condition is waited for.
const PATIENCE: Duration = Duration::from_secs(20);

/// What the stand-in was asked: the path with its query, and the bearer token.
#[derive(Clone, Debug)]
struct Asked {
  path: String,
  token: Option<String>,
}

#[derive(Clone)]
struct StandIn {
  document: Arc<Mutex<Value>>,
  saves: broadcast::Sender<f64>,
  /// Raw frames for every open listener stream, after its own run frame.
  heard: broadcast::Sender<String>,
  /// Listener streams to cut after their run frame and the frames already fed, one each.
  cut: Arc<Mutex<u32>>,
  asked: Arc<Mutex<Vec<Asked>>>,
}

/// An event stream's body: its first frame at once, then every frame fed to it, or nothing more where
/// it is cut after the first.
fn frames(
  first: String,
  later: broadcast::Receiver<String>,
  cut: bool,
) -> impl Stream<Item = Result<Bytes, std::io::Error>> {
  futures_util::stream::unfold((Some(first), later), move |(first, mut later)| async move {
    if let Some(first) = first {
      return Some((Ok(Bytes::from(first)), (None, later)));
    }
    if cut {
      return None;
    }
    loop {
      match later.recv().await {
        Ok(frame) => return Some((Ok(Bytes::from(frame)), (None, later))),
        Err(broadcast::error::RecvError::Lagged(_)) => continue,
        Err(broadcast::error::RecvError::Closed) => return None,
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

async fn answer(State(stand_in): State<StandIn>, request: Request) -> Response {
  let path = request.uri().path_and_query().map_or(String::new(), |path| path.as_str().to_owned());
  let token = bearer(request.headers());
  stand_in.asked.lock().expect("the log").push(Asked {
    path: path.clone(),
    token: token.clone(),
  });
  let route = path.split('?').next().unwrap_or_default().to_owned();
  if route == "/bmo-face/bmo/api/app/login" {
    return login(request).await;
  }
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
          if tx.send(format!("event: faces\ndata: {}\n\n", json!({ "savedAt": version }))).is_err() {
            return;
          }
        }
      });
      event_stream(frames(format!("event: faces\ndata: {}\n\n", json!({ "savedAt": first })), rx, false))
    }
    "/bmo-face/bmo/api/briefing/events" => {
      if token.as_deref() != Some(TOKEN) {
        return StatusCode::UNAUTHORIZED.into_response();
      }
      let cut = {
        let mut cut = stand_in.cut.lock().expect("the cuts");
        let now = *cut > 0;
        *cut = cut.saturating_sub(1);
        now
      };
      let first = "id: 5\nevent: run\ndata: {\"run\":\"run-a\"}\n\n".to_owned();
      event_stream(frames(first, stand_in.heard.subscribe(), cut))
    }
    _ if route.starts_with("/bmo-face/bmo/api/media/audio/") => {
      if token.as_deref() != Some(TOKEN) {
        return StatusCode::UNAUTHORIZED.into_response();
      }
      ([("content-type", "audio/ogg")], SOUND).into_response()
    }
    _ => StatusCode::NOT_FOUND.into_response(),
  }
}

/// BMO's app login through the pass-through: refused unless the write names the face server's own site,
/// as the server refuses it, and unless the account is the stand-in's.
async fn login(request: Request) -> Response {
  let host = request.headers().get("host").and_then(|host| host.to_str().ok()).map(str::to_owned);
  let origin = request
    .headers()
    .get("origin")
    .and_then(|origin| origin.to_str().ok())
    .map(str::to_owned);
  if host.is_none() || origin != host.map(|host| format!("http://{host}")) {
    return StatusCode::FORBIDDEN.into_response();
  }
  let body = axum::body::to_bytes(request.into_body(), 4_096).await.unwrap_or_default();
  let asked: Value = serde_json::from_slice(&body).unwrap_or_default();
  if asked == json!({ "name": "tester", "password": "bench-pass", "device": "bench-panel" }) {
    axum::Json(json!({ "token": TOKEN })).into_response()
  } else {
    StatusCode::UNAUTHORIZED.into_response()
  }
}

struct Bench {
  stand_in: StandIn,
  base: String,
  data: tempfile::TempDir,
  frames: PathBuf,
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

fn epoch_ms() -> f64 {
  SystemTime::now().duration_since(UNIX_EPOCH).expect("after 1970").as_secs_f64() * 1_000.0
}

impl Bench {
  async fn start(token: Option<&str>, cuts: u32) -> Bench {
    let (saves, _) = broadcast::channel(16);
    let (heard, _) = broadcast::channel(64);
    let stand_in = StandIn {
      document: Arc::new(Mutex::new(document(1.0, "#102030"))),
      saves,
      heard,
      cut: Arc::new(Mutex::new(cuts)),
      asked: Arc::new(Mutex::new(Vec::new())),
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
    let address = listener.local_addr().expect("its address");
    let app = Router::new().fallback(answer).with_state(stand_in.clone());
    let server = tokio::spawn(async move {
      let _ = axum::serve(listener, app).await;
    });
    let data = tempfile::tempdir().expect("a scratch data root");
    if let Some(token) = token {
      TokenFile::new(data.path()).write(token).expect("the token kept");
    }
    let scratch = tempfile::tempdir().expect("a scratch folder");
    let frames = scratch.path().join("frames");
    std::fs::create_dir_all(&frames).expect("the frames' folder");
    let gaze = scratch.path().join("gaze.sock");
    let settings = [
      ("BMO_FACE_URL", format!("http://{address}/bmo-face")),
      ("BMO_FACE_DISPLAYER_DATA", data.path().display().to_string()),
      ("BMO_FACE_DISPLAYER_SOUND", "off".to_owned()),
      ("BMO_FACE_DISPLAYER_GAZE", gaze.display().to_string()),
      ("BMO_FACE_DISPLAYER_RETRY_MS", "20".to_owned()),
    ];
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
      base: format!("http://{address}/bmo-face"),
      data,
      frames,
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

  fn hear(&self, event: &str, data: Value) {
    let _ = self.stand_in.heard.send(format!("event: {event}\ndata: {data}\n\n"));
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
  let bench = Bench::start(Some(TOKEN), 0).await;
  until("the served document drawn", || {
    bench.frames().last().and_then(|frame| screen_of(frame)).as_deref() == Some("#102030")
  })
  .await;
  *bench.stand_in.document.lock().expect("the document") = document(2.0, "#405060");
  let _ = bench.stand_in.saves.send(2.0);
  until("the saved document drawn", || {
    bench.frames().last().and_then(|frame| screen_of(frame)).as_deref() == Some("#405060")
  })
  .await;
}

#[tokio::test]
async fn a_fresh_briefing_is_fetched_and_spoken_and_a_late_one_is_left() {
  let bench = Bench::start(Some(TOKEN), 0).await;
  until("the listener stream opened with the token", || {
    !bench.asked("/bmo-face/bmo/api/briefing/events").is_empty()
  })
  .await;
  until("a frame drawn", || !bench.frames().is_empty()).await;
  let briefing = |message: i64, sent_at: f64| json!({ "type": "briefing", "chatId": 3, "messageId": message, "briefing": { "sentAt": sent_at, "emotion": "unknown" } });
  bench.hear("bot", briefing(1, epoch_ms() - 10.0 * 60_000.0));
  bench.hear("bot", briefing(2, epoch_ms()));
  until("the fresh briefing's sound fetched", || {
    !bench.asked("/bmo-face/bmo/api/media/audio/3/2").is_empty()
  })
  .await;
  assert!(
    bench.asked("/bmo-face/bmo/api/media/audio/3/1").is_empty(),
    "the late briefing is not played"
  );
  assert_eq!(bench.asked("/bmo-face/bmo/api/media/audio/3/2")[0].token.as_deref(), Some(TOKEN));
  // Spoken: the mouth moves frame after frame over the second of sound, far faster than a blink.
  let before = bench.frames().len();
  until("the mouth moving", || bench.frames().len() >= before + 4).await;
  // Heard again, it is not played twice.
  bench.hear("bot", briefing(2, epoch_ms()));
  bench.hear("presence", json!({ "state": "thinking", "since": epoch_ms() }));
  until("the presence drawn after the briefing", || bench.frames().len() >= before + 6).await;
  assert_eq!(bench.asked("/bmo-face/bmo/api/media/audio/3/2").len(), 1);
}

#[tokio::test]
async fn a_refused_token_is_forgotten() {
  let bench = Bench::start(Some(REVOKED), 0).await;
  let token = TokenFile::new(bench.data.path());
  until("the refused token forgotten", || token.read().is_ok_and(|kept| kept.is_none())).await;
  // Signed in again, the stream opens with the new token.
  token.write(TOKEN).expect("a new token kept");
  until("the stream opened with the new token", || {
    bench
      .asked("/bmo-face/bmo/api/briefing/events")
      .iter()
      .any(|asked| asked.token.as_deref() == Some(TOKEN))
  })
  .await;
}

#[tokio::test]
async fn a_dropped_stream_is_opened_again_from_its_cursor() {
  let bench = Bench::start(Some(TOKEN), 1).await;
  until("the stream opened twice", || {
    bench.asked("/bmo-face/bmo/api/briefing/events").len() >= 2
  })
  .await;
  let opened = bench.asked("/bmo-face/bmo/api/briefing/events");
  assert_eq!(opened[0].path, "/bmo-face/bmo/api/briefing/events?after=0");
  assert_eq!(opened[1].path, "/bmo-face/bmo/api/briefing/events?after=5&run=run%2Da");
}

#[tokio::test]
async fn signing_in_writes_from_the_servers_own_site_and_answers_the_token() {
  let bench = Bench::start(None, 0).await;
  let server = FaceServer::new(&bench.base);
  assert_eq!(server.sign_in("tester", "bench-pass", "bench-panel").await.ok().as_deref(), Some(TOKEN));
  assert!(matches!(
    server.sign_in("tester", "wrong", "bench-panel").await,
    Err(Fault::Refused(StatusCode::UNAUTHORIZED))
  ));
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
