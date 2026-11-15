//! A drawing laid down in pixels (docs/face.md, 「Drawing」): where the screen and the body stand, a
//! stroke's round ends, a clip's cut and the order paint is laid down in, read off the pixels.

use bmo_face_core::default;
use bmo_face_core::document::Paint as Painted;
use bmo_face_core::drawing::{Drawing, Paint};
use bmo_face_core::geometry::{Cubic, Path};
use bmo_face_core::scene::Scene;
use bmo_face_displayer::render::{Canvas, Layout};

const BODY: &str = "#204060";
const SCREEN: &str = "#c0e0d0";
const INK: &str = "#000000";
const RED: &str = "#ff0000";
const BLUE: &str = "#0000ff";

fn drawing(clips: Vec<Path>, paint: Vec<Paint>) -> Drawing {
  Drawing {
    body: BODY.to_owned(),
    screen: SCREEN.to_owned(),
    clips,
    paint,
  }
}

fn line(from: [f64; 2], to: [f64; 2]) -> Path {
  Path {
    segments: vec![Cubic::line(from, to)],
    closed: false,
  }
}

fn square(left: f64, top: f64, side: f64) -> Path {
  let corners = [[left, top], [left + side, top], [left + side, top + side], [left, top + side]];
  Path {
    segments: (0..4).map(|at| Cubic::line(corners[at], corners[(at + 1) % 4])).collect(),
    closed: true,
  }
}

/// The colour standing at a point of the document's units, on a 1024 by 600 panel.
fn at(canvas: &Canvas, [x, y]: [f32; 2]) -> String {
  let layout = canvas.layout();
  let (px, py) = ((layout.left + x * layout.scale) as u32, (layout.top + y * layout.scale) as u32);
  pixel(canvas, px, py)
}

fn pixel(canvas: &Canvas, x: u32, y: u32) -> String {
  let colour = canvas.pixmap().pixel(x, y).expect("a pixel on the panel");
  format!("#{:02x}{:02x}{:02x}", colour.red(), colour.green(), colour.blue())
}

fn drawn(drawing: &Drawing) -> Canvas {
  let mut canvas = Canvas::new(1024, 600).expect("a canvas");
  canvas.draw(drawing);
  canvas
}

#[test]
fn the_screen_is_fitted_whole_and_centred() {
  assert_eq!(
    Layout::fit(1024, 600),
    Layout {
      width: 1024,
      height: 600,
      scale: 8.0,
      left: 112.0,
      top: 0.0
    }
  );
  assert_eq!(
    Layout::fit(800, 600),
    Layout {
      width: 800,
      height: 600,
      scale: 8.0,
      left: 0.0,
      top: 0.0
    }
  );
  assert_eq!(
    Layout::fit(640, 480),
    Layout {
      width: 640,
      height: 480,
      scale: 6.0,
      left: 20.0,
      top: 15.0
    }
  );
}

#[test]
fn the_body_stands_either_side_and_in_the_screens_turned_corners() {
  let canvas = drawn(&drawing(Vec::new(), Vec::new()));
  assert_eq!(pixel(&canvas, 0, 300), BODY);
  assert_eq!(pixel(&canvas, 1023, 300), BODY);
  assert_eq!(pixel(&canvas, 112, 0), BODY, "the corner is turned");
  assert_eq!(pixel(&canvas, 112, 300), SCREEN);
  assert_eq!(pixel(&canvas, 512, 0), SCREEN);
  assert_eq!(pixel(&canvas, 911, 300), SCREEN);
}

#[test]
fn a_stroke_ends_round() {
  let stroke = Paint::Stroke {
    path: line([20.0, 40.0], [80.0, 40.0]),
    colour: INK.to_owned(),
    width: 4.0,
    clip: None,
  };
  let canvas = drawn(&drawing(Vec::new(), vec![stroke]));
  assert_eq!(at(&canvas, [50.0, 40.0]), INK);
  assert_eq!(at(&canvas, [81.5, 40.0]), INK, "the cap reaches past the end");
  assert_eq!(at(&canvas, [81.6, 41.6]), SCREEN, "a square cap would stand here");
}

#[test]
fn what_stands_inside_is_cut_at_its_clip_and_later_paint_lies_over_earlier() {
  let paint = vec![
    Paint::Fill {
      path: square(10.0, 10.0, 40.0),
      colour: RED.to_owned(),
      clip: None,
    },
    Paint::Fill {
      path: square(20.0, 20.0, 40.0),
      colour: BLUE.to_owned(),
      clip: Some(0),
    },
  ];
  let canvas = drawn(&drawing(vec![square(10.0, 10.0, 40.0)], paint));
  assert_eq!(at(&canvas, [15.0, 15.0]), RED);
  assert_eq!(at(&canvas, [30.0, 30.0]), BLUE, "the later fill lies over the earlier");
  assert_eq!(at(&canvas, [55.0, 55.0]), SCREEN, "cut off at the clip");
}

#[test]
fn a_default_face_is_drawn_in_its_own_colours() {
  let document = default::document();
  let mut scene = Scene::new(document.clone(), 1, 0);
  let canvas = drawn(&scene.drawing(0));
  assert_eq!(pixel(&canvas, 0, 0), document.colors.body.to_lowercase());
  assert_eq!(pixel(&canvas, 120, 300), document.colors.screen.to_lowercase());
  let ink = document.colors.paint(Painted::Ink).to_lowercase();
  let inked = (0..600)
    .flat_map(|y| (112..912).map(move |x| (x, y)))
    .filter(|(x, y)| pixel(&canvas, *x, *y) == ink)
    .count();
  assert!(inked > 500, "the eyes and the mouth are inked: {inked} pixels");
}
