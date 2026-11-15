//! The blink (docs/face.md, 「The blink」).

use bmo_face_core::blink::{Blinks, Lid, SEQUENCE, lidded};
use bmo_face_core::shapes::{dot, stroke};

#[test]
fn a_blink_is_half_shut_shut_half_one_frame_two_frames_one_frame() {
  assert_eq!(SEQUENCE, [Lid::Half, Lid::Shut, Lid::Shut, Lid::Half]);
}

#[test]
fn blinks_come_every_four_to_six_seconds_one_in_seven_doubled() {
  let mut blinks = Blinks::new(11);
  let mut starts = Vec::new();
  let mut frame = 0;
  while frame < 240_000 {
    if let Some((_, start)) = blinks.at(frame)
      && starts.last() != Some(&start)
    {
      starts.push(start);
    }
    frame += 1;
  }
  let gaps: Vec<i64> = starts.windows(2).map(|pair| pair[1] - pair[0]).collect();
  let doubles = gaps.iter().filter(|gap| **gap == 6).count();
  let apart: Vec<&i64> = gaps.iter().filter(|gap| **gap != 6).collect();
  assert!(apart.iter().all(|gap| (90..=144).contains(*gap)), "{apart:?}");
  let share = doubles as f64 / apart.len() as f64;
  assert!((share - 1.0 / 7.0).abs() < 0.03, "{share}");
}

#[test]
fn the_lid_cuts_an_open_eye_and_leaves_a_closed_one_alone() {
  let eye = dot([23.2, 24.6], 3.0, 3.25);
  let half = lidded(&eye, Lid::Half).path().bounds();
  assert!((half.height() - 6.5 * 0.62).abs() < 1e-6);
  let shut = lidded(&eye, Lid::Shut);
  assert!(!shut.closed && shut.line.is_some());
  assert!(shut.path().bounds().width() > 6.0);
  let closed_eye = stroke(&[[18.0, 25.0], [23.0, 27.0], [28.0, 25.0]], 0.9);
  assert_eq!(lidded(&closed_eye, Lid::Shut), closed_eye);
}
