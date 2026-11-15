//! The default document (docs/expressions.md, 「What each default face draws」): every face drawn from the
//! eye set and the talking mouth set's shapes, on the measured palette, which a new data root starts
//! from and an unsaved one is replaced by.

use crate::document::{Colors, Document, Eyes, Face, Paint, Pair, Part, TalkFamily};
use crate::geometry::{Point, add};
use crate::names::{EMOTIONS, STATES};
use crate::sets::{LEFT_EYE, MOUTH, corners, eye, line_mouth, opening, outline_width, talk_set};
use crate::shapes::{Insides, Opening, STROKE, Turn, mouth, stroke};

/// The daylight palette measured on the animation.
fn colors() -> Colors {
  let paint = |paint: Paint, colour: &str| (paint, colour.to_owned());
  Colors {
    body: "#5fbb9c".into(),
    screen: "#cdfeda".into(),
    paints: vec![
      paint(Paint::Ink, "#000201"),
      paint(Paint::Light, "#fcfbfc"),
      paint(Paint::Mouth, "#2c843b"),
      paint(Paint::Tongue, "#50dc66"),
      paint(Paint::TongueLight, "#64ea80"),
      paint(Paint::Cheek, "#fecbc0"),
      paint(Paint::Tear, "#74f1f0"),
    ],
  }
}

/// Both eyes of the set's shape, moved by the same step from their place; the right mirrors the left.
fn eyes(name: &str, step: Point) -> Eyes {
  let shape = eye(name);
  Eyes {
    pair: Pair {
      left: shape.parts(add(LEFT_EYE, step)),
      right: None,
    },
    reach: shape.reach,
  }
}

/// Both brows: the left brow's points, the right mirroring it.
fn brows(points: &[Point]) -> Pair {
  Pair {
    left: vec![stroke(points, STROKE)],
    right: None,
  }
}

/// An open mouth on the mouth's place, its round side down (a smile) or up (a wail).
fn open(width: f64, height: f64, turn: Turn, insides: Insides) -> Vec<Part> {
  vec![mouth(
    &Opening {
      turn,
      ..opening(width, height)
    },
    insides,
    outline_width(height),
  )]
}

fn face(eyes: Eyes, brows: Pair, mouth: Vec<Part>) -> Face {
  Face {
    blush: Pair::empty(),
    eyes,
    brows,
    mouth,
    marks: Vec::new(),
  }
}

/// The faces, one per emotion and per state, in that order.
fn faces() -> Vec<(String, Face)> {
  let [x, y] = LEFT_EYE;
  let none = Pair::empty;
  let teeth_and_tongue = Insides {
    teeth: 0.22,
    lower: 0.0,
    tongue: 0.38,
  };
  let drawn: [Face; 13] = [
    // Calm: dot eyes and a calm line, smiling a little.
    face(eyes("dot", [0.0, 0.0]), none(), vec![line_mouth(14.0, -1.0)]),
    // Joy: dot eyes and the large bean of the talking set, teeth and tongue.
    face(eyes("dot", [0.0, 0.0]), none(), open(28.8, 15.8, Turn::Down, teeth_and_tongue)),
    // Sadness: tall eyes under brows raised toward the middle, and a small wail.
    face(eyes("oval", [0.0, 0.0]), brows(&[[x - 4.6, y - 9.2], [x + 4.4, y - 11.6]]), {
      open(
        18.0,
        9.0,
        Turn::Up,
        Insides {
          teeth: 0.0,
          lower: 0.0,
          tongue: 0.34,
        },
      )
    }),
    // Surprise: surprised ovals and a round O.
    face(eyes("oval-wide", [0.0, -0.6]), none(), {
      vec![mouth(
        &Opening {
          round: 0.5,
          sag: 0.0,
          ..opening(15.0, 14.0)
        },
        Insides {
          teeth: 0.16,
          lower: 0.0,
          tongue: 0.3,
        },
        STROKE,
      )]
    }),
    // Anger: dot eyes under brows slanted down toward the middle, and a wide wail with its teeth.
    face(eyes("dot", [0.0, 0.6]), brows(&[[x - 4.8, y - 7.6], [x + 4.6, y - 4.6]]), {
      open(
        24.0,
        10.0,
        Turn::Up,
        Insides {
          teeth: 0.3,
          lower: 0.0,
          tongue: 0.0,
        },
      )
    }),
    // Sleepiness: lids half down and a short level line.
    face(eyes("half-lid", [0.0, 0.0]), none(), vec![line_mouth(9.0, 0.0)]),
    // Laughter: eyes shut in a deep U and the widest bean.
    face(
      eyes("closed-u-deep", [0.0, 0.0]),
      none(),
      open(38.0, 18.0, Turn::Down, teeth_and_tongue),
    ),
    // Worry: small eyes under brows raised toward the middle, and a slit of clenched teeth.
    face(eyes("dot-small", [0.0, 0.0]), brows(&[[x - 4.4, y - 8.0], [x + 4.4, y - 10.2]]), {
      open(
        18.0,
        5.4,
        Turn::Down,
        Insides {
          teeth: 0.42,
          lower: 0.42,
          tongue: 0.0,
        },
      )
    }),
    // Pain: x x eyes and a slit under the teeth band.
    face(
      Eyes {
        pair: Pair {
          left: vec![
            corners(LEFT_EYE, &[[-2.8, -2.8], [2.8, 2.8]], 0.9),
            corners(LEFT_EYE, &[[2.8, -2.8], [-2.8, 2.8]], 0.9),
          ],
          right: None,
        },
        reach: [0.0, 0.0],
      },
      none(),
      open(
        24.0,
        7.0,
        Turn::Down,
        Insides {
          teeth: 0.5,
          lower: 0.0,
          tongue: 0.0,
        },
      ),
    ),
    // Strain: eyes squeezed shut and the grimace of both rows of teeth.
    face(
      eyes("squeezed", [0.0, 0.0]),
      none(),
      open(
        34.8,
        10.5,
        Turn::Down,
        Insides {
          teeth: 0.44,
          lower: 0.44,
          tongue: 0.0,
        },
      ),
    ),
    // writing: dot eyes a little lower, looking down, level brows and a short line.
    face(
      eyes("dot", [0.0, 2.0]),
      brows(&[[x - 4.4, y - 5.6], [x + 4.4, y - 5.6]]),
      vec![line_mouth(8.0, 0.0)],
    ),
    // thinking: dot eyes up and to the left, one brow raised, and a line rising to the right.
    face(
      eyes("dot", [-2.0, -2.0]),
      Pair {
        left: vec![stroke(&[[x - 5.6, y - 9.0], [x - 1.6, y - 11.4], [x + 2.4, y - 10.0]], STROKE)],
        right: Some(Vec::new()),
      },
      {
        let [cx, cy] = MOUTH;
        vec![stroke(&[[cx - 6.0, cy + 0.4], [cx + 1.0, cy], [cx + 7.0, cy - 2.2]], STROKE)]
      },
    ),
    // idle: dot eyes and a short level line.
    face(eyes("dot", [0.0, 0.0]), none(), vec![line_mouth(10.0, 0.0)]),
  ];
  EMOTIONS.into_iter().chain(STATES).map(str::to_owned).zip(drawn).collect()
}

/// The default document.
pub fn document() -> Document {
  Document {
    colors: colors(),
    faces: faces(),
    talk: talk_set(),
  }
}

/// The default talking mouth set, as a document carried from an older form takes it.
pub fn talk() -> Vec<TalkFamily> {
  talk_set()
}
