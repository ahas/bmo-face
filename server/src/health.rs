//! `bmo-face health` and `bmo-face console-reach` (docs/deploy.md, 「What a deployment does」): the
//! deployment's two questions of the server, each asked at the address the settings give it.

use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use rand::RngCore;
use serde_json::json;

use crate::handler::PREFIX;
use crate::settings::Settings;

/// The server's loopback address under its base path, at the port the settings name.
pub fn address(settings: &Settings) -> String {
  format!("http://127.0.0.1:{}{PREFIX}", settings.port())
}

/// Asks the server at `address` for its document and its generated screen; ready only where the
/// document answers a success and the screen a success holding HTML.
pub async fn ready(address: &str) -> Result<(), String> {
  let client: Client<_, Empty<Bytes>> = Client::builder(TokioExecutor::new()).build_http();
  let ask = |path: &str| {
    let uri = format!("{address}{path}");
    let client = client.clone();
    async move {
      let answer = client
        .get(uri.parse().map_err(|why| format!("{uri} is no address: {why}"))?)
        .await
        .map_err(|why| format!("{uri} did not answer: {why}"))?;
      let status = answer.status();
      let body = answer
        .into_body()
        .collect()
        .await
        .map_err(|why| format!("{uri} broke off: {why}"))?
        .to_bytes();
      if !status.is_success() {
        return Err(format!("{uri} answered {status}"));
      }
      Ok::<_, String>(body)
    }
  };

  ask("/api/faces").await?;
  let screen = ask("/").await?;
  if !String::from_utf8_lossy(&screen).to_ascii_lowercase().contains("<html") {
    return Err(format!("{address}/ answered no HTML"));
  }
  Ok(())
}

/// How a console reaches the face, as the testbed's console probe reads it: the server's address, the
/// console's credential the settings name, and, since the face holds no token of its own, a value made new
/// for each call in its place, which the console's routes refuse as they refuse every credential but the
/// console's.
pub fn console_reach(settings: &Settings) -> Result<String, String> {
  let console_token = settings
    .named("BMO_FACE_TOKEN")
    .ok_or("the settings name no console credential, BMO_FACE_TOKEN")?;
  let mut stand_in = [0u8; 24];
  rand::rng().fill_bytes(&mut stand_in);
  let token: String = stand_in.iter().map(|byte| format!("{byte:02x}")).collect();
  Ok(json!({ "address": address(settings), "consoleToken": console_token, "token": token }).to_string())
}
