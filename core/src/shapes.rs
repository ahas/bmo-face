//! The shapes the eye and mouth sets are drawn from (docs/expressions.md, 「How a shape is drawn」): the
//! smooth run through points, the oval, the open mouth with round ends and what stands inside it, and a
//! closed shape with its top cut away.

use crate::document::{Kind, Line, Node, Paint, Part};
use crate::geometry::{Cubic, Path, Point, add, scale, sub};

/// How far along a quarter of an ellipse its handles reach, as a share of the radius.
pub const OVAL_HANDLE: f64 = (4.0 / 3.0) * (std::f64::consts::SQRT_2 - 1.0);

/// The width of every ink stroke the sets draw: a line mouth, a closed eye, a brow.
pub const STROKE: f64 = 0.8;

/// Smooth nodes for the run through `points`. Inside the run a node's leaving handle is a sixth of the
/// step from the point before it to the point after it and its coming handle that step reversed; the
/// end of an open run leaves along the parabola its first three points make, so three points draw an
/// arc and never two strokes meeting in a bend. A closed run's neighbours wrap round; two points are a
/// straight line.
pub fn smooth_nodes(points: &[Point], closed: bool) -> Vec<Node> {
  let count = points.len();
  let at = |index: isize| points[index.rem_euclid(count as isize) as usize];
  let tangent = |index: usize| -> Point {
    if closed {
      return scale(sub(at(index as isize + 1), at(index as isize - 1)), 0.5);
    }
    match (index, count) {
      (_, 2) => sub(points[1], points[0]),
      (0, _) => sub(scale(sub(points[1], points[0]), 2.0), scale(sub(points[2], points[0]), 0.5)),
      (last, _) if last == count - 1 => {
        let (end, before, further) = (points[last], points[last - 1], points[last - 2]);
        sub(scale(sub(end, before), 2.0), scale(sub(end, further), 0.5))
      }
      _ => scale(sub(points[index + 1], points[index - 1]), 0.5),
    }
  };
  (0..count)
    .map(|index| {
      let handle = scale(tangent(index), 1.0 / 3.0);
      Node {
        anchor: points[index],
        coming: scale(handle, -1.0),
        leaving: handle,
        kind: Kind::Smooth,
      }
    })
    .collect()
}

/// Four smooth nodes at the ends of the radii, clockwise from the right, that draw the ellipse.
pub fn oval([cx, cy]: Point, rx: f64, ry: f64) -> Vec<Node> {
  let (hx, hy) = (rx * OVAL_HANDLE, ry * OVAL_HANDLE);
  let node = |anchor: Point, leaving: Point| Node {
    anchor,
    coming: scale(leaving, -1.0),
    leaving,
    kind: Kind::Smooth,
  };
  vec![
    node([cx + rx, cy], [0.0, hy]),
    node([cx, cy + ry], [-hx, 0.0]),
    node([cx - rx, cy], [0.0, -hy]),
    node([cx, cy - ry], [hx, 0.0]),
  ]
}

/// A filled ink oval: a dot eye, a pupil.
pub fn dot(centre: Point, rx: f64, ry: f64) -> Part {
  Part {
    nodes: oval(centre, rx, ry),
    closed: true,
    fill: Some(Paint::Ink),
    line: None,
    inside: Vec::new(),
  }
}

/// An open ink stroke through its points: a closed eye, a brow, a line mouth.
pub fn stroke(points: &[Point], width: f64) -> Part {
  Part {
    nodes: smooth_nodes(points, false),
    closed: false,
    fill: None,
    line: Some(Line {
      paint: Paint::Ink,
      width,
    }),
    inside: Vec::new(),
  }
}

/// A filled shape of one paint, with no line.
pub fn filled(path: &Path, paint: Paint) -> Part {
  let part = Part {
    nodes: Vec::new(),
    closed: true,
    fill: Some(paint),
    line: None,
    inside: Vec::new(),
  };
  part.with_path(path)
}

/// Which way an opening's round side faces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Turn {
  /// A level top and a round bottom: a smile, a talking mouth.
  Down,
  /// A round top and a level bottom: a wail, a frown.
  Up,
}

/// An open mouth's outline: `width` by `height` around `centre`, its top edge level and sagging toward
/// the middle by `sag` (a share of the height; below nothing it bows up), its two ends turned on a
/// radius of `round` times the height, and its round side as full as `belly` (the share of each half
/// its handles reach: 0.55 an ellipse, more a squarer bowl).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opening {
  pub centre: Point,
  pub width: f64,
  pub height: f64,
  pub sag: f64,
  pub round: f64,
  pub belly: f64,
  pub turn: Turn,
}

/// The measured radius of a talking mouth's ends, as a share of its height.
pub const END_ROUND: f64 = 0.36;

impl Opening {
  pub fn new(centre: Point, width: f64, height: f64) -> Opening {
    Opening {
      centre,
      width,
      height,
      sag: 0.06,
      round: END_ROUND,
      belly: 0.6,
      turn: Turn::Down,
    }
  }

  pub fn top(&self) -> f64 {
    self.centre[1] - self.height / 2.0
  }

  pub fn bottom(&self) -> f64 {
    self.centre[1] + self.height / 2.0
  }

  pub fn left(&self) -> f64 {
    self.centre[0] - self.width / 2.0
  }

  pub fn right(&self) -> f64 {
    self.centre[0] + self.width / 2.0
  }

  /// The outline as segments, clockwise from its left end.
  pub fn path(&self) -> Path {
    let [cx, cy] = self.centre;
    let (w, h) = (self.width, self.height);
    let r = (self.round * h).min(w / 2.0 - 0.25).min(h / 2.0);
    let (top, bottom, left, right) = (cy - h / 2.0, cy + h / 2.0, cx - w / 2.0, cx + w / 2.0);
    let sag = self.sag * h;
    let k = OVAL_HANDLE * r;
    // The level edge between the two turned ends, a parabola through its middle.
    let run = cx - (left + r);
    let slope = if run > 0.0 { 2.0 * sag / run } else { 0.0 };
    let along = {
      let size = (1.0 + slope * slope).sqrt();
      [1.0 / size, slope / size]
    };
    let left_end = [left, top + r];
    let top_left = [left + r, top];
    let middle = [cx, top + sag];
    let top_right = [right - r, top];
    let right_end = [right, top + r];
    let low = [cx, bottom];
    let down = self.belly * (bottom - (top + r));
    let across = self.belly * (w / 2.0);
    let segments = vec![
      Cubic([left_end, [left, top + r - k], sub(top_left, scale(along, k)), top_left]),
      Cubic([top_left, add(top_left, scale(along, run / 3.0)), [cx - run / 3.0, top + sag], middle]),
      Cubic([
        middle,
        [cx + run / 3.0, top + sag],
        add(top_right, scale([-along[0], along[1]], run / 3.0)),
        top_right,
      ]),
      Cubic([
        top_right,
        add(top_right, scale([along[0], -along[1]], k)),
        [right, top + r - k],
        right_end,
      ]),
      Cubic([right_end, [right, top + r + down], [cx + across, bottom], low]),
      Cubic([low, [cx - across, bottom], [left, top + r + down], left_end]),
    ];
    let path = Path {
      segments,
      closed: true,
    };
    match self.turn {
      Turn::Down => path,
      Turn::Up => flipped(&path, cy),
    }
  }
}

/// A path turned upside down about the level `y`, still running clockwise.
pub fn flipped(path: &Path, y: f64) -> Path {
  let flip = |[px, py]: Point| [px, 2.0 * y - py];
  Path {
    segments: path.segments.iter().map(|segment| Cubic(segment.0.map(flip))).collect(),
    closed: path.closed,
  }
  .reversed()
}

/// What shows inside an open mouth, each as a share of its height; nothing where a share is nought.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insides {
  /// The upper teeth band, as deep as this in its middle.
  pub teeth: f64,
  /// The lower teeth band.
  pub lower: f64,
  /// The tongue, rising this high from the bottom, in two tones.
  pub tongue: f64,
}

/// An open mouth: its opening filled with the cavity's paint and outlined in ink `line` wide, with the
/// teeth and the two-tone tongue inside it, cut off at the outline.
pub fn mouth(opening: &Opening, insides: Insides, line: f64) -> Part {
  let mut inside = Vec::new();
  if insides.teeth > 0.0 {
    inside.push(filled(&teeth(opening, insides.teeth, false), Paint::Light));
  }
  if insides.lower > 0.0 {
    inside.push(filled(&teeth(opening, insides.lower, true), Paint::Light));
  }
  if insides.tongue > 0.0 {
    let base = tongue(opening, insides.tongue);
    inside.push(filled(&tongue_light(&base), Paint::TongueLight));
    inside.push(filled(&base, Paint::Tongue));
  }
  let outline = Part {
    nodes: Vec::new(),
    closed: true,
    fill: Some(Paint::Mouth),
    line: Some(Line {
      paint: Paint::Ink,
      width: line,
    }),
    inside,
  };
  outline.with_path(&opening.path())
}

/// A teeth band hanging from the top of an opening, or rising from its bottom: reaching past the edge it
/// stands on, to be cut off there, `depth` of the height deep in its middle and thinning toward its ends,
/// which stop short of the corners.
pub fn teeth(opening: &Opening, depth: f64, lower: bool) -> Path {
  let [cx, cy] = opening.centre;
  let h = opening.height;
  let inset = opening.width * 0.07 + opening.round * h * 0.3;
  let (x0, x1) = (opening.left() + inset, opening.right() - inset);
  let edge = opening.top() - 2.0;
  let middle = opening.top() + opening.sag.max(0.0) * h + depth * h;
  let end = opening.top() + depth * h * 0.4;
  let reach = (cx - x0) * 0.55;
  let fall = (middle - end) * 0.9;
  let path = Path {
    segments: vec![
      Cubic::line([x0, edge], [x1, edge]),
      Cubic::line([x1, edge], [x1, end]),
      Cubic([[x1, end], [x1, end + fall], [cx + reach, middle], [cx, middle]]),
      Cubic([[cx, middle], [cx - reach, middle], [x0, end + fall], [x0, end]]),
      Cubic::line([x0, end], [x0, edge]),
    ],
    closed: true,
  };
  if lower { flipped(&path, cy) } else { path }
}

/// The tongue's light tone: the tongue raised a little, so that drawn under it, it shows as a rim along
/// the tongue's top edge.
pub fn tongue_light(tongue: &Path) -> Path {
  tongue.shifted([0.0, -0.45])
}

/// The tongue: a dome rising from below an opening's bottom edge, a little right of its middle.
pub fn tongue(opening: &Opening, rise: f64) -> Path {
  let w = opening.width;
  let base = opening.bottom() + 2.0;
  let (from, to) = (opening.left() + w * 0.1, opening.right() - w * 0.04);
  let peak = [opening.centre[0] + w * 0.08, opening.bottom() - rise * opening.height];
  let up = (base - peak[1]) * 0.6;
  Path {
    segments: vec![
      Cubic([[from, base], [from, base - up], [peak[0] - (peak[0] - from) * 0.6, peak[1]], peak]),
      Cubic([peak, [peak[0] + (to - peak[0]) * 0.6, peak[1]], [to, base - up], [to, base]]),
      Cubic::line([to, base], [from, base]),
    ],
    closed: true,
  }
}

/// A closed shape with everything above the level `y` cut away and the cut drawn straight across, as a
/// lid half down cuts an eye; what stands inside it is kept, to be cut off at its new outline. A shape
/// the level does not cross twice is handed back as it stands.
pub fn cut_above(part: &Part, y: f64) -> Part {
  let path = part.path();
  let mut pieces: Vec<Cubic> = Vec::new();
  for segment in &path.segments {
    let mut rest = *segment;
    let mut done = 0.0;
    for t in segment.crossings(y) {
      let local = (t - done) / (1.0 - done);
      if local <= 1e-9 || local >= 1.0 - 1e-9 {
        continue;
      }
      let (before, after) = rest.split(local);
      pieces.push(before);
      rest = after;
      done = t;
    }
    pieces.push(rest);
  }
  let below = |piece: &Cubic| piece.at(0.5)[1] >= y;
  let count = pieces.len();
  // Start the kept run just after a piece that is cut away.
  let Some(first) =
    (0..count).find(|&index| below(&pieces[index]) && !below(&pieces[(index + count - 1) % count]))
  else {
    return part.clone();
  };
  let kept: Vec<Cubic> = (0..count).map(|step| pieces[(first + step) % count]).take_while(below).collect();
  if kept.len() == count {
    return part.clone();
  }
  let (start, end) = (kept[0].start(), kept[kept.len() - 1].end());
  let mut segments = kept;
  segments.push(Cubic::line(end, start));
  part.with_path(&Path {
    segments,
    closed: true,
  })
}
