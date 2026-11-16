//! The eyes while BMO speaks (docs/face.md, 「The eyes while speaking」): a restrained answer to the voice,
//! each eye a little taller with the level and on a sharp start, a little shorter through a long loud
//! stretch, and its shine, where it has one, a little larger with the hiss — each no further than the eye
//! itself allows, so a face stays the face it is. A stroke, an eye already closed, does not answer.

use crate::document::{EyeVoice, Node, Paint, Part};
use crate::geometry::{Point, add, scale, sub};
use crate::talk::Pose;

/// The part grown about a point by `factor` every way, with what stands inside it.
fn scaled(part: &Part, centre: Point, factor: f64) -> Part {
  Part {
    nodes: part
      .nodes
      .iter()
      .map(|node| Node {
        anchor: add(centre, scale(sub(node.anchor, centre), factor)),
        coming: scale(node.coming, factor),
        leaving: scale(node.leaving, factor),
        kind: node.kind,
      })
      .collect(),
    inside: part.inside.iter().map(|inner| scaled(inner, centre, factor)).collect(),
    ..part.clone()
  }
}

/// An eye answering a pose: a solid eye stretched about its own middle, top to bottom, by what the pose
/// asks of its growing and squinting as far as the eye allows, and each shine inside it grown about its
/// own middle; a stroke as it stands.
pub fn voiced(eye: &Part, allows: EyeVoice, pose: &Pose) -> Part {
  if !eye.is_solid() {
    return eye.clone();
  }
  let stretch = 1.0 + allows.grow * pose.grow.clamp(0.0, 1.0) - allows.squint * pose.squint.clamp(0.0, 1.0);
  let middle = eye.path().bounds().centre()[1];
  let mut answered = if stretch == 1.0 { eye.clone() } else { eye.squashed(middle, stretch) };
  let shimmer = 1.0 + allows.shine * pose.shine.clamp(0.0, 1.0);
  if shimmer != 1.0 {
    for inner in &mut answered.inside {
      if inner.fill == Some(Paint::Light) {
        *inner = scaled(inner, inner.path().bounds().centre(), shimmer);
      }
    }
  }
  answered
}
