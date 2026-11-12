//! The narrow pass-through to BMO's listener routes: the four a display uses and nothing else, with
//! the bearer token passed on untouched and no cookie either way (docs/face.md, 「What BMO sends, and
//! by which road」).

use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, HeaderValue, Method, Request, StatusCode};
use axum::response::Response;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use serde_json::json;

use crate::handler::json_answer;
use crate::stop::Stop;

const SENT_HEADERS: [&str; 4] = ["authorization", "content-type", "range", "accept"];
const RETURNED_HEADERS: [&str; 5] =
  ["content-type", "content-length", "content-range", "accept-ranges", "cache-control"];

/// Whether a request for this path under `/bmo` is one passed through, each route with the one method
/// it takes.
pub fn passes_through(method: &Method, path: &str) -> bool {
  let digits = |text: &str| !text.is_empty() && text.chars().all(|one| one.is_ascii_digit());
  match (method.as_str(), path) {
    ("POST", "/api/app/login") | ("GET", "/api/briefing/events") | ("POST", "/api/briefing/stop") => true,
    ("GET", _) => path
      .strip_prefix("/api/media/audio/")
      .and_then(|rest| rest.split_once('/'))
      .is_some_and(|(chat, message)| digits(chat.strip_prefix('-').unwrap_or(chat)) && digits(message)),
    _ => false,
  }
}

pub type Upstream = Client<HttpConnector, Body>;

pub fn upstream() -> Upstream {
  Client::builder(TokioExecutor::new()).build_http()
}

fn refusal(status: StatusCode, error: &str) -> Response {
  json_answer(status, &json!({ "error": error }), &[])
}

/// BMO's answer to a passed-through request, streamed as it comes, and an event stream ended once
/// `stop` is asked. `console_url` is BMO's console with its base path; `path` is what follows
/// `/bmo-face/bmo`, with its query.
pub async fn pass_through(
  client: &Upstream,
  stop: &Stop,
  method: Method,
  headers: &HeaderMap,
  body: Bytes,
  path_and_query: &str,
  console_url: &str,
) -> Response {
  let mut asked = Request::builder()
    .method(method.clone())
    .uri(format!("{}{path_and_query}", console_url.trim_end_matches('/')));
  for name in SENT_HEADERS {
    if let Some(value) = headers.get(name) {
      asked = asked.header(name, value);
    }
  }
  let body = if method == Method::POST { Body::from(body) } else { Body::empty() };
  let Ok(asked) = asked.body(body) else {
    return refusal(StatusCode::BAD_GATEWAY, "비모에 닿지 않아요.");
  };
  let Ok(answer) = client.request(asked).await else {
    return refusal(StatusCode::BAD_GATEWAY, "비모에 닿지 않아요.");
  };

  // A refusal is answered in this server's own words with BMO's status: what BMO writes about a refused
  // request names its own address inside this machine, which is nobody's business outside it.
  if answer.status().as_u16() >= 400 {
    return refusal(answer.status(), "비모가 거절했어요.");
  }

  let (parts, body) = answer.into_parts();
  let mut returned = HeaderMap::new();
  returned.insert("x-accel-buffering", HeaderValue::from_static("no"));
  for name in RETURNED_HEADERS {
    if let Some(value) = parts.headers.get(name) {
      returned.insert(name, value.clone());
    }
  }
  // An event stream has no end of its own, so it ends with the server; any other answer is let finish.
  let endless = returned
    .get("content-type")
    .and_then(|value| value.to_str().ok())
    .is_some_and(|kind| kind.trim_start().starts_with("text/event-stream"));
  let body = Body::new(body);
  let body = if endless { Body::from_stream(stop.ends(body.into_data_stream())) } else { body };
  let mut response = Response::new(body);
  *response.status_mut() = parts.status;
  *response.headers_mut() = returned;
  response
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn only_the_four_listener_routes_pass_each_with_its_one_method() {
    assert!(passes_through(&Method::POST, "/api/app/login"));
    assert!(passes_through(&Method::GET, "/api/briefing/events"));
    assert!(passes_through(&Method::POST, "/api/briefing/stop"));
    assert!(passes_through(&Method::GET, "/api/media/audio/-100123/8784"));
    assert!(!passes_through(&Method::GET, "/api/media/audio/-/8784"));
    assert!(!passes_through(&Method::GET, "/api/media/audio/12/x"));
    assert!(!passes_through(&Method::GET, "/api/briefing/stop"));
    assert!(!passes_through(&Method::POST, "/api/briefing/events"));
    assert!(!passes_through(&Method::GET, "/api/todo"));
  }
}
