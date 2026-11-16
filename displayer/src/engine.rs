//! The voice engine as the displayer calls it (docs/face.md, 「What it hears」; bmo-voice's
//! `docs/boundary-api.md`, 「Events」 and 「Briefings」): its one event stream and a briefing's sound, each
//! with the engine's bearer token, read afresh from the file the settings name on every call.

use std::io;
use std::path::{Path, PathBuf};

use hyper::body::Bytes;

use crate::http::{Events, Fault, Service};

/// The voice engine at its address, with the file its token stands in.
#[derive(Clone)]
pub struct VoiceEngine {
  service: Service,
  token_file: Option<PathBuf>,
}

/// Why the engine could not be asked.
#[derive(Debug)]
pub enum Unasked {
  /// No token file is named, or the one named holds nothing.
  NoToken,
  /// The token file did not read.
  Unreadable(io::Error),
  /// The engine was asked and the call came to nothing.
  Fault(Fault),
}

impl std::fmt::Display for Unasked {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Unasked::NoToken => write!(formatter, "no token: the token file is not named or holds nothing"),
      Unasked::Unreadable(fault) => write!(formatter, "the token file: {fault}"),
      Unasked::Fault(fault) => fault.fmt(formatter),
    }
  }
}

impl From<Fault> for Unasked {
  fn from(fault: Fault) -> Unasked {
    Unasked::Fault(fault)
  }
}

/// The token a file holds, its surrounding blanks left out; none where it holds nothing.
fn token_in(path: &Path) -> Result<String, Unasked> {
  let text = std::fs::read_to_string(path).map_err(Unasked::Unreadable)?;
  Some(text.trim().to_owned()).filter(|token| !token.is_empty()).ok_or(Unasked::NoToken)
}

impl VoiceEngine {
  /// The engine at `base`, its token in `token_file`.
  pub fn new(base: &str, token_file: Option<&Path>) -> VoiceEngine {
    VoiceEngine {
      service: Service::new(base),
      token_file: token_file.map(Path::to_owned),
    }
  }

  fn token(&self) -> Result<String, Unasked> {
    token_in(self.token_file.as_deref().ok_or(Unasked::NoToken)?)
  }

  /// The engine's event stream, from a cursor's query.
  pub async fn events(&self, query: &str) -> Result<Events, Unasked> {
    let token = self.token()?;
    Ok(self.service.events(&format!("/events{query}"), Some(&token)).await?)
  }

  /// A briefing's sound, whole.
  pub async fn sound(&self, briefing: i64) -> Result<Bytes, Unasked> {
    let token = self.token()?;
    Ok(self.service.bytes(&format!("/briefings/{briefing}/sound"), Some(&token)).await?)
  }
}
