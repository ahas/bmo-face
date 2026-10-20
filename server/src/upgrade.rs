//! Turning a stored document of the point-and-ellipse shape into the path shape, with what it does
//! not hold taken from the default (docs/face.md, 「The expression document」). The server runs it
//! once, as it starts; it is the one rule that reads a group's name, and the screen never carries it.

use serde_json::{Map, Value, json};

use crate::faces::{PART_GROUPS, SCREEN, default_document, oval_nodes, spline_nodes};
use crate::numbers::number;

/// The document in the path shape. A part of the old shape is converted; one already a path stays as
/// it is. Colours and faces the document does not hold come from the default. What is not a document
/// at all is handed back untouched, for the check to refuse.
pub fn upgrade_document(value: &Value) -> Value {
  let Some(document) = value.as_object() else {
    return value.clone();
  };
  let default = default_document();

  let faces = match document.get("faces") {
    Some(Value::Object(held)) => {
      let mut faces = default["faces"].as_object().cloned().unwrap_or_default();
      for (name, face) in held {
        let upgraded = match face {
          Value::Object(face) => upgrade_face(face),
          other => other.clone(),
        };
        faces.insert(name.clone(), upgraded);
      }
      Some(Value::Object(faces))
    }
    other => other.cloned(),
  };
  let mouths = match document.get("mouths") {
    Some(Value::Array(stages)) => {
      Some(Value::Array(stages.iter().map(|stage| upgrade_parts(stage, "ink")).collect()))
    }
    other => other.cloned(),
  };
  let mut colors = default["colors"].as_object().cloned().unwrap_or_default();
  if let Some(Value::Object(held)) = document.get("colors") {
    for (key, color) in held {
      colors.insert(key.clone(), color.clone());
    }
  }

  let mut upgraded = document.clone();
  upgraded.insert("colors".into(), Value::Object(colors));
  // A key a document does not hold stays away, as spreading an absent value writes nothing.
  match faces {
    Some(faces) => upgraded.insert("faces".into(), faces),
    None => upgraded.remove("faces"),
  };
  match mouths {
    Some(mouths) => upgraded.insert("mouths".into(), mouths),
    None => upgraded.remove("mouths"),
  };
  Value::Object(upgraded)
}

fn upgrade_face(face: &Map<String, Value>) -> Value {
  let mut upgraded = face.clone();
  for group in PART_GROUPS {
    let parts = face.get(group).filter(|parts| !parts.is_null()).cloned().unwrap_or_else(|| json!([]));
    upgraded.insert(group.into(), upgrade_parts(&parts, if group == "cheeks" { "cheek" } else { "ink" }));
  }
  Value::Object(upgraded)
}

fn upgrade_parts(parts: &Value, paint: &str) -> Value {
  match parts {
    Value::Array(parts) => Value::Array(
      parts
        .iter()
        .map(|part| match part {
          Value::Object(old) if old.contains_key("shape") => upgrade_part(old, paint),
          other => other.clone(),
        })
        .collect(),
    ),
    other => other.clone(),
  }
}

fn float(part: &Map<String, Value>, key: &str) -> f64 {
  part.get(key).and_then(Value::as_f64).unwrap_or(f64::NAN)
}

fn truthy(part: &Map<String, Value>, key: &str) -> bool {
  part.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// One old part as a path: its nodes, and the paint its group drew it in.
fn upgrade_part(part: &Map<String, Value>, paint: &str) -> Value {
  let mut line = Map::new();
  line.insert("paint".into(), json!(paint));
  if let Some(width) = part.get("width") {
    line.insert("width".into(), width.clone());
  }

  if part.get("shape").and_then(Value::as_str) == Some("ellipse") {
    let nodes: Vec<Value> =
      oval_nodes(float(part, "cx"), float(part, "cy"), float(part, "rx"), float(part, "ry"))
        .iter()
        .map(on_screen)
        .collect();
    return if truthy(part, "filled") {
      json!({ "nodes": nodes, "closed": true, "fill": paint, "line": null })
    } else {
      json!({ "nodes": nodes, "closed": true, "fill": null, "line": line })
    };
  }

  let points: Vec<(f64, f64)> = part
    .get("points")
    .and_then(Value::as_array)
    .map(|points| {
      points
        .iter()
        .filter_map(|point| Some((point.get(0)?.as_f64()?, point.get(1)?.as_f64()?)))
        .collect()
    })
    .unwrap_or_default();
  let closed = truthy(part, "closed");
  let nodes: Vec<Value> = spline_nodes(&points, closed).iter().map(on_screen).collect();
  json!({
    "nodes": nodes,
    "closed": part.get("closed").cloned().unwrap_or(Value::Null),
    "fill": if closed && truthy(part, "filled") { json!(paint) } else { Value::Null },
    "line": line,
  })
}

/// The node with each handle whose end falls off the screen shortened along itself to the edge.
fn on_screen(node: &Value) -> Value {
  let pair = |key: &str| (node[key][0].as_f64().unwrap_or(0.0), node[key][1].as_f64().unwrap_or(0.0));
  let anchor = pair("anchor");
  let mut kept = node.clone();
  for key in ["coming", "leaving"] {
    let (dx, dy) = shortened(anchor, pair(key));
    kept[key] = json!([dx, dy]);
  }
  kept
}

fn shortened((x, y): (f64, f64), (dx, dy): (f64, f64)) -> (Value, Value) {
  let room = |at: f64, step: f64, size: f64| {
    if step < 0.0 {
      at / -step
    } else if step > 0.0 {
      (size - at) / step
    } else {
      f64::INFINITY
    }
  };
  let share = room(x, dx, SCREEN.0).min(room(y, dy, SCREEN.1)).clamp(0.0, 1.0);
  if share == 1.0 {
    (number(dx), number(dy))
  } else {
    (number(dx * share), number(dy * share))
  }
}
