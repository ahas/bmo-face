//! The displayer's calls over plain HTTP on the loopback, shared by the face server and the voice engine:
//! a `GET` with or without a bearer token, its answer read whole, and an event stream read off an
//! answer's body, held alive by any byte it carries. A call waits for its answer; nothing here cuts one
//! short.

use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::{BodyExt, Empty};
use hyper::body::{Bytes, Incoming};
use hyper::header::{ACCEPT, AUTHORIZATION};
use hyper::{Request, Response, StatusCode};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use serde_json::Value;
use sse_stream::{Sse, SseStream};
use tokio::time::sleep_until;

const EVENT_STREAM: &str = "text/event-stream";

/// Why a call came to nothing.
#[derive(Debug)]
pub enum Fault {
  /// The token was refused: 401 or 403.
  Refused(StatusCode),
  /// Any other status.
  Status(StatusCode),
  /// The service was not reached, or the answer broke off or did not read.
  Unreachable(String),
}

impl fmt::Display for Fault {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Fault::Refused(status) => write!(formatter, "refused with {status}"),
      Fault::Status(status) => write!(formatter, "answered {status}"),
      Fault::Unreachable(why) => write!(formatter, "not reached: {why}"),
    }
  }
}

/// A stream of server-sent events off an answer's body, with the moment its body last carried anything.
pub struct Events {
  stream: SseStream<UnsyncBoxBody<Bytes, hyper::Error>>,
  heard: Arc<Mutex<Instant>>,
}

impl Events {
  fn new(body: Incoming) -> Events {
    let heard = Arc::new(Mutex::new(Instant::now()));
    let marks = Arc::clone(&heard);
    let body = body
      .map_frame(move |frame| {
        *marks.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Instant::now();
        frame
      })
      .boxed_unsync();
    Events {
      stream: SseStream::new(body),
      heard,
    }
  }

  fn last_heard(&self) -> Instant {
    *self.heard.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
  }

  /// The next event, or none once the stream ends, breaks off, or carries nothing at all — not an event,
  /// not a comment — for `silence`. A comment line keeps the stream alive without being an event.
  pub async fn next_within(&mut self, silence: Duration) -> Option<Sse> {
    loop {
      let due = self.last_heard() + silence;
      tokio::select! {
        next = self.stream.next() => return next.and_then(Result::ok),
        () = sleep_until(due.into()) => {
          if self.last_heard().elapsed() >= silence {
            return None;
          }
        }
      }
    }
  }
}

/// A service at its address.
#[derive(Clone)]
pub struct Service {
  base: String,
  client: Client<HttpConnector, Empty<Bytes>>,
}

impl Service {
  /// The service whose address, base path included, is `base`.
  pub fn new(base: &str) -> Service {
    Service {
      base: base.trim_end_matches('/').to_owned(),
      client: Client::builder(TokioExecutor::new()).build_http(),
    }
  }

  async fn get(
    &self,
    path: &str,
    token: Option<&str>,
    accept: Option<&str>,
  ) -> Result<Response<Incoming>, Fault> {
    let mut request = Request::get(format!("{}{path}", self.base));
    if let Some(token) = token {
      request = request.header(AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(accept) = accept {
      request = request.header(ACCEPT, accept);
    }
    let request = request.body(Empty::new()).map_err(|fault| Fault::Unreachable(fault.to_string()))?;
    let response = self
      .client
      .request(request)
      .await
      .map_err(|fault| Fault::Unreachable(fault.to_string()))?;
    match response.status() {
      status if status.is_success() => Ok(response),
      status @ (StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) => Err(Fault::Refused(status)),
      status => Err(Fault::Status(status)),
    }
  }

  /// The answer to a `GET`, whole.
  pub async fn bytes(&self, path: &str, token: Option<&str>) -> Result<Bytes, Fault> {
    Ok(
      self
        .get(path, token, None)
        .await?
        .into_body()
        .collect()
        .await
        .map_err(|fault| Fault::Unreachable(fault.to_string()))?
        .to_bytes(),
    )
  }

  /// The answer to a `GET`, read as JSON.
  pub async fn json(&self, path: &str, token: Option<&str>) -> Result<Value, Fault> {
    serde_json::from_slice(&self.bytes(path, token).await?)
      .map_err(|fault| Fault::Unreachable(format!("the answer is not JSON: {fault}")))
  }

  /// The event stream a `GET` answers.
  pub async fn events(&self, path: &str, token: Option<&str>) -> Result<Events, Fault> {
    Ok(Events::new(self.get(path, token, Some(EVENT_STREAM)).await?.into_body()))
  }
}
