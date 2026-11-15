//! The held clock as the displayer counts it, and the playback clock mapped onto it (docs/face.md,
//! 「Speaking」).

use std::time::{Duration, Instant};

use bmo_face_core::clock::ms_at;
use bmo_face_displayer::clock::HeldClock;
use bmo_face_displayer::playback::{heard_seconds, silence_before};

fn after(origin: Instant, ms: f64) -> Instant {
  origin + Duration::from_secs_f64(ms / 1_000.0)
}

#[test]
fn frames_are_counted_from_the_origin_and_each_begins_where_it_says() {
  let origin = Instant::now();
  let mut clock = HeldClock::new(origin);
  assert_eq!(clock.frame(origin), 0);
  assert_eq!(clock.frame(after(origin, 1_000.0)), 23);
  for frame in [24, 240, 24_000] {
    let begins = clock.instant_of(frame);
    assert_eq!(clock.frame(begins), frame);
  }
}

#[test]
fn the_clock_follows_the_playback_clock_forward_and_back_but_never_behind_a_frame_asked() {
  let origin = Instant::now();
  let mut clock = HeldClock::new(origin);
  let at = after(origin, 10_000.0);
  assert_eq!(clock.frame(at), 239);

  // The device says the sound stands a little further on than the monotonic clock: the frames move on.
  clock.follow(at, ms_at(240) + 5.0);
  assert_eq!(clock.frame(at), 240);

  // It says the sound is behind again: later frames wait for it, but the frame asked stays.
  clock.follow(at, ms_at(236));
  assert_eq!(clock.frame(at), 240);
  assert_eq!(clock.frame(after(origin, 10_000.0 + ms_at(5))), 241);
}

#[test]
fn the_sound_heard_is_what_was_given_less_what_the_device_holds() {
  assert_eq!(heard_seconds(48_000, 2_400, 48_000), 0.95);
  assert_eq!(heard_seconds(1_200, 2_400, 48_000), -0.025);
}

#[test]
fn silence_puts_the_first_sample_on_its_moment_or_says_how_late_it_is() {
  let now = Instant::now();
  let target = now + Duration::from_millis(100);
  // A hundred milliseconds ahead with 1,200 frames already held: 3,600 more to give.
  assert_eq!(silence_before(target, now, 1_200, 48_000), 3_600);
  // Already 10 ms late with nothing held: 480 frames of the sound to drop.
  assert_eq!(silence_before(now, now + Duration::from_millis(10), 0, 48_000), -480);
}
