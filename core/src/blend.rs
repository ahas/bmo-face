//! The blend of one part into another by correspondence (docs/face.md, 「Blending one drawing into
//! another」), for the in-between drawings of an expression change. Both paths are brought to one winding
//! and one start and cut into the same runs — a closed shape into its top from its leftmost point to its
//! rightmost and its bottom back, an open line into itself left to right and, against a closed shape,
//! back again along itself, so it opens as a mouth does rather than folding; the closed shape it meets is
//! then cut where it stands, seen from its middle, in the direction of the line's ends, and taken first the
//! way the line bows. Each run of a pair is then
//! cut wherever the other has a node, by the share of its length behind that node, which changes neither
//! drawing; so the two runs hold one count of segments whose ends stand at the same shares of their
//! lengths, and the segments are mixed pairwise in order along the runs. Where the counts still differ
//! by rounding, the run with fewer is made up by splitting its longest segment at its middle.

use crate::document::{Line, Paint, Part};
use crate::geometry::{Cubic, Path, Point, length, sub};

/// The run of segments the open path draws, from its left end to its right.
fn left_to_right(path: &Path) -> Vec<Cubic> {
  let (Some(first), Some(last)) = (path.segments.first(), path.segments.last()) else {
    return Vec::new();
  };
  if first.start()[0] <= last.end()[0] {
    path.segments.clone()
  } else {
    path.reversed().segments
  }
}

/// Where along a run of segments it stands furthest one way along x: the segment and the parameter.
fn furthest(segments: &[Cubic], leftmost: bool) -> (usize, f64) {
  let mut best = (0, 0.0, if leftmost { f64::INFINITY } else { f64::NEG_INFINITY });
  for (index, segment) in segments.iter().enumerate() {
    for t in segment.extremes(0) {
      let x = segment.at(t)[0];
      let further = if leftmost { x < best.2 - 1e-9 } else { x > best.2 + 1e-9 };
      if further {
        best = (index, t, x);
      }
    }
  }
  (best.0, best.1)
}

/// The segments with the one at `index` split at `t`, and the index of the first segment after the
/// split point.
fn split_at(segments: &[Cubic], index: usize, t: f64) -> (Vec<Cubic>, usize) {
  if t <= 1e-6 {
    return (segments.to_vec(), index);
  }
  if t >= 1.0 - 1e-6 {
    return (segments.to_vec(), index + 1);
  }
  let (before, after) = segments[index].split(t);
  let mut split = segments.to_vec();
  split.splice(index..=index, [before, after]);
  (split, index + 1)
}

/// Where along a run of segments it stands in the direction of a point, seen from a centre: the segment
/// and the parameter.
fn toward(segments: &[Cubic], centre: Point, point: Point) -> (usize, f64) {
  const STEPS: usize = 64;
  let aim = sub(point, centre);
  let mut best = (0, 0.0, f64::NEG_INFINITY);
  for (index, segment) in segments.iter().enumerate() {
    for step in 0..=STEPS {
      let t = step as f64 / STEPS as f64;
      let seen = sub(segment.at(t), centre);
      let size = length(seen) * length(aim);
      let alike = if size > 0.0 { (seen[0] * aim[0] + seen[1] * aim[1]) / size } else { -1.0 };
      if alike > best.2 + 1e-12 {
        best = (index, t, alike);
      }
    }
  }
  (best.0, best.1)
}

/// A closed path as its two runs between two places on it — its leftmost and rightmost points, or the
/// points that stand, seen from its middle, in the direction of the two ends of the line it meets:
/// clockwise, the run over its top and then the one back under its bottom; the other way round, under
/// and then over.
fn halves(path: &Path, clockwise: bool, ends: Option<(Point, Point)>) -> [Vec<Cubic>; 2] {
  let clockwise = if (path.area() >= 0.0) == clockwise { path.clone() } else { path.reversed() };
  let centre = clockwise.bounds().centre();
  let (index, t) = match ends {
    Some((left, _)) => toward(&clockwise.segments, centre, left),
    None => furthest(&clockwise.segments, true),
  };
  let (split, start) = split_at(&clockwise.segments, index, t);
  let count = split.len();
  let rotated: Vec<Cubic> = (0..count).map(|step| split[(start + step) % count]).collect();
  let (index, t) = match ends {
    // The far place is sought past the near one, never at it.
    Some((_, right)) => toward(&rotated[1..], centre, right),
    None => furthest(&rotated, false),
  };
  let index = if ends.is_some() { index + 1 } else { index };
  let (split, middle) = split_at(&rotated, index, t);
  let (top, bottom) = split.split_at(middle);
  let still = |at| vec![Cubic([at; 4])];
  let top = if top.is_empty() { still(split[0].start()) } else { top.to_vec() };
  let bottom = if bottom.is_empty() { still(split[0].start()) } else { bottom.to_vec() };
  [top, bottom]
}

/// Whether an open path bows down between its ends, as a U or a smile does, rather than up.
fn bows_down(path: &Path) -> bool {
  let run = left_to_right(path);
  let (Some(first), Some(last)) = (run.first(), run.last()) else {
    return false;
  };
  let chord = (first.start()[1] + last.end()[1]) / 2.0;
  let points = path.polyline(8);
  points.iter().map(|point| point[1] - chord).sum::<f64>() > 0.0
}

/// The runs a path is cut into to meet another path. An open line meeting a closed shape runs out along
/// itself and back, and the shape is taken the way round that sends its first half the way the line
/// bows: over its top for a line bowing up, under its bottom for one bowing down.
fn runs(path: &Path, other: &Path) -> Vec<Vec<Cubic>> {
  if path.closed {
    if other.closed {
      return halves(path, true, None).to_vec();
    }
    let line = left_to_right(other);
    let ends = line.first().zip(line.last()).map(|(first, last)| (first.start(), last.end()));
    return halves(path, !bows_down(other), ends).to_vec();
  }
  let out = left_to_right(path);
  if !other.closed {
    return vec![out];
  }
  let back = out.iter().rev().map(Cubic::reversed).collect();
  vec![out, back]
}

/// The shares of a run's length behind each of its inner nodes.
fn shares(run: &[Cubic]) -> Vec<f64> {
  let lengths: Vec<f64> = run.iter().map(Cubic::length).collect();
  let whole: f64 = lengths.iter().sum();
  if whole <= 0.0 {
    return Vec::new();
  }
  let mut walked = 0.0;
  lengths[..lengths.len().saturating_sub(1)]
    .iter()
    .map(|length| {
      walked += length;
      walked / whole
    })
    .collect()
}

/// A run cut at the given shares of its length, wherever it holds no node there already.
fn cut_at(run: &[Cubic], cuts: &[f64]) -> Vec<Cubic> {
  let lengths: Vec<f64> = run.iter().map(Cubic::length).collect();
  let whole: f64 = lengths.iter().sum();
  let mut cut = Vec::with_capacity(run.len() + cuts.len());
  let mut walked = 0.0;
  for (segment, length) in run.iter().zip(&lengths) {
    let (from, to) = (walked / whole, (walked + length) / whole);
    let inner: Vec<f64> = cuts
      .iter()
      .copied()
      .filter(|share| *share > from + CLOSE && *share < to - CLOSE)
      .collect();
    let mut rest = *segment;
    let mut done = 0.0;
    for share in inner {
      let local = (share - from) / (to - from);
      let t = segment.at_share(local);
      let (before, after) = rest.split((t - done) / (1.0 - done));
      cut.push(before);
      rest = after;
      done = t;
    }
    cut.push(rest);
    walked += length;
  }
  cut
}

/// How near two shares of a length are taken as one place.
const CLOSE: f64 = 1e-3;

/// A run brought up to `count` segments, each time splitting its longest segment at its middle.
fn lengthened(run: &[Cubic], count: usize) -> Vec<Cubic> {
  let mut run = run.to_vec();
  while run.len() < count {
    let longest = (0..run.len())
      .max_by(|a, b| run[*a].length().total_cmp(&run[*b].length()))
      .expect("a run holds a segment");
    let (before, after) = run[longest].split(run[longest].middle());
    run.splice(longest..=longest, [before, after]);
  }
  run
}

/// Two runs cut at each other's nodes, so their segments pair by the share of length behind them.
fn aligned(from: &[Cubic], to: &[Cubic]) -> (Vec<Cubic>, Vec<Cubic>) {
  let (from_cut, to_cut) = (cut_at(from, &shares(to)), cut_at(to, &shares(from)));
  let count = from_cut.len().max(to_cut.len());
  (lengthened(&from_cut, count), lengthened(&to_cut, count))
}

/// A path whose straight segments run at an even pace: a segment whose control points stand on its ends
/// draws the same straight line as one whose control points stand a third of the way along, but stands
/// still at its ends, where a blend would let the other path's motion run on alone into a hook.
fn evened(path: &Path) -> Path {
  let still = |segment: &Cubic| segment.0[1] == segment.0[0] && segment.0[2] == segment.0[3];
  Path {
    segments: path
      .segments
      .iter()
      .map(|segment| if still(segment) { Cubic::line(segment.start(), segment.end()) } else { *segment })
      .collect(),
    closed: path.closed,
  }
}

/// The path between two paths, `t` of the way from the first to the second.
pub fn blend_paths(a: &Path, b: &Path, t: f64) -> Path {
  let (a, b) = (&evened(a), &evened(b));
  let closed = a.closed || b.closed;
  let segments = runs(a, b)
    .iter()
    .zip(runs(b, a))
    .flat_map(|(from, to)| {
      let (from, to) = aligned(from, &to);
      from.iter().zip(&to).map(|(one, other)| one.mixed(other, t)).collect::<Vec<_>>()
    })
    .collect();
  Path { segments, closed }
}

fn either<T: Clone>(a: &Option<T>, b: &Option<T>, t: f64) -> Option<T> {
  if t < 0.5 { a.clone().or_else(|| b.clone()) } else { b.clone().or_else(|| a.clone()) }
}

/// The part between two parts, `t` of the way: its path blended by correspondence, its line's width
/// mixed, its paints and what it holds inside taken from the nearer of the two, and the parts inside
/// paired by their paint, top to bottom.
pub fn blend_parts(a: &Part, b: &Part, t: f64) -> Part {
  let path = blend_paths(&a.path(), &b.path(), t);
  let line = match (&a.line, &b.line) {
    (Some(from), Some(to)) => Some(Line {
      paint: if t < 0.5 { from.paint } else { to.paint },
      width: (from.width + (to.width - from.width) * t).max(0.2),
    }),
    (from, to) => either(from, to, t),
  };
  let fill = if path.closed { either(&a.fill, &b.fill, t) } else { None };
  let inside = if path.closed { blend_inside(&a.inside, &b.inside, t) } else { Vec::new() };
  Part {
    nodes: Vec::new(),
    closed: path.closed,
    fill,
    line,
    inside,
  }
  .with_path(&path)
}

/// The parts inside, each paired with the part of the same paint at the same place counted from the
/// top; one with no counterpart shows in the nearer half alone.
fn blend_inside(a: &[Part], b: &[Part], t: f64) -> Vec<Part> {
  let top = |part: &Part| part.path().bounds().top;
  let by_paint = |parts: &[Part], paint: Option<Paint>| {
    let mut found: Vec<Part> = parts.iter().filter(|part| part.fill == paint).cloned().collect();
    found.sort_by(|one, other| top(one).total_cmp(&top(other)));
    found
  };
  let mut paints: Vec<Option<Paint>> = Vec::new();
  for part in a.iter().chain(b) {
    if !paints.contains(&part.fill) {
      paints.push(part.fill);
    }
  }
  paints
    .into_iter()
    .flat_map(|paint| blend_lists(&by_paint(a, paint), &by_paint(b, paint), t))
    .collect()
}

/// Two lists of parts blended place by place; a part with no counterpart shows in the nearer half alone.
pub fn blend_lists(a: &[Part], b: &[Part], t: f64) -> Vec<Part> {
  (0..a.len().max(b.len()))
    .filter_map(|index| match (a.get(index), b.get(index)) {
      (Some(from), Some(to)) => Some(blend_parts(from, to, t)),
      (Some(from), None) => (t < 0.5).then(|| from.clone()),
      (None, Some(to)) => (t >= 0.5).then(|| to.clone()),
      (None, None) => None,
    })
    .collect()
}
