//! The check every document goes through before it is kept or drawn (docs/face.md, 「Saving is
//! checked」): a document of this form, every face, part and drawing of the talking set well formed and
//! within the screen and every tuning value within its bounds, or the one refusal saying where and why. A
//! refusal is a code with the place it was met at; the screen words it from its language pack.

use serde_json::{Map, Value, json};

use crate::document::{
  Anchor, Colors, EyeVoice, Eyes, FORM, Face, Kind, Line, Node, Paint, Pair, Part, SCREEN, Talk, limits,
};
use crate::geometry::Point;
use crate::json::number;
use crate::names::{FALLBACK, is_face_name};
use crate::tuning::Tuning;

/// Where in a document a refusal was met, from the outside in.
#[derive(Clone, Debug, PartialEq)]
pub enum Place {
  Color(String),
  Face(String),
  Element(&'static str),
  Side(&'static str),
  Part(usize),
  Inside(usize),
  Node(usize),
  Anchor(usize),
  Tuning(&'static str),
}

impl Place {
  fn to_json(&self) -> Value {
    match self {
      Place::Color(key) => json!({ "color": key }),
      Place::Face(name) => json!({ "face": name }),
      Place::Element(name) => json!({ "element": name }),
      Place::Side(side) => json!({ "side": side }),
      // Counted from one, as a person counts them.
      Place::Part(index) => json!({ "part": index + 1 }),
      Place::Inside(index) => json!({ "inside": index + 1 }),
      Place::Node(index) => json!({ "node": index + 1 }),
      Place::Anchor(index) => json!({ "anchor": index + 1 }),
      Place::Tuning(key) => json!({ "tuning": key }),
    }
  }
}

/// Why a document is refused.
#[derive(Clone, Debug, PartialEq)]
pub enum Why {
  /// It is no document of this form.
  NotADocument,
  /// A colour is not `#rrggbb`.
  Color,
  /// A face of a name nobody sends.
  UnknownFace(String),
  /// The fallback face is missing.
  NoFallback,
  /// A face is not a face of elements.
  NotAFace,
  /// An element is not a pair of sides.
  NotAPair,
  /// How far the eyes move with the gaze is not two numbers within the bound.
  Reach,
  /// How far the eyes answer the voice is not three shares within the bound.
  EyeVoice,
  /// How far the talking mouth's corners lift is not a number from −1 to 1.
  Corners,
  /// A list of parts is no list, or holds more than it may.
  Parts,
  NotAPart,
  /// A part does not say whether it is closed.
  Closedness,
  NodeCount,
  NodeValue,
  OffScreen,
  FillPaint,
  LineValue,
  LineWidth,
  NeitherFillNorLine,
  OpenFilled,
  NestedInside,
  InsideOpen,
  InsideCount,
  /// The talking set is no set of anchors and tuning, or holds too few or too many anchors.
  Anchors,
  /// An anchor's place is not two numbers from nought to one, or it does not say whether its teeth
  /// stand together.
  AnchorValue,
  /// Two anchors of one plane stand at one place.
  AnchorPlace,
  /// A tuning value is missing or outside its bounds.
  Tuning {
    least: f64,
    most: f64,
  },
}

impl Why {
  fn code(&self) -> &'static str {
    match self {
      Why::NotADocument => "not-a-document",
      Why::Color => "color",
      Why::UnknownFace(_) => "unknown-face",
      Why::NoFallback => "no-fallback",
      Why::NotAFace => "not-a-face",
      Why::NotAPair => "not-a-pair",
      Why::Reach => "reach",
      Why::EyeVoice => "eye-voice",
      Why::Corners => "corners",
      Why::Parts => "parts",
      Why::NotAPart => "not-a-part",
      Why::Closedness => "closedness",
      Why::NodeCount => "node-count",
      Why::NodeValue => "node-value",
      Why::OffScreen => "off-screen",
      Why::FillPaint => "fill-paint",
      Why::LineValue => "line-value",
      Why::LineWidth => "line-width",
      Why::NeitherFillNorLine => "neither-fill-nor-line",
      Why::OpenFilled => "open-filled",
      Why::NestedInside => "nested-inside",
      Why::InsideOpen => "inside-open",
      Why::InsideCount => "inside-count",
      Why::Anchors => "anchors",
      Why::AnchorValue => "anchor-value",
      Why::AnchorPlace => "anchor-place",
      Why::Tuning { .. } => "tuning",
    }
  }

  /// The limits the refusal names, for its words.
  fn bounds(&self) -> Option<(f64, f64)> {
    match self {
      Why::Parts => Some((0.0, limits::PARTS as f64)),
      Why::NodeCount => Some((limits::MIN_NODES as f64, limits::MAX_NODES as f64)),
      Why::LineWidth => Some((limits::MIN_WIDTH, limits::MAX_WIDTH)),
      Why::InsideCount => Some((0.0, limits::INSIDE as f64)),
      Why::Reach => Some((0.0, limits::MAX_REACH)),
      Why::EyeVoice => Some((0.0, limits::MAX_EYE_VOICE)),
      Why::Corners => Some((-1.0, 1.0)),
      Why::Anchors => Some((1.0, limits::ANCHORS as f64)),
      Why::Tuning { least, most } => Some((*least, *most)),
      _ => None,
    }
  }
}

/// A document refused: why, and where.
#[derive(Clone, Debug, PartialEq)]
pub struct Refusal {
  pub why: Why,
  pub at: Vec<Place>,
}

impl Refusal {
  fn new(why: Why, at: &[Place]) -> Refusal {
    Refusal {
      why,
      at: at.to_vec(),
    }
  }

  /// The refusal as a reader is handed it: its code, where it was met, and the limits it names.
  pub fn to_json(&self) -> Value {
    let mut refusal = Map::new();
    refusal.insert("code".into(), json!(self.why.code()));
    refusal.insert("at".into(), Value::Array(self.at.iter().map(Place::to_json).collect()));
    if let Some((least, most)) = self.why.bounds() {
      refusal.insert("least".into(), number(least));
      refusal.insert("most".into(), number(most));
    }
    if let Why::UnknownFace(name) = &self.why {
      refusal.insert("name".into(), json!(name));
    }
    Value::Object(refusal)
  }

  /// The refusal in one line of English, for a log.
  pub fn describe(&self) -> String {
    let at: Vec<String> = self
      .at
      .iter()
      .map(|place| match place {
        Place::Color(key) => format!("colour {key}"),
        Place::Face(name) => format!("face {name}"),
        Place::Element(name) => (*name).to_owned(),
        Place::Side(side) => (*side).to_owned(),
        Place::Part(index) => format!("part {}", index + 1),
        Place::Inside(index) => format!("inside {}", index + 1),
        Place::Node(index) => format!("node {}", index + 1),
        Place::Anchor(index) => format!("anchor {}", index + 1),
        Place::Tuning(key) => format!("tuning {key}"),
      })
      .collect();
    if at.is_empty() {
      self.why.code().to_owned()
    } else {
      format!("{}: {}", at.join(" "), self.why.code())
    }
  }
}

type Checked<T> = Result<T, Refusal>;

/// Room for the last bit of floating point in a handle end computed from an anchor on the edge.
const EDGE: f64 = 1e-9;

fn within([x, y]: Point) -> bool {
  (-EDGE..=SCREEN[0] + EDGE).contains(&x) && (-EDGE..=SCREEN[1] + EDGE).contains(&y)
}

fn point(value: Option<&Value>) -> Option<Point> {
  match value?.as_array()?.as_slice() {
    [x, y] => Some([x.as_f64()?, y.as_f64()?]),
    _ => None,
  }
}

fn check_node(value: &Value, at: &[Place]) -> Checked<Node> {
  let wrong = || Refusal::new(Why::NodeValue, at);
  let node = value.as_object().ok_or_else(wrong)?;
  let (Some(anchor), Some(coming), Some(leaving)) =
    (point(node.get("anchor")), point(node.get("coming")), point(node.get("leaving")))
  else {
    return Err(wrong());
  };
  let kind = node.get("kind").and_then(Value::as_str).and_then(Kind::named).ok_or_else(wrong)?;
  let ends = [anchor, crate::geometry::add(anchor, coming), crate::geometry::add(anchor, leaving)];
  if !ends.into_iter().all(within) {
    return Err(Refusal::new(Why::OffScreen, at));
  }
  Ok(Node {
    anchor,
    coming,
    leaving,
    kind,
  })
}

fn check_line(value: Option<&Value>, at: &[Place]) -> Checked<Option<Line>> {
  if value == Some(&Value::Null) {
    return Ok(None);
  }
  let line = value.and_then(Value::as_object);
  let paint = line.and_then(|line| line.get("paint")).and_then(Value::as_str).and_then(Paint::named);
  let width = line.and_then(|line| line.get("width")).and_then(Value::as_f64);
  let (Some(paint), Some(width)) = (paint, width) else {
    return Err(Refusal::new(Why::LineValue, at));
  };
  if !(limits::MIN_WIDTH..=limits::MAX_WIDTH).contains(&width) {
    return Err(Refusal::new(Why::LineWidth, at));
  }
  Ok(Some(Line { paint, width }))
}

fn with(at: &[Place], place: Place) -> Vec<Place> {
  let mut deeper = at.to_vec();
  deeper.push(place);
  deeper
}

fn check_part(value: &Value, at: &[Place], nested: bool) -> Checked<Part> {
  let part = value.as_object().ok_or_else(|| Refusal::new(Why::NotAPart, at))?;
  let closed = part
    .get("closed")
    .and_then(Value::as_bool)
    .ok_or_else(|| Refusal::new(Why::Closedness, at))?;
  let nodes = part
    .get("nodes")
    .and_then(Value::as_array)
    .filter(|nodes| (limits::MIN_NODES..=limits::MAX_NODES).contains(&nodes.len()))
    .ok_or_else(|| Refusal::new(Why::NodeCount, at))?;
  let nodes = nodes
    .iter()
    .enumerate()
    .map(|(index, node)| check_node(node, &with(at, Place::Node(index))))
    .collect::<Checked<Vec<Node>>>()?;

  let fill = match part.get("fill") {
    Some(Value::Null) => None,
    Some(Value::String(name)) => Some(Paint::named(name).ok_or_else(|| Refusal::new(Why::FillPaint, at))?),
    _ => return Err(Refusal::new(Why::FillPaint, at)),
  };
  let line = check_line(part.get("line"), at)?;
  if fill.is_none() && line.is_none() {
    return Err(Refusal::new(Why::NeitherFillNorLine, at));
  }
  if fill.is_some() && !closed {
    return Err(Refusal::new(Why::OpenFilled, at));
  }

  let inside = match part.get("inside") {
    None => Vec::new(),
    Some(_) if nested => return Err(Refusal::new(Why::NestedInside, at)),
    Some(_) if !closed => return Err(Refusal::new(Why::InsideOpen, at)),
    Some(inside) => {
      let inside = inside
        .as_array()
        .filter(|inside| inside.len() <= limits::INSIDE)
        .ok_or_else(|| Refusal::new(Why::InsideCount, at))?;
      inside
        .iter()
        .enumerate()
        .map(|(index, inner)| check_part(inner, &with(at, Place::Inside(index)), true))
        .collect::<Checked<Vec<Part>>>()?
    }
  };
  Ok(Part {
    nodes,
    closed,
    fill,
    line,
    inside,
  })
}

/// A list of parts as the editor holds them, checked as a face's element is.
pub fn parts(value: &Value) -> Checked<Vec<Part>> {
  check_parts(Some(value), &[])
}

/// One part, checked as a face's part is.
pub fn part(value: &Value) -> Checked<Part> {
  check_part(value, &[], false)
}

fn check_parts(value: Option<&Value>, at: &[Place]) -> Checked<Vec<Part>> {
  let parts = match value {
    // A list a face does not hold is an empty one.
    None | Some(Value::Null) => return Ok(Vec::new()),
    Some(parts) => parts.as_array().filter(|parts| parts.len() <= limits::PARTS),
  };
  let parts = parts.ok_or_else(|| Refusal::new(Why::Parts, at))?;
  parts
    .iter()
    .enumerate()
    .map(|(index, part)| check_part(part, &with(at, Place::Part(index)), false))
    .collect()
}

fn check_pair(value: Option<&Value>, at: &[Place]) -> Checked<Pair> {
  let pair = match value {
    None | Some(Value::Null) => return Ok(Pair::empty()),
    Some(pair) => pair.as_object().ok_or_else(|| Refusal::new(Why::NotAPair, at))?,
  };
  let left = check_parts(pair.get("left"), &with(at, Place::Side("left")))?;
  let right = match pair.get("right") {
    None | Some(Value::Null) => None,
    some => Some(check_parts(some, &with(at, Place::Side("right")))?),
  };
  Ok(Pair { left, right })
}

/// How far the eyes move with the gaze each way; nothing where the document says nothing.
fn check_reach(value: Option<&Value>, at: &[Place]) -> Checked<Point> {
  match value {
    None | Some(Value::Null) => Ok([0.0, 0.0]),
    some => point(some)
      .filter(|reach| reach.iter().all(|one| (0.0..=limits::MAX_REACH).contains(one)))
      .ok_or_else(|| Refusal::new(Why::Reach, at)),
  }
}

/// How far the eyes answer the voice: each share from nought to the bound; nothing where the document
/// says nothing.
fn check_eye_voice(value: Option<&Value>, at: &[Place]) -> Checked<EyeVoice> {
  let wrong = || Refusal::new(Why::EyeVoice, at);
  let voice = match value {
    None | Some(Value::Null) => return Ok(EyeVoice::STILL),
    Some(voice) => voice.as_object().ok_or_else(wrong)?,
  };
  let share = |key: &str| {
    voice
      .get(key)
      .and_then(Value::as_f64)
      .filter(|share| (0.0..=limits::MAX_EYE_VOICE).contains(share))
      .ok_or_else(wrong)
  };
  let [grow, squint, shine] = EyeVoice::KEYS;
  Ok(EyeVoice {
    grow: share(grow)?,
    squint: share(squint)?,
    shine: share(shine)?,
  })
}

fn check_face(value: &Value, name: &str) -> Checked<Face> {
  let at = [Place::Face(name.to_owned())];
  let face = value.as_object().ok_or_else(|| Refusal::new(Why::NotAFace, &at))?;
  let element = |name: &'static str| with(&at, Place::Element(name));
  let eyes = face.get("eyes");
  let corners = match face.get("corners") {
    None | Some(Value::Null) => 0.0,
    Some(corners) => corners
      .as_f64()
      .filter(|corners| (-1.0..=1.0).contains(corners))
      .ok_or_else(|| Refusal::new(Why::Corners, &at))?,
  };
  Ok(Face {
    blush: check_pair(face.get("blush"), &element("blush"))?,
    eyes: Eyes {
      pair: check_pair(eyes, &element("eyes"))?,
      reach: check_reach(eyes.and_then(|eyes| eyes.get("reach")), &element("eyes"))?,
      voice: check_eye_voice(eyes.and_then(|eyes| eyes.get("voice")), &element("eyes"))?,
    },
    brows: check_pair(face.get("brows"), &element("brows"))?,
    mouth: check_parts(face.get("mouth"), &element("mouth"))?,
    marks: check_parts(face.get("marks"), &element("marks"))?,
    corners,
  })
}

/// How near two anchors of one plane may stand, on each of the plane's two ways.
const APART: f64 = 0.005;

fn check_anchor(value: &Value, index: usize) -> Checked<Anchor> {
  let at = [Place::Anchor(index)];
  let anchor = value.as_object().ok_or_else(|| Refusal::new(Why::AnchorValue, &at))?;
  let place = point(anchor.get("at"))
    .filter(|place| place.iter().all(|one| (0.0..=1.0).contains(one)))
    .ok_or_else(|| Refusal::new(Why::AnchorValue, &at))?;
  let teeth = anchor
    .get("teeth")
    .and_then(Value::as_bool)
    .ok_or_else(|| Refusal::new(Why::AnchorValue, &at))?;
  Ok(Anchor {
    at: place,
    teeth,
    parts: check_parts(anchor.get("parts"), &at)?,
  })
}

/// The talking set: one to the most anchors, no two of one plane at one place, and its tuning.
fn check_talk(value: Option<&Value>) -> Checked<Talk> {
  let talk = value.and_then(Value::as_object).ok_or_else(|| Refusal::new(Why::Anchors, &[]))?;
  let anchors = talk
    .get("anchors")
    .and_then(Value::as_array)
    .filter(|anchors| (1..=limits::ANCHORS).contains(&anchors.len()))
    .ok_or_else(|| Refusal::new(Why::Anchors, &[]))?;
  let anchors = anchors
    .iter()
    .enumerate()
    .map(|(index, anchor)| check_anchor(anchor, index))
    .collect::<Checked<Vec<Anchor>>>()?;
  for (index, anchor) in anchors.iter().enumerate() {
    let near = |other: &Anchor| {
      other.teeth == anchor.teeth
        && (other.at[0] - anchor.at[0]).abs() < APART
        && (other.at[1] - anchor.at[1]).abs() < APART
    };
    if anchors[..index].iter().any(near) {
      return Err(Refusal::new(Why::AnchorPlace, &[Place::Anchor(index)]));
    }
  }
  let tuning = Tuning::from_json(talk.get("tuning").unwrap_or(&Value::Null)).map_err(|wrong| {
    Refusal::new(
      Why::Tuning {
        least: wrong.least,
        most: wrong.most,
      },
      &[Place::Tuning(wrong.key)],
    )
  })?;
  Ok(Talk { anchors, tuning })
}

fn is_colour(colour: &str) -> bool {
  colour.len() == 7 && colour.starts_with('#') && colour[1..].chars().all(|one| one.is_ascii_hexdigit())
}

/// A document's colours: the body, the screen and every paint, each `#rrggbb`, kept in small letters.
pub fn check_colors(colors: &Map<String, Value>) -> Checked<Colors> {
  let colour = |key: &str| {
    colors
      .get(key)
      .and_then(Value::as_str)
      .filter(|colour| is_colour(colour))
      .map(str::to_ascii_lowercase)
      .ok_or_else(|| Refusal::new(Why::Color, &[Place::Color(key.to_owned())]))
  };
  Ok(Colors {
    body: colour("body")?,
    screen: colour("screen")?,
    paints: Paint::ALL
      .into_iter()
      .map(|paint| Ok((paint, colour(paint.name())?)))
      .collect::<Checked<_>>()?,
  })
}

/// The document as it may be kept and drawn — its colours, faces and talking set, and nothing else —
/// or the refusal saying where and why not. `savedAt` is not checked here.
pub fn check(value: &Value) -> Checked<crate::document::Document> {
  let not_a_document = || Refusal::new(Why::NotADocument, &[]);
  let document = value.as_object().ok_or_else(not_a_document)?;
  if document.get("form").and_then(Value::as_u64) != Some(FORM) {
    return Err(not_a_document());
  }
  let (Some(colors), Some(faces)) = (
    document.get("colors").and_then(Value::as_object),
    document.get("faces").and_then(Value::as_object),
  ) else {
    return Err(not_a_document());
  };

  let colors = check_colors(colors)?;

  let mut checked_faces = Vec::with_capacity(faces.len());
  for (name, face) in faces {
    if !is_face_name(name) {
      return Err(Refusal::new(Why::UnknownFace(name.clone()), &[]));
    }
    checked_faces.push((name.clone(), check_face(face, name)?));
  }
  if !checked_faces.iter().any(|(name, _)| name == FALLBACK) {
    return Err(Refusal::new(Why::NoFallback, &[]));
  }

  let talk = check_talk(document.get("talk"))?;

  Ok(crate::document::Document {
    colors,
    faces: checked_faces,
    talk,
  })
}
