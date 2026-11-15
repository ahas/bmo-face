//! The device token BMO gave this displayer (docs/face.md, 「Signing in」): one file in the data root,
//! the folder `0700` and the file `0600` before anything is written into them, replaced whole.

use std::fs::{self, DirBuilder, OpenOptions, Permissions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

const FILE: &str = "token";
const PARTIAL: &str = "token.partial";
const FOLDER_MODE: u32 = 0o700;
const FILE_MODE: u32 = 0o600;

/// The token file in a data root.
#[derive(Clone, Debug)]
pub struct TokenFile {
  data: PathBuf,
}

impl TokenFile {
  pub fn new(data: &Path) -> TokenFile {
    TokenFile {
      data: data.to_owned(),
    }
  }

  pub fn path(&self) -> PathBuf {
    self.data.join(FILE)
  }

  /// The token kept, or none where none is.
  pub fn read(&self) -> io::Result<Option<String>> {
    match fs::read_to_string(self.path()) {
      Ok(text) => Ok(Some(text.trim().to_owned()).filter(|token| !token.is_empty())),
      Err(fault) if fault.kind() == io::ErrorKind::NotFound => Ok(None),
      Err(fault) => Err(fault),
    }
  }

  /// Keeps a token: written beside the file with its mode set at creation, then renamed over it.
  pub fn write(&self, token: &str) -> io::Result<()> {
    DirBuilder::new().recursive(true).mode(FOLDER_MODE).create(&self.data)?;
    fs::set_permissions(&self.data, Permissions::from_mode(FOLDER_MODE))?;
    let partial = self.data.join(PARTIAL);
    let _ = fs::remove_file(&partial);
    let mut file = OpenOptions::new().write(true).create_new(true).mode(FILE_MODE).open(&partial)?;
    file.write_all(token.as_bytes())?;
    file.sync_all()?;
    fs::rename(&partial, self.path())
  }

  /// Forgets the token BMO refused.
  pub fn forget(&self) -> io::Result<()> {
    match fs::remove_file(self.path()) {
      Err(fault) if fault.kind() != io::ErrorKind::NotFound => Err(fault),
      _ => Ok(()),
    }
  }
}
