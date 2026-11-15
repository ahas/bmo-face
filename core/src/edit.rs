//! The geometry the editor's actions need (docs/face.md, 「The editor」): a node added without changing
//! the drawing, the handles a smooth node takes from its neighbours, and the new parts it makes — a part
//! of its own, or one of the parts that stand inside an opening, sized to it.

use crate::document::{Kind, Node, Paint, Part, SCREEN, limits};
use crate::geometry::{Point, sub};
use crate::shapes::{
  Opening, STROKE, dot, filled, oval, smooth_nodes, stroke, teeth, tongue, tongue_light,
};

/// The part with a node added after the chosen one by cutting the segment it leaves by at its middle —
/// at the end of an open path, the segment coming into it — which leaves the drawing as it was, and the
/// index of the new node. A part at its limit is handed back as it stands.
pub fn add_node(part: &Part, index: usize) -> (Part, usize) {
  let count = part.nodes.len();
  if count >= limits::MAX_NODES || index >= count {
    return (part.clone(), index);
  }
  let segment = if !part.closed && index == count - 1 { index - 1 } else { index };
  let (first, second) = part.path().segments[segment].split(0.5);
  let middle = first.end();
  let next = (segment + 1) % count;
  let mut nodes = part.nodes.clone();
  nodes[segment].leaving = sub(first.0[1], nodes[segment].anchor);
  nodes[next].coming = sub(second.0[2], nodes[next].anchor);
  nodes.insert(
    segment + 1,
    Node {
      anchor: middle,
      coming: sub(first.0[2], middle),
      leaving: sub(second.0[1], middle),
      kind: Kind::Smooth,
    },
  );
  (
    Part {
      nodes,
      ..part.clone()
    },
    segment + 1,
  )
}

/// The handles a node takes when it is made smooth with both its handles at nothing: those of the smooth
/// run through every anchor of its part, coming first.
pub fn smooth_handles(part: &Part, index: usize) -> Option<(Point, Point)> {
  let anchors: Vec<Point> = part.nodes.iter().map(|node| node.anchor).collect();
  smooth_nodes(&anchors, part.closed).get(index).map(|node| (node.coming, node.leaving))
}

/// A part the editor adds of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewPart {
  /// An ink dot of four smooth nodes, the size of an eye.
  Oval,
  /// An ink line of three smooth nodes.
  Line,
}

/// A new part in the middle of the screen.
pub fn new_part(kind: NewPart) -> Part {
  let [cx, cy] = [SCREEN[0] / 2.0, SCREEN[1] / 2.0];
  match kind {
    NewPart::Oval => dot([cx, cy], 3.0, 3.25),
    NewPart::Line => stroke(&[[cx - 8.0, cy], [cx, cy + 2.0], [cx + 8.0, cy]], STROKE),
  }
}

/// How high a new tongue rises, and how deep a new teeth band hangs, as shares of the opening's height.
const TONGUE_RISE: f64 = 0.38;
const TEETH_DEPTH: f64 = 0.24;

/// A part the editor adds inside a closed part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewInside {
  Teeth,
  LowerTeeth,
  Tongue,
  TongueLight,
  Shine,
}

/// A new part to stand inside `outer`, sized to its box: a teeth band hanging from its top or rising from
/// its bottom, the tongue rising from its bottom or its light tone, or a white shine high on its left.
pub fn new_inside(kind: NewInside, outer: &Part) -> Part {
  let bounds = outer.path().bounds();
  let opening = Opening::new(bounds.centre(), bounds.width(), bounds.height());
  match kind {
    NewInside::Teeth => filled(&teeth(&opening, TEETH_DEPTH, false), Paint::Light),
    NewInside::LowerTeeth => filled(&teeth(&opening, TEETH_DEPTH, true), Paint::Light),
    NewInside::Tongue => filled(&tongue(&opening, TONGUE_RISE), Paint::Tongue),
    NewInside::TongueLight => filled(&tongue_light(&tongue(&opening, TONGUE_RISE)), Paint::TongueLight),
    NewInside::Shine => {
      let [cx, cy] = bounds.centre();
      let at = [cx - bounds.width() * 0.18, cy - bounds.height() * 0.18];
      Part {
        nodes: oval(at, bounds.width() * 0.15, bounds.height() * 0.13),
        closed: true,
        fill: Some(Paint::Light),
        line: None,
        inside: Vec::new(),
      }
    }
  }
}
