//! The blend by correspondence (docs/face.md, 「Blending one drawing into another」): no in-between of
//! any two default faces crosses itself, turns inside out or grows a hook past the two drawings it
//! stands between.

use bmo_face_core::blend::blend_paths;
use bmo_face_core::document::Part;
use bmo_face_core::look::Look;
use bmo_face_core::sets::{line_mouth, opening};
use bmo_face_core::shapes::{Insides, mouth};

use crate::support::looks;

const SHARES: [f64; 7] = [0.1, 0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 0.75, 0.9];

/// The parts of two looks standing at the same place, side by side.
fn pairs<'a>(from: &'a Look, to: &'a Look) -> Vec<(&'a Part, &'a Part)> {
  let lists = |look: &'a Look| -> Vec<&'a Vec<Part>> {
    vec![
      &look.blush[0],
      &look.blush[1],
      &look.eyes[0],
      &look.eyes[1],
      &look.brows[0],
      &look.brows[1],
      &look.mouth,
      &look.marks,
    ]
  };
  lists(from)
    .into_iter()
    .zip(lists(to))
    .flat_map(|(a, b)| a.iter().zip(b.iter()))
    .collect()
}

fn assert_clean(from: &Part, to: &Part, share: f64, what: &str) {
  let blended = blend_paths(&from.path(), &to.path(), share);
  assert!(!blended.crosses_itself(), "{what} at {share}: crosses itself");
  let held = from.path().bounds().union(&to.path().bounds());
  assert!(held.holds(&blended.bounds(), 0.05), "{what} at {share}: reaches past both drawings");
  if from.closed && to.closed {
    assert!(blended.area() > 0.0, "{what} at {share}: turned inside out");
  } else if from.closed || to.closed {
    assert!(blended.area().abs() > 0.0, "{what} at {share}: closed on nothing");
  }
}

#[test]
fn no_in_between_of_two_default_faces_folds() {
  let looks = looks();
  for (from_name, from) in &looks {
    for (to_name, to) in &looks {
      if from_name == to_name {
        continue;
      }
      for (index, (a, b)) in pairs(from, to).into_iter().enumerate() {
        for share in SHARES {
          assert_clean(a, b, share, &format!("{from_name} to {to_name}, part {index}"));
        }
      }
      for share in SHARES {
        assert!(!from.blend(to, share).crosses_itself(), "{from_name} to {to_name} at {share}");
      }
    }
  }
}

#[test]
fn an_open_line_opens_into_a_closed_bowl_as_a_mouth_does() {
  let line = line_mouth(16.0, 0.0);
  let bowl = mouth(
    &opening(28.8, 15.8),
    Insides {
      teeth: 0.22,
      lower: 0.0,
      tongue: 0.38,
    },
    0.8,
  );
  for share in SHARES {
    assert_clean(&line, &bowl, share, "line to bowl");
    assert_clean(&bowl, &line, share, "bowl to line");
    let blended = blend_paths(&line.path(), &bowl.path(), share);
    assert!(blended.closed);
    // It opens: the further along, the taller.
    assert!(blended.bounds().height() > share * bowl.path().bounds().height() * 0.9);
  }
}

#[test]
fn bringing_the_counts_together_leaves_both_ends_of_the_blend_as_drawn() {
  let line = line_mouth(16.0, 0.0);
  let bowl = mouth(&opening(28.8, 15.8), Insides::default(), 0.8);
  let at_bowl = blend_paths(&line.path(), &bowl.path(), 1.0);
  for point in at_bowl.polyline(10) {
    let nearest = bowl
      .path()
      .polyline(2_000)
      .into_iter()
      .map(|p| (p[0] - point[0]).hypot(p[1] - point[1]))
      .fold(f64::INFINITY, f64::min);
    assert!(nearest < 0.05, "{point:?} {nearest}");
  }
}
