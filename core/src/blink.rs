//! The blink (docs/face.md, 「The blink」): on the held clock, a lid half down for one frame, the eye shut
//! in a shallow arc for two, half down again for one, then open; every four to six seconds with jitter,
//! one in seven followed at once by a second. It runs whatever else the face does, and leaves alone an
//! eye already closed: a stroke has no lid to lower.

use crate::document::Part;
use crate::geometry::Point;
use crate::random::Random;
use crate::shapes::{cut_above, stroke};

/// Where the lid stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lid {
  Open,
  Half,
  Shut,
}

/// One blink, frame by frame.
pub const SEQUENCE: [Lid; 4] = [Lid::Half, Lid::Shut, Lid::Shut, Lid::Half];

/// How many frames lie between one blink's start and the next's: four to six seconds.
const INTERVAL: (i64, i64) = (96, 144);

/// How often a blink is followed at once by a second, and the open frames between them.
const DOUBLE_SHARE: f64 = 1.0 / 7.0;
const DOUBLE_GAP: i64 = 2;

/// The width of a shut eye's arc.
const SHUT_LINE: f64 = 0.9;

/// How much of an eye's height a lid half down leaves.
const HALF_OPEN: f64 = 0.62;

/// The blinks of one seed, from frame nought on: the start of each, found in order and kept.
#[derive(Clone, Debug)]
pub struct Blinks {
  random: Random,
  starts: Vec<i64>,
  /// Where the next blink after the last one found starts.
  next: i64,
}

impl Blinks {
  pub fn new(seed: u64) -> Blinks {
    let mut random = Random::for_use(seed, 3);
    let next = random.between(INTERVAL.0 / 2, INTERVAL.1);
    Blinks {
      random,
      starts: Vec::new(),
      next,
    }
  }

  /// Finds the blinks up to a frame.
  fn reach(&mut self, frame: i64) {
    while self.next <= frame + SEQUENCE.len() as i64 {
      let start = self.next;
      self.starts.push(start);
      let mut after = start + SEQUENCE.len() as i64;
      if self.random.chance(DOUBLE_SHARE) {
        self.starts.push(after + DOUBLE_GAP);
        after += DOUBLE_GAP + SEQUENCE.len() as i64;
      }
      self.next = after.max(start + self.random.between(INTERVAL.0, INTERVAL.1));
      // Only the blinks near the frames asked of are kept.
      let keep_from = self.starts.len().saturating_sub(8);
      self.starts.drain(..keep_from);
    }
  }

  /// The blink standing at a frame, and the frame it began; none between blinks.
  pub fn at(&mut self, frame: i64) -> Option<(Lid, i64)> {
    self.reach(frame);
    self.starts.iter().rev().find(|start| **start <= frame).and_then(|start| {
      let step = frame - start;
      SEQUENCE.get(step as usize).map(|lid| (*lid, *start))
    })
  }

  /// The next frame after this one where the lid moves.
  pub fn next_move(&mut self, frame: i64) -> i64 {
    self.reach(frame + INTERVAL.1 * 2);
    for start in &self.starts {
      for step in 0..=SEQUENCE.len() as i64 {
        if start + step > frame {
          return start + step;
        }
      }
    }
    self.next
  }
}

/// An eye under the lid: a solid eye half covered keeps its lower part, a shut one is a shallow ink arc
/// across its lower half, a little wider than the eye; a stroke stays as it stands.
pub fn lidded(eye: &Part, lid: Lid) -> Part {
  if !eye.is_solid() || lid == Lid::Open {
    return eye.clone();
  }
  let bounds = eye.path().bounds();
  match lid {
    Lid::Open => eye.clone(),
    Lid::Half => cut_above(eye, bounds.bottom - bounds.height() * HALF_OPEN),
    Lid::Shut => {
      let [cx, cy] = bounds.centre();
      let half = (bounds.width() * 0.8).max(3.0);
      let level = cy + bounds.height() * 0.12;
      let depth = (bounds.width() * 0.3).clamp(1.2, 2.4);
      let ends: [Point; 3] = [[cx - half, level], [cx, level + depth], [cx + half, level]];
      stroke(&ends, SHUT_LINE)
    }
  }
}
