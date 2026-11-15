//! The expression document as the core holds it (docs/face.md, 「The expression document」): its colours,
//! its faces element by element, and the talking mouth set, with the form the document is written in.

use serde_json::{Map, Value, json};

use crate::geometry::{Cubic, Path, Point, add, sub};
use crate::json::number;
use crate::names::FALLBACK;

/// The form of the document this core reads and writes.
pub const FORM: u64 = 2;

/// The screen's own coordinates: every anchor and every handle end stands inside this box.
pub const SCREEN: Point = [100.0, 75.0];

/// The limits a document is held to.
pub mod limits {
  pub const PARTS: usize = 16;
  pub const INSIDE: usize = 8;
  pub const MIN_NODES: usize = 2;
  pub const MAX_NODES: usize = 32;
  pub const MIN_WIDTH: f64 = 0.2;
  pub const MAX_WIDTH: f64 = 10.0;
  pub const MAX_REACH: f64 = 6.0;
  pub const FAMILIES: usize = 12;
  pub const MOUTHS: usize = 12;
}

/// Whether a node's two handles stand on one line through its anchor, which the editor keeps, or move
/// apart. The drawing reads only the handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
  Smooth,
  Corner,
}

impl Kind {
  pub fn name(self) -> &'static str {
    match self {
      Kind::Smooth => "smooth",
      Kind::Corner => "corner",
    }
  }

  pub fn named(name: &str) -> Option<Kind> {
    [Kind::Smooth, Kind::Corner].into_iter().find(|kind| kind.name() == name)
  }
}

/// One node of a path: the anchor it passes through and its two handles, each an offset from the
/// anchor — the one the path comes in by and the one it leaves by.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
  pub anchor: Point,
  pub coming: Point,
  pub leaving: Point,
  pub kind: Kind,
}

/// The paints a part is filled and lined with; each is a colour of the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Paint {
  Ink,
  Light,
  Mouth,
  Tongue,
  TongueLight,
  Cheek,
  Tear,
}

impl Paint {
  pub const ALL: [Paint; 7] = [
    Paint::Ink,
    Paint::Light,
    Paint::Mouth,
    Paint::Tongue,
    Paint::TongueLight,
    Paint::Cheek,
    Paint::Tear,
  ];

  pub fn name(self) -> &'static str {
    match self {
      Paint::Ink => "ink",
      Paint::Light => "light",
      Paint::Mouth => "mouth",
      Paint::Tongue => "tongue",
      Paint::TongueLight => "tongueLight",
      Paint::Cheek => "cheek",
      Paint::Tear => "tear",
    }
  }

  pub fn named(name: &str) -> Option<Paint> {
    Paint::ALL.into_iter().find(|paint| paint.name() == name)
  }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
  pub paint: Paint,
  pub width: f64,
}

/// One path of cubic Bézier segments, with its own paints and, when closed, the parts cut off inside it.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
  pub nodes: Vec<Node>,
  pub closed: bool,
  pub fill: Option<Paint>,
  pub line: Option<Line>,
  pub inside: Vec<Part>,
}

impl Part {
  /// The segments the nodes make: between two nodes, the curve from the first anchor pulled by its
  /// leaving handle and the second's coming handle to the second anchor; a closed path's last segment
  /// runs back to its first node.
  pub fn path(&self) -> Path {
    let count = if self.closed { self.nodes.len() } else { self.nodes.len().saturating_sub(1) };
    let segments = (0..count)
      .map(|index| {
        let from = &self.nodes[index];
        let to = &self.nodes[(index + 1) % self.nodes.len()];
        Cubic([from.anchor, add(from.anchor, from.leaving), add(to.anchor, to.coming), to.anchor])
      })
      .collect();
    Path {
      segments,
      closed: self.closed,
    }
  }

  /// The part drawing a path's segments, its paints and inside kept. A node is smooth where its two
  /// handles stand on one line, a corner otherwise.
  pub fn with_path(&self, path: &Path) -> Part {
    Part {
      nodes: nodes_of(path),
      closed: path.closed,
      ..self.clone()
    }
  }

  /// The part reflected across the screen's middle, from left to right, with what stands inside it.
  pub fn mirrored(&self) -> Part {
    let flip = |[x, y]: Point| [SCREEN[0] - x, y];
    let turn = |[x, y]: Point| [-x, y];
    Part {
      nodes: self
        .nodes
        .iter()
        .map(|node| Node {
          anchor: flip(node.anchor),
          coming: turn(node.coming),
          leaving: turn(node.leaving),
          kind: node.kind,
        })
        .collect(),
      inside: self.inside.iter().map(Part::mirrored).collect(),
      ..self.clone()
    }
  }

  /// The part moved by an offset, with what stands inside it moved `inside` times as far.
  pub fn shifted(&self, by: Point, inside: f64) -> Part {
    Part {
      nodes: self
        .nodes
        .iter()
        .map(|node| Node {
          anchor: add(node.anchor, by),
          ..node.clone()
        })
        .collect(),
      inside: self
        .inside
        .iter()
        .map(|inner| inner.shifted([by[0] * inside, by[1] * inside], 1.0))
        .collect(),
      ..self.clone()
    }
  }

  /// The part pressed toward the level `y` by `factor` (below one) and stretched away from it (above),
  /// with what stands inside it.
  pub fn squashed(&self, y: f64, factor: f64) -> Part {
    Part {
      nodes: self
        .nodes
        .iter()
        .map(|node| Node {
          anchor: [node.anchor[0], y + (node.anchor[1] - y) * factor],
          coming: [node.coming[0], node.coming[1] * factor],
          leaving: [node.leaving[0], node.leaving[1] * factor],
          kind: node.kind,
        })
        .collect(),
      inside: self.inside.iter().map(|inner| inner.squashed(y, factor)).collect(),
      ..self.clone()
    }
  }

  /// Whether the part is a filled closed shape, as an open eye is, rather than a stroke.
  pub fn is_solid(&self) -> bool {
    self.closed && self.fill.is_some()
  }

  pub fn to_json(&self) -> Value {
    let mut part = Map::new();
    part.insert("nodes".into(), Value::Array(self.nodes.iter().map(node_json).collect()));
    part.insert("closed".into(), Value::Bool(self.closed));
    part.insert("fill".into(), self.fill.map_or(Value::Null, |paint| json!(paint.name())));
    part.insert(
      "line".into(),
      self
        .line
        .as_ref()
        .map_or(Value::Null, |line| json!({ "paint": line.paint.name(), "width": number(line.width) })),
    );
    if !self.inside.is_empty() {
      part.insert("inside".into(), parts_json(&self.inside));
    }
    Value::Object(part)
  }
}

/// The nodes that draw a run of segments: each anchor where a segment starts, its leaving handle that
/// segment's first control point, and its coming handle the second control point of the segment
/// before it.
pub fn nodes_of(path: &Path) -> Vec<Node> {
  let segments = &path.segments;
  let count = segments.len();
  let mut nodes: Vec<Node> = (0..count)
    .map(|index| {
      let segment = segments[index];
      let anchor = segment.start();
      let coming = match (index, path.closed) {
        (0, false) => [0.0, 0.0],
        (0, true) => sub(segments[count - 1].0[2], anchor),
        _ => sub(segments[index - 1].0[2], anchor),
      };
      let leaving = sub(segment.0[1], anchor);
      Node {
        anchor,
        coming,
        leaving,
        kind: kind_of(coming, leaving),
      }
    })
    .collect();
  if !path.closed
    && let Some(last) = segments.last()
  {
    let anchor = last.end();
    nodes.push(Node {
      anchor,
      coming: sub(last.0[2], anchor),
      leaving: [0.0, 0.0],
      kind: Kind::Smooth,
    });
  }
  nodes
}

fn kind_of(coming: Point, leaving: Point) -> Kind {
  let cross = coming[0] * leaving[1] - coming[1] * leaving[0];
  let dot = coming[0] * leaving[0] + coming[1] * leaving[1];
  let size = crate::geometry::length(coming) * crate::geometry::length(leaving);
  if size == 0.0 || (cross.abs() <= size * 1e-6 && dot < 0.0) {
    Kind::Smooth
  } else {
    Kind::Corner
  }
}

fn point_json([x, y]: Point) -> Value {
  json!([number(x), number(y)])
}

fn node_json(node: &Node) -> Value {
  json!({
    "anchor": point_json(node.anchor),
    "coming": point_json(node.coming),
    "leaving": point_json(node.leaving),
    "kind": node.kind.name(),
  })
}

pub fn parts_json(parts: &[Part]) -> Value {
  Value::Array(parts.iter().map(Part::to_json).collect())
}

/// An element drawn once a side: its left parts, and its right ones — none where the right is the left
/// reflected across the screen's middle, which it is by default.
#[derive(Clone, Debug, PartialEq)]
pub struct Pair {
  pub left: Vec<Part>,
  pub right: Option<Vec<Part>>,
}

impl Pair {
  pub fn empty() -> Pair {
    Pair {
      left: Vec::new(),
      right: None,
    }
  }

  /// The left's parts and the right's, the right reflected from the left where it mirrors it.
  pub fn sides(&self) -> [Vec<Part>; 2] {
    let right = match &self.right {
      Some(parts) => parts.clone(),
      None => self.left.iter().map(Part::mirrored).collect(),
    };
    [self.left.clone(), right]
  }

  fn to_json(&self) -> Map<String, Value> {
    let mut pair = Map::new();
    pair.insert("left".into(), parts_json(&self.left));
    pair.insert("right".into(), self.right.as_ref().map_or(Value::Null, |parts| parts_json(parts)));
    pair
  }
}

/// The eyes: a pair, and how far each eye moves each way as the gaze turns, as its shape allows.
#[derive(Clone, Debug, PartialEq)]
pub struct Eyes {
  pub pair: Pair,
  pub reach: Point,
}

/// One face: its elements, drawn in this order, each over the ones before it.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
  pub blush: Pair,
  pub eyes: Eyes,
  pub brows: Pair,
  pub mouth: Vec<Part>,
  pub marks: Vec<Part>,
}

impl Face {
  pub fn to_json(&self) -> Value {
    let mut eyes = self.eyes.pair.to_json();
    eyes.insert("reach".into(), point_json(self.eyes.reach));
    json!({
      "eyes": eyes,
      "brows": self.brows.to_json(),
      "blush": self.blush.to_json(),
      "mouth": parts_json(&self.mouth),
      "marks": parts_json(&self.marks),
    })
  }
}

/// The sound a talking mouth is drawn for, as a family of mouths names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Voice {
  /// A pause, between words or before and after a line.
  Silence,
  /// The lips meeting, as on P, B and M.
  Closing,
  /// A rounded vowel, as on O and U.
  Rounded,
  /// An open vowel, as on A.
  Open,
  /// Any other voiced sound.
  Other,
}

impl Voice {
  pub const ALL: [Voice; 5] = [Voice::Silence, Voice::Closing, Voice::Rounded, Voice::Open, Voice::Other];

  pub fn name(self) -> &'static str {
    match self {
      Voice::Silence => "silence",
      Voice::Closing => "closing",
      Voice::Rounded => "rounded",
      Voice::Open => "open",
      Voice::Other => "other",
    }
  }

  pub fn named(name: &str) -> Option<Voice> {
    Voice::ALL.into_iter().find(|voice| voice.name() == name)
  }
}

/// How open a talking mouth is, among the mouths of its family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Size {
  Small,
  Medium,
  Large,
}

impl Size {
  pub const ALL: [Size; 3] = [Size::Small, Size::Medium, Size::Large];

  pub fn name(self) -> &'static str {
    match self {
      Size::Small => "small",
      Size::Medium => "medium",
      Size::Large => "large",
    }
  }

  pub fn named(name: &str) -> Option<Size> {
    Size::ALL.into_iter().find(|size| size.name() == name)
  }

  /// The openness this size stands for, on the scale the speaking driver aims the loudness at.
  pub const fn openness(self) -> f64 {
    match self {
      Size::Small => 0.45,
      Size::Medium => 0.6,
      Size::Large => 0.75,
    }
  }
}

/// One drawing of the talking mouth set.
#[derive(Clone, Debug, PartialEq)]
pub struct TalkMouth {
  pub size: Size,
  pub parts: Vec<Part>,
}

/// A family of talking mouths: its name, the sounds it is drawn for, how often it is drawn while
/// talking, and its drawings.
#[derive(Clone, Debug, PartialEq)]
pub struct TalkFamily {
  pub name: String,
  pub voices: Vec<Voice>,
  pub weight: f64,
  pub mouths: Vec<TalkMouth>,
}

impl TalkFamily {
  pub fn voices(&self, voice: Voice) -> bool {
    self.voices.contains(&voice)
  }

  pub fn to_json(&self) -> Value {
    json!({
      "name": self.name,
      "voices": self.voices.iter().map(|voice| voice.name()).collect::<Vec<_>>(),
      "weight": number(self.weight),
      "mouths": self.mouths.iter().map(|mouth| json!({ "size": mouth.size.name(), "parts": parts_json(&mouth.parts) })).collect::<Vec<_>>(),
    })
  }
}

/// The colours a document paints with: the body around the screen, the screen, and one per paint.
#[derive(Clone, Debug, PartialEq)]
pub struct Colors {
  pub body: String,
  pub screen: String,
  pub paints: Vec<(Paint, String)>,
}

impl Colors {
  pub fn paint(&self, paint: Paint) -> &str {
    self
      .paints
      .iter()
      .find(|(own, _)| *own == paint)
      .map_or("#000000", |(_, colour)| colour.as_str())
  }

  pub fn to_json(&self) -> Value {
    let mut colors = Map::new();
    colors.insert("body".into(), json!(self.body));
    colors.insert("screen".into(), json!(self.screen));
    for (paint, colour) in &self.paints {
      colors.insert(paint.name().into(), json!(colour));
    }
    Value::Object(colors)
  }
}

/// The expression document, as checked.
#[derive(Clone, Debug, PartialEq)]
pub struct Document {
  pub colors: Colors,
  pub faces: Vec<(String, Face)>,
  pub talk: Vec<TalkFamily>,
}

impl Document {
  /// The face drawn for a name: its own, or the fallback's where the document holds none.
  pub fn face(&self, name: &str) -> &Face {
    let find = |wanted: &str| self.faces.iter().find(|(own, _)| own == wanted).map(|(_, face)| face);
    find(name)
      .or_else(|| find(FALLBACK))
      .expect("a checked document holds the fallback face")
  }

  /// The document as written, without its version.
  pub fn to_json(&self) -> Value {
    let mut faces = Map::new();
    for (name, face) in &self.faces {
      faces.insert(name.clone(), face.to_json());
    }
    json!({
      "form": FORM,
      "colors": self.colors.to_json(),
      "faces": faces,
      "talk": self.talk.iter().map(TalkFamily::to_json).collect::<Vec<_>>(),
    })
  }
}
