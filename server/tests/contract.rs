//! The face server's wire contract (docs/face.md, 「The server」), asked of the command under test as a
//! display, the editor and a stranger ask it: the server started as a process on a free port over a
//! scratch data root, a scratch screen, a scratch archive holding one BMO account, and a stand-in for
//! BMO's console that writes down what it was asked. The command is the package's binary, or the one
//! `FACE_CONTRACT_COMMAND` names.

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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

const HOST: &str = "face.test";
const ORIGIN: &str = "http://face.test";
const DEFAULT: &str = include_str!("../../tests/fixtures/default-faces.json");

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
      let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
      let at = listener.local_addr().unwrap();
      let app = Router::new().fallback(console).with_state(Arc::clone(&log));
      tokio::spawn(async move { axum::serve(listener, app).await });
      (format!("http://{at}/bmo"), None)
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
      _absent: absent,
    }
  }

  fn root(&self) -> &Path {
    self.dir.path()
  }

  fn faces_file(&self) -> PathBuf {
    self.root().join("data/faces.json")
  }

  async fn start(mut self) -> Face {
    let log = std::fs::File::create(self.root().join("server.log")).unwrap();
    self.server = Some(
      command()
        .env("BMO_FACE_DATA", self.root().join("data"))
        // A port of its own choosing, which it names as it starts answering, so no other process
        // can take the port between its choosing and its binding.
        .env("BMO_FACE_PORT", "0")
        .env("BMO_FACE_STATIC", self.root().join("public"))
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
    let ready = Instant::now() + Duration::from_secs(20);
    self.port = loop {
      let heard = self.said().lines().find_map(|line| {
        line.strip_prefix("BMO face: http://127.0.0.1:")?.split('/').next()?.parse::<u16>().ok()
      });
      if let Some(port) = heard {
        break port;
      }
      assert!(Instant::now() < ready, "the server never listened; it said {}", self.said());
      tokio::time::sleep(Duration::from_millis(20)).await;
    };
    self
  }

  fn said(&self) -> String {
    std::fs::read_to_string(self.root().join("server.log")).unwrap_or_default()
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
    let stream = TcpStream::connect(("127.0.0.1", self.port)).await.unwrap();
    let (mut sender, connection) =
      hyper::client::conn::http1::handshake(TokioIo::new(stream)).await.unwrap();
    tokio::spawn(connection);
    let mut request = hyper::Request::builder().method(method).uri(path).header("host", HOST);
    for (name, value) in headers {
      request = request.header(*name, *value);
    }
    sender.send_request(request.body(Full::new(body)).unwrap()).await.unwrap()
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

/// The next frame of an event stream within a bound, as text.
async fn frame(body: &mut Incoming) -> String {
  let next = tokio::time::timeout(Duration::from_secs(10), body.frame())
    .await
    .unwrap()
    .unwrap()
    .unwrap();
  String::from_utf8_lossy(&next.into_data().unwrap()).into_owned()
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
  let answer = tokio::time::timeout(Duration::from_secs(10), sender.send_request(request))
    .await
    .unwrap()
    .unwrap();
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
