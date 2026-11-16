//! The expression document as the core holds it (docs/face.md, 「The expression document」): its colours,
//! its faces element by element, and the talking set, with the form the document is written in.

use serde_json::{Map, Value, json};

use crate::geometry::{Cubic, Path, Point, add, sub};
use crate::json::number;
use crate::names::FALLBACK;
use crate::tuning::Tuning;

/// The form of the document this core reads and writes.
pub const FORM: u64 = 3;

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
  pub const ANCHORS: usize = 32;
  /// The most of its own size an eye may grow, squint or shimmer with the voice.
  pub const MAX_EYE_VOICE: f64 = 0.5;
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

pub fn point_json([x, y]: Point) -> Value {
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

/// The eyes: a pair, how far each eye moves each way as the gaze turns, and how far it answers the
/// voice, as its shape allows.
#[derive(Clone, Debug, PartialEq)]
pub struct Eyes {
  pub pair: Pair,
  pub reach: Point,
  pub voice: EyeVoice,
}

/// One face: its elements, drawn in this order, each over the ones before it, and how far the talking
/// mouth's corners lift while it speaks, from −1 (dropped) to 1 (lifted).
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
  pub blush: Pair,
  pub eyes: Eyes,
  pub brows: Pair,
  pub mouth: Vec<Part>,
  pub marks: Vec<Part>,
  pub corners: f64,
}

impl Face {
  pub fn to_json(&self) -> Value {
    let mut eyes = self.eyes.pair.to_json();
    eyes.insert("reach".into(), point_json(self.eyes.reach));
    eyes.insert("voice".into(), self.eyes.voice.to_json());
    json!({
      "eyes": eyes,
      "brows": self.brows.to_json(),
      "blush": self.blush.to_json(),
      "mouth": parts_json(&self.mouth),
      "marks": parts_json(&self.marks),
      "corners": number(self.corners),
    })
  }
}

/// How far each eye answers the voice while BMO speaks, as its own shape allows (「The eyes while
/// speaking」): how much taller it may grow, how much shorter it may squint, and how much larger its shine
/// may shimmer, each a share of its own size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EyeVoice {
  pub grow: f64,
  pub squint: f64,
  pub shine: f64,
}

impl EyeVoice {
  /// The eyes of a stroke, already closed: they do not answer.
  pub const STILL: EyeVoice = EyeVoice {
    grow: 0.0,
    squint: 0.0,
    shine: 0.0,
  };

  /// The shares, in the order the document writes them.
  pub const KEYS: [&'static str; 3] = ["grow", "squint", "shine"];

  pub fn mixed(&self, other: &EyeVoice, t: f64) -> EyeVoice {
    let mix = |a: f64, b: f64| a + (b - a) * t;
    EyeVoice {
      grow: mix(self.grow, other.grow),
      squint: mix(self.squint, other.squint),
      shine: mix(self.shine, other.shine),
    }
  }

  pub fn to_json(&self) -> Value {
    json!({ "grow": number(self.grow), "squint": number(self.squint), "shine": number(self.shine) })
  }
}

/// One drawing of the talking set: where it stands among the others — how open it is and how wide,
/// each from nought to one — whether it shows the teeth together, and its parts.
#[derive(Clone, Debug, PartialEq)]
pub struct Anchor {
  pub at: Point,
  pub teeth: bool,
  pub parts: Vec<Part>,
}

impl Anchor {
  pub fn to_json(&self) -> Value {
    json!({ "at": point_json(self.at), "teeth": self.teeth, "parts": parts_json(&self.parts) })
  }
}

/// The talking set: the drawings the talking mouth passes through, and how the sound moves it.
#[derive(Clone, Debug, PartialEq)]
pub struct Talk {
  pub anchors: Vec<Anchor>,
  pub tuning: Tuning,
}

impl Talk {
  pub fn to_json(&self) -> Value {
    json!({
      "anchors": self.anchors.iter().map(Anchor::to_json).collect::<Vec<_>>(),
      "tuning": self.tuning.to_json(),
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
  pub talk: Talk,
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
      "talk": self.talk.to_json(),
    })
  }
}
