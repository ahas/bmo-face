//! The expression document in the data root, and the displays that want to hear of each save
//! (docs/face.md, 「The expression document」).

use std::path::PathBuf;
use std::sync::Mutex;

use serde_json::Value;
use tokio::sync::broadcast;

use crate::faces::{check_document, default_document};
use crate::numbers::{number, same};
use crate::upgrade::upgrade_document;

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
  Refused(String),
}

/// The document as the server writes it to its file.
pub fn document_text(document: &Value) -> String {
  format!("{}\n", serde_json::to_string_pretty(document).expect("a document is plain data"))
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
  /// there is none or none was ever saved, the old shape turned into this one with its version
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

    let checked = match check_document(&upgrade_document(&parsed)) {
      Ok(checked) => checked,
      Err(reason) => return self.set_aside(&reason),
    };
    let mut document = checked;
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
    let mut saved = match check_document(sent) {
      Ok(checked) => checked,
      Err(reason) => return Ok(Saved::Refused(reason)),
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

  // A stored case of the shared upgrade fixture, rebuilt whole: the faces it names as the default's.
  fn rebuilt(case: &Value, faces_key: &str, named_key: &str, document: &Value) -> Value {
    let default = default_document();
    let mut whole = document.clone();
    let mut faces = serde_json::Map::new();
    let names = case.get(named_key).or_else(|| case["prepared"].get(named_key));
    for name in names.and_then(Value::as_array).into_iter().flatten() {
      let name = name.as_str().unwrap();
      faces.insert(name.into(), default["faces"][name].clone());
    }
    for (name, face) in document[faces_key].as_object().into_iter().flatten() {
      faces.insert(name.clone(), face.clone());
    }
    whole[faces_key] = Value::Object(faces);
    whole
  }

  #[test]
  fn a_stored_document_is_prepared_as_the_typescript_server_prepared_it() {
    let cases: Vec<Value> = serde_json::from_str(include_str!("../tests/fixtures/upgrades.json")).unwrap();
    for case in cases {
      let name = case["name"].as_str().unwrap();
      let stored_document = if case.get("storedDefaultFaces").is_some() {
        rebuilt(&case, "faces", "storedDefaultFaces", &case["stored"])
      } else {
        case["stored"].clone()
      };
      let (dir, store) = store(Some(&serde_json::to_string(&stored_document).unwrap()));
      let prepared = store.prepare().unwrap();
      match case["prepared"].get("error") {
        Some(error) => assert_eq!(
          prepared,
          Preparation::Broken {
            reason: error.as_str().unwrap().to_owned(),
            moved_to: dir.path().join(format!("{FACES_FILE}.broken-42")),
          },
          "{name}"
        ),
        None => {
          let expected = rebuilt(&case, "faces", "defaultFaces", &case["prepared"]["ok"]);
          let written = stored(&dir);
          assert!(same(&written, &expected), "{name}: {written}");
          let order: Vec<&str> = written["faces"].as_object().unwrap().keys().map(String::as_str).collect();
          let wanted: Vec<&str> = case["prepared"]["faceOrder"]
            .as_array()
            .unwrap()
            .iter()
            .map(|one| one.as_str().unwrap())
            .collect();
          assert_eq!(order, wanted, "{name}");
          let kind = if same(&stored_document, &expected) {
            Preparation::Kept
          } else {
            Preparation::Upgraded
          };
          assert_eq!(prepared, kind, "{name}");
        }
      }
    }
  }

  #[test]
  fn with_no_document_the_default_is_written() {
    let (dir, store) = store(None);
    assert_eq!(store.prepare().unwrap(), Preparation::Created);
    assert_eq!(
      std::fs::read_to_string(dir.path().join(FACES_FILE)).unwrap(),
      include_str!("../../tests/fixtures/default-faces.json")
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
