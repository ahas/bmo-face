//! The held clock as the displayer counts it (docs/face.md, 「The held clock」 and 「Speaking」): frames
//! of the animation from the moment the displayer started, on the monotonic clock, moved to follow the
//! sound device while a briefing is heard, and never back past a frame already asked.

use std::time::{Duration, Instant};

use bmo_face_core::clock::{frame_at, ms_at};

/// The frames counted from a moment, shifted by what the playback clock has said.
#[derive(Clone, Debug)]
pub struct HeldClock {
  origin: Instant,
  /// Milliseconds added to the monotonic clock's reading: where the sound device says the clock stands.
  shift_ms: f64,
  /// The latest frame handed out, so a shift backwards never hands out an earlier one.
  asked: i64,
}

impl HeldClock {
  pub fn new(origin: Instant) -> HeldClock {
    HeldClock {
      origin,
      shift_ms: 0.0,
      asked: 0,
    }
  }

  /// Milliseconds on the held clock at an instant.
  pub fn ms_at(&self, at: Instant) -> f64 {
    at.saturating_duration_since(self.origin).as_secs_f64() * 1_000.0 + self.shift_ms
  }

  /// The frame standing at an instant, never one before a frame already handed out.
  pub fn frame(&mut self, at: Instant) -> i64 {
    self.asked = self.asked.max(frame_at(self.ms_at(at)));
    self.asked
  }

  /// The instant a frame begins.
  pub fn instant_of(&self, frame: i64) -> Instant {
    let ms = ms_at(frame) - self.shift_ms;
    if ms <= 0.0 {
      self.origin
    } else {
      self.origin + Duration::from_secs_f64(ms / 1_000.0)
    }
  }

  /// The playback clock says that at `at` the held clock stands at `ms`: the clock follows it from now.
  pub fn follow(&mut self, at: Instant, ms: f64) {
    self.shift_ms += ms - self.ms_at(at);
  }
}
