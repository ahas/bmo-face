//! One drawing of the face in the document's terms (docs/face.md, 「The faces」): every element's parts
//! with both sides of a pair drawn out, in drawing order, and how far its eyes move with the gaze. A face
//! is shown as a look; an in-between is the blend of two looks; the blink, the gaze, the talking mouth
//! and the eyes' answer to the voice each change a look before it is drawn.

use crate::blend::blend_lists;
use crate::document::{EyeVoice, Face, Part};
use crate::geometry::{Point, mix};

#[derive(Clone, Debug, PartialEq)]
pub struct Look {
  pub blush: [Vec<Part>; 2],
  pub eyes: [Vec<Part>; 2],
  pub brows: [Vec<Part>; 2],
  pub mouth: Vec<Part>,
  pub marks: Vec<Part>,
  pub reach: Point,
  /// How far the eyes answer the voice, and how far the talking mouth's corners lift.
  pub voice: EyeVoice,
  pub corners: f64,
}

fn both(a: &[Vec<Part>; 2], b: &[Vec<Part>; 2], t: f64) -> [Vec<Part>; 2] {
  [blend_lists(&a[0], &b[0], t), blend_lists(&a[1], &b[1], t)]
}

impl Look {
  pub fn of(face: &Face) -> Look {
    Look {
      blush: face.blush.sides(),
      eyes: face.eyes.pair.sides(),
      brows: face.brows.sides(),
      mouth: face.mouth.clone(),
      marks: face.marks.clone(),
      reach: face.eyes.reach,
      voice: face.eyes.voice,
      corners: face.corners,
    }
  }

  /// The look `t` of the way from this one to another, element by element and side by side.
  pub fn blend(&self, other: &Look, t: f64) -> Look {
    Look {
      blush: both(&self.blush, &other.blush, t),
      eyes: both(&self.eyes, &other.eyes, t),
      brows: both(&self.brows, &other.brows, t),
      mouth: blend_lists(&self.mouth, &other.mouth, t),
      marks: blend_lists(&self.marks, &other.marks, t),
      reach: mix(self.reach, other.reach, t),
      voice: self.voice.mixed(&other.voice, t),
      corners: self.corners + (other.corners - self.corners) * t,
    }
  }

  /// The look with every part pressed toward its own middle, top to bottom, by `factor`: the squash a
  /// change is anticipated with.
  pub fn squashed(&self, factor: f64) -> Look {
    let squash = |parts: &[Part]| -> Vec<Part> {
      parts
        .iter()
        .map(|part| part.squashed(part.path().bounds().centre()[1], factor))
        .collect()
    };
    let both = |sides: &[Vec<Part>; 2]| [squash(&sides[0]), squash(&sides[1])];
    Look {
      blush: both(&self.blush),
      eyes: both(&self.eyes),
      brows: both(&self.brows),
      mouth: squash(&self.mouth),
      marks: squash(&self.marks),
      ..self.clone()
    }
  }

  /// Every part, in the order it is drawn: the blush, the eyes, the brows, the mouth, the marks.
  pub fn parts(&self) -> impl Iterator<Item = &Part> {
    let [blush_left, blush_right] = &self.blush;
    let [eye_left, eye_right] = &self.eyes;
    let [brow_left, brow_right] = &self.brows;
    [
      blush_left,
      blush_right,
      eye_left,
      eye_right,
      brow_left,
      brow_right,
      &self.mouth,
      &self.marks,
    ]
    .into_iter()
    .flatten()
  }

  /// Whether any part, or any part inside one, crosses itself.
  pub fn crosses_itself(&self) -> bool {
    self
      .parts()
      .flat_map(|part| std::iter::once(part).chain(&part.inside))
      .any(|part| part.path().crosses_itself())
  }

  /// Whether the eyes are open shapes a lid can close, rather than strokes already closed.
  pub fn blinks(&self) -> bool {
    self.eyes.iter().flatten().any(Part::is_solid)
  }
}
