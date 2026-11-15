//! The gaze (docs/face.md, 「The gaze」): where the eyes look, a direction from −1 to 1 each way, the
//! middle by default. It moves each open eye within the reach the face's eyes declare, and what stands
//! inside an eye — a pupil, a shine — half as far again. It moves in held steps on twos, never eased,
//! holds still through a blink, and leaves a closed eye alone.

use crate::clock::ON_TWOS;
use crate::document::Part;
use crate::geometry::Point;

/// How much further than its eye a part inside it moves.
const INSIDE_MORE: f64 = 1.5;

/// A direction held to the square from −1 to 1 each way.
pub fn bounded([x, y]: Point) -> Point {
  let hold = |value: f64| if value.is_finite() { value.clamp(-1.0, 1.0) } else { 0.0 };
  [hold(x), hold(y)]
}

/// An eye looking along a direction: an open eye moved within its reach, a closed one as it stands.
pub fn gazed(eye: &Part, reach: Point, gaze: Point) -> Part {
  if !eye.is_solid() || gaze == [0.0, 0.0] {
    return eye.clone();
  }
  eye.shifted([gaze[0] * reach[0], gaze[1] * reach[1]], INSIDE_MORE)
}

/// The directions a caller has set, each from the frame it was set at.
#[derive(Clone, Debug, Default)]
pub struct GazeTrack {
  set: Vec<(i64, Point)>,
}

impl GazeTrack {
  pub fn set(&mut self, frame: i64, gaze: Point) {
    self.set.retain(|(at, _)| *at < frame);
    self.set.push((frame, bounded(gaze)));
    // What was set more than a few settings ago is never read again.
    let keep_from = self.set.len().saturating_sub(4);
    self.set.drain(..keep_from);
  }

  /// The direction shown at a frame: the last one set by the frame's step on twos, or, through a blink
  /// that began at `blink`, by the step before it.
  pub fn at(&self, frame: i64, blink: Option<i64>) -> Point {
    let mut reading = frame - frame.rem_euclid(ON_TWOS);
    if let Some(start) = blink {
      reading = reading.min(start - 1);
    }
    self
      .set
      .iter()
      .rev()
      .find(|(at, _)| *at <= reading)
      .map_or([0.0, 0.0], |(_, gaze)| *gaze)
  }

  /// The next frame after this one where a direction set takes hold.
  pub fn next_move(&self, frame: i64) -> Option<i64> {
    self
      .set
      .iter()
      .map(|(at, _)| at + (ON_TWOS - at.rem_euclid(ON_TWOS)) % ON_TWOS)
      .filter(|step| *step > frame)
      .min()
  }
}
