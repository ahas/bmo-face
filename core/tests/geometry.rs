//! The geometry a path is drawn by (docs/face.md, 「How a path is written」).

use bmo_face_core::document::{Paint, Part};
use bmo_face_core::geometry::{Cubic, Path};
use bmo_face_core::shapes::{Opening, cut_above, dot, smooth_nodes, stroke};

#[test]
fn three_points_make_an_arc_not_two_strokes_meeting_in_a_bend() {
  // The calm mouth of the old default: 20 wide, 4 deep.
  let arc = stroke(&[[40.0, 37.0], [50.0, 41.0], [60.0, 37.0]], 0.8).path();
  // Along a parabola through the three points, the curve stands within a hair of it everywhere.
  for segment in &arc.segments {
    for step in 0..=20 {
      let [x, y] = segment.at(step as f64 / 20.0);
      let parabola = 41.0 - 4.0 * ((x - 50.0) / 10.0).powi(2);
      assert!((y - parabola).abs() < 0.02, "{x} {y} {parabola}");
    }
  }
  // Its end leaves at the parabola's slope, twice that of the straight step to the middle.
  let first = smooth_nodes(&[[40.0, 37.0], [50.0, 41.0], [60.0, 37.0]], false)[0].leaving;
  assert!((first[1] / first[0] - 0.8).abs() < 1e-9);
}

#[test]
fn a_run_of_more_points_stays_smooth_through_every_node() {
  for node in smooth_nodes(&[[10.0, 10.0], [20.0, 16.0], [30.0, 12.0], [40.0, 18.0]], false) {
    let cross = node.coming[0] * node.leaving[1] - node.coming[1] * node.leaving[0];
    assert!(cross.abs() < 1e-9);
  }
}

#[test]
fn an_opening_has_round_ends_on_the_measured_radius_and_no_corner() {
  let opening = Opening::new([50.0, 48.0], 23.0, 9.5);
  let path = opening.path();
  let bounds = path.bounds();
  assert!((bounds.width() - 23.0).abs() < 1e-6 && (bounds.height() - 9.5).abs() < 0.05);
  // Every node is smooth: where one segment ends the next leaves the same way.
  let count = path.segments.len();
  for index in 0..count {
    let (before, after) = (path.segments[index], path.segments[(index + 1) % count]);
    let coming = [before.0[3][0] - before.0[2][0], before.0[3][1] - before.0[2][1]];
    let leaving = [after.0[1][0] - after.0[0][0], after.0[1][1] - after.0[0][1]];
    let cross = coming[0] * leaving[1] - coming[1] * leaving[0];
    let dot = coming[0] * leaving[0] + coming[1] * leaving[1];
    assert!(cross.abs() < 1e-6 * (1.0 + dot.abs()) && dot > 0.0, "a corner at node {index}");
  }
  // The end turns on a radius of 0.36 of the height: the left end stands that far below the top.
  assert!((path.segments[0].start()[1] - (opening.top() + 0.36 * 9.5)).abs() < 1e-9);
}

#[test]
fn splitting_a_segment_leaves_its_drawing_unchanged() {
  let segment = Cubic([[0.0, 0.0], [3.0, 8.0], [9.0, -2.0], [12.0, 4.0]]);
  let (before, after) = segment.split(segment.middle());
  assert!((before.length() - after.length()).abs() < 0.05);
  for step in 0..=10 {
    let t = step as f64 / 10.0;
    let on = if t <= 0.5 { before.at(t * 2.0) } else { after.at(t * 2.0 - 1.0) };
    let distance = (0..=200)
      .map(|at| segment.at(at as f64 / 200.0))
      .map(|p| (p[0] - on[0]).hypot(p[1] - on[1]))
      .fold(f64::INFINITY, f64::min);
    assert!(distance < 0.05);
  }
}

#[test]
fn a_lid_half_down_keeps_the_lower_part_of_an_eye_and_cuts_it_straight() {
  let eye = dot([25.0, 25.0], 3.0, 3.25);
  let lidded = cut_above(&eye, 24.0);
  let bounds = lidded.path().bounds();
  assert!((bounds.top - 24.0).abs() < 1e-6 && (bounds.bottom - 28.25).abs() < 1e-6);
  assert_eq!(lidded.fill, Some(Paint::Ink));
  assert!(!lidded.path().crosses_itself());
}

#[test]
fn a_path_that_crosses_itself_is_told_from_one_that_does_not() {
  let figure_eight = Path {
    segments: vec![
      Cubic::line([0.0, 0.0], [10.0, 10.0]),
      Cubic::line([10.0, 10.0], [10.0, 2.0]),
      Cubic::line([10.0, 2.0], [0.0, 6.0]),
    ],
    closed: true,
  };
  assert!(figure_eight.crosses_itself());
  let part: Part = dot([50.0, 40.0], 5.0, 3.0);
  assert!(!part.path().crosses_itself());
  assert!(part.path().area() > 0.0);
}
