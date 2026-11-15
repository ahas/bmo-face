//! The held clock (docs/face.md, 「The held clock」): the face is drawn on the animation's own frames,
//! 23.976 of them a second, and every drawing is held for whole frames. Frames are counted by the
//! caller from a moment of its own; the core keeps no clock.

/// Frames a second: the animation's film rate.
pub const FPS: f64 = 24_000.0 / 1_001.0;

/// How many frames an ordinary drawing is held: the animation draws on twos.
pub const ON_TWOS: i64 = 2;

/// The frame standing at a moment, `ms` milliseconds after the caller's frame nought.
pub fn frame_at(ms: f64) -> i64 {
  (ms * FPS / 1_000.0 + 1e-6).floor() as i64
}

/// The moment a frame begins, in milliseconds after the caller's frame nought.
pub fn ms_at(frame: i64) -> f64 {
  frame as f64 * 1_000.0 / FPS
}

/// How many whole frames a span of seconds holds.
pub fn frames_in(seconds: f64) -> i64 {
  (seconds * FPS).round() as i64
}
