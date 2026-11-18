//! The face from frame to frame (docs/face.md, 「The face's core」): which face shows — at rest, the face
//! held, the blank face and sleep, each counted on the held clock — the talking mouth following its sound
//! every frame or on twos and closing in silence, the blink, the change between faces and the gaze, and
//! the frames a caller is told to wake at.

use bmo_face_core::clock::frames_in;
use bmo_face_core::document::{Document, Part};
use bmo_face_core::drawing::Drawing;
use bmo_face_core::hearing::{Heard, hear};
use bmo_face_core::idle::Idle;
use bmo_face_core::look::Look;
use bmo_face_core::names::{ASLEEP, EMOTIONS, FALLBACK, IDLE};
use bmo_face_core::scene::{LINGER_SECONDS, Scene};
use bmo_face_core::talk::{LEAD, Speech};
use bmo_face_core::voice::{sample_voice, stretches};

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
fn silence_shows_a_line_mouthed_face_its_own_mouth_and_the_same_sound_the_same_face_whatever_the_seed() {
  let silent = Speech {
    heard: vec![Heard::silent(); 48],
    cues: Vec::new(),
  };
  let mut scene = scene();
  scene.pin(Some(FALLBACK));
  scene.speak(&silent, FALLBACK, Vec::new(), 10);
  let own = Look::of(document().face(FALLBACK)).mouth;
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

/// Frames in a span of minutes, on the held clock.
fn minutes(count: f64) -> i64 {
  frames_in(count * 60.0)
}

/// Whether a mouth is closed: drawn, and no part of it a filled outline.
fn is_closed(mouth: &[Part]) -> bool {
  !mouth.is_empty() && !mouth.iter().any(Part::is_solid)
}

/// How far a closed mouth's ends stand above its middle: a smile above nought.
fn smile(mouth: &[Part]) -> f64 {
  let nodes = &mouth[0].nodes;
  nodes[1].anchor[1] - nodes[0].anchor[1]
}

/// A scene of a document that has spoken a sound with a face from frame 10, drawn once its sound ended,
/// and the frame it ended at.
fn spoken_in(document: Document, speech: &Speech, face: &str) -> (Scene, i64) {
  let mut scene = Scene::new(document, 21, 0);
  scene.set_blinking(false);
  scene.speak(speech, face, Vec::new(), 10);
  let end = 10 + speech.heard.len() as i64;
  scene.look(end);
  (scene, end)
}

fn spoken(face: &str) -> (Scene, i64) {
  spoken_in(document(), &voice(), face)
}

/// The sampled voice up to the end of the stretch after one of its stretches.
fn said_through(stretch: usize) -> Speech {
  let (_, to, _) = stretches()[stretch + 1];
  let mut speech = voice();
  speech.heard.truncate(to as usize);
  speech
}

#[test]
fn a_briefing_s_face_is_held_through_nine_minutes_of_rest_then_goes_blank_at_ten_and_to_sleep_at_thirty() {
  let (mut scene, end) = spoken(EMOTIONS[1]);
  for (after, face) in [(0.0, EMOTIONS[1]), (9.0, EMOTIONS[1]), (10.0, IDLE), (29.9, IDLE), (30.0, ASLEEP)]
  {
    scene.look(end + minutes(after));
    assert_eq!(scene.face_at(end + minutes(after)), face, "{after} minutes");
  }
}

#[test]
fn a_rest_is_counted_from_the_later_of_going_idle_and_the_briefing_s_end() {
  let (mut scene, end) = spoken(EMOTIONS[1]);
  scene.presence(IDLE, end + minutes(2.0));
  assert_eq!(scene.face_at(end + minutes(11.0)), EMOTIONS[1]);
  assert_eq!(scene.face_at(end + minutes(12.0)), IDLE);
}

#[test]
fn the_rest_s_stages_are_the_document_s_idle_settings() {
  let mut quick = document();
  quick.idle = Idle {
    blank: 60.0,
    asleep: 120.0,
  };
  let (scene, end) = spoken_in(quick, &voice(), EMOTIONS[1]);
  assert_eq!(scene.face_at(end + minutes(0.9)), EMOTIONS[1]);
  assert_eq!(scene.face_at(end + minutes(1.0)), IDLE);
  assert_eq!(scene.face_at(end + minutes(2.0)), ASLEEP);
}

#[test]
fn work_breaks_the_hold_at_once_and_a_rest_that_went_blank_forgets_the_briefing() {
  let (mut scene, end) = spoken(EMOTIONS[1]);
  let writing = end + minutes(1.0);
  scene.presence("writing", writing);
  assert_eq!(scene.face_at(writing), "writing");
  scene.presence(IDLE, writing + 100);
  assert_eq!(scene.face_at(writing + 100), EMOTIONS[1]);
  let thinking = writing + 100 + minutes(10.0);
  scene.presence("thinking", thinking);
  assert_eq!(scene.face_at(thinking), "thinking");
  scene.presence(IDLE, thinking + 100);
  assert_eq!(scene.face_at(thinking + 100), FALLBACK);
}

#[test]
fn a_face_whose_eyes_are_strokes_is_held_only_while_it_lingers_and_then_the_fallback_face() {
  let (scene, end) = spoken(EMOTIONS[6]);
  assert_eq!(scene.face_at(end + frames_in(LINGER_SECONDS) - 1), EMOTIONS[6]);
  assert_eq!(scene.face_at(end + frames_in(LINGER_SECONDS)), FALLBACK);
  assert_eq!(scene.face_at(end + minutes(9.0)), FALLBACK);
}

#[test]
fn the_blank_face_and_sleep_are_announced_as_the_frames_the_drawing_next_changes_at() {
  let (mut scene, end) = spoken(EMOTIONS[1]);
  scene.look(end + minutes(9.0));
  assert_eq!(scene.next_change(end + minutes(9.0)), Some(end + minutes(10.0)));
  // Past the change into the blank face.
  let blank = end + minutes(20.0);
  scene.look(blank);
  scene.look(blank + 20);
  assert_eq!(scene.next_change(blank + 20), Some(end + minutes(30.0)));
}

#[test]
fn an_open_mouth_is_closed_in_silence_before_between_and_after_its_words_and_open_in_them() {
  let mut scene = scene();
  scene.set_blinking(false);
  let words = said_through(1).heard;
  let quiet = vec![Heard::silent(); 24];
  let heard = [quiet.clone(), words.clone(), quiet.clone(), words.clone()].concat();
  let (start, length) = (10, heard.len() as i64);
  scene.speak(
    &Speech {
      heard,
      cues: Vec::new(),
    },
    EMOTIONS[1],
    Vec::new(),
    start,
  );
  let shown = |at: i64| start + at - LEAD;
  let words_at = 24 + words.len() as i64;
  assert!(is_closed(&scene.look(shown(2)).mouth), "before");
  let open = (shown(24)..shown(words_at)).any(|frame| !is_closed(&scene.look(frame).mouth));
  assert!(open, "the words open it");
  // Once the talking mouth has shut, past its release.
  for frame in shown(words_at + 12)..shown(words_at + 24) {
    let between = scene.look(frame).mouth;
    assert!(is_closed(&between) && smile(&between) > -1e-9, "between, frame {frame}");
  }
  let after = scene.look(start + length).mouth;
  assert!(is_closed(&after), "after");
  assert_eq!(scene.look(start + length + minutes(9.0)).mouth, after);
}

#[test]
fn the_closed_mouth_smiles_after_a_spread_vowel_and_lies_nearly_level_after_a_round_one() {
  // The sampled voice opens on a pause, then says a, i and u with a pause after each: stretch 1 is a,
  // stretch 5 is u.
  let mouth = |document: Document, stretch: usize, face: &str| {
    let (mut scene, end) = spoken_in(document, &said_through(stretch), face);
    scene.look(end).mouth
  };
  let after_a = mouth(document(), 1, EMOTIONS[1]);
  let after_u = mouth(document(), 5, EMOTIONS[1]);
  assert!(is_closed(&after_a) && is_closed(&after_u));
  let width = |mouth: &[Part]| mouth[0].path().bounds().width();
  assert!(width(&after_a) > width(&after_u), "{} {}", width(&after_a), width(&after_u));
  assert!(
    smile(&after_a) > 0.5 && smile(&after_a) > smile(&after_u) * 1.5,
    "{} {}",
    smile(&after_a),
    smile(&after_u)
  );
  // A face that drops its corners closes in a level line.
  let sad = mouth(document(), 1, EMOTIONS[2]);
  assert!(is_closed(&sad) && smile(&sad).abs() < 1e-9, "{}", smile(&sad));
}
