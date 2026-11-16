//! The face from frame to frame (docs/face.md, 「The face's core」): which face shows, the talking mouth
//! following its sound every frame or on twos, the blink, the change between faces and the gaze, and the
//! frames a caller is told to wake at.

use bmo_face_core::drawing::Drawing;
use bmo_face_core::hearing::{Heard, hear};
use bmo_face_core::look::Look;
use bmo_face_core::names::EMOTIONS;
use bmo_face_core::scene::Scene;
use bmo_face_core::talk::{LEAD, Speech};
use bmo_face_core::voice::sample_voice;

use crate::support::document;

fn scene() -> Scene {
  Scene::new(document(), 21, 0)
}

/// The sampled voice as the face hears it.
fn voice() -> Speech {
  Speech {
    heard: hear(&sample_voice(48_000), 48_000),
    cues: Vec::new(),
  }
}

/// The frames the drawing changes at, over a span, with whether each was announced by the frame before.
fn changes(scene: &mut Scene, from: i64, to: i64) -> Vec<(i64, bool)> {
  let mut found = Vec::new();
  let mut last: Drawing = scene.drawing(from);
  let mut announced = scene.next_change(from);
  for frame in from + 1..to {
    let drawing = scene.drawing(frame);
    if drawing != last {
      found.push((frame, announced == Some(frame)));
    }
    if drawing != last || announced.is_some_and(|at| at <= frame) {
      announced = scene.next_change(frame);
    }
    last = drawing;
  }
  found
}

#[test]
fn every_change_of_the_drawing_is_announced_and_nothing_moves_between() {
  let mut scene = scene();
  scene.speak(&voice(), EMOTIONS[1], Vec::new(), 300);
  scene.gaze([0.6, -0.3], 1_001);
  for (frame, announced) in changes(&mut scene, 0, 1_400) {
    assert!(announced, "the drawing changed unannounced at {frame}");
  }
}

#[test]
fn a_briefing_shows_its_face_and_its_mouth_follows_the_sound_every_frame() {
  let mut scene = scene();
  scene.set_blinking(false);
  let speech = voice();
  let end = 100 + speech.heard.len() as i64;
  scene.speak(&speech, EMOTIONS[1], Vec::new(), 100);
  let joy = Look::of(document().face(EMOTIONS[1]));
  let mut changed = 0;
  let mut last = scene.look(120).mouth;
  for frame in 121..end {
    let look = scene.look(frame);
    assert_eq!(scene.face_at(frame), EMOTIONS[1]);
    assert_eq!(look.eyes[0].len(), joy.eyes[0].len(), "frame {frame}");
    changed += usize::from(look.mouth != last);
    last = look.mouth;
  }
  assert!(changed * 2 > (end - 121) as usize, "{changed}");
  // After the sound and the linger, the face of the presence shows again with its own mouth.
  let idle = Look::of(document().face("idle"));
  assert_eq!(scene.look(end + 96 + 20).mouth, idle.mouth);
}

#[test]
fn on_twos_the_mouth_holds_each_drawing_two_frames() {
  let mut on_twos = document();
  on_twos.talk.tuning.on_twos = true;
  let mut scene = Scene::new(on_twos, 21, 0);
  scene.set_blinking(false);
  scene.pin(Some(EMOTIONS[1]));
  scene.speak(&voice(), EMOTIONS[1], Vec::new(), 101);
  let mut last = scene.look(100 - LEAD).mouth;
  for frame in 100 - LEAD + 1..200 {
    let mouth = scene.look(frame).mouth;
    if mouth != last {
      assert_eq!((frame - 101 + LEAD).rem_euclid(2), 0, "frame {frame}");
    }
    last = mouth;
  }
}

#[test]
fn silence_shows_the_face_its_own_mouth_and_the_same_sound_the_same_face_whatever_the_seed() {
  let silent = Speech {
    heard: vec![Heard::silent(); 48],
    cues: Vec::new(),
  };
  let mut scene = scene();
  scene.pin(Some(EMOTIONS[1]));
  scene.speak(&silent, EMOTIONS[1], Vec::new(), 10);
  let own = Look::of(document().face(EMOTIONS[1])).mouth;
  for frame in 8..58 {
    assert_eq!(scene.look(frame).mouth, own, "frame {frame}");
  }
  let mouths = |seed: u64| {
    let mut scene = Scene::new(document(), seed, 0);
    scene.pin(Some(EMOTIONS[0]));
    scene.speak(&voice(), EMOTIONS[0], Vec::new(), 4);
    (0..120).map(|frame| scene.look(frame).mouth).collect::<Vec<_>>()
  };
  assert_eq!(mouths(1), mouths(99));
}

#[test]
fn a_face_whose_eyes_are_already_closed_never_blinks() {
  let mut scene = scene();
  scene.pin(Some(EMOTIONS[6]));
  let laughing = Look::of(document().face(EMOTIONS[6]));
  for frame in 0..2_000 {
    assert_eq!(scene.look(frame).eyes, laughing.eyes, "frame {frame}");
  }
}

#[test]
fn an_open_eyed_face_blinks_every_few_seconds() {
  let mut scene = scene();
  let open = Look::of(document().face("idle")).eyes;
  let shut = (0..2_000).filter(|frame| scene.look(*frame).eyes != open).count();
  // Some 14 blinks of four frames each over 2,000 frames, one in seven doubled.
  assert!((40..=110).contains(&shut), "{shut}");
}

#[test]
fn a_change_of_face_moves_in_held_steps_on_twos() {
  let mut scene = scene();
  scene.set_blinking(false);
  let mut moves = Vec::new();
  for seed_frame in [10, 300, 600, 900, 1_200] {
    scene.presence(if seed_frame % 600 == 0 { "writing" } else { "thinking" }, seed_frame);
    let mut last = scene.look(seed_frame);
    for frame in seed_frame + 1..seed_frame + 20 {
      let look = scene.look(frame);
      if look != last {
        moves.push(frame - seed_frame);
      }
      last = look;
    }
  }
  assert!(moves.iter().all(|offset| offset % 2 == 0), "{moves:?}");
}

#[test]
fn the_gaze_takes_hold_on_the_next_step_and_moves_open_eyes_within_their_reach() {
  let mut scene = scene();
  scene.set_blinking(false);
  let centre = scene.look(40).eyes[0][0].path().bounds().centre();
  scene.gaze([1.0, 0.0], 41);
  assert_eq!(scene.look(41).eyes[0][0].path().bounds().centre(), centre);
  let moved = scene.look(42).eyes[0][0].path().bounds().centre();
  assert!((moved[0] - centre[0] - 2.2).abs() < 1e-9);
}
