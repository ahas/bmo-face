//! The face server as the displayer calls it (docs/face.md, 「The server」 and 「What BMO sends, and by
//! which road」): the expression document and its saves stream, and BMO's four listener routes through
//! the server's pass-through, each with the device token as a bearer token. A call waits for its answer;
//! nothing here cuts one short.

use std::fmt;

use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, ORIGIN};
use hyper::{Method, Request, Response, StatusCode, Uri};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use serde_json::{Value, json};
use sse_stream::SseStream;

const EVENT_STREAM: &str = "text/event-stream";

/// Why a call came to nothing.
#[derive(Debug)]
pub enum Fault {
  /// BMO refused the token or the account: 401 or 403.
  Refused(StatusCode),
  /// Any other status.
  Status(StatusCode),
  /// The server was not reached, or the answer broke off or did not read.
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

/// A stream of server-sent events off an answer's body.
pub type Events = SseStream<Incoming>;

/// The face server at its address.
#[derive(Clone)]
pub struct FaceServer {
  base: String,
  client: Client<HttpConnector, Full<Bytes>>,
}

impl FaceServer {
  /// The server whose address, base path included, is `base`.
  pub fn new(base: &str) -> FaceServer {
    FaceServer {
      base: base.trim_end_matches('/').to_owned(),
      client: Client::builder(TokioExecutor::new()).build_http(),
    }
  }

  /// The site the server's address names: its scheme and its host with the port.
  fn origin(&self) -> Option<String> {
    let uri: Uri = self.base.parse().ok()?;
    Some(format!("{}://{}", uri.scheme_str()?, uri.authority()?))
  }

  async fn call(
    &self,
    method: Method,
    path: &str,
    token: Option<&str>,
    accept: Option<&str>,
    body: Option<Value>,
  ) -> Result<Response<Incoming>, Fault> {
    let mut request = Request::builder().method(method).uri(format!("{}{path}", self.base));
    if let Some(token) = token {
      request = request.header(AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(accept) = accept {
      request = request.header(ACCEPT, accept);
    }
    let body = match body {
      Some(body) => {
        // A write to the face server must come from its own site, as a page served by it does.
        if let Some(origin) = self.origin() {
          request = request.header(ORIGIN, origin);
        }
        request = request.header(CONTENT_TYPE, "application/json");
        Full::new(Bytes::from(body.to_string()))
      }
      None => Full::new(Bytes::new()),
    };
    let request = request.body(body).map_err(|fault| Fault::Unreachable(fault.to_string()))?;
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

  async fn bytes(response: Response<Incoming>) -> Result<Bytes, Fault> {
    Ok(
      response
        .into_body()
        .collect()
        .await
        .map_err(|fault| Fault::Unreachable(fault.to_string()))?
        .to_bytes(),
    )
  }

  async fn json(response: Response<Incoming>) -> Result<Value, Fault> {
    serde_json::from_slice(&FaceServer::bytes(response).await?)
      .map_err(|fault| Fault::Unreachable(format!("the answer is not JSON: {fault}")))
  }

  /// The expression document, as the server keeps it.
  pub async fn document(&self) -> Result<Value, Fault> {
    FaceServer::json(self.call(Method::GET, "/api/faces", None, None, None).await?).await
  }

  /// The stream of the document's saves.
  pub async fn saves(&self) -> Result<Events, Fault> {
    let response = self.call(Method::GET, "/api/faces/events", None, Some(EVENT_STREAM), None).await?;
    Ok(SseStream::new(response.into_body()))
  }

  /// BMO's listener stream from a cursor's query.
  pub async fn listener(&self, token: &str, query: &str) -> Result<Events, Fault> {
    let path = format!("/bmo/api/briefing/events{query}");
    let response = self.call(Method::GET, &path, Some(token), Some(EVENT_STREAM), None).await?;
    Ok(SseStream::new(response.into_body()))
  }

  /// A briefing's sound, whole.
  pub async fn sound(&self, token: &str, chat_id: i64, message_id: i64) -> Result<Bytes, Fault> {
    let path = format!("/bmo/api/media/audio/{chat_id}/{message_id}");
    FaceServer::bytes(self.call(Method::GET, &path, Some(token), None, None).await?).await
  }

  /// Signs a device in with a BMO web account: the token BMO answers.
  pub async fn sign_in(&self, name: &str, password: &str, device: &str) -> Result<String, Fault> {
    let body = json!({ "name": name, "password": password, "device": device });
    let answer =
      FaceServer::json(self.call(Method::POST, "/bmo/api/app/login", None, None, Some(body)).await?)
        .await?;
    answer["token"]
      .as_str()
      .map(str::to_owned)
      .ok_or_else(|| Fault::Unreachable("the login answered no token".to_owned()))
  }
}
