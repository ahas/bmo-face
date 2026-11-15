//! Numbers in the expression document as its other readers see them: a JavaScript number has no
//! separate integer, so a whole number is written without a point and two documents are the same
//! where their numbers are equal, however each was written.

use serde_json::{Number, Value};

/// A number as the document writes it: whole numbers as integers, the rest as they are.
pub fn number(value: f64) -> Value {
  // Adding zero turns −0 into 0, as JavaScript writes it.
  let value = value + 0.0;
  if value.fract() == 0.0 && value.abs() < 9_007_199_254_740_992.0 {
    return Value::from(value as i64);
  }
  Number::from_f64(value).map_or(Value::Null, Value::Number)
}

/// A number as a drawing writes it: to a thousandth of a unit, far below a pixel of any screen.
pub fn rounded(value: f64) -> Value {
  number((value * 1000.0).round() / 1000.0)
}

/// Whether two documents hold the same thing: the same keys in any order, and equal numbers.
pub fn same(left: &Value, right: &Value) -> bool {
  match (left, right) {
    (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
    (Value::Array(a), Value::Array(b)) => a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same(a, b)),
    (Value::Object(a), Value::Object(b)) => {
      a.len() == b.len() && a.iter().all(|(key, value)| b.get(key).is_some_and(|other| same(value, other)))
    }
    _ => left == right,
  }
}

/// The document as it is written to its file and handed to a reader.
pub fn document_text(document: &Value) -> String {
  format!("{}\n", serde_json::to_string_pretty(document).expect("a document is plain data"))
}
