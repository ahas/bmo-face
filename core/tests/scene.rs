//! The face from frame to frame (docs/face.md, 「The face's core」): which face shows, the talking mouth
//! cut on the held clock, the blink, the change between faces and the gaze, and the frames a caller is
//! told to wake at.

use bmo_face_core::drawing::Drawing;
use bmo_face_core::look::Look;
use bmo_face_core::names::EMOTIONS;
use bmo_face_core::scene::Scene;
use bmo_face_core::talk::{LEAD, Speech, sample_loudness};

use crate::support::document;

fn scene() -> Scene {
  Scene::new(document(), 21, 0)
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
  scene.speak(
    &Speech {
      loudness: sample_loudness(4, 600),
      cues: Vec::new(),
      seed: 4,
    },
    EMOTIONS[1],
    Vec::new(),
    300,
  );
  scene.gaze([0.6, -0.3], 1_001);
  for (frame, announced) in changes(&mut scene, 0, 1_400) {
    assert!(announced, "the drawing changed unannounced at {frame}");
  }
}

#[test]
fn a_briefing_shows_its_face_and_cuts_its_mouth_on_the_held_clock() {
  let mut scene = scene();
  let loudness = sample_loudness(8, 480);
  scene.speak(
    &Speech {
      loudness,
      cues: Vec::new(),
      seed: 8,
    },
    EMOTIONS[1],
    Vec::new(),
    100,
  );
  let joy = Look::of(document().face(EMOTIONS[1]));
  // Past the change into the briefing's face, the eyes are joy's and the mouth one of the talking set.
  let talking: Vec<_> = document()
    .talk
    .iter()
    .flat_map(|family| family.mouths.iter().map(|mouth| mouth.parts.clone()))
    .collect();
  for frame in 120..560 {
    let look = scene.look(frame);
    assert!(talking.contains(&look.mouth), "frame {frame}");
    assert_eq!(scene.face_at(frame), EMOTIONS[1]);
    if look.eyes == joy.eyes {
      continue;
    }
    // A blink or the gaze moves the eyes; their count of parts stays joy's.
    assert_eq!(look.eyes[0].len(), joy.eyes[0].len(), "frame {frame}");
  }
  // The mouth changes only at a step: never at an odd frame where the hold was two.
  let mut changed = Vec::new();
  let mut last = scene.look(100 - LEAD).mouth;
  for frame in 100 - LEAD + 1..580 {
    let mouth = scene.look(frame).mouth;
    if mouth != last {
      changed.push(frame);
    }
    last = mouth;
  }
  let holds: Vec<i64> = changed.windows(2).map(|pair| pair[1] - pair[0]).collect();
  assert!(holds.iter().all(|hold| [1, 2, 3, 4, 5, 6, 8].contains(hold)), "{holds:?}");
  // After the sound and the linger, the face of the presence shows again with its own mouth.
  let idle = Look::of(document().face("idle"));
  assert_eq!(scene.look(580 + 96 + 20).mouth, idle.mouth);
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
