//! The talking mouth drawn from its anchors (docs/face.md, 「The mouth while speaking」 and 「The talking
//! set」): each of the set's 23 drawings standing where it measures and drawn as it stands there, every
//! mouth between them and every lift of its corners never crossing itself, the teeth and the tongue
//! shown as far as asked, and the eyes answering the voice no further than they allow.

use bmo_face_core::document::{Paint, Part};
use bmo_face_core::geometry::Bounds;
use bmo_face_core::hearing::hear;
use bmo_face_core::look::Look;
use bmo_face_core::mouth::{LIFT_WIDTH, Mouths, Plane, Shape, lifted, measure, tongued};
use bmo_face_core::names::{EMOTIONS, FALLBACK, face_names};
use bmo_face_core::scene::Scene;
use bmo_face_core::talk::Speech;
use bmo_face_core::voice::sample_voice;

use crate::support::document;

fn bounds(parts: &[Part]) -> Bounds {
  parts
    .iter()
    .map(|part| part.path().bounds())
    .reduce(|one, other| one.union(&other))
    .unwrap()
}

fn crosses(parts: &[Part]) -> bool {
  parts
    .iter()
    .flat_map(|part| std::iter::once(part).chain(&part.inside))
    .any(|part| part.path().crosses_itself())
}

#[test]
fn the_set_stands_where_each_drawing_measures_two_with_the_teeth_together() {
  let anchors = document().talk.anchors;
  assert_eq!(anchors.len(), 23);
  for anchor in &anchors {
    assert_eq!(measure(&anchor.parts), Some((anchor.at, anchor.teeth)));
  }
  assert_eq!(anchors.iter().filter(|anchor| anchor.teeth).count(), 2);
  // Lips drawn as a line stand at nought; the largest round O is fully open.
  assert_eq!(anchors.iter().filter(|anchor| anchor.at[0] == 0.0).count(), 3);
  assert!(anchors.iter().any(|anchor| anchor.at[0] == 1.0));
}

#[test]
fn every_anchor_is_drawn_as_it_stands_at_its_place() {
  let anchors = document().talk.anchors;
  let mouths = Mouths::new(&anchors);
  for anchor in &anchors {
    assert_eq!(mouths.at(anchor.at, anchor.teeth), anchor.parts, "{:?}", anchor.at);
  }
}

#[test]
fn the_shape_that_reaches_an_anchor_draws_it() {
  let anchors = document().talk.anchors;
  let mouths = Mouths::new(&anchors);
  for teeth in [false, true] {
    let plane = Plane::new(
      anchors
        .iter()
        .enumerate()
        .filter(|(_, anchor)| anchor.teeth == teeth)
        .map(|(index, anchor)| (index, anchor.at))
        .collect(),
    )
    .unwrap();
    for anchor in anchors.iter().filter(|anchor| anchor.teeth == teeth) {
      let [open, wide] = anchor.at;
      let (widest, narrowest) = (plane.place(open, 0.0)[1], plane.place(open, 1.0)[1]);
      let round = if widest > narrowest { (widest - wide) / (widest - narrowest) } else { 0.0 };
      let shape = Shape {
        open,
        round,
        teeth: if teeth { 1.0 } else { 0.0 },
        tongue: 1.0,
      };
      let (drawn, own) = (bounds(&mouths.draw(shape, 0.0)), bounds(&anchor.parts));
      for (one, other) in [
        (drawn.left, own.left),
        (drawn.top, own.top),
        (drawn.right, own.right),
        (drawn.bottom, own.bottom),
      ] {
        assert!((one - other).abs() < 0.01, "{:?}: {drawn:?} against {own:?}", anchor.at);
      }
    }
  }
}

#[test]
fn no_mouth_of_the_whole_plane_crosses_itself_at_any_lift() {
  let mouths = Mouths::new(&document().talk.anchors);
  for open in 0..=10 {
    for round in 0..=4 {
      for teeth in [0.0, 0.5, 1.0] {
        for (tongue, lift) in [(1.0, -4.0), (0.5, 0.0), (1.0, 4.0)] {
          let shape = Shape {
            open: f64::from(open) / 10.0,
            round: f64::from(round) / 4.0,
            teeth,
            tongue,
          };
          let drawn = mouths.draw(shape, lift);
          assert!(!drawn.is_empty() && !crosses(&drawn), "{shape:?} lifted {lift}");
        }
      }
    }
  }
}

#[test]
fn a_wider_and_more_open_shape_draws_a_wider_and_taller_mouth() {
  let mouths = Mouths::new(&document().talk.anchors);
  let size = |open: f64, round: f64| {
    let drawn = bounds(&mouths.draw(
      Shape {
        open,
        round,
        teeth: 0.0,
        tongue: 1.0,
      },
      0.0,
    ));
    (drawn.width(), drawn.height())
  };
  assert!(size(0.8, 0.0).1 > size(0.4, 0.0).1 + 4.0);
  assert!(size(0.5, 0.0).0 > size(0.5, 1.0).0 + 10.0);
}

#[test]
fn the_tongue_sinks_into_its_base_and_the_corners_lift_by_the_width() {
  let joy = document().face(EMOTIONS[1]).mouth.clone();
  let tongue = |parts: &[Part]| {
    parts[0]
      .inside
      .iter()
      .find(|inner| inner.fill == Some(Paint::Tongue))
      .map(|inner| inner.path().bounds())
  };
  let whole = tongue(&joy).unwrap();
  let half = tongue(&tongued(&joy, 0.5)).unwrap();
  assert!((half.bottom - whole.bottom).abs() < 1e-9);
  assert!((half.height() - whole.height() / 2.0).abs() < 1e-6);
  assert_eq!(tongue(&tongued(&joy, 0.01)), None);

  let line = document().face("idle").mouth.clone();
  let width = bounds(&line).width();
  let ends = |parts: &[Part]| parts[0].nodes[0].anchor[1];
  let middle = |parts: &[Part]| parts[0].nodes[1].anchor[1];
  let up = lifted(&line, 2.0);
  assert!((ends(&line) - ends(&up) - 2.0 * width / LIFT_WIDTH).abs() < 1e-9);
  assert!((middle(&line) - middle(&up)).abs() < 1e-9);
}

#[test]
fn the_eyes_answer_the_voice_within_what_each_allows_and_closed_ones_not_at_all() {
  let heard = hear(&sample_voice(48_000), 48_000);
  for name in face_names() {
    let own = Look::of(document().face(name));
    let mut scene = Scene::new(document(), 3, 0);
    scene.pin(Some(name));
    scene.set_blinking(false);
    scene.speak(
      &Speech {
        heard: heard.clone(),
        cues: Vec::new(),
      },
      name,
      Vec::new(),
      2,
    );
    let mut grew = false;
    for frame in 0..heard.len() as i64 {
      let look = scene.look(frame);
      for (side, eyes) in look.eyes.iter().enumerate() {
        for (eye, rest) in eyes.iter().zip(&own.eyes[side]) {
          if !rest.is_solid() {
            assert_eq!(eye, rest, "{name}");
            continue;
          }
          let stretch = eye.path().bounds().height() / rest.path().bounds().height();
          assert!(
            stretch <= 1.0 + own.voice.grow + 1e-9 && stretch >= 1.0 - own.voice.squint - 1e-9,
            "{name} {stretch}"
          );
          grew |= stretch > 1.0 + 1e-6;
          for (shine, still) in eye.inside.iter().zip(&rest.inside) {
            let shimmer = shine.path().bounds().width() / still.path().bounds().width();
            assert!(shimmer <= 1.0 + own.voice.shine + 1e-9, "{name} {shimmer}");
          }
        }
      }
    }
    assert_eq!(grew, own.blinks(), "{name}");
  }
}

#[test]
fn a_blink_wins_over_the_eyes_answer() {
  let heard = hear(&sample_voice(48_000), 48_000);
  let mut quiet = Scene::new(document(), 3, 0);
  let mut speaking = Scene::new(document(), 3, 0);
  for scene in [&mut quiet, &mut speaking] {
    scene.pin(Some(FALLBACK));
  }
  speaking.speak(
    &Speech {
      heard: heard.clone(),
      cues: Vec::new(),
    },
    FALLBACK,
    Vec::new(),
    2,
  );
  let open = Look::of(document().face(FALLBACK)).eyes;
  let mut blinks = 0;
  for frame in 0..heard.len() as i64 {
    let still = quiet.look(frame).eyes;
    let answered = speaking.look(frame).eyes;
    if still != open {
      blinks += 1;
      assert_eq!(answered, still, "frame {frame}");
    }
  }
  assert!(blinks > 0);
}
