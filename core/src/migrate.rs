//! Carrying a stored document of an older form into this one (docs/face.md, 「Preparing the document」),
//! with what it does not hold taken from the default. The server runs it once, as it starts; it is the
//! one place that reads an older form's group names.
//!
//! - Form 0 drew a part as an ellipse or a curve through points; each becomes a path of smooth nodes.
//! - Form 1 drew a face in five groups and the speaking mouth as a ladder of stages; its eyes, brows and
//!   cheeks become pairs, split at the screen's middle, and the stages give way to the default talking
//!   set.
//! - Form 2 talked in families of mouths picked by chance; each of its mouths becomes an anchor at the
//!   place measured from its drawing, and its faces take how far their eyes answer the voice and how far
//!   their talking mouth's corners lift.

use serde_json::{Map, Value, json};

use crate::check;
use crate::default;
use crate::document::{FORM, SCREEN, limits};
use crate::json::{number, same};
use crate::mouth::measure;
use crate::sets::{anchor, eye};

/// The form whose faces were drawn in pairs and whose talking set was families of mouths.
const FAMILIES_FORM: u64 = 2;

/// How near the places of two carried anchors of one plane may stand before the later is left out, as
/// the check holds them apart.
const APART: f64 = 0.005;

/// How far each dot eye of a carried face moves with the gaze.
const CARRIED_REACH: [f64; 2] = [2.2, 1.6];

/// The document in this form. A document already of it is given the default's faces, colours, tuning
/// and `idle` values it lacks; one of an older form is carried over whole, with the default's `idle`.
/// What is not a document at all is handed back untouched, for the check to refuse.
pub fn migrate(value: &Value) -> Value {
  let Some(document) = value.as_object() else {
    return value.clone();
  };
  let defaults = default::document().to_json();
  let form = document.get("form").and_then(Value::as_u64).unwrap_or(0);

  let faces = match document.get("faces") {
    Some(Value::Object(held)) => {
      let mut faces = defaults["faces"].as_object().cloned().unwrap_or_default();
      for (held_name, face) in held {
        let name = crate::names::present_name(held_name);
        let carried = match face {
          Value::Object(face) if form < FAMILIES_FORM => answering(name, &carry_face(face), &defaults),
          Value::Object(_) if form < FORM => answering(name, face, &defaults),
          other => other.clone(),
        };
        faces.insert(name.to_owned(), carried);
      }
      Some(Value::Object(faces))
    }
    other => other.cloned(),
  };
  let mut colors = defaults["colors"].as_object().cloned().unwrap_or_default();
  if let Some(Value::Object(held)) = document.get("colors") {
    for (key, color) in held {
      colors.insert(key.clone(), color.clone());
    }
  }

  let mut carried = document.clone();
  carried.insert("form".into(), json!(FORM));
  carried.insert("colors".into(), Value::Object(colors));
  // A key a document does not hold stays away, for the check to refuse.
  match faces {
    Some(faces) => carried.insert("faces".into(), faces),
    None => carried.remove("faces"),
  };
  let talk = match (form, document.get("talk")) {
    (FORM, Some(Value::Object(talk))) => Value::Object(tuned(talk, &defaults["talk"]["tuning"])),
    (FORM, talk) => talk.cloned().unwrap_or(Value::Null),
    (FAMILIES_FORM, Some(families)) => carry_families(families).unwrap_or_else(|| defaults["talk"].clone()),
    _ => defaults["talk"].clone(),
  };
  carried.remove("mouths");
  carried.insert("talk".into(), talk);
  let idle = match document.get("idle") {
    Some(Value::Object(held)) => Value::Object(overlaid(&defaults["idle"], held)),
    Some(other) => other.clone(),
    None => defaults["idle"].clone(),
  };
  carried.insert("idle".into(), idle);
  Value::Object(carried)
}

/// The default's values with every value held laid over them.
fn overlaid(defaults: &Value, held: &Map<String, Value>) -> Map<String, Value> {
  let mut values = defaults.as_object().cloned().unwrap_or_default();
  for (key, value) in held {
    values.insert(key.clone(), value.clone());
  }
  values
}

/// A talking set of this form given the tuning values it lacks from the default's.
fn tuned(talk: &Map<String, Value>, defaults: &Value) -> Map<String, Value> {
  let mut talk = talk.clone();
  if let Some(Value::Object(held)) = talk.get("tuning") {
    let tuning = overlaid(defaults, held);
    talk.insert("tuning".into(), Value::Object(tuning));
  }
  talk
}

/// The families of form 2 as anchors: each mouth, in order, at the place measured from its drawing,
/// leaving out one that draws nothing, that the check refuses, or that stands where an earlier one of its
/// plane stands, up to the most a set holds; with the default tuning. None where no mouth is left.
fn carry_families(families: &Value) -> Option<Value> {
  let mut anchors: Vec<crate::document::Anchor> = Vec::new();
  let mouths = families.as_array()?.iter().filter_map(|family| family["mouths"].as_array()).flatten();
  for mouth in mouths {
    let Ok(parts) = check::parts(&mouth["parts"]) else {
      continue;
    };
    let Some((at, teeth)) = measure(&parts) else {
      continue;
    };
    let crowded = anchors.iter().any(|other| {
      other.teeth == teeth && (other.at[0] - at[0]).abs() < APART && (other.at[1] - at[1]).abs() < APART
    });
    if !crowded && anchors.len() < limits::ANCHORS {
      anchors.push(anchor(parts));
    }
  }
  (!anchors.is_empty()).then(|| {
    json!({
      "anchors": anchors.iter().map(crate::document::Anchor::to_json).collect::<Vec<_>>(),
      "tuning": default::talk().tuning.to_json(),
    })
  })
}

/// A face of form 2 as a face of this form: its eyes answering the voice as the default face of its name
/// does where they draw as its eyes do, else as a dot eye does where they are solid and not at all where
/// they are strokes; and its talking mouth's corners lifted as the default face of its name lifts them.
fn answering(name: &str, face: &Value, defaults: &Value) -> Value {
  let mut face = face.clone();
  let own = &defaults["faces"][name];
  if let Some(eyes) = face.get_mut("eyes").and_then(Value::as_object_mut)
    && !eyes.contains_key("voice")
  {
    let drawn_alike = ["left", "right"].iter().all(|side| {
      let held = eyes.get(*side).cloned().unwrap_or(Value::Null);
      same(&held, &own["eyes"][*side])
    });
    let solid = ["left", "right"]
      .iter()
      .filter_map(|side| eyes.get(*side)?.as_array())
      .flatten()
      .any(|part| part["closed"] == json!(true) && !part["fill"].is_null());
    let voice = if drawn_alike && own.is_object() {
      own["eyes"]["voice"].clone()
    } else if solid {
      eye("dot").voice.to_json()
    } else {
      crate::document::EyeVoice::STILL.to_json()
    };
    eyes.insert("voice".into(), voice);
  }
  if let Some(face) = face.as_object_mut()
    && !face.contains_key("corners")
  {
    face.insert("corners".into(), own.get("corners").cloned().unwrap_or(json!(0)));
  }
  face
}

/// A face of form 1 or 0 as a face of form 2.
fn carry_face(face: &Map<String, Value>) -> Value {
  let group = |name: &str, paint: &str| match face.get(name) {
    None | Some(Value::Null) => json!([]),
    Some(parts) => carry_parts(parts, paint),
  };
  let eyes = group("eyes", "ink");
  let solid = eyes
    .as_array()
    .is_some_and(|parts| parts.iter().any(|part| part["closed"] == json!(true) && !part["fill"].is_null()));
  let mut eye_pair = pair(&eyes);
  eye_pair.insert("reach".into(), if solid { json!(CARRIED_REACH) } else { json!([0, 0]) });
  json!({
    "eyes": eye_pair,
    "brows": pair(&group("brows", "ink")),
    "blush": pair(&group("cheeks", "cheek")),
    "mouth": group("mouth", "ink"),
    "marks": group("marks", "ink"),
  })
}

/// A group's parts as a pair: the parts left of the screen's middle on the left, the rest on the right,
/// and the right left out where it is the left reflected.
fn pair(parts: &Value) -> Map<String, Value> {
  let mut pair = Map::new();
  let Some(parts) = parts.as_array() else {
    // Not a list: handed on for the check to refuse.
    pair.insert("left".into(), parts.clone());
    return pair;
  };
  let (left, right): (Vec<Value>, Vec<Value>) =
    parts.iter().cloned().partition(|part| centre_x(part) < SCREEN[0] / 2.0);
  let mirrored: Vec<Value> = left.iter().map(mirror).collect();
  let mirrors = right.len() == left.len() && {
    let mut unmatched = mirrored.clone();
    right
      .iter()
      .all(|own| match unmatched.iter().position(|reflected| draws_alike(own, reflected)) {
        Some(found) => {
          unmatched.remove(found);
          true
        }
        None => false,
      })
  };
  pair.insert("left".into(), Value::Array(left));
  pair.insert("right".into(), if mirrors { Value::Null } else { Value::Array(right) });
  pair
}

fn centre_x(part: &Value) -> f64 {
  let xs: Vec<f64> = part["nodes"]
    .as_array()
    .into_iter()
    .flatten()
    .filter_map(|node| node["anchor"][0].as_f64())
    .collect();
  if xs.is_empty() { 0.0 } else { xs.iter().sum::<f64>() / xs.len() as f64 }
}

/// A part of the document's shape reflected across the screen's middle.
fn mirror(part: &Value) -> Value {
  let mut mirrored = part.clone();
  if let Some(nodes) = mirrored.get_mut("nodes").and_then(Value::as_array_mut) {
    for node in nodes {
      if let Some(x) = node["anchor"][0].as_f64() {
        node["anchor"][0] = number(SCREEN[0] - x);
      }
      for handle in ["coming", "leaving"] {
        if let Some(x) = node[handle][0].as_f64() {
          node[handle][0] = number(-x);
        }
      }
    }
  }
  if let Some(inside) = mirrored.get_mut("inside").and_then(Value::as_array_mut) {
    for inner in inside.iter_mut() {
      *inner = mirror(inner);
    }
  }
  mirrored
}

/// Whether two parts draw the same path, whichever node each starts at and whichever way each runs.
fn draws_alike(own: &Value, other: &Value) -> bool {
  let rotations = |part: &Value| -> Vec<Value> {
    let count = part["nodes"].as_array().map_or(0, Vec::len);
    if part["closed"] != json!(true) || count == 0 {
      return vec![part.clone()];
    }
    (0..count)
      .map(|start| {
        let mut turned = part.clone();
        if let Some(nodes) = turned.get_mut("nodes").and_then(Value::as_array_mut) {
          nodes.rotate_left(start);
        }
        turned
      })
      .collect()
  };
  [other.clone(), reverse(other)]
    .iter()
    .flat_map(rotations)
    .any(|candidate| close(own, &candidate))
}

/// A part run the other way: its nodes in reverse, each node's handles swapped.
fn reverse(part: &Value) -> Value {
  let mut reversed = part.clone();
  if let Some(nodes) = reversed.get_mut("nodes").and_then(Value::as_array_mut) {
    nodes.reverse();
    for node in nodes {
      let coming = node["coming"].clone();
      node["coming"] = node["leaving"].clone();
      node["leaving"] = coming;
    }
  }
  reversed
}

/// Whether two values hold the same, numbers within a hair of each other.
fn close(left: &Value, right: &Value) -> bool {
  match (left, right) {
    (Value::Number(a), Value::Number(b)) => {
      (a.as_f64().unwrap_or(f64::NAN) - b.as_f64().unwrap_or(f64::NAN)).abs() < 1e-9
    }
    (Value::Array(a), Value::Array(b)) => a.len() == b.len() && a.iter().zip(b).all(|(a, b)| close(a, b)),
    (Value::Object(a), Value::Object(b)) => {
      a.len() == b.len() && a.iter().all(|(key, value)| b.get(key).is_some_and(|other| close(value, other)))
    }
    _ => left == right,
  }
}

fn carry_parts(parts: &Value, paint: &str) -> Value {
  match parts {
    Value::Array(parts) => Value::Array(
      parts
        .iter()
        .map(|part| match part {
          Value::Object(old) if old.contains_key("shape") => carry_shape(old, paint),
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

/// One part of form 0 as a path, in the paint its group drew it in: an ellipse as the four smooth nodes
/// that draw it, a curve as the smooth nodes of form 0's own spline through its points — each leaving
/// handle a sixth of the step between its neighbours, the end of an open curve standing for its own
/// missing neighbour — so it draws as it drew.
fn carry_shape(part: &Map<String, Value>, paint: &str) -> Value {
  let mut line = Map::new();
  line.insert("paint".into(), json!(paint));
  if let Some(width) = part.get("width") {
    line.insert("width".into(), width.clone());
  }

  if part.get("shape").and_then(Value::as_str) == Some("ellipse") {
    let nodes: Vec<Value> =
      crate::shapes::oval([float(part, "cx"), float(part, "cy")], float(part, "rx"), float(part, "ry"))
        .iter()
        .map(|node| on_screen(node.anchor, node.coming, node.leaving))
        .collect();
    return if truthy(part, "filled") {
      json!({ "nodes": nodes, "closed": true, "fill": paint, "line": null })
    } else {
      json!({ "nodes": nodes, "closed": true, "fill": null, "line": line })
    };
  }

  let points: Vec<[f64; 2]> = part
    .get("points")
    .and_then(Value::as_array)
    .map(|points| {
      points
        .iter()
        .filter_map(|point| Some([point.get(0)?.as_f64()?, point.get(1)?.as_f64()?]))
        .collect()
    })
    .unwrap_or_default();
  let closed = truthy(part, "closed");
  let count = points.len() as isize;
  let at = |index: isize| {
    let index = if closed { index.rem_euclid(count.max(1)) } else { index.clamp(0, count - 1) };
    points[index as usize]
  };
  let nodes: Vec<Value> = (0..count)
    .map(|index| {
      let [px, py] = at(index - 1);
      let [nx, ny] = at(index + 1);
      on_screen(
        points[index as usize],
        [(px - nx) / 6.0, (py - ny) / 6.0],
        [(nx - px) / 6.0, (ny - py) / 6.0],
      )
    })
    .collect();
  json!({
    "nodes": nodes,
    "closed": part.get("closed").cloned().unwrap_or(Value::Null),
    "fill": if closed && truthy(part, "filled") { json!(paint) } else { Value::Null },
    "line": line,
  })
}

/// A smooth node with each handle whose end falls off the screen shortened along itself to the edge.
fn on_screen(anchor: [f64; 2], coming: [f64; 2], leaving: [f64; 2]) -> Value {
  json!({
    "anchor": [number(anchor[0]), number(anchor[1])],
    "coming": shortened(anchor, coming),
    "leaving": shortened(anchor, leaving),
    "kind": "smooth",
  })
}

fn shortened([x, y]: [f64; 2], [dx, dy]: [f64; 2]) -> Value {
  let room = |at: f64, step: f64, size: f64| {
    if step < 0.0 {
      at / -step
    } else if step > 0.0 {
      (size - at) / step
    } else {
      f64::INFINITY
    }
  };
  let share = room(x, dx, SCREEN[0]).min(room(y, dy, SCREEN[1])).clamp(0.0, 1.0);
  if share == 1.0 {
    json!([number(dx), number(dy)])
  } else {
    json!([number(dx * share), number(dy * share)])
  }
}
