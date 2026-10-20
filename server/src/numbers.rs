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

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  #[test]
  fn a_whole_number_is_written_without_a_point_and_minus_zero_as_zero() {
    assert_eq!(number(25.0).to_string(), "25");
    assert_eq!(number(-0.0).to_string(), "0");
    assert_eq!(number(36.2).to_string(), "36.2");
  }

  #[test]
  fn documents_are_the_same_whatever_their_key_order_and_number_form() {
    assert!(same(&json!({ "a": 1, "b": [2.0, 3] }), &json!({ "b": [2, 3.0], "a": 1.0 })));
    assert!(!same(&json!({ "a": 1 }), &json!({ "a": 1, "b": 2 })));
  }
}
