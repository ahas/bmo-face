//! The eye set and the talking mouth set (docs/expressions.md, 「The eye set」 and 「The talking mouth
//! set」): the shapes the animation draws, redrawn clean at the sizes measured on it, in the screen's
//! units. Each set is a list of entries; a shape added to one is one more entry, and nothing that draws
//! or drives the face changes with it.

use crate::document::{Anchor, EyeVoice, Kind, Line, Node, Paint, Part};
use crate::geometry::{Point, add};
use crate::mouth::measure;
use crate::shapes::{Insides, Opening, STROKE, Turn, cut_above, dot, mouth, oval, stroke};

/// Where the left eye's centre stands: 53.6 from the right eye's, 24.6 down the screen.
pub const LEFT_EYE: Point = [50.0 - 53.6 / 2.0, 24.6];

/// How far below the eyes' centres the mouth's centre stands.
pub const MOUTH_DROP: f64 = 23.8;

/// Where the mouth's centre stands.
pub const MOUTH: Point = [50.0, LEFT_EYE[1] + MOUTH_DROP];

/// The width of a closed eye's stroke.
const LID: f64 = 0.9;

/// One eye of the eye set: its name, how far it moves each way as the gaze turns and how far it answers
/// the voice (nothing for a closed eye), and how it is drawn as the left eye around a centre; the right
/// eye is its reflection.
pub struct EyeShape {
  pub name: &'static str,
  pub reach: Point,
  pub voice: EyeVoice,
  draw: fn(Point) -> Vec<Part>,
}

impl EyeShape {
  pub fn parts(&self, centre: Point) -> Vec<Part> {
    (self.draw)(centre)
  }
}

/// An eye with its top cut away, `height` of it left: a lid half down.
fn lidded(centre: Point, rx: f64, ry: f64, height: f64) -> Vec<Part> {
  vec![cut_above(&dot(centre, rx, ry), centre[1] + ry - height)]
}

/// A stroke through points around a centre.
fn around(centre: Point, points: &[Point], width: f64) -> Vec<Part> {
  vec![stroke(&points.iter().map(|point| add(centre, *point)).collect::<Vec<_>>(), width)]
}

/// Straight ink strokes meeting at corners, through points around a centre.
pub fn corners(centre: Point, points: &[Point], width: f64) -> Part {
  Part {
    nodes: points
      .iter()
      .map(|point| Node {
        anchor: add(centre, *point),
        coming: [0.0, 0.0],
        leaving: [0.0, 0.0],
        kind: Kind::Corner,
      })
      .collect(),
    closed: false,
    fill: None,
    line: Some(Line {
      paint: Paint::Ink,
      width,
    }),
    inside: Vec::new(),
  }
}

/// A large eye with two white shines inside it.
fn shining(centre: Point) -> Vec<Part> {
  let shine = |at: Point, rx: f64, ry: f64| Part {
    nodes: oval(add(centre, at), rx, ry),
    closed: true,
    fill: Some(Paint::Light),
    line: None,
    inside: Vec::new(),
  };
  let mut eye = dot(centre, 5.1, 4.9);
  eye.inside = vec![shine([-1.8, -1.6], 1.5, 1.3), shine([1.7, 1.6], 0.7, 0.65)];
  vec![eye]
}

pub const EYE_SET: [EyeShape; 15] = [
  EyeShape {
    name: "dot",
    reach: [2.2, 1.6],
    voice: EyeVoice {
      grow: 0.1,
      squint: 0.15,
      shine: 0.0,
    },
    draw: |c| vec![dot(c, 3.0, 3.25)],
  },
  EyeShape {
    name: "dot-small",
    reach: [2.2, 1.6],
    voice: EyeVoice {
      grow: 0.1,
      squint: 0.15,
      shine: 0.0,
    },
    draw: |c| vec![dot(c, 2.4, 3.1)],
  },
  EyeShape {
    name: "oval",
    reach: [1.8, 1.2],
    voice: EyeVoice {
      grow: 0.08,
      squint: 0.12,
      shine: 0.0,
    },
    draw: |c| vec![dot(c, 2.9, 4.8)],
  },
  EyeShape {
    name: "oval-tall",
    reach: [1.6, 1.0],
    voice: EyeVoice {
      grow: 0.06,
      squint: 0.1,
      shine: 0.0,
    },
    draw: |c| vec![dot(c, 3.4, 6.5)],
  },
  EyeShape {
    name: "oval-wide",
    reach: [1.6, 1.0],
    voice: EyeVoice {
      grow: 0.06,
      squint: 0.1,
      shine: 0.0,
    },
    draw: |c| vec![dot(c, 4.1, 5.6)],
  },
  EyeShape {
    name: "surprise",
    reach: [1.4, 0.8],
    voice: EyeVoice {
      grow: 0.05,
      squint: 0.08,
      shine: 0.0,
    },
    draw: |c| vec![dot(c, 4.0, 8.6)],
  },
  EyeShape {
    name: "shining",
    reach: [1.6, 1.2],
    voice: EyeVoice {
      grow: 0.06,
      squint: 0.1,
      shine: 0.25,
    },
    draw: shining,
  },
  EyeShape {
    name: "half-lid-small",
    reach: [1.6, 0.6],
    voice: EyeVoice {
      grow: 0.04,
      squint: 0.1,
      shine: 0.0,
    },
    draw: |c| lidded(c, 2.6, 3.1, 3.75),
  },
  EyeShape {
    name: "half-lid",
    reach: [1.8, 0.6],
    voice: EyeVoice {
      grow: 0.04,
      squint: 0.1,
      shine: 0.0,
    },
    draw: |c| lidded(c, 3.1, 3.4, 5.0),
  },
  EyeShape {
    name: "half-lid-wide",
    reach: [1.2, 0.4],
    voice: EyeVoice {
      grow: 0.04,
      squint: 0.1,
      shine: 0.0,
    },
    draw: |c| lidded(c, 5.0, 3.6, 5.5),
  },
  EyeShape {
    name: "closed-arc",
    reach: [0.0, 0.0],
    voice: EyeVoice::STILL,
    draw: |c| around(c, &[[-5.0, 0.0], [0.0, 1.9], [5.0, 0.0]], LID),
  },
  EyeShape {
    name: "closed-cap",
    reach: [0.0, 0.0],
    voice: EyeVoice::STILL,
    draw: |c| around(c, &[[-4.5, 1.6], [0.0, -1.8], [4.5, 1.6]], LID),
  },
  EyeShape {
    name: "closed-u",
    reach: [0.0, 0.0],
    voice: EyeVoice::STILL,
    draw: |c| around(c, &[[-5.5, -2.2], [0.0, 2.3], [5.5, -2.2]], LID),
  },
  EyeShape {
    name: "closed-u-deep",
    reach: [0.0, 0.0],
    voice: EyeVoice::STILL,
    draw: |c| around(c, &[[-3.4, -4.2], [-2.9, 1.2], [0.0, 3.6], [2.9, 1.2], [3.4, -4.2]], LID),
  },
  EyeShape {
    name: "squeezed",
    reach: [0.0, 0.0],
    voice: EyeVoice::STILL,
    draw: |c| vec![corners(c, &[[-3.2, -2.8], [2.6, 0.0], [-3.2, 2.8]], LID)],
  },
];

/// The eye of the set by its name.
pub fn eye(name: &str) -> &'static EyeShape {
  EYE_SET.iter().find(|shape| shape.name == name).expect("an eye of the set")
}

/// How one talking mouth is drawn.
#[derive(Clone, Copy)]
enum Drawn {
  /// A line, `width` wide, its middle `bow` above its ends (below them where negative).
  Line { width: f64, bow: f64 },
  /// A small dark oval, with nothing inside.
  Round { width: f64, height: f64 },
  /// An opening with round ends and what shows inside it.
  Open {
    width: f64,
    height: f64,
    sag: f64,
    round: f64,
    belly: f64,
    insides: Insides,
  },
}

/// The line of a talking mouth or of a calm face, centred on the mouth's place.
pub fn line_mouth(width: f64, bow: f64) -> Part {
  let [cx, cy] = MOUTH;
  stroke(&[[cx - width / 2.0, cy], [cx, cy - bow], [cx + width / 2.0, cy]], STROKE)
}

/// An opening of the given size centred on the mouth's place, with the measured round ends.
pub fn opening(width: f64, height: f64) -> Opening {
  Opening::new(MOUTH, width, height)
}

/// The outline's width for an opening of a height.
pub fn outline_width(height: f64) -> f64 {
  if height < 9.0 { 0.7 } else { STROKE }
}

impl Drawn {
  fn parts(self) -> Vec<Part> {
    match self {
      Drawn::Line { width, bow } => vec![line_mouth(width, bow)],
      Drawn::Round { width, height } => vec![Part {
        nodes: oval(MOUTH, width / 2.0, height / 2.0),
        closed: true,
        fill: Some(Paint::Mouth),
        line: Some(Line {
          paint: Paint::Ink,
          width: outline_width(height),
        }),
        inside: Vec::new(),
      }],
      Drawn::Open {
        width,
        height,
        sag,
        round,
        belly,
        insides,
      } => vec![mouth(
        &Opening {
          sag,
          round,
          belly,
          turn: Turn::Down,
          ..opening(width, height)
        },
        insides,
        outline_width(height),
      )],
    }
  }
}

const fn open(width: f64, height: f64, sag: f64, belly: f64, insides: Insides) -> Drawn {
  Drawn::Open {
    width,
    height,
    sag,
    round: 0.36,
    belly,
    insides,
  }
}

const fn round_open(width: f64, height: f64, insides: Insides) -> Drawn {
  Drawn::Open {
    width,
    height,
    sag: 0.0,
    round: 0.5,
    belly: 0.6,
    insides,
  }
}

const fn inside(teeth: f64, lower: f64, tongue: f64) -> Insides {
  Insides {
    teeth,
    lower,
    tongue,
  }
}

/// The talking set's drawings: the 23 mouths that occur twenty times or more inside the studied lines,
/// in the animation's families — the beans small, medium and large, the lines, the slits, the round Os,
/// the grimace and the small dark o — each standing among the others where its own size puts it.
const TALK_SET: [Drawn; 23] = [
  open(14.9, 7.2, 0.04, 0.6, inside(0.0, 0.0, 0.38)),
  open(20.5, 7.8, 0.06, 0.58, inside(0.22, 0.0, 0.34)),
  open(21.8, 9.8, 0.04, 0.62, inside(0.24, 0.0, 0.36)),
  open(28.2, 8.5, -0.08, 0.58, inside(0.3, 0.0, 0.0)),
  open(22.0, 11.2, 0.08, 0.62, inside(0.0, 0.0, 0.55)),
  open(27.0, 12.2, 0.05, 0.6, inside(0.24, 0.0, 0.4)),
  open(23.8, 13.8, 0.06, 0.62, inside(0.24, 0.0, 0.36)),
  open(32.2, 12.2, 0.04, 0.58, inside(0.22, 0.0, 0.4)),
  open(28.8, 15.8, 0.06, 0.63, inside(0.22, 0.0, 0.38)),
  open(35.2, 14.2, 0.05, 0.6, inside(0.24, 0.0, 0.4)),
  open(32.5, 15.5, 0.08, 0.62, inside(0.2, 0.0, 0.42)),
  open(42.1, 18.2, 0.07, 0.62, inside(0.2, 0.0, 0.38)),
  Drawn::Line {
    width: 16.8,
    bow: 0.5,
  },
  Drawn::Line {
    width: 27.5,
    bow: 0.9,
  },
  Drawn::Line {
    width: 30.2,
    bow: 3.2,
  },
  open(15.5, 4.8, 0.0, 0.6, inside(0.0, 0.0, 0.4)),
  open(21.1, 5.8, 0.0, 0.6, inside(0.45, 0.0, 0.0)),
  round_open(15.5, 10.5, inside(0.0, 0.0, 0.34)),
  round_open(17.2, 13.8, inside(0.16, 0.0, 0.32)),
  round_open(19.8, 17.8, inside(0.16, 0.0, 0.3)),
  round_open(26.5, 20.2, inside(0.18, 0.0, 0.28)),
  open(34.8, 10.5, 0.0, 0.62, inside(0.44, 0.44, 0.0)),
  Drawn::Round {
    width: 7.2,
    height: 8.0,
  },
];

/// The talking set as the default document holds it, each drawing at the place measured from it.
pub fn anchors() -> Vec<Anchor> {
  TALK_SET.iter().map(|drawn| anchor(drawn.parts())).collect()
}

/// A drawing of the talking set at the place measured from it.
pub fn anchor(parts: Vec<Part>) -> Anchor {
  let (at, teeth) = measure(&parts).expect("a drawing of the set draws something");
  Anchor { at, teeth, parts }
}
