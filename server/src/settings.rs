//! The server's settings (docs/face.md, 「The server」): each named by its process's environment, else by
//! the environment file the install writes under the data root, `env/face.env`, read by a dotenv reader.
//! The server, `bmo-face address`, `bmo-face health` and `bmo-face console-reach` read the same two, so a
//! command run outside the unit finds the server where the unit started it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The port the server listens on where neither its environment nor its file names one.
pub const PORT: u16 = 3217;

/// The settings: the process's environment over the file's lines.
#[derive(Clone, Debug, Default)]
pub struct Settings {
  file: HashMap<String, String>,
  environment: HashMap<String, String>,
}

impl Settings {
  /// The settings of this process: its environment, and the file under the data root where it stands.
  pub fn load() -> Result<Self, String> {
    let environment: HashMap<String, String> = std::env::vars().collect();
    let file = environment_file(&environment);
    Self::from_parts(environment, &file)
  }

  /// The settings `environment` and the file at `file` give; a file that does not stand gives none, and one
  /// that does not read is refused.
  pub fn from_parts(environment: HashMap<String, String>, file: &Path) -> Result<Self, String> {
    let lines = match dotenvy::from_path_iter(file) {
      Ok(lines) => lines
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|why| format!("{} does not read: {why}", file.display()))?,
      Err(dotenvy::Error::Io(why)) if why.kind() == std::io::ErrorKind::NotFound => HashMap::new(),
      Err(why) => return Err(format!("{} does not read: {why}", file.display())),
    };
    Ok(Self {
      file: lines,
      environment,
    })
  }

  /// The value a setting is given, an empty one as none.
  pub fn named(&self, variable: &str) -> Option<String> {
    self
      .environment
      .get(variable)
      .or_else(|| self.file.get(variable))
      .filter(|value| !value.is_empty())
      .cloned()
  }

  /// The port the server listens on: `BMO_FACE_PORT`, else [`PORT`].
  pub fn port(&self) -> u16 {
    self.named("BMO_FACE_PORT").and_then(|port| port.parse().ok()).unwrap_or(PORT)
  }

  /// The home folder the defaults stand under.
  pub fn home(&self) -> PathBuf {
    PathBuf::from(self.environment.get("HOME").map_or("/", String::as_str))
  }
}

/// The install-written environment file: `env/face.env` under the data root `BMO_DATA_DIR` names, else
/// under `~/.local/share/bmo/data`.
fn environment_file(environment: &HashMap<String, String>) -> PathBuf {
  let data_root = environment
    .get("BMO_DATA_DIR")
    .filter(|named| !named.is_empty())
    .map(PathBuf::from)
    .unwrap_or_else(|| {
      PathBuf::from(environment.get("HOME").map_or("/", String::as_str)).join(".local/share/bmo/data")
    });
  data_root.join("env/face.env")
}
