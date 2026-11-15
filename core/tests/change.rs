//! An expression change (docs/face.md, 「Moving between faces」).

use bmo_face_core::change::plan;
use bmo_face_core::random::Random;

use crate::support::looks;

#[test]
fn a_change_is_a_cut_a_third_of_the_time_otherwise_one_to_three_in_betweens() {
  let looks = looks();
  let (from, to) = (&looks[0].1, &looks[1].1);
  let mut random = Random::new(1);
  let counts: Vec<usize> = (0..900).map(|_| plan(from, to, &mut random, false).len()).collect();
  let cuts = counts.iter().filter(|count| **count == 0).count() as f64 / counts.len() as f64;
  assert!((cuts - 1.0 / 3.0).abs() < 0.05, "{cuts}");
  assert!(counts.iter().all(|count| *count <= 4));
}

#[test]
fn a_calm_change_is_always_a_cut() {
  let looks = looks();
  let mut random = Random::new(2);
  for _ in 0..50 {
    assert!(plan(&looks[0].1, &looks[2].1, &mut random, true).is_empty());
  }
}
