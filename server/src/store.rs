//! The expression document in the data root, and the displays that want to hear of each save
//! (docs/face.md, 「The expression document」).

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use bmo_face_core::check::{Refusal, check};
use bmo_face_core::json::{number, same};
use bmo_face_core::migrate::migrate;
use serde_json::{Value, json};
use tokio::sync::broadcast;

pub use bmo_face_core::json::document_text;

pub const FACES_FILE: &str = "faces.json";

/// What preparing the data root found, and what it did about it.
#[derive(Debug, PartialEq)]
pub enum Preparation {
  Created,
  Default,
  Upgraded,
  Kept,
  Broken { reason: String, moved_to: PathBuf },
}

#[derive(Debug, PartialEq)]
pub enum Saved {
  Saved(Value),
  /// One saved in between: the document standing now.
  Conflict(Value),
  Refused(Refusal),
}

/// The default document the core draws, `savedAt` zero included, as the server writes it.
pub fn default_document() -> &'static Value {
  static DEFAULT: OnceLock<Value> = OnceLock::new();
  DEFAULT.get_or_init(|| {
    let mut document = bmo_face_core::default::document().to_json();
    document["savedAt"] = json!(0);
    document
  })
}

/// The document as it may be kept, with its version: what the check hands back.
fn checked(value: &Value) -> Result<Value, Refusal> {
  check(value).map(|document| document.to_json())
}

pub struct FaceStore {
  path: PathBuf,
  now: Box<dyn Fn() -> u64 + Send + Sync>,
  // Saves one at a time, so the version each is based on is the one standing when it is written.
  saving: Mutex<()>,
  saves: broadcast::Sender<u64>,
}

impl FaceStore {
  pub fn new(root: PathBuf, now: impl Fn() -> u64 + Send + Sync + 'static) -> Self {
    FaceStore {
      path: root.join(FACES_FILE),
      now: Box::new(now),
      saving: Mutex::new(()),
      saves: broadcast::channel(16).0,
    }
  }

  /// Makes the stored document one the check passes, before anything is served: the default where
  /// there is none or none was ever saved, an older form carried into this one with its version
  /// unchanged, and a document the check still refuses moved aside, never deleted, for the default.
  pub fn prepare(&self) -> std::io::Result<Preparation> {
    let Ok(stored) = std::fs::read_to_string(&self.path) else {
      self.put(default_document())?;
      return Ok(Preparation::Created);
    };
    let Ok(parsed) = serde_json::from_str::<Value>(&stored) else {
      return self.set_aside("it is not JSON");
    };
    let saved_at = parsed
      .get("savedAt")
      .and_then(Value::as_f64)
      .filter(|at| at.fract() == 0.0 && *at >= 0.0);
    let Some(saved_at) = saved_at else {
      return self.set_aside("it names no version");
    };

    if saved_at == 0.0 {
      if !same(&parsed, default_document()) {
        self.put(default_document())?;
      }
      return Ok(Preparation::Default);
    }

    let mut document = match checked(&migrate(&parsed)) {
      Ok(checked) => checked,
      Err(refusal) => return self.set_aside(&refusal.describe()),
    };
    document["savedAt"] = number(saved_at);
    if same(&parsed, &document) {
      return Ok(Preparation::Kept);
    }
    self.put(&document)?;
    Ok(Preparation::Upgraded)
  }

  /// The document as saved. Nothing a reader does writes it.
  pub fn read(&self) -> Result<Value, String> {
    let text = std::fs::read_to_string(&self.path).map_err(|fault| fault.to_string())?;
    serde_json::from_str(&text).map_err(|fault| fault.to_string())
  }

  /// Saves what the editor sent, which names the version it was edited from. One saved in between is
  /// a conflict and nothing is written; a document that does not check out is refused.
  pub fn save(&self, sent: &Value, based_on: &Value) -> Result<Saved, String> {
    let _one = self.saving.lock().unwrap();
    let current = self.read()?;
    let standing = current["savedAt"].as_f64().unwrap_or(0.0);
    if based_on.as_f64() != Some(standing) {
      return Ok(Saved::Conflict(current));
    }
    let mut saved = match checked(sent) {
      Ok(checked) => checked,
      Err(refusal) => return Ok(Saved::Refused(refusal)),
    };
    let at = ((self.now)() as f64).max(standing + 1.0);
    saved["savedAt"] = number(at);
    self.put(&saved).map_err(|fault| fault.to_string())?;
    // A display gone is no reason to keep the save from the rest.
    let _ = self.saves.send(at as u64);
    Ok(Saved::Saved(saved))
  }

  /// Hears the version of every save from now on.
  pub fn subscribe(&self) -> broadcast::Receiver<u64> {
    self.saves.subscribe()
  }

  fn set_aside(&self, reason: &str) -> std::io::Result<Preparation> {
    let moved_to = PathBuf::from(format!("{}.broken-{}", self.path.display(), (self.now)()));
    std::fs::rename(&self.path, &moved_to)?;
    self.put(default_document())?;
    Ok(Preparation::Broken {
      reason: reason.to_owned(),
      moved_to,
    })
  }

  // Written beside and moved into place, so a reader never meets half a document.
  fn put(&self, document: &Value) -> std::io::Result<()> {
    let next = PathBuf::from(format!("{}.next", self.path.display()));
    std::fs::write(&next, document_text(document))?;
    std::fs::rename(next, &self.path)
  }
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  fn store(stored: Option<&str>) -> (tempfile::TempDir, FaceStore) {
    let dir = tempfile::tempdir().unwrap();
    if let Some(stored) = stored {
      std::fs::write(dir.path().join(FACES_FILE), stored).unwrap();
    }
    let store = FaceStore::new(dir.path().to_owned(), || 42);
    (dir, store)
  }

  fn stored(dir: &tempfile::TempDir) -> Value {
    serde_json::from_str(&std::fs::read_to_string(dir.path().join(FACES_FILE)).unwrap()).unwrap()
  }

  #[test]
  fn a_saved_document_of_an_older_form_is_carried_with_its_version_kept() {
    let old = json!({
      "colors": { "body": "#58bfae", "screen": "#c9f0cf", "ink": "#1f3b35", "cheek": "#f4a3b4" },
      "faces": { "idle": { "eyes": [{ "shape": "ellipse", "cx": 33, "cy": 30, "rx": 3.2, "ry": 3.8, "filled": true, "width": 1 }] } },
      "mouths": [[], []],
      "savedAt": 1700,
    });
    let (dir, store) = store(Some(&old.to_string()));
    assert_eq!(store.prepare().unwrap(), Preparation::Upgraded);
    let written = stored(&dir);
    assert_eq!(written["savedAt"], 1700);
    assert_eq!(written["form"], 2);
    assert_eq!(written["colors"]["ink"], "#1f3b35");
    assert_eq!(written["faces"]["idle"]["eyes"]["left"][0]["nodes"].as_array().unwrap().len(), 4);
    assert_eq!(written["talk"], default_document()["talk"]);
    assert_eq!(store.prepare().unwrap(), Preparation::Kept);
  }

  #[test]
  fn a_saved_document_the_check_still_refuses_is_moved_aside_for_the_default() {
    let mut refused = default_document().clone();
    refused["savedAt"] = json!(3);
    refused["talk"] = json!([]);
    let (dir, store) = store(Some(&refused.to_string()));
    assert_eq!(
      store.prepare().unwrap(),
      Preparation::Broken {
        reason: "families".into(),
        moved_to: dir.path().join(format!("{FACES_FILE}.broken-42"))
      }
    );
    assert!(same(&stored(&dir), default_document()));
  }

  #[test]
  fn with_no_document_the_default_is_written() {
    let (dir, store) = store(None);
    assert_eq!(store.prepare().unwrap(), Preparation::Created);
    assert_eq!(
      std::fs::read_to_string(dir.path().join(FACES_FILE)).unwrap(),
      document_text(default_document())
    );
  }

  #[test]
  fn a_document_never_saved_from_the_editor_is_the_default_drawn_now() {
    let mut old = default_document().clone();
    old["colors"]["body"] = json!("#010203");
    let (dir, store) = store(Some(&old.to_string()));
    assert_eq!(store.prepare().unwrap(), Preparation::Default);
    assert!(same(&stored(&dir), default_document()));
  }

  #[test]
  fn a_document_already_of_this_shape_is_left_alone() {
    let mut saved = default_document().clone();
    saved["savedAt"] = json!(9);
    let text = document_text(&saved);
    let (dir, store) = store(Some(&text));
    assert_eq!(store.prepare().unwrap(), Preparation::Kept);
    assert_eq!(std::fs::read_to_string(dir.path().join(FACES_FILE)).unwrap(), text);
  }

  #[test]
  fn a_file_that_is_not_json_or_names_no_version_is_moved_aside_never_deleted() {
    for (stored, reason) in
      [("{ half a docu", "it is not JSON"), (r#"{"savedAt":-1}"#, "it names no version")]
    {
      let (dir, store) = store(Some(stored));
      let moved_to = dir.path().join(format!("{FACES_FILE}.broken-42"));
      assert_eq!(
        store.prepare().unwrap(),
        Preparation::Broken {
          reason: reason.into(),
          moved_to: moved_to.clone()
        }
      );
      assert_eq!(std::fs::read_to_string(moved_to).unwrap(), stored);
      let mut names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
      names.sort();
      assert_eq!(names, [FACES_FILE.to_owned(), format!("{FACES_FILE}.broken-42")]);
    }
  }

  #[test]
  fn a_save_is_heard_names_its_version_and_refuses_a_stale_base() {
    let (_dir, store) = store(None);
    store.prepare().unwrap();
    let mut heard = store.subscribe();
    let document = default_document().clone();

    let Saved::Saved(saved) = store.save(&document, &json!(0)).unwrap() else {
      panic!("the save was not taken")
    };
    assert_eq!(saved["savedAt"], 42);
    assert_eq!(heard.try_recv().unwrap(), 42);

    // A clock behind the standing version still moves the version on.
    let Saved::Saved(again) = store.save(&document, &json!(42)).unwrap() else {
      panic!("the save was not taken")
    };
    assert_eq!(again["savedAt"], 43);
    assert!(
      matches!(store.save(&document, &json!(42)).unwrap(), Saved::Conflict(current) if current["savedAt"] == 43)
    );
    assert!(matches!(store.save(&json!({}), &json!(43)).unwrap(), Saved::Refused(_)));
  }
}
