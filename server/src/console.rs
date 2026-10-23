//! The console screen's side of the server (docs/face.md, 「The console screen」): the manifest a
//! console reads, the built bundle it loads, and the guard every console route stands behind.

use std::path::PathBuf;

use axum::http::{HeaderMap, StatusCode, header};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The manifest's fields that do not follow from the built bundle, stated once for the server and
/// the bundle.
const FIXED: &str = include_str!("../../console-manifest.json");

/// The route a console reads the manifest at.
pub const MANIFEST_ROUTE: &str = "/console-manifest";

/// The headers a console sets on everything it relays: the reader's owner key and role.
pub const OWNER_KEY_HEADER: &str = "owner-key";
pub const OWNER_ROLE_HEADER: &str = "owner-role";

/// The roles a reader holds, the one holding more first: an administrator holds every role a member
/// holds.
const ROLES: [&str; 2] = ["administrator", "member"];

/// The built bundle's files in its folder.
const MODULE_FILE: &str = "bundle.js";
const STYLESHEET_FILE: &str = "bundle.css";
const WORDS_FILE: &str = "words.json";

/// How many hexadecimal digits of a file's SHA-256 its path carries.
const HASH_DIGITS: usize = 16;

/// What a console route asks of a call beyond the console's credential.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Need {
  /// The credential alone: the manifest and the words, which a console reads for no reader.
  Credential,
  /// A reader the manifest's role allows, with the owner key where the manifest asks for it.
  Reader,
}

/// A built file a console route answers, with its media type.
pub struct Built {
  pub bytes: Vec<u8>,
  pub content_type: &'static str,
}

pub struct Console {
  /// The built bundle's folder, read on every request so a bundle built again is answered at once.
  dir: PathBuf,
  /// The credential a console calls with; with none, every console route is refused.
  token: Option<String>,
  fixed: Value,
}

/// The path a built file is answered at: its name with the first digits of its bytes' SHA-256, so
/// the path changes whenever the bytes do.
pub fn hashed_path(prefix: &str, file: &str, bytes: &[u8]) -> String {
  let digest = Sha256::digest(bytes);
  let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
  let (stem, extension) = file.rsplit_once('.').unwrap_or((file, ""));
  format!("{prefix}/{stem}.{}.{extension}", &hex[..HASH_DIGITS])
}

/// Whether a reader holding one role may open a screen needing another.
pub fn role_allows(held: &str, needed: &str) -> bool {
  let rank = |role: &str| ROLES.iter().position(|each| *each == role);
  matches!((rank(held), rank(needed)), (Some(held), Some(needed)) if held <= needed)
}

impl Console {
  pub fn new(dir: PathBuf, token: Option<String>) -> Console {
    let fixed = serde_json::from_str(FIXED).expect("console-manifest.json is a JSON object");
    Console {
      dir,
      token: token.filter(|token| !token.is_empty()),
      fixed,
    }
  }

  /// The path of the service's API every call the screen makes stands under.
  pub fn api_prefix(&self) -> &str {
    self.fixed["apiPrefix"].as_str().unwrap_or("/")
  }

  /// Whether a route is one of the console's: the manifest, or anything under the API prefix.
  pub fn owns(&self, route: &str) -> bool {
    let prefix = self.api_prefix();
    route == MANIFEST_ROUTE || route == prefix || route.starts_with(&format!("{prefix}/"))
  }

  /// The console's credential first, then the role and the owner key where the route needs a
  /// reader: a 401 for the first, a 403 for the others.
  pub fn guard(&self, headers: &HeaderMap, need: Need) -> Result<(), StatusCode> {
    let header = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
    let bearer = header(header::AUTHORIZATION.as_str())
      .and_then(|value| value.split_once(' '))
      .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
      .map(|(_, token)| token.trim());
    let credited = match (&self.token, bearer) {
      // Compared as digests, so the comparison takes no longer for a closer guess.
      (Some(token), Some(sent)) => Sha256::digest(token.as_bytes()) == Sha256::digest(sent.as_bytes()),
      _ => false,
    };
    if !credited {
      return Err(StatusCode::UNAUTHORIZED);
    }
    if need == Need::Credential {
      return Ok(());
    }
    let needed = self.fixed["role"].as_str().unwrap_or(ROLES[0]);
    if !header(OWNER_ROLE_HEADER).is_some_and(|held| role_allows(held.trim(), needed)) {
      return Err(StatusCode::FORBIDDEN);
    }
    let owner_key = self.fixed["ownerKey"].as_bool().unwrap_or(false);
    if owner_key && header(OWNER_KEY_HEADER).is_none_or(|key| key.trim().is_empty()) {
      return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
  }

  /// The manifest as a console reads it: the fixed fields with the built module's, stylesheet's and
  /// words' paths. None while no module and no words are built, so a console leaves the face out.
  pub async fn manifest(&self) -> Option<Value> {
    let prefix = self.api_prefix();
    let module = tokio::fs::read(self.dir.join(MODULE_FILE)).await.ok()?;
    if !self.dir.join(WORDS_FILE).is_file() {
      return None;
    }
    let mut manifest = self.fixed.clone();
    let fields = manifest.as_object_mut()?;
    fields.insert("bundle".into(), hashed_path(prefix, MODULE_FILE, &module).into());
    if let Ok(stylesheet) = tokio::fs::read(self.dir.join(STYLESHEET_FILE)).await {
      fields.insert("stylesheet".into(), hashed_path(prefix, STYLESHEET_FILE, &stylesheet).into());
    }
    fields.insert("words".into(), self.words_route().into());
    Some(manifest)
  }

  /// The route the words are answered at.
  pub fn words_route(&self) -> String {
    format!("{}/words", self.api_prefix())
  }

  /// The words as built.
  pub async fn words(&self) -> Option<Vec<u8>> {
    tokio::fs::read(self.dir.join(WORDS_FILE)).await.ok()
  }

  /// The module or the stylesheet a route names, while the path names the bytes built now.
  pub async fn built(&self, route: &str) -> Option<Built> {
    let prefix = self.api_prefix();
    for (file, content_type) in [
      (MODULE_FILE, "text/javascript;charset=utf-8"),
      (STYLESHEET_FILE, "text/css;charset=utf-8"),
    ] {
      let Ok(bytes) = tokio::fs::read(self.dir.join(file)).await else {
        continue;
      };
      if hashed_path(prefix, file, &bytes) == route {
        return Some(Built {
          bytes,
          content_type,
        });
      }
    }
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_path_follows_the_bytes() {
    let first = hashed_path("/console", "bundle.js", b"export default {}");
    assert!(first.starts_with("/console/bundle.") && first.ends_with(".js"));
    assert_eq!(first.len(), "/console/bundle..js".len() + HASH_DIGITS);
    assert_eq!(first, hashed_path("/console", "bundle.js", b"export default {}"));
    assert_ne!(first, hashed_path("/console", "bundle.js", b"export default { routes: [] }"));
  }

  #[test]
  fn an_administrator_holds_every_role_a_member_holds() {
    assert!(role_allows("administrator", "member"));
    assert!(role_allows("member", "member"));
    assert!(role_allows("administrator", "administrator"));
    assert!(!role_allows("member", "administrator"));
    assert!(!role_allows("guest", "member"));
  }

  #[test]
  fn the_fixed_fields_name_the_console_routes() {
    let console = Console::new(PathBuf::from("/nowhere"), None);
    assert_eq!(console.api_prefix(), "/console");
    assert!(console.owns(MANIFEST_ROUTE));
    assert!(console.owns("/console/faces"));
    assert!(!console.owns("/consoles"));
    assert!(!console.owns("/api/faces"));
  }
}
