//! The geometry of a path (docs/face.md, 「How a path is written」): a run of cubic Bézier segments, open
//! or closed, and what is read off it — a point, a split, an arc length, an extreme, a box — and the
//! checks the blend is held to: a path that crosses itself, its winding.

/// A point, or an offset, in the screen's units: x to the right, y down.
pub type Point = [f64; 2];

pub const fn add(a: Point, b: Point) -> Point {
  [a[0] + b[0], a[1] + b[1]]
}

pub const fn sub(a: Point, b: Point) -> Point {
  [a[0] - b[0], a[1] - b[1]]
}

pub const fn scale(a: Point, by: f64) -> Point {
  [a[0] * by, a[1] * by]
}

pub fn mix(a: Point, b: Point, t: f64) -> Point {
  [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

pub fn length(a: Point) -> f64 {
  a[0].hypot(a[1])
}

pub fn distance(a: Point, b: Point) -> f64 {
  length(sub(a, b))
}

/// One cubic Bézier segment: its start, its two control points and its end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cubic(pub [Point; 4]);

impl Cubic {
  /// The straight segment from one point to another, its control points a third of the way along.
  pub fn line(from: Point, to: Point) -> Cubic {
    Cubic([from, mix(from, to, 1.0 / 3.0), mix(from, to, 2.0 / 3.0), to])
  }

  pub fn start(&self) -> Point {
    self.0[0]
  }

  pub fn end(&self) -> Point {
    self.0[3]
  }

  /// The point the segment passes through at parameter `t`.
  pub fn at(&self, t: f64) -> Point {
    let [p0, p1, p2, p3] = self.0;
    let (a, b, c) = (mix(p0, p1, t), mix(p1, p2, t), mix(p2, p3, t));
    mix(mix(a, b, t), mix(b, c, t), t)
  }

  /// The segment cut in two at parameter `t` by de Casteljau's construction; together they draw it
  /// unchanged.
  pub fn split(&self, t: f64) -> (Cubic, Cubic) {
    let [p0, p1, p2, p3] = self.0;
    let (a, b, c) = (mix(p0, p1, t), mix(p1, p2, t), mix(p2, p3, t));
    let (d, e) = (mix(a, b, t), mix(b, c, t));
    let middle = mix(d, e, t);
    (Cubic([p0, a, d, middle]), Cubic([middle, e, c, p3]))
  }

  /// The same segment run the other way.
  pub fn reversed(&self) -> Cubic {
    let [p0, p1, p2, p3] = self.0;
    Cubic([p3, p2, p1, p0])
  }

  /// The segment moved by an offset.
  pub fn shifted(&self, by: Point) -> Cubic {
    Cubic(self.0.map(|point| add(point, by)))
  }

  /// The segment between this one and another, `t` of the way, control point by control point.
  pub fn mixed(&self, other: &Cubic, t: f64) -> Cubic {
    Cubic([0, 1, 2, 3].map(|index| mix(self.0[index], other.0[index], t)))
  }

  /// Its length along the curve, summed over short chords.
  pub fn length(&self) -> f64 {
    const STEPS: usize = 24;
    let mut total = 0.0;
    let mut last = self.start();
    for step in 1..=STEPS {
      let point = self.at(step as f64 / STEPS as f64);
      total += distance(last, point);
      last = point;
    }
    total
  }

  /// The parameter at which `share` of its length is behind it.
  pub fn at_share(&self, share: f64) -> f64 {
    const STEPS: usize = 48;
    let goal = self.length() * share;
    let mut walked = 0.0;
    let mut last = self.start();
    for step in 1..=STEPS {
      let t = step as f64 / STEPS as f64;
      let point = self.at(t);
      let piece = distance(last, point);
      if walked + piece >= goal && piece > 0.0 {
        return t - (walked + piece - goal) / piece / STEPS as f64;
      }
      walked += piece;
      last = point;
    }
    share
  }

  /// The parameter at which half its length is behind it.
  pub fn middle(&self) -> f64 {
    self.at_share(0.5)
  }

  /// The parameters inside the segment where one coordinate stops growing or shrinking.
  fn turns(&self, axis: usize) -> Vec<f64> {
    let [p0, p1, p2, p3] = self.0.map(|point| point[axis]);
    // The derivative is a quadratic a·t² + b·t + c.
    let a = 3.0 * (-p0 + 3.0 * p1 - 3.0 * p2 + p3);
    let b = 6.0 * (p0 - 2.0 * p1 + p2);
    let c = 3.0 * (p1 - p0);
    let mut roots = Vec::new();
    if a.abs() < 1e-12 {
      if b.abs() > 1e-12 {
        roots.push(-c / b);
      }
    } else {
      let disc = b * b - 4.0 * a * c;
      if disc >= 0.0 {
        let root = disc.sqrt();
        roots.push((-b + root) / (2.0 * a));
        roots.push((-b - root) / (2.0 * a));
      }
    }
    roots.into_iter().filter(|t| *t > 1e-9 && *t < 1.0 - 1e-9).collect()
  }

  /// The parameters where the segment stands furthest along an axis: its ends and its turns.
  pub fn extremes(&self, axis: usize) -> Vec<f64> {
    let mut candidates = vec![0.0, 1.0];
    candidates.extend(self.turns(axis));
    candidates
  }

  /// The parameters where the segment crosses the level `y`, in order.
  pub fn crossings(&self, y: f64) -> Vec<f64> {
    const STEPS: usize = 32;
    let mut found = Vec::new();
    let level = |t: f64| self.at(t)[1] - y;
    let mut low = 0.0;
    let mut below = level(0.0);
    for step in 1..=STEPS {
      let high = step as f64 / STEPS as f64;
      let at_high = level(high);
      if below == 0.0 && step == 1 {
        found.push(0.0);
      }
      if (below < 0.0 && at_high >= 0.0) || (below > 0.0 && at_high <= 0.0) {
        let (mut a, mut b) = (low, high);
        for _ in 0..48 {
          let middle = (a + b) / 2.0;
          if (level(a) < 0.0) == (level(middle) < 0.0) {
            a = middle;
          } else {
            b = middle;
          }
        }
        found.push((a + b) / 2.0);
      }
      low = high;
      below = at_high;
    }
    found
  }
}

/// A box: its least and greatest x and y.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
  pub left: f64,
  pub top: f64,
  pub right: f64,
  pub bottom: f64,
}

impl Bounds {
  pub fn width(&self) -> f64 {
    self.right - self.left
  }

  pub fn height(&self) -> f64 {
    self.bottom - self.top
  }

  pub fn centre(&self) -> Point {
    [(self.left + self.right) / 2.0, (self.top + self.bottom) / 2.0]
  }

  pub fn union(&self, other: &Bounds) -> Bounds {
    Bounds {
      left: self.left.min(other.left),
      top: self.top.min(other.top),
      right: self.right.max(other.right),
      bottom: self.bottom.max(other.bottom),
    }
  }

  /// Whether a box stands inside this one, with room for rounding.
  pub fn holds(&self, other: &Bounds, room: f64) -> bool {
    other.left >= self.left - room
      && other.top >= self.top - room
      && other.right <= self.right + room
      && other.bottom <= self.bottom + room
  }
}

/// A run of segments, each starting where the one before ended; a closed path's last one ends where its
/// first starts.
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
  pub segments: Vec<Cubic>,
  pub closed: bool,
}

impl Path {
  pub fn reversed(&self) -> Path {
    Path {
      segments: self.segments.iter().rev().map(Cubic::reversed).collect(),
      closed: self.closed,
    }
  }

  pub fn shifted(&self, by: Point) -> Path {
    Path {
      segments: self.segments.iter().map(|segment| segment.shifted(by)).collect(),
      closed: self.closed,
    }
  }

  pub fn length(&self) -> f64 {
    self.segments.iter().map(Cubic::length).sum()
  }

  /// The box the drawn path stands in.
  pub fn bounds(&self) -> Bounds {
    let mut bounds = Bounds {
      left: f64::INFINITY,
      top: f64::INFINITY,
      right: f64::NEG_INFINITY,
      bottom: f64::NEG_INFINITY,
    };
    for segment in &self.segments {
      for axis in 0..2 {
        for t in segment.extremes(axis) {
          let point = segment.at(t);
          bounds.left = bounds.left.min(point[0]);
          bounds.right = bounds.right.max(point[0]);
          bounds.top = bounds.top.min(point[1]);
          bounds.bottom = bounds.bottom.max(point[1]);
        }
      }
    }
    bounds
  }

  /// Points along the path, `per_segment` to each segment, the last point of a closed path left out.
  pub fn polyline(&self, per_segment: usize) -> Vec<Point> {
    let mut points = Vec::with_capacity(self.segments.len() * per_segment + 1);
    for segment in &self.segments {
      for step in 0..per_segment {
        points.push(segment.at(step as f64 / per_segment as f64));
      }
    }
    if !self.closed
      && let Some(last) = self.segments.last()
    {
      points.push(last.end());
    }
    points
  }

  /// The signed area a closed path encloses: positive where it runs clockwise on the screen, whose y
  /// grows downward.
  pub fn area(&self) -> f64 {
    let points = self.polyline(16);
    let count = points.len();
    (0..count)
      .map(|index| {
        let [ax, ay] = points[index];
        let [bx, by] = points[(index + 1) % count];
        ax * by - bx * ay
      })
      .sum::<f64>()
      / 2.0
  }

  /// Whether the drawn path crosses itself anywhere, read on a fine polyline: two of its pieces that
  /// do not share an end crossing each other.
  pub fn crosses_itself(&self) -> bool {
    let points = self.polyline(20);
    let count = points.len();
    let pieces = if self.closed { count } else { count - 1 };
    let piece = |index: usize| (points[index], points[(index + 1) % count]);
    for first in 0..pieces {
      for second in first + 2..pieces {
        if self.closed && first == 0 && second == pieces - 1 {
          continue;
        }
        let (a, b) = piece(first);
        let (c, d) = piece(second);
        if properly_cross(a, b, c, d) {
          return true;
        }
      }
    }
    false
  }
}

/// Whether two straight pieces cross at a point inside both.
fn properly_cross(a: Point, b: Point, c: Point, d: Point) -> bool {
  let side = |p: Point, q: Point, r: Point| (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
  let (d1, d2) = (side(c, d, a), side(c, d, b));
  let (d3, d4) = (side(a, b, c), side(a, b, d));
  const ROOM: f64 = 1e-9;
  ((d1 > ROOM && d2 < -ROOM) || (d1 < -ROOM && d2 > ROOM))
    && ((d3 > ROOM && d4 < -ROOM) || (d3 < -ROOM && d4 > ROOM))
}
