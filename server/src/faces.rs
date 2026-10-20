//! The expression document as the server checks it (docs/face.md, 「The expression document」): the
//! same rules and the same Korean sentences as the screen's `src/faces.ts`, held to the shared
//! fixtures both test suites read. The default is the one `src/faces.ts` draws, as
//! `tests/fixtures/default-faces.json` holds it.

use std::sync::OnceLock;

use serde_json::{Map, Value, json};

use crate::numbers::number;

/// The emotions BMO speaks a briefing with, and what it is doing; one face each.
pub const FACE_NAMES: [&str; 13] = [
  "기본",
  "기쁨",
  "슬픔",
  "놀람",
  "화남",
  "졸림",
  "폭소",
  "걱정",
  "아픔",
  "안간힘",
  "writing",
  "thinking",
  "idle",
];

/// The face every missing face falls back to, which a document must therefore hold.
pub const FALLBACK_FACE: &str = "기본";

/// The groups a face is drawn in, in drawing order.
pub const PART_GROUPS: [&str; 5] = ["eyes", "brows", "cheeks", "mouth", "marks"];

/// The paints a part is filled and lined with; each is a colour of the document.
pub const PAINTS: [&str; 6] = ["ink", "light", "mouth", "tongue", "cheek", "tear"];

/// The screen's own coordinates: every anchor and every handle end stands inside this box.
pub const SCREEN: (f64, f64) = (100.0, 75.0);

const PARTS_PER_GROUP: usize = 16;
const PARTS_INSIDE: usize = 8;
const MIN_NODES: usize = 2;
const MAX_NODES: usize = 32;
const MIN_STAGES: usize = 2;
const MAX_STAGES: usize = 8;
const MIN_WIDTH: f64 = 0.2;
const MAX_WIDTH: f64 = 10.0;

/// Room for the last bit of floating point in a handle end computed from an anchor on the edge.
const EDGE: f64 = 1e-9;

/// The default document, `savedAt` zero included, as the server writes it.
pub fn default_document() -> &'static Value {
  static DEFAULT: OnceLock<Value> = OnceLock::new();
  DEFAULT.get_or_init(|| {
    serde_json::from_str(include_str!("../../tests/fixtures/default-faces.json"))
      .expect("the default document is JSON")
  })
}

fn is_paint(value: &Value) -> bool {
  value.as_str().is_some_and(|paint| PAINTS.contains(&paint))
}

fn point(value: &Value) -> Option<(f64, f64)> {
  match value.as_array()?.as_slice() {
    [x, y] => Some((x.as_f64()?, y.as_f64()?)),
    _ => None,
  }
}

fn within((x, y): (f64, f64)) -> bool {
  x >= -EDGE && x <= SCREEN.0 + EDGE && y >= -EDGE && y <= SCREEN.1 + EDGE
}

fn check_node(value: &Value, at: &str) -> Result<Value, String> {
  let wrong = || format!("{at}: 노드의 값이 맞지 않아요.");
  let node = value.as_object().ok_or_else(wrong)?;
  let field = |name: &str| node.get(name).and_then(point);
  let (Some(anchor), Some(coming), Some(leaving)) = (field("anchor"), field("coming"), field("leaving"))
  else {
    return Err(wrong());
  };
  let kind = node
    .get("kind")
    .and_then(Value::as_str)
    .filter(|kind| *kind == "smooth" || *kind == "corner")
    .ok_or_else(wrong)?;
  let ends = [
    anchor,
    (anchor.0 + coming.0, anchor.1 + coming.1),
    (anchor.0 + leaving.0, anchor.1 + leaving.1),
  ];
  if !ends.into_iter().all(within) {
    return Err(format!("{at}: 노드나 손잡이가 화면 밖으로 나가요."));
  }
  Ok(
    json!({ "anchor": node["anchor"], "coming": node["coming"], "leaving": node["leaving"], "kind": kind }),
  )
}

fn check_line(value: Option<&Value>, at: &str) -> Result<Value, String> {
  if value == Some(&Value::Null) {
    return Ok(Value::Null);
  }
  let line = value.and_then(Value::as_object);
  let (Some(paint), Some(width)) = (
    line.and_then(|line| line.get("paint")).filter(|paint| is_paint(paint)),
    line.and_then(|line| line.get("width")),
  ) else {
    return Err(format!("{at}: 선의 값이 맞지 않아요."));
  };
  let Some(thickness) = width.as_f64() else {
    return Err(format!("{at}: 선의 값이 맞지 않아요."));
  };
  if !(MIN_WIDTH..=MAX_WIDTH).contains(&thickness) {
    return Err(format!("{at}: 선 굵기는 {MIN_WIDTH}에서 {MAX_WIDTH} 사이여야 해요."));
  }
  Ok(json!({ "paint": paint, "width": width }))
}

fn check_part(value: &Value, at: &str, nested: bool) -> Result<Value, String> {
  let part = value.as_object().ok_or_else(|| format!("{at}: 모양이 아니에요."))?;
  let closed = part
    .get("closed")
    .and_then(Value::as_bool)
    .ok_or_else(|| format!("{at}: 닫힌 선인지가 맞지 않아요."))?;
  let nodes = part
    .get("nodes")
    .and_then(Value::as_array)
    .filter(|nodes| (MIN_NODES..=MAX_NODES).contains(&nodes.len()))
    .ok_or_else(|| format!("{at}: 노드는 {MIN_NODES}개에서 {MAX_NODES}개 사이여야 해요."))?;
  let mut checked = Vec::with_capacity(nodes.len());
  for (index, node) in nodes.iter().enumerate() {
    checked.push(check_node(node, &format!("{at} 노드 {}", index + 1))?);
  }

  let fill = part.get("fill");
  if fill != Some(&Value::Null) && !fill.is_some_and(is_paint) {
    return Err(format!("{at}: 채우기 칠이 맞지 않아요."));
  }
  let fill = fill.cloned().unwrap_or(Value::Null);
  let line = check_line(part.get("line"), at)?;
  if fill.is_null() && line.is_null() {
    return Err(format!("{at}: 채우기나 선 가운데 하나는 있어야 해요."));
  }
  if !fill.is_null() && !closed {
    return Err(format!("{at}: 열린 선은 채울 수 없어요."));
  }

  let mut shape = Map::new();
  shape.insert("nodes".into(), Value::Array(checked));
  shape.insert("closed".into(), Value::Bool(closed));
  shape.insert("fill".into(), fill);
  shape.insert("line".into(), line);

  let Some(inside) = part.get("inside") else {
    return Ok(Value::Object(shape));
  };
  if nested {
    return Err(format!("{at}: 안에 넣은 모양 안에는 다시 넣을 수 없어요."));
  }
  if !closed {
    return Err(format!("{at}: 열린 선 안에는 모양을 넣을 수 없어요."));
  }
  let inside = inside
    .as_array()
    .filter(|inside| inside.len() <= PARTS_INSIDE)
    .ok_or_else(|| format!("{at}: 안에 넣는 모양은 {PARTS_INSIDE}개까지예요."))?;
  let mut within_part = Vec::with_capacity(inside.len());
  for (index, inner) in inside.iter().enumerate() {
    within_part.push(check_part(inner, &format!("{at} 안 {}", index + 1), true)?);
  }
  if !within_part.is_empty() {
    shape.insert("inside".into(), Value::Array(within_part));
  }
  Ok(Value::Object(shape))
}

fn check_parts(value: &Value, at: &str) -> Result<Value, String> {
  let parts = value
    .as_array()
    .filter(|parts| parts.len() <= PARTS_PER_GROUP)
    .ok_or_else(|| format!("{at}: 모양 목록이 맞지 않아요."))?;
  let mut checked = Vec::with_capacity(parts.len());
  for (index, part) in parts.iter().enumerate() {
    checked.push(check_part(part, &format!("{at} {}", index + 1), false)?);
  }
  Ok(Value::Array(checked))
}

fn check_face(value: &Value, name: &str) -> Result<Value, String> {
  let face = value.as_object().ok_or_else(|| format!("{name} 표정이 맞지 않아요."))?;
  let mut checked = Map::new();
  for group in PART_GROUPS {
    let parts = face.get(group).filter(|parts| !parts.is_null()).cloned().unwrap_or_else(|| json!([]));
    checked.insert(group.into(), check_parts(&parts, &format!("{name} {group}"))?);
  }
  Ok(Value::Object(checked))
}

/// The document as it may be saved — its colours, faces and mouth stages, and nothing else — or the
/// one sentence saying why not. `savedAt` is not checked here.
pub fn check_document(value: &Value) -> Result<Value, String> {
  let shape = || "표정 문서의 모양이 맞지 않아요.".to_owned();
  let document = value.as_object().ok_or_else(shape)?;
  let (Some(colors), Some(faces), Some(mouths)) = (
    document.get("colors").and_then(Value::as_object),
    document.get("faces").and_then(Value::as_object),
    document.get("mouths").and_then(Value::as_array),
  ) else {
    return Err(shape());
  };

  let mut checked_colors = Map::new();
  for key in ["body", "screen"].into_iter().chain(PAINTS) {
    let color = colors
      .get(key)
      .and_then(Value::as_str)
      .filter(|color| {
        color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|one| one.is_ascii_hexdigit())
      })
      .ok_or_else(|| format!("색 {key}는 #rrggbb 꼴이어야 해요."))?;
    checked_colors.insert(key.into(), Value::String(color.to_ascii_lowercase()));
  }

  let mut checked_faces = Map::new();
  for (name, face) in faces {
    if !FACE_NAMES.contains(&name.as_str()) {
      return Err(format!("모르는 표정 이름이에요: {name}"));
    }
    checked_faces.insert(name.clone(), check_face(face, name)?);
  }
  if !checked_faces.contains_key(FALLBACK_FACE) {
    return Err(format!("{FALLBACK_FACE} 표정은 지울 수 없어요."));
  }

  if !(MIN_STAGES..=MAX_STAGES).contains(&mouths.len()) {
    return Err(format!("입 모양 단계는 {MIN_STAGES}개에서 {MAX_STAGES}개 사이여야 해요."));
  }
  let mut checked_mouths = Vec::with_capacity(mouths.len());
  for (index, stage) in mouths.iter().enumerate() {
    checked_mouths.push(check_parts(stage, &format!("입 모양 {}단계", index + 1))?);
  }

  Ok(json!({ "colors": checked_colors, "faces": checked_faces, "mouths": checked_mouths }))
}

/// One node as the document holds it.
fn node(anchor: (f64, f64), coming: (f64, f64), leaving: (f64, f64)) -> Value {
  json!({
    "anchor": [number(anchor.0), number(anchor.1)],
    "coming": [number(coming.0), number(coming.1)],
    "leaving": [number(leaving.0), number(leaving.1)],
    "kind": "smooth",
  })
}

/// Smooth nodes for the spline through `points`: each leaving handle a sixth of the step from the
/// point before to the point after, and the coming handle that step reversed. The end of an open run
/// stands for its own missing neighbour; a closed run's neighbours wrap round.
pub fn spline_nodes(points: &[(f64, f64)], closed: bool) -> Vec<Value> {
  let count = points.len() as isize;
  let at = |index: isize| {
    let index = if closed {
      (index + count).rem_euclid(count.max(1))
    } else {
      index.clamp(0, count - 1)
    };
    points[index as usize]
  };
  (0..count)
    .map(|index| {
      let (x, y) = points[index as usize];
      let (px, py) = at(index - 1);
      let (nx, ny) = at(index + 1);
      node((x, y), ((px - nx) / 6.0, (py - ny) / 6.0), ((nx - px) / 6.0, (ny - py) / 6.0))
    })
    .collect()
}

/// How far along a quarter of an ellipse its handles reach, as a share of the radius.
pub const OVAL_HANDLE: f64 = (4.0 / 3.0) * (std::f64::consts::SQRT_2 - 1.0);

/// Four smooth nodes at the ends of the radii, clockwise from the right, that draw the ellipse.
pub fn oval_nodes(cx: f64, cy: f64, rx: f64, ry: f64) -> Vec<Value> {
  let (hx, hy) = (rx * OVAL_HANDLE, ry * OVAL_HANDLE);
  vec![
    node((cx + rx, cy), (0.0, -hy), (0.0, hy)),
    node((cx, cy + ry), (hx, 0.0), (-hx, 0.0)),
    node((cx - rx, cy), (0.0, hy), (0.0, -hy)),
    node((cx, cy - ry), (-hx, 0.0), (hx, 0.0)),
  ]
}

#[cfg(test)]
mod tests {
  use serde_json::Value;

  use super::*;
  use crate::numbers::same;

  #[test]
  fn the_server_accepts_and_refuses_what_the_screens_check_does() {
    let cases: Vec<Value> = serde_json::from_str(include_str!("../../tests/fixtures/checks.json")).unwrap();
    assert!(cases.len() > 25);
    for case in cases {
      let name = case["name"].as_str().unwrap();
      let verdict = check_document(&case["document"]);
      match (&verdict, case["verdict"].get("ok"), case["verdict"].get("error")) {
        (Ok(checked), Some(ok), None) => assert!(same(checked, ok), "{name}: {checked}"),
        (Err(said), None, Some(error)) => assert_eq!(said, error.as_str().unwrap(), "{name}"),
        _ => panic!("{name}: the server said {verdict:?}, the screen's check {}", case["verdict"]),
      }
    }
  }

  #[test]
  fn the_default_passes_its_own_check_as_it_stands() {
    let default = default_document();
    let checked = check_document(default).unwrap();
    for key in ["colors", "faces", "mouths"] {
      assert!(same(&checked[key], &default[key]), "{key}");
    }
    assert_eq!(default["savedAt"], 0);
    assert_eq!(
      checked["faces"].as_object().unwrap().keys().collect::<Vec<_>>(),
      FACE_NAMES.iter().collect::<Vec<_>>()
    );
  }

  #[test]
  fn an_oval_is_four_smooth_nodes_at_the_ends_of_its_radii() {
    let nodes = oval_nodes(33.0, 30.0, 3.2, 3.8);
    let anchors: Vec<&Value> = nodes.iter().map(|node| &node["anchor"]).collect();
    assert_eq!(
      anchors,
      [&json!([36.2, 30]), &json!([33, 33.8]), &json!([29.8, 30]), &json!([33, 26.2])]
    );
    assert!((nodes[1]["coming"][0].as_f64().unwrap() - 3.2 * OVAL_HANDLE).abs() < 1e-12);
  }

  #[test]
  fn the_spline_leaves_each_node_by_a_sixth_of_the_step_between_its_neighbours() {
    let nodes = spline_nodes(&[(10.0, 10.0), (16.0, 22.0), (40.0, 10.0)], false);
    assert_eq!(nodes[1]["leaving"], json!([5, 0]));
    assert_eq!(nodes[0]["coming"], json!([-1, -2]));
    let closed = spline_nodes(&[(10.0, 10.0), (16.0, 22.0), (40.0, 10.0)], true);
    assert_eq!(closed[0]["leaving"], json!([-4, 2]));
  }
}
