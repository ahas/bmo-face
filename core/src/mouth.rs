//! The talking mouth drawn from its anchors (docs/face.md, 「The mouth while speaking」): a mouth of any
//! openness and roundness found among the talking set's drawings, each standing at a place of its own on
//! a plane of how open and how wide a mouth is, and blended by correspondence from the ones around the
//! place asked for. The drawings that show the teeth together stand on a plane of their own, and the
//! teeth are a blend between the two planes' mouths. The tongue is pressed down into its own base as it
//! is shown less, and the corners are bent up or down by the face that speaks. A place on an anchor draws
//! that anchor as it stands.

use crate::blend::blend_lists;
use crate::document::{Anchor, Paint, Part};
use crate::geometry::{Bounds, Cubic, Path, Point, add, scale, sub};
use crate::shapes::smooth_nodes;

/// The height of a mouth fully open and the widths of the narrowest and the widest mouth, in screen
/// units: the largest round O, the small dark o and the widest bean of the talking set.
const OPEN_HEIGHT: f64 = 20.2;
const NARROWEST: f64 = 6.0;
const WIDEST: f64 = 44.0;

/// How deep, as a share of the opening's height, a teeth band reaches when the teeth stand together.
const TEETH_TOGETHER: f64 = 0.4;

/// The width of a mouth whose corners a face's lift moves by the whole of it: a medium bean's.
pub const LIFT_WIDTH: f64 = 30.0;

/// Below this share of itself a tongue is no longer drawn.
const TONGUE_GONE: f64 = 0.05;

/// Room for rounding when a place is found on the plane.
const ROOM: f64 = 1e-9;

/// The union of the boxes the parts' paths stand in; none for no parts.
fn bounds_of<'a>(parts: impl IntoIterator<Item = &'a Part>) -> Option<Bounds> {
  parts
    .into_iter()
    .filter(|part| !part.nodes.is_empty())
    .map(|part| part.path().bounds())
    .reduce(|one, other| one.union(&other))
}

/// Where a drawing stands among the talking set's, measured from itself: how open it is, by the height of
/// its closed parts — nought for lips drawn as a line — and how wide, by the width of all its parts, each
/// from nought to one; and whether its teeth stand together, a teeth band reaching the measured share of
/// its opening's height. None for a drawing of nothing.
pub fn measure(parts: &[Part]) -> Option<(Point, bool)> {
  let whole = bounds_of(parts)?;
  let closed: Vec<&Part> = parts.iter().filter(|part| part.closed).collect();
  let open = bounds_of(closed.iter().copied()).map_or(0.0, |opening| opening.height() / OPEN_HEIGHT);
  let spread = (whole.width() - NARROWEST) / (WIDEST - NARROWEST);
  let teeth = closed.iter().any(|outline| {
    let opening = outline.path().bounds();
    outline.inside.iter().filter(|inner| inner.fill == Some(Paint::Light)).any(|band| {
      let band = band.path().bounds();
      let reach = (band.bottom - opening.top).min(opening.bottom - band.top);
      opening.height() > 0.0 && reach >= TEETH_TOGETHER * opening.height()
    })
  });
  let unit = |value: f64| (value.clamp(0.0, 1.0) * 1000.0).round() / 1000.0;
  Some(([unit(open), unit(spread)], teeth))
}

/// Twice the signed area of a triangle: positive where it turns one way, negative the other.
fn turn(a: Point, b: Point, c: Point) -> f64 {
  (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

/// Whether a point stands strictly inside the circle through a triangle's three corners.
fn in_circle(triangle: [Point; 3], point: Point) -> bool {
  let [a, b, c] = triangle;
  let rows = [a, b, c].map(|corner| {
    let (x, y) = (corner[0] - point[0], corner[1] - point[1]);
    [x, y, x * x + y * y]
  });
  let det = rows[0][0] * (rows[1][1] * rows[2][2] - rows[2][1] * rows[1][2])
    - rows[1][0] * (rows[0][1] * rows[2][2] - rows[2][1] * rows[0][2])
    + rows[2][0] * (rows[0][1] * rows[1][2] - rows[1][1] * rows[0][2]);
  if turn(a, b, c) > 0.0 { det > ROOM } else { det < -ROOM }
}

/// The places of one plane's anchors, cut into triangles that meet edge to edge, each triangle's circle
/// holding no other place (Delaunay's), so that a place is drawn from the three anchors nearest round it.
#[derive(Clone, Debug, PartialEq)]
pub struct Plane {
  /// The anchors of the plane, by their index in the talking set, and where each stands.
  places: Vec<(usize, Point)>,
  triangles: Vec<[usize; 3]>,
  /// The edges round the outside, each between two of the plane's places.
  rim: Vec<[usize; 2]>,
}

impl Plane {
  /// The plane of the anchors given, by their index in the talking set; none for no anchors.
  pub fn new(places: Vec<(usize, Point)>) -> Option<Plane> {
    if places.is_empty() {
      return None;
    }
    let triangles = triangulated(&places.iter().map(|(_, at)| *at).collect::<Vec<_>>());
    let rim = if triangles.is_empty() {
      line_through(&places)
    } else {
      let mut edges: Vec<[usize; 2]> = Vec::new();
      for [a, b, c] in &triangles {
        for edge in [[*a, *b], [*b, *c], [*c, *a]] {
          let key = [edge[0].min(edge[1]), edge[0].max(edge[1])];
          match edges.iter().position(|known| *known == key) {
            Some(found) => {
              edges.remove(found);
            }
            None => edges.push(key),
          }
        }
      }
      edges
    };
    Some(Plane {
      places,
      triangles,
      rim,
    })
  }

  fn at(&self, index: usize) -> Point {
    self.places[index].1
  }

  /// The narrowest and the widest place the plane reaches at an openness, the openness first held to
  /// the plane's own.
  fn widths_at(&self, open: f64) -> (f64, f64, f64) {
    let opens = self.places.iter().map(|(_, at)| at[0]);
    let (least, most) = opens.fold((f64::INFINITY, f64::NEG_INFINITY), |(least, most), one| {
      (least.min(one), most.max(one))
    });
    let open = open.clamp(least, most);
    let mut widths: Vec<f64> = Vec::new();
    for [a, b] in &self.rim {
      let (p, q) = (self.at(*a), self.at(*b));
      if (p[0] - open).abs() <= ROOM {
        widths.push(p[1]);
      }
      if (q[0] - open).abs() <= ROOM {
        widths.push(q[1]);
      }
      if (p[0] - open) * (q[0] - open) < 0.0 {
        widths.push(p[1] + (q[1] - p[1]) * (open - p[0]) / (q[0] - p[0]));
      }
    }
    if widths.is_empty() {
      widths.push(self.places[0].1[1]);
    }
    let narrowest = widths.iter().copied().fold(f64::INFINITY, f64::min);
    let widest = widths.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (open, narrowest, widest)
  }

  /// The place a mouth of an openness and a roundness stands at: the openness held to the plane's, and
  /// the roundness taken across the widths the plane reaches there, the widest at nought and the
  /// narrowest at one.
  pub fn place(&self, open: f64, round: f64) -> Point {
    let (open, narrowest, widest) = self.widths_at(open);
    [open, widest + (narrowest - widest) * round.clamp(0.0, 1.0)]
  }

  /// The anchors a place is drawn from, by their index in the talking set, each with its weight, in the
  /// order of their index: the corners of the triangle it stands in, or the two ends of the nearest edge
  /// round the outside where it stands in none.
  pub fn weights(&self, place: Point) -> Vec<(usize, f64)> {
    let mut found: Vec<(usize, f64)> = self
      .triangles
      .iter()
      .find_map(|[a, b, c]| {
        let (pa, pb, pc) = (self.at(*a), self.at(*b), self.at(*c));
        let whole = turn(pa, pb, pc);
        let wa = turn(place, pb, pc) / whole;
        let wb = turn(pa, place, pc) / whole;
        let wc = 1.0 - wa - wb;
        [wa, wb, wc].iter().all(|weight| *weight >= -ROOM).then(|| {
          vec![(*a, wa.max(0.0)), (*b, wb.max(0.0)), (*c, wc.max(0.0))]
        })
      })
      .unwrap_or_else(|| self.on_rim(place));
    found.retain(|(_, weight)| *weight > ROOM);
    let total: f64 = found.iter().map(|(_, weight)| weight).sum();
    let mut weighed: Vec<(usize, f64)> =
      found.into_iter().map(|(index, weight)| (self.places[index].0, weight / total)).collect();
    weighed.sort_by_key(|(anchor, _)| *anchor);
    weighed
  }

  /// The two ends of the edge round the outside nearest a place, weighed by where the place falls on it.
  fn on_rim(&self, place: Point) -> Vec<(usize, f64)> {
    let nearest = self
      .rim
      .iter()
      .map(|[a, b]| {
        let (p, q) = (self.at(*a), self.at(*b));
        let along = [q[0] - p[0], q[1] - p[1]];
        let size = along[0] * along[0] + along[1] * along[1];
        let t = if size > 0.0 {
          (((place[0] - p[0]) * along[0] + (place[1] - p[1]) * along[1]) / size).clamp(0.0, 1.0)
        } else {
          0.0
        };
        let foot = [p[0] + along[0] * t, p[1] + along[1] * t];
        let away = (foot[0] - place[0]).hypot(foot[1] - place[1]);
        (away, *a, *b, t)
      })
      .min_by(|one, other| one.0.total_cmp(&other.0));
    match nearest {
      Some((_, a, b, t)) => vec![(a, 1.0 - t), (b, t)],
      None => vec![(0, 1.0)],
    }
  }
}

/// The places' triangles by Bowyer and Watson's construction, as indices into the places; none where
/// the places stand on one line or are fewer than three.
fn triangulated(points: &[Point]) -> Vec<[usize; 3]> {
  if points.len() < 3 {
    return Vec::new();
  }
  let count = points.len();
  // A triangle far round every place, whose corners are taken away at the end.
  let mut all: Vec<Point> = points.to_vec();
  all.extend([[-20.0, -20.0], [40.0, -20.0], [-20.0, 40.0]]);
  let mut triangles: Vec<[usize; 3]> = vec![[count, count + 1, count + 2]];
  for (index, point) in points.iter().enumerate() {
    let (bad, good): (Vec<[usize; 3]>, Vec<[usize; 3]>) =
      triangles.into_iter().partition(|triangle| in_circle(triangle.map(|corner| all[corner]), *point));
    let mut edges: Vec<[usize; 2]> = Vec::new();
    for [a, b, c] in &bad {
      for edge in [[*a, *b], [*b, *c], [*c, *a]] {
        match edges.iter().position(|known| *known == [edge[1], edge[0]]) {
          Some(shared) => {
            edges.remove(shared);
          }
          None => edges.push(edge),
        }
      }
    }
    triangles = good;
    triangles.extend(edges.into_iter().map(|[a, b]| [a, b, index]));
  }
  triangles.retain(|triangle| {
    triangle.iter().all(|corner| *corner < count)
      && turn(points[triangle[0]], points[triangle[1]], points[triangle[2]]).abs() > ROOM
  });
  triangles
}

/// The edges joining places that stand on one line, each to the next along it.
fn line_through(places: &[(usize, Point)]) -> Vec<[usize; 2]> {
  let mut order: Vec<usize> = (0..places.len()).collect();
  let along = |index: &usize| {
    let at = places[*index].1;
    at[0] * 1_000.0 + at[1]
  };
  order.sort_by(|a, b| along(a).total_cmp(&along(b)));
  if order.len() == 1 {
    return vec![[order[0], order[0]]];
  }
  order.windows(2).map(|pair| [pair[0], pair[1]]).collect()
}

/// Two mouths `t` of the way from one to the other: the one or the other as it stands at either end, and
/// between them each pair of parts blended by correspondence, a line meeting an opening first drawn as
/// the thin closed band its stroke covers, so the opening grows from round ends rather than from points.
pub fn between(a: Vec<Part>, b: &[Part], t: f64) -> Vec<Part> {
  if t <= 0.0 {
    return a;
  }
  if t >= 1.0 {
    return b.to_vec();
  }
  let banded = |own: &[Part], other: &[Part]| -> Vec<Part> {
    own
      .iter()
      .enumerate()
      .map(|(index, part)| match other.get(index) {
        Some(meets) if !part.closed && meets.closed => band(part),
        _ => part.clone(),
      })
      .collect()
  };
  blend_lists(&banded(&a, b), &banded(b, &a), t)
}

/// How many points along each segment of a line its band is drawn through.
const BAND_STEPS: usize = 3;

/// The thin closed band a line's stroke covers: its outline half the line's width each side of it, round
/// at both ends, filled as a mouth's opening and outlined in the line's own paint, so that it draws as
/// the line does and opens like an opening.
fn band(line: &Part) -> Part {
  let Some(stroke) = &line.line else {
    return line.clone();
  };
  let path = line.path();
  let points = path.polyline(BAND_STEPS);
  let count = points.len();
  if count < 2 {
    return line.clone();
  }
  let half = stroke.width / 4.0;
  let unit = |[x, y]: Point| {
    let size = x.hypot(y);
    if size > 0.0 { [x / size, y / size] } else { [1.0, 0.0] }
  };
  let along = |index: usize| unit(sub(points[(index + 1).min(count - 1)], points[index.saturating_sub(1)]));
  let side = |index: usize, way: f64| {
    let [dx, dy] = along(index);
    add(points[index], scale([-dy, dx], half * way))
  };
  let mut outline: Vec<Point> = (0..count).map(|index| side(index, 1.0)).collect();
  outline.push(add(points[count - 1], scale(along(count - 1), half)));
  outline.extend((0..count).rev().map(|index| side(index, -1.0)));
  outline.push(add(points[0], scale(along(0), -half)));
  Part {
    nodes: smooth_nodes(&outline, true),
    closed: true,
    fill: Some(Paint::Mouth),
    line: Some(stroke.clone()),
    inside: Vec::new(),
  }
}

/// The talking set ready to draw from: its anchors, and the plane of the open mouths and that of the
/// teeth together.
#[derive(Clone, Debug, PartialEq)]
pub struct Mouths {
  anchors: Vec<Anchor>,
  lips: Option<Plane>,
  teeth: Option<Plane>,
}

/// What the talking mouth is asked to be at a frame: how open, how round, how much it shows the teeth
/// together and the tongue, each from nought to one.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shape {
  pub open: f64,
  pub round: f64,
  pub teeth: f64,
  pub tongue: f64,
}

impl Mouths {
  pub fn new(anchors: &[Anchor]) -> Mouths {
    let plane = |teeth: bool| {
      Plane::new(
        anchors
          .iter()
          .enumerate()
          .filter(|(_, anchor)| anchor.teeth == teeth)
          .map(|(index, anchor)| (index, anchor.at))
          .collect(),
      )
    };
    Mouths {
      anchors: anchors.to_vec(),
      lips: plane(false),
      teeth: plane(true),
    }
  }

  /// The drawing a plane holds for an openness and a roundness.
  fn on(&self, plane: &Plane, open: f64, round: f64) -> Vec<Part> {
    self.on_place(plane, plane.place(open, round))
  }

  /// The drawing the open mouths' plane, or that of the teeth together, holds at a place on it; nothing
  /// where that plane holds no anchor.
  pub fn at(&self, place: Point, teeth: bool) -> Vec<Part> {
    let plane = if teeth { &self.teeth } else { &self.lips };
    plane.as_ref().map(|plane| self.on_place(plane, place)).unwrap_or_default()
  }

  fn on_place(&self, plane: &Plane, place: Point) -> Vec<Part> {
    let mut weights = plane.weights(place).into_iter();
    let Some((first, mut total)) = weights.next() else {
      return Vec::new();
    };
    let mut drawn = self.anchors[first].parts.clone();
    for (index, weight) in weights {
      total += weight;
      drawn = between(drawn, &self.anchors[index].parts, weight / total);
    }
    drawn
  }

  /// The talking mouth of a shape, its corners lifted by `lift` screen units (dropped where negative).
  pub fn draw(&self, shape: Shape, lift: f64) -> Vec<Part> {
    let lips = self.lips.as_ref().map(|plane| self.on(plane, shape.open, shape.round));
    let teeth = self.teeth.as_ref().map(|plane| self.on(plane, shape.open, shape.round));
    let drawn = match (lips, teeth) {
      (Some(lips), Some(teeth)) => between(lips, &teeth, shape.teeth),
      (Some(one), None) | (None, Some(one)) => one,
      (None, None) => Vec::new(),
    };
    lifted(&tongued(&drawn, shape.tongue), lift)
  }
}

/// The parts with every tongue inside them pressed down into its own base to `share` of its height, and
/// left out where little of it would show.
pub fn tongued(parts: &[Part], share: f64) -> Vec<Part> {
  if share >= 1.0 {
    return parts.to_vec();
  }
  let is_tongue = |part: &Part| matches!(part.fill, Some(Paint::Tongue | Paint::TongueLight));
  parts
    .iter()
    .map(|part| Part {
      inside: part
        .inside
        .iter()
        .filter(|inner| !is_tongue(inner) || share >= TONGUE_GONE)
        .map(|inner| {
          if is_tongue(inner) { inner.squashed(inner.path().bounds().bottom, share) } else { inner.clone() }
        })
        .collect(),
      ..part.clone()
    })
    .collect()
}

/// The parts bent so their two ends stand higher than their middle — `lift` screen units for a mouth
/// `LIFT_WIDTH` wide, a narrower one less in proportion — along a parabola across their width; every
/// point moves straight up or down by an amount that turns on its place across alone, so a drawing that
/// does not cross itself does not come to.
pub fn lifted(parts: &[Part], lift: f64) -> Vec<Part> {
  let Some(bounds) = bounds_of(parts).filter(|bounds| lift != 0.0 && bounds.width() > 0.0) else {
    return parts.to_vec();
  };
  let [middle, _] = bounds.centre();
  let half = bounds.width() / 2.0;
  let rise = lift * (bounds.width() / LIFT_WIDTH).min(1.0);
  let bend = |[x, y]: Point| -> Point {
    let across = (x - middle) / half;
    [x, y - rise * across * across]
  };
  fn bent(part: &Part, bend: &dyn Fn(Point) -> Point) -> Part {
    let path = part.path();
    let moved = Path {
      segments: path.segments.iter().map(|segment| Cubic(segment.0.map(bend))).collect(),
      closed: path.closed,
    };
    Part {
      inside: part.inside.iter().map(|inner| bent(inner, bend)).collect(),
      ..part.with_path(&moved)
    }
  }
  parts.iter().map(|part| bent(part, &bend)).collect()
}
