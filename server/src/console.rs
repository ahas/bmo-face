//! The console screen's side of the server (docs/face.md, 「The console screen」): the console face
//! bmo-chat's `bmo-console-face` serves from this repository's declaration and built bundle, and the
//! console's credential the screens' calls take.

use std::path::Path;

use axum::Router;
use axum::http::{HeaderMap, header};
use bmo_console_face::{BundleFolder, CONSOLE_MANIFEST_ROUTE, ConsoleFace, ConsoleFaceError};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The manifest's fields the face decides, stated once for the server and the bundle.
const DECLARATION: &str = include_str!("../../console-manifest.json");

/// The bundle build's files in its folder.
const MODULE_FILE: &str = "bundle.js";
const STYLESHEET_FILE: &str = "bundle.css";
const WORDS_FILE: &str = "words.json";

pub struct Console {
  /// The console face, where a credential is set and a bundle is built.
  face: Option<ConsoleFace>,
  /// The credential a console calls with; with none, every console route is refused.
  token: Option<String>,
  /// The path of the service's API every call the screens make stands under.
  api_prefix: String,
}

impl Console {
  /// The console side of a server reading the bundle `dir` holds, guarded by `token`. A folder with
  /// no module is no bundle built, and stands no face; a bundle the console face refuses is refused
  /// here, so the server does not start on it.
  pub fn new(dir: &Path, token: Option<String>) -> Result<Console, ConsoleFaceError> {
    let declared: Value = serde_json::from_str(DECLARATION).expect("console-manifest.json is JSON");
    let api_prefix = declared["apiPrefix"].as_str().unwrap_or("/").trim_end_matches('/').to_owned();
    let token = token.filter(|token| !token.is_empty());
    let face = match &token {
      Some(token) if dir.join(MODULE_FILE).is_file() => {
        let mut bundle = BundleFolder::new(dir, MODULE_FILE, WORDS_FILE);
        if dir.join(STYLESHEET_FILE).is_file() {
          bundle = bundle.with_stylesheet(STYLESHEET_FILE);
        }
        Some(ConsoleFace::new(DECLARATION, &bundle, token)?)
      }
      _ => None,
    };
    Ok(Console {
      face,
      token,
      api_prefix,
    })
  }

  /// The route of the screens' one call, the expression document.
  pub fn document_route(&self) -> String {
    format!("{}/faces", self.api_prefix)
  }

  /// Whether a route is one of the console's: the manifest, or anything under the API prefix.
  pub fn owns(&self, route: &str) -> bool {
    let prefix = &self.api_prefix;
    route == CONSOLE_MANIFEST_ROUTE || route == prefix || route.starts_with(&format!("{prefix}/"))
  }

  /// Whether a call carries the console's credential.
  pub fn credited(&self, headers: &HeaderMap) -> bool {
    let sent = headers
      .get(header::AUTHORIZATION)
      .and_then(|value| value.to_str().ok())
      .and_then(|value| value.split_once(' '))
      .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
      .map(|(_, token)| token.trim());
    match (&self.token, sent) {
      // Compared as digests, so the comparison takes no longer for a closer guess.
      (Some(token), Some(sent)) => Sha256::digest(token.as_bytes()) == Sha256::digest(sent.as_bytes()),
      _ => false,
    }
  }

  /// The console face's routes beside the screens' calls, the face's guard laid over the calls. Where
  /// no face stands, none of them is routed, so the server refuses them as it refuses every console
  /// route then, and no call reaches the document without the role the manifest asks for.
  pub fn routes<S: Clone + Send + Sync + 'static>(&self, screens: Router<S>) -> Router<S> {
    match &self.face {
      Some(face) => face.router().merge(face.guard_screens(screens)),
      None => Router::new(),
    }
  }
}
