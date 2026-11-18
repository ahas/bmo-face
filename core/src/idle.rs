//! How long BMO rests before its face changes (docs/face.md, 「The faces」 and 「The expression
//! document」): when the face held after work gives way to the blank `idle` face, and when that gives way
//! to sleep. The document holds both under `idle`, each declared once here with its bounds and default.

use serde_json::{Value, json};

use crate::json::number;

/// The longest either stage may wait, in seconds: a day.
pub const MOST: f64 = 86_400.0;

/// The rest's stages, each in seconds of rest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Idle {
  /// When the held face gives way to the blank one.
  pub blank: f64,
  /// When the blank face gives way to sleep; never before `blank`.
  pub asleep: f64,
}

impl Default for Idle {
  fn default() -> Idle {
    Idle {
      blank: 600.0,
      asleep: 1_800.0,
    }
  }
}

/// An `idle` value that is not what its declaration allows: its key, and the bounds it was held to.
#[derive(Clone, Debug, PartialEq)]
pub struct Wrong {
  pub key: &'static str,
  pub least: f64,
  pub most: f64,
}

impl Idle {
  pub fn to_json(&self) -> Value {
    json!({ "blank": number(self.blank), "asleep": number(self.asleep) })
  }

  /// The stages a document holds, each within its bounds, or the first that is not.
  pub fn from_json(value: &Value) -> Result<Idle, Wrong> {
    let read = |key: &'static str, least: f64| {
      value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|read| (least..=MOST).contains(read))
        .ok_or(Wrong {
          key,
          least,
          most: MOST,
        })
    };
    let blank = read("blank", 0.0)?;
    let asleep = read("asleep", blank)?;
    Ok(Idle { blank, asleep })
  }
}
