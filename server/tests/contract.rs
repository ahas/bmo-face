//! The face server's wire contract (docs/face.md, 「The server」 and 「The console screen」), asked of the
//! command under test as a display, the editor, a console and a stranger ask it: the server started as
//! a process on a free port over a scratch data root, a scratch screen, a scratch console bundle, a
//! scratch archive holding one BMO account, and a stand-in for BMO's console that writes down what it
//! was asked. The command is the package's binary, or the one
//! `FACE_CONTRACT_COMMAND` names.

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use argon2::password_hash::{PasswordHasher, SaltString};
use axum::Router;
use axum::body::Bytes;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use futures_util::StreamExt;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::Frame;
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use serde_json::{Value, json};
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection};
use sqlx::{Connection, Executor};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;

const HOST: &str = "face.test";
const ORIGIN: &str = "http://face.test";
const DEFAULT: &str = include_str!("../../tests/fixtures/default-faces.json");
const FIXED: &str = include_str!("../../console-manifest.json");
/// The credential the scratch console calls with, as the server is given it.
const TOKEN: &str = "console-token";
const BEARER: &str = "Bearer console-token";

/// One request BMO's stand-in console was sent.
#[derive(Clone, Debug)]
struct Asked {
  method: String,
  target: String,
  headers: HeaderMap,
}

type Log = Arc<Mutex<Vec<Asked>>>;

// BMO's console, standing in: a refusal with a body naming an address inside the machine for a bearer
// that is gone, and otherwise an answer carrying a cookie the server must not pass on.
async fn console(State(log): State<Log>, request: Request) -> Response {
  let (parts, _) = request.into_parts();
  log.lock().unwrap().push(Asked {
    method: parts.method.to_string(),
    target: parts.uri.to_string(),
    headers: parts.headers.clone(),
  });
  if parts.headers.get("authorization").is_some_and(|value| value == "Bearer gone") {
    let said = json!({ "url": "http://127.0.0.1:3210/bmo/api/briefing/events", "statusCode": 401 });
    return (StatusCode::UNAUTHORIZED, said.to_string()).into_response();
  }
  if parts.uri.path().contains("/media/audio/") {
    return (
      StatusCode::PARTIAL_CONTENT,
      [
        ("content-type", "audio/ogg"),
        ("content-range", "bytes 0-3/4"),
        ("accept-ranges", "bytes"),
      ],
      "OggS",
    )
      .into_response();
  }
  (
    StatusCode::OK,
    [("content-type", "text/event-stream"), ("set-cookie", "bmo=1")],
    "event: keep\ndata: {}\n\n",
  )
    .into_response()
}

/// When BMO's stand-in for a drain has been asked for a sound, and when it may answer.
#[derive(Clone, Default)]
struct Held {
  asked: Arc<Notify>,
  release: Arc<Notify>,
}

// BMO's console, standing in for a drain: a listener stream with no end, and a sound answered only once
// the test releases it.
async fn held_console(State(held): State<Held>, request: Request) -> Response {
  if request.uri().path().contains("/media/audio/") {
    held.asked.notify_one();
    held.release.notified().await;
    return (StatusCode::OK, [("content-type", "audio/ogg")], "OggS").into_response();
  }
  let endless =
    futures_util::stream::once(async { Ok::<_, std::convert::Infallible>("event: keep\ndata: {}\n\n") })
      .chain(futures_util::stream::pending());
  (
    StatusCode::OK,
    [("content-type", "text/event-stream")],
    axum::body::Body::from_stream(endless),
  )
    .into_response()
}

// `app` answering on a free port, as BMO's console with its base path.
async fn served(app: Router) -> String {
  let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
  let at = listener.local_addr().unwrap();
  tokio::spawn(async move { axum::serve(listener, app).await });
  format!("http://{at}/bmo")
}

fn command() -> Command {
  match std::env::var("FACE_CONTRACT_COMMAND") {
    Ok(named) if !named.trim().is_empty() => {
      let mut words = named.split_whitespace();
      let mut command = Command::new(words.next().unwrap());
      command.args(words);
      command
    }
    _ => Command::new(env!("CARGO_BIN_EXE_bmo-face")),
  }
}

struct Face {
  dir: tempfile::TempDir,
  server: Option<Child>,
  port: u16,
  console: Log,
  console_url: String,
  /// The console's credential the server is started with; none for a server given no token.
  token: Option<&'static str>,
  /// The stop's deadline in milliseconds the server is started with; none for its own.
  drain_ms: Option<&'static str>,
  // The port of a console that is not there, held by a socket bound and never listening, so a
  // connection to it is refused and no other process takes it.
  _absent: Option<tokio::net::TcpSocket>,
}

impl Face {
  /// A scratch world, the stored document given in place, and BMO's console standing in or absent.
  async fn new(stored: Option<&str>, console_stands: bool) -> Face {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("data")).unwrap();
    std::fs::create_dir_all(root.join("public/_nuxt/builds")).unwrap();
    std::fs::write(root.join("public/index.html"), "<!doctype html><title>face</title>").unwrap();
    std::fs::write(root.join("public/_nuxt/app.js"), "export {}").unwrap();
    std::fs::write(root.join("public/_nuxt/builds/latest.json"), r#"{"id":"build-1"}"#).unwrap();
    std::fs::write(root.join("secret.txt"), "outside the screen").unwrap();
    if let Some(stored) = stored {
      std::fs::write(root.join("data/faces.json"), stored).unwrap();
    }
    std::fs::create_dir_all(root.join("console")).unwrap();
    std::fs::write(root.join("console/bundle.js"), "export default { routes: [] };\n").unwrap();
    std::fs::write(root.join("console/bundle.css"), ".face-test { color: red }\n").unwrap();
    std::fs::write(root.join("console/words.json"), r#"{"ko":{"face":{"title":"face"}}}"#).unwrap();
    std::fs::write(root.join("console/layer.json"), json!({ "layer": vendored_layer() }).to_string())
      .unwrap();

    let hash = argon2::Argon2::default()
      .hash_password(b"secret", &SaltString::from_b64("c2NyYXRjaHNhbHQxMjM0").unwrap())
      .unwrap()
      .to_string();
    let options = SqliteConnectOptions::new().filename(root.join("archive.db")).create_if_missing(true);
    let mut archive = SqliteConnection::connect_with(&options).await.unwrap();
    archive
      .execute("CREATE TABLE web_users (name TEXT PRIMARY KEY, password_hash TEXT NOT NULL)")
      .await
      .unwrap();
    sqlx::query("INSERT INTO web_users VALUES ('owner', ?)")
      .bind(hash)
      .execute(&mut archive)
      .await
      .unwrap();
    archive.close().await.unwrap();

    let log: Log = Arc::default();
    let (console_url, absent) = if console_stands {
      (served(Router::new().fallback(console).with_state(Arc::clone(&log))).await, None)
    } else {
      let held = tokio::net::TcpSocket::new_v4().unwrap();
      held.bind("127.0.0.1:0".parse().unwrap()).unwrap();
      (format!("http://{}/bmo", held.local_addr().unwrap()), Some(held))
    };
    Face {
      dir,
      server: None,
      port: 0,
      console: log,
      console_url,
      token: Some(TOKEN),
      drain_ms: None,
      _absent: absent,
    }
  }

  /// A scratch world beside the stand-in for a drain, which the returned `Held` steers.
  async fn beside_held() -> (Face, Held) {
    let held = Held::default();
    let mut face = Face::new(None, false).await;
    face.console_url = served(Router::new().fallback(held_console).with_state(held.clone())).await;
    face._absent = None;
    (face, held)
  }

  fn root(&self) -> &Path {
    self.dir.path()
  }

  fn faces_file(&self) -> PathBuf {
    self.root().join("data/faces.json")
  }

  async fn start(mut self) -> Face {
    self.spawn();
    // Ready when it names its port, and never ready once it has ended without naming one.
    self.port = loop {
      let heard = self.said().lines().find_map(|line| {
        line.strip_prefix("BMO face: http://127.0.0.1:")?.split('/').next()?.parse::<u16>().ok()
      });
      if let Some(port) = heard {
        break port;
      }
      let ended = self.server.as_mut().and_then(|server| server.try_wait().ok().flatten());
      assert!(
        ended.is_none(),
        "the server ended ({ended:?}) before it listened; it said {}",
        self.said()
      );
      tokio::time::sleep(Duration::from_millis(20)).await;
    };
    self
  }

  /// Started where it must refuse to start: what it said once it ended, failing.
  async fn refused(mut self) -> String {
    self.spawn();
    let status = loop {
      if let Some(status) = self.server.as_mut().and_then(|server| server.try_wait().ok().flatten()) {
        break status;
      }
      assert!(
        !self.said().contains("BMO face: http://"),
        "the server started; it said {}",
        self.said()
      );
      tokio::time::sleep(Duration::from_millis(20)).await;
    };
    assert!(!status.success(), "the server ended {status}; it said {}", self.said());
    self.said()
  }

  fn spawn(&mut self) {
    let log = std::fs::File::create(self.root().join("server.log")).unwrap();
    let mut command = command();
    match self.token {
      Some(token) => command.env("BMO_FACE_TOKEN", token),
      None => command.env_remove("BMO_FACE_TOKEN"),
    };
    match self.drain_ms {
      Some(ms) => command.env("BMO_FACE_DRAIN_MS", ms),
      None => command.env_remove("BMO_FACE_DRAIN_MS"),
    };
    self.server = Some(
      command
        .env("BMO_FACE_DATA", self.root().join("data"))
        // A port of its own choosing, which it names as it starts answering, so no other process
        // can take the port between its choosing and its binding; started again, the port it had.
        .env("BMO_FACE_PORT", self.port.to_string())
        .env("BMO_FACE_STATIC", self.root().join("public"))
        .env("BMO_FACE_CONSOLE", self.root().join("console"))
        .env("BMO_DB_PATH", self.root().join("archive.db"))
        .env("BMO_CONSOLE_URL", &self.console_url)
        .env("BMO_FACE_KEEP_MS", "200")
        .env_remove("BMO_ARCHIVE_URL")
        .stdin(Stdio::null())
        .stdout(log.try_clone().unwrap())
        .stderr(log)
        .process_group(0)
        .spawn()
        .expect("the server starts"),
    );
  }

  fn said(&self) -> String {
    std::fs::read_to_string(self.root().join("server.log")).unwrap_or_default()
  }

  /// SIGTERM to the server's process group, as the unit's stop sends it.
  fn terminate(&self) {
    let server = self.server.as_ref().expect("a started server");
    unsafe {
      libc::kill(-(server.id() as i32), libc::SIGTERM);
    }
  }

  /// The server's end, waited on for as long as a drain at its own deadline can take.
  async fn ended(&mut self) -> std::process::ExitStatus {
    let server = self.server.as_mut().expect("a started server");
    for _ in 0..1_000 {
      if let Some(status) = server.try_wait().unwrap() {
        return status;
      }
      tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the server had not ended 20 seconds after its stop; it said {}", self.said());
  }

  /// Whether a new connection to the server is still taken.
  async fn takes_connections(&self) -> bool {
    TcpStream::connect(("127.0.0.1", self.port)).await.is_ok()
  }

  fn asked(&self) -> Vec<Asked> {
    self.console.lock().unwrap().clone()
  }

  async fn open(
    &self,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Bytes,
  ) -> hyper::Response<Incoming> {
    self.try_open(method, path, headers, body).await.unwrap()
  }

  /// A request sent on a connection of its own, and its answer or the failure that ended it.
  async fn try_open(
    &self,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Bytes,
  ) -> hyper::Result<hyper::Response<Incoming>> {
    let stream = TcpStream::connect(("127.0.0.1", self.port)).await.unwrap();
    let (mut sender, connection) =
      hyper::client::conn::http1::handshake(TokioIo::new(stream)).await.unwrap();
    tokio::spawn(connection);
    let mut request = hyper::Request::builder().method(method).uri(path).header("host", HOST);
    for (name, value) in headers {
      request = request.header(*name, *value);
    }
    sender.send_request(request.body(Full::new(body)).unwrap()).await
  }

  async fn call(
    &self,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
  ) -> (u16, HeaderMap, String) {
    let answer = self.open(method, path, headers, Bytes::from(body.to_owned())).await;
    let (status, headers) = (answer.status().as_u16(), answer.headers().clone());
    let body = answer.into_body().collect().await.unwrap().to_bytes();
    (status, headers, String::from_utf8_lossy(&body).into_owned())
  }

  async fn get(&self, path: &str) -> (u16, HeaderMap, String) {
    self.call("GET", path, &[], "").await
  }

  /// A write from this site, with the headers given.
  async fn write(
    &self,
    method: &str,
    path: &str,
    body: &Value,
    headers: &[(&str, &str)],
  ) -> (u16, HeaderMap, String) {
    let mut all: Vec<(&str, &str)> = [("origin", ORIGIN), ("content-type", "application/json")]
      .into_iter()
      .filter(|(name, _)| headers.iter().all(|(given, _)| given != name))
      .collect();
    all.extend_from_slice(headers);
    self.call(method, path, &all, &body.to_string()).await
  }

  /// The editor signed in: the session cookie as the browser sends it back.
  async fn login(&self) -> String {
    let (status, headers, _) = self
      .write("POST", "/bmo-face/api/login", &json!({ "name": "owner", "password": "secret" }), &[])
      .await;
    assert_eq!(status, 200);
    headers["set-cookie"].to_str().unwrap().split(';').next().unwrap().to_owned()
  }

  async fn document(&self) -> Value {
    serde_json::from_str(&self.get("/bmo-face/api/faces").await.2).unwrap()
  }

  /// The manifest as a console reads it.
  async fn manifest(&self) -> (u16, Value) {
    let (status, _, body) =
      self.call("GET", "/bmo-face/console-manifest", &[("authorization", BEARER)], "").await;
    (status, serde_json::from_str(&body).unwrap_or(Value::Null))
  }
}

/// The screen's project, where the vendored bmo-ui stands.
fn client() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("../client")
}

/// The bmo-ui release file the screen vendors, as its package manifest names it.
fn vendored_release() -> PathBuf {
  let manifest: Value =
    serde_json::from_str(&std::fs::read_to_string(client().join("package.json")).unwrap()).unwrap();
  let entry = manifest["devDependencies"]["bmo-ui"].as_str().expect("the screen vendors bmo-ui");
  client().join(entry.strip_prefix("file:").expect("bmo-ui is vendored as a file"))
}

/// The layer the vendored release's bundle build names, as the installed release states it.
fn vendored_layer() -> String {
  let layer = client().join("node_modules/bmo-ui/shared/layer.json");
  let stated: Value = serde_json::from_str(&std::fs::read_to_string(&layer).unwrap()).unwrap();
  stated["layer"].as_str().unwrap().to_owned()
}

impl Drop for Face {
  fn drop(&mut self) {
    if let Some(server) = &mut self.server {
      unsafe {
        libc::kill(-(server.id() as i32), libc::SIGKILL);
      }
      let _ = server.wait();
    }
  }
}

fn default_document() -> Value {
  serde_json::from_str(DEFAULT).unwrap()
}

/// The next frame of an event stream, as text.
async fn frame(body: &mut Incoming) -> String {
  let next = body.frame().await.unwrap().unwrap();
  String::from_utf8_lossy(&next.into_data().unwrap()).into_owned()
}

/// An event stream read to its end, which must be a clean one rather than a cut connection.
async fn read_to_end(body: &mut Incoming) {
  while let Some(next) = body.frame().await {
    next.expect("a stream ended cleanly, not cut");
  }
}

#[tokio::test]
async fn the_screen_is_served_for_every_path_that_is_not_the_servers() {
  let face = Face::new(None, true).await.start().await;

  let (status, headers, body) = face.get("/bmo-face/edit").await;
  assert_eq!((status, body.as_str()), (200, "<!doctype html><title>face</title>"));
  assert!(headers["content-type"].to_str().unwrap().contains("text/html"));
  assert_eq!(headers["cache-control"], "no-cache");
  let (status, headers, _) = face.get("/bmo-face/_nuxt/app.js").await;
  assert_eq!(status, 200);
  assert!(headers["cache-control"].to_str().unwrap().contains("immutable"));
  assert_eq!(face.get("/bmo-face/_nuxt/builds/latest.json").await.2, r#"{"id":"build-1"}"#);
  // A path is read as a browser reads it: dots climbing out of the prefix leave it, and a file above
  // the screen is never served.
  assert_eq!(face.get("/bmo-face/%2e%2e/secret.txt").await.0, 404);
  assert_eq!(face.get("/bmo-face/edit/%2e%2e/_nuxt/app.js").await.2, "export {}");
  assert_eq!(face.get("/bmo-face/_nuxt/%2e%2e/%2e%2e/%2e%2e/secret.txt").await.0, 404);

  let (status, headers, _) = face.get("/bmo-face?x=1").await;
  assert_eq!((status, headers["location"].to_str().unwrap()), (308, "/bmo-face/?x=1"));
  assert_eq!(face.get("/elsewhere").await.0, 404);
  let (status, _, body) = face.call("HEAD", "/bmo-face/", &[], "").await;
  assert_eq!((status, body.as_str()), (200, ""));
}

#[tokio::test]
async fn the_document_is_read_openly_and_a_new_data_root_starts_from_the_default() {
  let face = Face::new(None, true).await.start().await;

  let (status, headers, body) = face.get("/bmo-face/api/faces").await;
  assert_eq!(status, 200);
  assert_eq!(headers["cache-control"], "no-store");
  assert_eq!(serde_json::from_str::<Value>(&body).unwrap(), default_document());
  assert_eq!(std::fs::read_to_string(face.faces_file()).unwrap(), DEFAULT);
}

#[tokio::test]
async fn a_read_never_writes_the_document() {
  let face = Face::new(None, true).await.start().await;
  let mut underneath = default_document();
  underneath["colors"]["body"] = json!("#010203");
  let text = underneath.to_string();
  std::fs::write(face.faces_file(), &text).unwrap();

  assert_eq!(face.document().await["colors"]["body"], "#010203");
  assert_eq!(std::fs::read_to_string(face.faces_file()).unwrap(), text);
}

#[tokio::test]
async fn a_save_needs_a_session_this_site_and_the_version_it_was_edited_from() {
  let face = Face::new(None, true).await.start().await;
  let mut edited = default_document();
  edited["colors"]["body"] = json!("#112233");
  let save = json!({ "document": edited, "basedOn": 0 });

  assert_eq!(face.write("PUT", "/bmo-face/api/faces", &save, &[]).await.0, 401);
  let cookie = face.login().await;
  assert_eq!(
    face
      .write(
        "PUT",
        "/bmo-face/api/faces",
        &save,
        &[("cookie", &cookie), ("origin", "http://evil.test")]
      )
      .await
      .0,
    403
  );

  let (status, _, body) = face.write("PUT", "/bmo-face/api/faces", &save, &[("cookie", &cookie)]).await;
  assert_eq!(status, 200);
  let saved: Value = serde_json::from_str(&body).unwrap();
  assert!(saved["savedAt"].as_u64().unwrap() > 0);
  assert_eq!(saved["colors"]["body"], "#112233");
  assert_eq!(face.document().await, saved);

  let (status, _, body) = face.write("PUT", "/bmo-face/api/faces", &save, &[("cookie", &cookie)]).await;
  let stale: Value = serde_json::from_str(&body).unwrap();
  assert_eq!(status, 409);
  assert!(stale["error"].as_str().unwrap().contains("다른 창"));
  assert_eq!(stale["current"]["savedAt"], saved["savedAt"]);

  // A document the check refuses is refused, and nothing is written.
  let mut broken = saved.clone();
  broken["mouths"] = json!([]);
  let (status, _, body) = face
    .write(
      "PUT",
      "/bmo-face/api/faces",
      &json!({ "document": broken, "basedOn": saved["savedAt"] }),
      &[("cookie", &cookie)],
    )
    .await;
  assert_eq!(
    (status, serde_json::from_str::<Value>(&body).unwrap()["error"].as_str().unwrap()),
    (400, "입 모양 단계는 2개에서 8개 사이여야 해요.")
  );
  assert_eq!(face.document().await, saved);

  // Nothing larger than the server takes is read: a body declared past the limit is answered before
  // the rest of it is sent, which here never comes.
  let stream = TcpStream::connect(("127.0.0.1", face.port)).await.unwrap();
  let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream)).await.unwrap();
  tokio::spawn(connection);
  let first = futures_util::stream::iter([Ok::<_, std::convert::Infallible>(Frame::data(
    Bytes::from_static(b"{\"document\":\""),
  ))]);
  let body = StreamBody::new(first.chain(futures_util::stream::pending()));
  let request = hyper::Request::put("/bmo-face/api/faces")
    .header("host", HOST)
    .header("origin", ORIGIN)
    .header("cookie", &cookie)
    .header("content-length", "2500000")
    .body(body)
    .unwrap();
  let answer = sender.send_request(request).await.unwrap();
  assert_eq!(answer.status(), 413);
}

#[tokio::test]
async fn every_open_display_hears_each_save_and_a_keep_frame_between() {
  let face = Face::new(None, true).await.start().await;
  let answer = face.open("GET", "/bmo-face/api/faces/events", &[], Bytes::new()).await;
  assert_eq!(answer.headers()["content-type"], "text/event-stream");
  let mut stream = answer.into_body();
  assert_eq!(frame(&mut stream).await, "event: faces\ndata: {\"savedAt\":0}\n\n");

  let cookie = face.login().await;
  let (_, _, body) = face
    .write(
      "PUT",
      "/bmo-face/api/faces",
      &json!({ "document": default_document(), "basedOn": 0 }),
      &[("cookie", &cookie)],
    )
    .await;
  let saved: Value = serde_json::from_str(&body).unwrap();
  let mut heard = Vec::new();
  while !heard.iter().any(|one: &String| one.starts_with("event: faces"))
    || !heard.iter().any(|one| one.starts_with("event: keep"))
  {
    heard.push(frame(&mut stream).await);
  }
  assert!(
    heard.contains(&format!("event: faces\ndata: {}\n\n", json!({ "savedAt": saved["savedAt"] }))),
    "{heard:?}"
  );
  assert!(heard.contains(&"event: keep\ndata: {}\n\n".to_owned()));
}

#[tokio::test]
async fn the_editor_logs_in_against_bmos_accounts_with_a_session_of_this_servers_own() {
  let face = Face::new(None, true).await.start().await;

  assert_eq!(
    face
      .write("POST", "/bmo-face/api/login", &json!({ "name": "owner", "password": "wrong" }), &[])
      .await
      .0,
    401
  );
  assert_eq!(face.write("POST", "/bmo-face/api/login", &json!({ "name": 7 }), &[]).await.0, 400);
  assert_eq!(face.get("/bmo-face/api/me").await.0, 401);

  let (_, headers, _) = face
    .write(
      "POST",
      "/bmo-face/api/login",
      &json!({ "name": "owner", "password": "secret" }),
      &[("x-forwarded-proto", "https")],
    )
    .await;
  let set = headers["set-cookie"].to_str().unwrap();
  assert!(set.starts_with("bmo_face="));
  assert!(
    set.ends_with("; Path=/bmo-face; HttpOnly; SameSite=Strict; Max-Age=604800; Secure"),
    "{set}"
  );

  let cookie = face.login().await;
  let (status, _, body) = face.call("GET", "/bmo-face/api/me", &[("cookie", &cookie)], "").await;
  assert_eq!(
    (status, serde_json::from_str::<Value>(&body).unwrap()),
    (200, json!({ "name": "owner" }))
  );
  let (status, headers, _) =
    face.write("POST", "/bmo-face/api/logout", &json!({}), &[("cookie", &cookie)]).await;
  assert_eq!(status, 204);
  assert_eq!(headers["set-cookie"], "bmo_face=; Path=/bmo-face; HttpOnly; SameSite=Strict; Max-Age=0");
  assert_eq!(face.call("GET", "/bmo-face/api/me", &[("cookie", &cookie)], "").await.0, 401);
  assert_eq!(
    face
      .call("GET", "/bmo-face/api/nothing", &[("cookie", &face.login().await)], "")
      .await
      .0,
    404
  );
}

#[tokio::test]
async fn an_address_that_keeps_failing_to_log_in_is_held_off() {
  let face = Face::new(None, true).await.start().await;
  let mut last = 0;
  for _ in 0..21 {
    last = face
      .write("POST", "/bmo-face/api/login", &json!({ "name": "owner", "password": "wrong" }), &[])
      .await
      .0;
  }
  assert_eq!(last, 429);
}

#[tokio::test]
async fn the_listener_routes_pass_to_bmo_with_the_bearer_and_no_cookie_either_way() {
  let face = Face::new(None, true).await.start().await;

  let (status, headers, body) = face
    .call(
      "GET",
      "/bmo-face/bmo/api/briefing/events?after=7&run=r1",
      &[("authorization", "Bearer tok"), ("cookie", "bmo_face=x")],
      "",
    )
    .await;
  assert_eq!((status, body.as_str()), (200, "event: keep\ndata: {}\n\n"));
  assert_eq!(headers["content-type"], "text/event-stream");
  assert!(headers.get("set-cookie").is_none());
  let asked = face.asked();
  assert_eq!(
    (asked[0].method.as_str(), asked[0].target.as_str()),
    ("GET", "/bmo/api/briefing/events?after=7&run=r1")
  );
  assert_eq!(asked[0].headers["authorization"], "Bearer tok");
  assert!(asked[0].headers.get("cookie").is_none());

  let (status, headers, _) = face
    .call(
      "GET",
      "/bmo-face/bmo/api/media/audio/-100123/8784",
      &[("authorization", "Bearer tok"), ("range", "bytes=0-")],
      "",
    )
    .await;
  assert_eq!((status, headers["content-range"].to_str().unwrap()), (206, "bytes 0-3/4"));
  face
    .write(
      "POST",
      "/bmo-face/bmo/api/app/login",
      &json!({ "name": "owner", "password": "secret", "device": "케이스" }),
      &[],
    )
    .await;
  face
    .write(
      "POST",
      "/bmo-face/bmo/api/briefing/stop",
      &json!({ "chatId": 1, "messageId": 2 }),
      &[("authorization", "Bearer tok")],
    )
    .await;
  let asked: Vec<String> =
    face.asked().iter().map(|one| format!("{} {}", one.method, one.target)).collect();
  assert_eq!(
    asked[1..],
    [
      "GET /bmo/api/media/audio/-100123/8784",
      "POST /bmo/api/app/login",
      "POST /bmo/api/briefing/stop"
    ]
  );
  assert_eq!(face.asked()[1].headers["range"], "bytes=0-");

  // Every other route of BMO's is refused without asking it.
  let before = face.asked().len();
  assert_eq!(face.get("/bmo-face/bmo/api/todo").await.0, 404);
  assert_eq!(face.get("/bmo-face/bmo/api/briefing/stop").await.0, 404);
  assert_eq!(face.write("POST", "/bmo-face/bmo/api/briefing/events", &json!({}), &[]).await.0, 404);
  assert_eq!(face.asked().len(), before);

  // A refusal keeps BMO's status and none of its body, which names addresses inside the machine.
  let (status, _, body) = face
    .call("GET", "/bmo-face/bmo/api/briefing/events", &[("authorization", "Bearer gone")], "")
    .await;
  assert_eq!(status, 401);
  assert!(!body.contains("127.0.0.1"), "{body}");
}

#[tokio::test]
async fn a_bmo_that_does_not_answer_is_a_502() {
  let face = Face::new(None, false).await.start().await;
  assert_eq!(face.get("/bmo-face/bmo/api/briefing/events").await.0, 502);
}

#[tokio::test]
async fn a_stored_document_is_made_right_before_the_server_answers() {
  // The old shape, turned into this one with its version kept.
  let old = json!({
    "colors": { "body": "#58bfae", "screen": "#c9f0cf", "ink": "#1f3b35", "cheek": "#f4a3b4" },
    "faces": { "기본": { "eyes": [{ "shape": "ellipse", "cx": 33, "cy": 30, "rx": 3.2, "ry": 3.8, "filled": true, "width": 1 }] } },
    "mouths": [[{ "shape": "curve", "points": [[40, 47], [50, 53], [60, 47]], "width": 2.2, "closed": false, "filled": false }], []],
    "savedAt": 1700,
  });
  let face = Face::new(Some(&old.to_string()), true).await.start().await;
  let document = face.document().await;
  assert_eq!(document["savedAt"], 1700);
  assert_eq!(document["colors"]["ink"], "#1f3b35");
  assert_eq!(document["faces"]["기본"]["eyes"][0]["nodes"].as_array().unwrap().len(), 4);
  assert_eq!(document["faces"]["폭소"], default_document()["faces"]["폭소"]);
  drop(face);

  // A file that is not JSON is moved aside, never deleted, and the default written in its place.
  let face = Face::new(Some("{ half a docu"), true).await.start().await;
  assert_eq!(face.document().await, default_document());
  let aside: Vec<PathBuf> = std::fs::read_dir(face.root().join("data"))
    .unwrap()
    .map(|entry| entry.unwrap().path())
    .filter(|path| path.file_name().unwrap().to_string_lossy().starts_with("faces.json.broken-"))
    .collect();
  assert_eq!(aside.len(), 1);
  assert_eq!(std::fs::read_to_string(&aside[0]).unwrap(), "{ half a docu");
  assert!(face.said().contains("was refused (it is not JSON)"), "{}", face.said());
}

#[tokio::test]
async fn the_manifest_reads_through_the_vendored_release_with_the_faces_own_fields_and_layer() {
  let face = Face::new(None, true).await.start().await;
  let (status, manifest) = face.manifest().await;
  assert_eq!(status, 200);

  let read = bmo_console_face::read_manifest(&manifest, &vendored_release())
    .unwrap_or_else(|why| panic!("the manifest does not read: {why}"));
  let declared: Value = serde_json::from_str(FIXED).unwrap();
  for (field, value) in declared.as_object().unwrap() {
    assert_eq!(&read[field], value, "{field}");
  }
  assert_eq!(read["layer"], vendored_layer());
}

#[tokio::test]
async fn the_screens_call_needs_the_credential_and_a_role_the_manifest_allows() {
  let face = Face::new(None, true).await.start().await;
  let path = "/bmo-face/console/faces";

  assert_eq!(face.call("GET", path, &[("owner-role", "member")], "").await.0, 401);
  assert_eq!(
    face
      .call("GET", path, &[("authorization", "Bearer other"), ("owner-role", "member")], "")
      .await
      .0,
    401
  );
  assert_eq!(face.call("GET", path, &[("authorization", BEARER)], "").await.0, 403);
  assert_eq!(
    face
      .call("GET", path, &[("authorization", BEARER), ("owner-role", "guest")], "")
      .await
      .0,
    403
  );
  for role in ["member", "administrator"] {
    let (status, _, body) =
      face.call("GET", path, &[("authorization", BEARER), ("owner-role", role)], "").await;
    assert_eq!((status, serde_json::from_str::<Value>(&body).unwrap()), (200, default_document()));
  }
}

#[tokio::test]
async fn the_console_screen_saves_as_the_editor_does_without_a_session_or_this_sites_origin() {
  let face = Face::new(None, true).await.start().await;
  let mut edited = default_document();
  edited["colors"]["body"] = json!("#223344");
  let save = json!({ "document": edited, "basedOn": 0 });
  let console = [("origin", "http://console.test"), ("authorization", BEARER), ("owner-role", "member")];

  assert_eq!(face.write("PUT", "/bmo-face/console/faces", &save, &[]).await.0, 401);
  assert_eq!(
    face
      .write("PUT", "/bmo-face/console/faces", &save, &[("authorization", BEARER)])
      .await
      .0,
    403
  );

  let (status, _, body) = face.write("PUT", "/bmo-face/console/faces", &save, &console).await;
  assert_eq!(status, 200);
  let saved: Value = serde_json::from_str(&body).unwrap();
  assert_eq!(saved["colors"]["body"], "#223344");
  assert_eq!(face.document().await, saved);

  let (status, _, body) = face.write("PUT", "/bmo-face/console/faces", &save, &console).await;
  assert_eq!(status, 409);
  assert_eq!(serde_json::from_str::<Value>(&body).unwrap()["current"]["savedAt"], saved["savedAt"]);
}

#[tokio::test]
async fn no_bundle_built_is_no_manifest_and_a_bundle_the_console_face_refuses_stops_the_start() {
  let unbuilt = Face::new(None, true).await;
  std::fs::remove_file(unbuilt.root().join("console/bundle.js")).unwrap();
  let unbuilt = unbuilt.start().await;
  assert_eq!(unbuilt.manifest().await.0, 404);
  assert_eq!(unbuilt.call("GET", "/bmo-face/console-manifest", &[], "").await.0, 401);
  assert_eq!(
    unbuilt.call("GET", "/bmo-face/console/faces", &[("authorization", BEARER)], "").await.0,
    404
  );
  assert_eq!(unbuilt.get("/bmo-face/api/faces").await.0, 200);

  let unlayered = Face::new(None, true).await;
  std::fs::remove_file(unlayered.root().join("console/layer.json")).unwrap();
  let said = unlayered.refused().await;
  assert!(said.contains("the console face could not be built"), "{said}");
}

#[tokio::test]
async fn a_server_given_no_token_refuses_every_console_route_and_serves_the_rest() {
  let mut face = Face::new(None, true).await;
  face.token = None;
  let face = face.start().await;

  assert_eq!(face.manifest().await.0, 401);
  assert_eq!(
    face
      .call(
        "GET",
        "/bmo-face/console/faces",
        &[("authorization", BEARER), ("owner-role", "member")],
        ""
      )
      .await
      .0,
    401
  );
  assert_eq!(face.get("/bmo-face/api/faces").await.0, 200);
}

#[tokio::test]
async fn a_stop_ends_the_streams_takes_no_new_connection_and_answers_the_call_in_hand_whole() {
  let (face, held) = Face::beside_held().await;
  let mut face = face.start().await;
  let bearer = [("authorization", "Bearer tok")];

  let mut saves = face.open("GET", "/bmo-face/api/faces/events", &[], Bytes::new()).await.into_body();
  assert!(frame(&mut saves).await.starts_with("event: faces"));
  let mut listener = face
    .open("GET", "/bmo-face/bmo/api/briefing/events", &bearer, Bytes::new())
    .await
    .into_body();
  assert_eq!(frame(&mut listener).await, "event: keep\ndata: {}\n\n");

  let sound = face.call("GET", "/bmo-face/bmo/api/media/audio/1/2", &bearer, "");
  let stopping = async {
    held.asked.notified().await;
    face.terminate();
    read_to_end(&mut saves).await;
    read_to_end(&mut listener).await;
    // The sound is still held, so a refused connection is the drain's and not the server's end.
    for _ in 0..500 {
      if !face.takes_connections().await {
        break;
      }
      tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(!face.takes_connections().await, "a stopping server still takes connections");
    held.release.notify_one();
  };
  let ((status, _, body), ()) = tokio::join!(sound, stopping);
  assert_eq!((status, body.as_str()), (200, "OggS"));

  assert!(face.ended().await.success());
  assert!(face.said().contains("bmo-face: stopping"), "{}", face.said());
}

#[tokio::test]
async fn a_call_still_in_hand_past_the_stops_deadline_ends_with_the_server() {
  let (mut face, held) = Face::beside_held().await;
  face.drain_ms = Some("100");
  let mut face = face.start().await;

  let sound = face.try_open(
    "GET",
    "/bmo-face/bmo/api/media/audio/1/2",
    &[("authorization", "Bearer tok")],
    Bytes::new(),
  );
  let stopping = async {
    held.asked.notified().await;
    face.terminate();
  };
  let (answered, ()) = tokio::join!(sound, stopping);
  assert!(answered.is_err(), "a held call was answered: {answered:?}");

  assert!(face.ended().await.success());
  assert!(face.said().contains("end with the server"), "{}", face.said());
}

#[tokio::test]
async fn a_display_whose_stream_a_stop_ended_finds_the_restarted_server_and_the_editor_saves_on() {
  let mut face = Face::new(None, true).await.start().await;
  let cookie = face.login().await;
  let mut saves = face.open("GET", "/bmo-face/api/faces/events", &[], Bytes::new()).await.into_body();
  assert_eq!(frame(&mut saves).await, "event: faces\ndata: {\"savedAt\":0}\n\n");

  face.terminate();
  read_to_end(&mut saves).await;
  assert!(face.ended().await.success());
  let port = face.port;
  let face = face.start().await;
  assert_eq!(face.port, port, "the restarted server took another port");

  // The display opens its stream again where it was, and the editor saves with the session it signed
  // in with before the restart.
  let mut saves = face.open("GET", "/bmo-face/api/faces/events", &[], Bytes::new()).await.into_body();
  assert_eq!(frame(&mut saves).await, "event: faces\ndata: {\"savedAt\":0}\n\n");
  let (status, _, body) = face
    .write(
      "PUT",
      "/bmo-face/api/faces",
      &json!({ "document": default_document(), "basedOn": 0 }),
      &[("cookie", &cookie)],
    )
    .await;
  assert_eq!(status, 200, "{body}");
  let saved: Value = serde_json::from_str(&body).unwrap();
  let announced = format!("event: faces\ndata: {}\n\n", json!({ "savedAt": saved["savedAt"] }));
  while frame(&mut saves).await != announced {}
}
