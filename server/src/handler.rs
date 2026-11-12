//! Everything the server answers under `/bmo-face` (docs/face.md, 「The server」 and 「The console
//! screen」).

use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use futures_util::stream;
use serde_json::{Value, json};
use tokio::sync::broadcast;

use crate::console::{Console, MANIFEST_ROUTE, Need};
use crate::login::{EditorLogin, LoginResult, cleared_cookie, session_cookie};
use crate::pass::{Upstream, pass_through, passes_through, upstream};
use crate::stop::Stop;
use crate::store::{FaceStore, Saved};

pub const PREFIX: &str = "/bmo-face";

/// The most of a request body the server reads.
pub const BODY_LIMIT: usize = 2_000_000;

pub struct Face {
  pub store: FaceStore,
  pub login: EditorLogin,
  /// The generated screen, served under the prefix, read through its path on every request.
  pub static_dir: PathBuf,
  /// BMO's console with its base path, which the listener routes are passed through to.
  pub console_url: String,
  /// How often an open display's stream of saves says it is still there.
  pub keep: Duration,
  pub upstream: Upstream,
  /// The console screen's manifest, bundle and guard.
  pub console: Console,
  /// Asked once the server is to stop, which ends every open stream.
  pub stop: Stop,
}

impl Face {
  pub fn new(
    store: FaceStore,
    login: EditorLogin,
    static_dir: PathBuf,
    console_url: String,
    keep: Duration,
    console: Console,
  ) -> Face {
    Face {
      store,
      login,
      static_dir,
      console_url,
      keep,
      upstream: upstream(),
      console,
      stop: Stop::default(),
    }
  }
}

/// A JSON answer no cache keeps, with any headers given.
pub fn json_answer(status: StatusCode, body: &Value, headers: &[(header::HeaderName, String)]) -> Response {
  let mut response = (status, body.to_string()).into_response();
  let kept = response.headers_mut();
  kept.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json;charset=utf-8"));
  kept.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
  for (name, value) in headers {
    if let Ok(value) = HeaderValue::from_str(value) {
      kept.insert(name.clone(), value);
    }
  }
  response
}

fn error(status: StatusCode, said: &str) -> Response {
  json_answer(status, &json!({ "error": said }), &[])
}

fn not_found() -> Response {
  (StatusCode::NOT_FOUND, [(header::CONTENT_TYPE, "text/plain;charset=utf-8")], "Not found").into_response()
}

/// The paths under the prefix that belong to the server; every other GET is the screen's.
fn server_route(route: &str) -> bool {
  ["/api", "/bmo"]
    .iter()
    .any(|own| route == *own || route.starts_with(&format!("{own}/")))
}

// The path a route names under the screen's directory, its dots walked, and never above it.
fn screen_path(root: &Path, route: &str) -> Option<PathBuf> {
  let decoded = percent_encoding::percent_decode_str(route).decode_utf8().ok()?;
  let mut path = root.to_owned();
  for part in Path::new(decoded.as_ref()).components() {
    match part {
      Component::Normal(name) => path.push(name),
      Component::ParentDir if !path.pop() || !path.starts_with(root) => return None,
      _ => {}
    }
  }
  (path != root && path.starts_with(root)).then_some(path)
}

fn content_type(path: &Path) -> String {
  let guessed = mime_guess::from_path(path).first_or_octet_stream();
  let textual = guessed.type_() == "text"
    || matches!(guessed.essence_str(), "application/json" | "application/javascript");
  if textual {
    format!("{};charset=utf-8", guessed.essence_str())
  } else {
    guessed.essence_str().to_owned()
  }
}

async fn screen(face: &Face, route: &str, head: bool) -> Response {
  let root = &face.static_dir;
  let file = screen_path(root, route).filter(|path| path.is_file());
  let is_file = file.is_some();
  let target = file.unwrap_or_else(|| root.join("index.html"));
  if !target.is_file() {
    return not_found();
  }
  let cache = if is_file && route.starts_with("/_nuxt/") {
    "public, max-age=31536000, immutable"
  } else {
    "no-cache"
  };
  let body = if head {
    Body::empty()
  } else {
    match tokio::fs::read(&target).await {
      Ok(bytes) => Body::from(bytes),
      Err(_) => return not_found(),
    }
  };
  let mut response = Response::new(body);
  let headers = response.headers_mut();
  headers.insert(
    header::CONTENT_TYPE,
    HeaderValue::from_str(&content_type(&target)).expect("a media type"),
  );
  headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
  response
}

// Every save as it happens, and a keep frame between, for as long as the display holds it open and
// the server is not stopping.
fn saves(face: &Arc<Face>) -> Response {
  let heard = face.store.subscribe();
  let first = face
    .store
    .read()
    .ok()
    .and_then(|document| document.get("savedAt").cloned())
    .unwrap_or(json!(0));
  let opening = format!("event: faces\ndata: {}\n\n", json!({ "savedAt": first }));
  let mut keep = tokio::time::interval_at(tokio::time::Instant::now() + face.keep, face.keep);
  keep.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

  let frames = stream::unfold((Some(opening), heard, keep), |(opening, mut heard, mut keep)| async move {
    if let Some(opening) = opening {
      return Some((Ok::<_, std::convert::Infallible>(Bytes::from(opening)), (None, heard, keep)));
    }
    let frame = loop {
      tokio::select! {
        saved = heard.recv() => match saved {
          Ok(at) => break format!("event: faces\ndata: {}\n\n", json!({ "savedAt": at })),
          Err(broadcast::error::RecvError::Lagged(_)) => continue,
          Err(broadcast::error::RecvError::Closed) => return None,
        },
        _ = keep.tick() => break "event: keep\ndata: {}\n\n".to_owned(),
      }
    };
    Some((Ok(Bytes::from(frame)), (None, heard, keep)))
  });

  let mut response = Response::new(Body::from_stream(face.stop.ends(frames)));
  let headers = response.headers_mut();
  headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/event-stream"));
  headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
  headers.insert("x-accel-buffering", HeaderValue::from_static("no"));
  response
}

// The expression document as it stands, for a display, the editor and the console screen alike.
fn document_answer(face: &Face) -> Response {
  match face.store.read() {
    Ok(document) => json_answer(StatusCode::OK, &document, &[]),
    Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
  }
}

// A save the editor or the console screen sends: the document with the version it was edited from.
fn save_answer(face: &Face, body: &Bytes) -> Response {
  let sent = serde_json::from_slice::<Value>(body).unwrap_or(Value::Null);
  let nothing = Value::Null;
  match face
    .store
    .save(sent.get("document").unwrap_or(&nothing), sent.get("basedOn").unwrap_or(&nothing))
  {
    Ok(Saved::Saved(saved)) => json_answer(StatusCode::OK, &saved, &[]),
    Ok(Saved::Conflict(current)) => json_answer(
      StatusCode::CONFLICT,
      &json!({ "error": "다른 창에서 먼저 저장했어요. 새로 불러온 뒤 다시 고쳐 주세요.", "current": current }),
      &[],
    ),
    Ok(Saved::Refused(said)) => error(StatusCode::BAD_REQUEST, &said),
    Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
  }
}

// A console's call (docs/face.md, 「The console screen」): the console's credential before anything,
// the reader's role before anything the screen reads, and no origin to match, since a console's
// server makes the call.
async fn console_answer(
  face: &Face,
  route: &str,
  method: &Method,
  headers: &HeaderMap,
  body: Body,
) -> Response {
  let refused = |status: StatusCode| {
    let said = if status == StatusCode::UNAUTHORIZED {
      "The console's credential is missing or not this service's."
    } else {
      "The reader's role does not open this screen."
    };
    error(status, said)
  };
  let open = *method == Method::GET;
  let words = face.console.words_route();

  if route == MANIFEST_ROUTE || route == words {
    if let Err(status) = face.console.guard(headers, Need::Credential) {
      return refused(status);
    }
    if !open {
      return error(StatusCode::METHOD_NOT_ALLOWED, "Only read here.");
    }
    if route == MANIFEST_ROUTE {
      return match face.console.manifest().await {
        Some(manifest) => json_answer(StatusCode::OK, &manifest, &[]),
        None => error(StatusCode::NOT_FOUND, "No console bundle is built."),
      };
    }
    return match face.console.words().await {
      Some(bytes) => {
        let mut response = Response::new(Body::from(bytes));
        let kept = response.headers_mut();
        kept.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json;charset=utf-8"));
        kept.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        response
      }
      None => error(StatusCode::NOT_FOUND, "No console bundle is built."),
    };
  }

  if let Err(status) = face.console.guard(headers, Need::Reader) {
    return refused(status);
  }

  if route == format!("{}/faces", face.console.api_prefix()) {
    if open {
      return document_answer(face);
    }
    if *method == Method::PUT {
      return match axum::body::to_bytes(body, BODY_LIMIT).await {
        Ok(body) => save_answer(face, &body),
        Err(_) => StatusCode::PAYLOAD_TOO_LARGE.into_response(),
      };
    }
    return error(StatusCode::METHOD_NOT_ALLOWED, "Only read and save here.");
  }

  if open && let Some(built) = face.console.built(route).await {
    let mut response = Response::new(Body::from(built.bytes));
    let kept = response.headers_mut();
    kept.insert(header::CONTENT_TYPE, HeaderValue::from_static(built.content_type));
    kept.insert(header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=31536000, immutable"));
    return response;
  }

  error(StatusCode::NOT_FOUND, "Nothing of the console's stands here.")
}

// Whether a write comes from this site: its `Origin` names this host.
fn from_this_site(headers: &HeaderMap) -> bool {
  let header = |name: header::HeaderName| headers.get(name).and_then(|value| value.to_str().ok());
  let (Some(origin), Some(host)) = (header(header::ORIGIN), header(header::HOST)) else {
    return false;
  };
  let Ok(origin) = url::Url::parse(origin) else {
    return false;
  };
  let named = match (origin.host_str(), origin.port()) {
    (Some(name), Some(port)) => format!("{name}:{port}"),
    (Some(name), None) => name.to_owned(),
    (None, _) => return false,
  };
  named == host
}

async fn answer(
  State(face): State<Arc<Face>>,
  ConnectInfo(peer): ConnectInfo<SocketAddr>,
  request: Request,
) -> Response {
  let (parts, body) = request.into_parts();
  // The path as a browser reads it, dot segments and their encoded forms walked, as the screen's own
  // links are resolved.
  let read =
    url::Url::parse(&format!("http://localhost{}", parts.uri.path())).map(|url| url.path().to_owned());
  let path = read.as_deref().unwrap_or(parts.uri.path());
  if path != PREFIX && !path.starts_with(&format!("{PREFIX}/")) {
    return not_found();
  }
  let route = &path[PREFIX.len()..];
  let query = parts
    .uri
    .query()
    .filter(|query| !query.is_empty())
    .map(|query| format!("?{query}"))
    .unwrap_or_default();
  let method = parts.method.clone();
  let headers = parts.headers;

  if route.is_empty() {
    return (
      StatusCode::PERMANENT_REDIRECT,
      [
        (header::LOCATION, format!("{PREFIX}/{query}")),
        (header::CACHE_CONTROL, "no-store".into()),
      ],
    )
      .into_response();
  }

  // Nothing larger than the server takes is read at all.
  let declared = headers
    .get(header::CONTENT_LENGTH)
    .and_then(|value| value.to_str().ok()?.parse::<usize>().ok());
  if declared.is_some_and(|length| length > BODY_LIMIT) {
    return StatusCode::PAYLOAD_TOO_LARGE.into_response();
  }

  // A console's routes stand behind its credential rather than this site's origin.
  if face.console.owns(route) {
    return console_answer(&face, route, &method, &headers, body).await;
  }

  // A write must come from this site; the check stands before anything is read.
  if method != Method::GET && method != Method::HEAD && !from_this_site(&headers) {
    return error(StatusCode::FORBIDDEN, "요청 출처를 확인할 수 없습니다.");
  }

  if (method == Method::GET || method == Method::HEAD) && !server_route(route) {
    return screen(&face, route, method == Method::HEAD).await;
  }

  let Ok(body) = axum::body::to_bytes(body, BODY_LIMIT).await else {
    return StatusCode::PAYLOAD_TOO_LARGE.into_response();
  };

  if let Some(passed) = route.strip_prefix("/bmo").filter(|rest| rest.starts_with('/')) {
    return if passes_through(&method, passed) {
      let path_and_query = format!("{passed}{query}");
      pass_through(&face.upstream, &face.stop, method, &headers, body, &path_and_query, &face.console_url)
        .await
    } else {
      error(StatusCode::NOT_FOUND, "지나갈 수 없는 길이에요.")
    };
  }

  let sent = || serde_json::from_slice::<Value>(&body).unwrap_or(Value::Null);

  if route == "/api/faces" && method == Method::GET {
    return document_answer(&face);
  }
  if route == "/api/faces/events" && method == Method::GET {
    return saves(&face);
  }
  if route == "/api/login" && method == Method::POST {
    return match face.login.login(&peer.ip().to_string(), &sent()).await {
      LoginResult::Refused {
        status,
        error: said,
      } => error(StatusCode::from_u16(status).expect("a status"), said),
      LoginResult::Signed { token, name } => {
        let secure = headers.get("x-forwarded-proto").is_some_and(|proto| proto == "https");
        json_answer(
          StatusCode::OK,
          &json!({ "name": name }),
          &[(header::SET_COOKIE, session_cookie(&token, secure))],
        )
      }
    };
  }

  let cookie = headers.get(header::COOKIE).and_then(|value| value.to_str().ok());
  let Some(editor) = face.login.session_of(cookie).await else {
    return error(StatusCode::UNAUTHORIZED, "로그인이 필요합니다.");
  };

  if route == "/api/me" && method == Method::GET {
    return json_answer(StatusCode::OK, &json!({ "name": editor }), &[]);
  }
  if route == "/api/logout" && method == Method::POST {
    face.login.logout(cookie).await;
    return (StatusCode::NO_CONTENT, [(header::SET_COOKIE, cleared_cookie())]).into_response();
  }
  if route == "/api/faces" && method == Method::PUT {
    return save_answer(&face, &body);
  }

  error(StatusCode::NOT_FOUND, "찾을 수 없습니다.")
}

pub fn router(face: Arc<Face>) -> Router {
  Router::new().fallback(answer).with_state(face)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_route_names_a_file_under_the_screen_and_never_above_it() {
    let root = Path::new("/srv/public");
    assert_eq!(screen_path(root, "/_nuxt/app.js"), Some(PathBuf::from("/srv/public/_nuxt/app.js")));
    assert_eq!(screen_path(root, "/a/%2e%2e/index.html"), Some(PathBuf::from("/srv/public/index.html")));
    assert_eq!(screen_path(root, "/%2e%2e/secret"), None);
    assert_eq!(screen_path(root, "/"), None);
  }

  #[test]
  fn a_text_file_is_named_with_its_character_set() {
    assert_eq!(content_type(Path::new("index.html")), "text/html;charset=utf-8");
    assert_eq!(content_type(Path::new("app.js")), "text/javascript;charset=utf-8");
    assert_eq!(content_type(Path::new("latest.json")), "application/json;charset=utf-8");
    assert_eq!(content_type(Path::new("face.png")), "image/png");
  }
}
