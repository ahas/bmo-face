import { checkFaceDocument, FALLBACK_FACE, LIMITS, ovalNodes, SCREEN, splineNodes } from "#faces";

import { pointOn, segmentsOf, splitSegment } from "./draw";

import type { Face, FaceDocument, FaceName, Part, PartGroup, PathNode, Point } from "#faces";

// The editor's own logic, kept apart from its screen (docs/face.md, 「The editor」): which parts are
// being edited, the handles each part is dragged by, and every change as a new part or a new
// document, so the screen only ever swaps one for the next.

/** What is being edited: a group of one face, or one mouth stage. */
export type EditTarget = { tab: "faces"; face: FaceName; group: PartGroup } | { tab: "mouths"; stage: number };

/** A part among the target's parts: one of them, or one standing inside one of them. */
export interface PartAt {
  index: number;
  inside: number | null;
}

/** A handle on a part: a node's anchor, or the end of one of its two handles. */
export interface Handle {
  node: number;
  kind: "anchor" | "coming" | "leaving";
}

/** A handle and where it stands on the screen. */
export interface HandleAt extends Handle {
  at: Point;
}

export function partsOf(document: FaceDocument, target: EditTarget): Part[] {
  if (target.tab === "mouths") {
    return document.mouths[target.stage] ?? [];
  }

  return document.faces[target.face]?.[target.group] ?? [];
}

/** The document with the target's parts replaced. A face the document lacks is made from 기본 first. */
export function withParts(document: FaceDocument, target: EditTarget, parts: Part[]): FaceDocument {
  if (target.tab === "mouths") {
    return { ...document, mouths: document.mouths.map((stage, index) => (index === target.stage ? parts : stage)) };
  }

  const face: Face = document.faces[target.face] ?? structuredClone(document.faces[FALLBACK_FACE] as Face);

  return { ...document, faces: { ...document.faces, [target.face]: { ...face, [target.group]: parts } } };
}

/** The part a place names among the parts, or none. */
export function partAt(parts: Part[], at: PartAt): Part | undefined {
  const part = parts[at.index];

  return at.inside === null ? part : part?.inside?.[at.inside];
}

/** The parts with the one a place names replaced. */
export function withPart(parts: Part[], at: PartAt, part: Part): Part[] {
  return parts.map((own, index) => {
    if (index !== at.index) {
      return own;
    }

    if (at.inside === null) {
      return part;
    }

    return { ...own, inside: (own.inside ?? []).map((inner, place) => (place === at.inside ? part : inner)) };
  });
}

// ---------------------------------------------------------------------------------------------
// Points and handles.

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));
const round = (value: number) => Math.round(value * 10) / 10;
const add = ([ax, ay]: Point, [bx, by]: Point): Point => [ax + bx, ay + by];
const length = ([x, y]: Point) => Math.hypot(x, y);
const isNothing = (offset: Point) => length(offset) === 0;

/** A point held inside the screen, to a tenth. */
export function onScreen([x, y]: Point): Point {
  return [round(clamp(x, 0, SCREEN.width)), round(clamp(y, 0, SCREEN.height))];
}

/** The offset from an anchor to the point a handle's end lands on, to a tenth, and on the screen. */
function offsetTo(anchor: Point, to: Point): Point {
  const end = onScreen(to);
  // An anchor on a tenth gives an offset on a tenth; one that is not (a converted document's) keeps
  // the exact difference, so the end still lands on the screen. Adding zero turns −0 into 0.
  const step = (axis: 0 | 1) => {
    const difference = end[axis] - anchor[axis];

    return (round(anchor[axis]) === anchor[axis] ? round(difference) : difference) + 0;
  };

  return [step(0), step(1)];
}

/** A handle turned to point along `direction`, `size` long, its end shortened to the screen's edge. */
function along(anchor: Point, direction: Point, size: number): Point {
  const unit = length(direction);

  if (unit === 0 || size === 0) {
    return [0, 0];
  }

  const [dx, dy] = [(direction[0] / unit) * size, (direction[1] / unit) * size];
  const room = (at: number, step: number, most: number) => (step < 0 ? at / -step : step > 0 ? (most - at) / step : Infinity);
  const share = Math.max(0, Math.min(1, room(anchor[0], dx, SCREEN.width), room(anchor[1], dy, SCREEN.height)));

  return offsetTo(anchor, [anchor[0] + dx * share, anchor[1] + dy * share]);
}

/**
 * The handles a part is dragged by: every node's anchor, and for the chosen node the ends of its two
 * handles — a handle of no length shows none.
 */
export function handlesOf(part: Part, chosen: number | null = null): HandleAt[] {
  const handles: HandleAt[] = part.nodes.map((node, index) => ({ node: index, kind: "anchor", at: node.anchor }));
  const node = chosen === null ? undefined : part.nodes[chosen];

  if (chosen !== null && node !== undefined) {
    for (const kind of ["coming", "leaving"] as const) {
      if (!isNothing(node[kind])) {
        handles.push({ node: chosen, kind, at: add(node.anchor, node[kind]) });
      }
    }
  }

  return handles;
}

// How far a handle can be taken hold of from, in the screen's units: a finger's width where there is
// room — the canvas says how many units that is at the size it is drawn — and never past halfway to
// the part's nearest other handle, so two handles close together are each still reached where they
// stand.
const REACH = 3.5;
const LEAST_REACH = 0.8;

export function reachOf(handles: HandleAt[], index: number, most: number = REACH): number {
  const own = handles[index];

  if (own === undefined) {
    return REACH;
  }

  let nearest = Number.POSITIVE_INFINITY;

  for (const other of handles) {
    if (other !== own) {
      nearest = Math.min(nearest, Math.hypot(other.at[0] - own.at[0], other.at[1] - own.at[1]));
    }
  }

  return Math.max(LEAST_REACH, Math.min(most, nearest / 2));
}

/**
 * The handle a press at `point` takes hold of: the nearest one within its reach. Where handles stand
 * on one another, the anchor is taken, so a node is never lost under its own handle.
 */
export function handleAt(handles: HandleAt[], point: Point, most: number = REACH): Handle | null {
  let best: { handle: HandleAt; distance: number } | null = null;

  for (const [index, handle] of handles.entries()) {
    const distance = Math.hypot(handle.at[0] - point[0], handle.at[1] - point[1]);

    if (distance > reachOf(handles, index, most)) {
      continue;
    }

    const closer = best === null || distance < best.distance - 1e-9;
    const tiedAnchor = best !== null && Math.abs(distance - best.distance) <= 1e-9 && handle.kind === "anchor" && best.handle.kind !== "anchor";

    if (closer || tiedAnchor) {
      best = { handle, distance };
    }
  }

  return best === null ? null : { node: best.handle.node, kind: best.handle.kind };
}

/** The node's anchor moved to a point; its handles come along, and it stops where one would leave the screen. */
function movedAnchor(node: PathNode, to: Point): Point {
  const offsets = [[0, 0], node.coming, node.leaving];
  const tenthUp = (value: number) => Math.ceil(value * 10 - 1e-9) / 10;
  const tenthDown = (value: number) => Math.floor(value * 10 + 1e-9) / 10;
  const bound = (axis: 0 | 1, most: number, value: number) => {
    const low = tenthUp(Math.max(...offsets.map((offset) => -offset[axis]!)));
    const high = tenthDown(most - Math.max(...offsets.map((offset) => offset[axis]!)));

    return clamp(round(value), low, high);
  };

  return [bound(0, SCREEN.width, to[0]), bound(1, SCREEN.height, to[1])];
}

/**
 * The part with one handle dragged to a point. An anchor carries its handles; a handle's end moves
 * that handle, and on a smooth node the other handle turns to stay on the same line, keeping its
 * own length. Nothing leaves the screen, and what moves lands on a tenth.
 */
export function moveHandle(part: Part, handle: Handle, to: Point): Part {
  const node = part.nodes[handle.node];

  if (node === undefined) {
    return part;
  }

  let moved: PathNode;

  if (handle.kind === "anchor") {
    moved = { ...node, anchor: movedAnchor(node, to) };
  } else {
    const offset = offsetTo(node.anchor, to);
    const other = handle.kind === "coming" ? "leaving" : "coming";

    moved = { ...node, [handle.kind]: offset };

    if (node.kind === "smooth" && !isNothing(offset)) {
      moved[other] = along(node.anchor, [-offset[0], -offset[1]], length(node[other]));
    }
  }

  return { ...part, nodes: part.nodes.map((own, index) => (index === handle.node ? moved : own)) };
}

/**
 * The part with a node made smooth or a corner. Made smooth, its coming handle turns to stand
 * opposite its leaving one, keeping its length or taking the leaving one's where it had none; a node
 * with both handles at nothing takes the spline's handles through its neighbours.
 */
export function setSmooth(part: Part, index: number, smooth: boolean): Part {
  const node = part.nodes[index];

  if (node === undefined) {
    return part;
  }

  let changedNode: PathNode = { ...node, kind: smooth ? "smooth" : "corner" };

  if (smooth) {
    const { anchor, coming, leaving } = node;

    if (!isNothing(leaving)) {
      changedNode = { ...changedNode, coming: along(anchor, [-leaving[0], -leaving[1]], length(isNothing(coming) ? leaving : coming)) };
    } else if (!isNothing(coming)) {
      changedNode = { ...changedNode, leaving: along(anchor, [-coming[0], -coming[1]], length(coming)) };
    } else {
      const spline = splineNodes(
        part.nodes.map((own) => own.anchor),
        part.closed,
      )[index]!;

      changedNode = { ...changedNode, coming: along(anchor, spline.coming, length(spline.coming)), leaving: along(anchor, spline.leaving, length(spline.leaving)) };
    }
  }

  return { ...part, nodes: part.nodes.map((own, place) => (place === index ? changedNode : own)) };
}

/**
 * The part with a node added after the chosen one by cutting the segment it leaves by at its middle
 * — at the end of an open path, the segment coming into it — which leaves the drawing as it was.
 * `chosen` is the new node. A part at its limit is left as it stands.
 */
export function addNode(part: Part, index: number): { part: Part; chosen: number } {
  const count = part.nodes.length;

  if (count >= LIMITS.maxNodes || index < 0 || index >= count) {
    return { part, chosen: index };
  }

  const segment = !part.closed && index === count - 1 ? index - 1 : index;
  const [first, second] = splitSegment(segmentsOf(part)[segment]!, 0.5);
  const from = part.nodes[segment]!;
  const toIndex = (segment + 1) % count;
  const to = part.nodes[toIndex]!;
  const middle = first[3];
  const offset = ([x, y]: Point, [ax, ay]: Point): Point => [x - ax, y - ay];
  const added: PathNode = { anchor: middle, coming: offset(first[2], middle), leaving: offset(second[1], middle), kind: "smooth" };
  const nodes = part.nodes.map((own, place) => {
    if (place === segment) {
      return { ...own, leaving: offset(first[1], from.anchor) };
    }

    if (place === toIndex) {
      return { ...own, coming: offset(second[2], to.anchor) };
    }

    return own;
  });

  nodes.splice(segment + 1, 0, added);

  return { part: { ...part, nodes }, chosen: segment + 1 };
}

/** The part with a node taken out, down to two. */
export function removeNode(part: Part, index: number): Part {
  if (part.nodes.length <= LIMITS.minNodes) {
    return part;
  }

  return { ...part, nodes: part.nodes.filter((_, place) => place !== index) };
}

/**
 * The part opened or closed. Opening drops the fill, a part with no line taking its fill's paint as
 * one; a part with anything inside it is not opened, and null says so.
 */
export function setClosed(part: Part, closed: boolean): Part | null {
  if (closed) {
    return { ...part, closed: true };
  }

  if ((part.inside ?? []).length > 0) {
    return null;
  }

  const line = part.line ?? (part.fill === null ? null : { paint: part.fill, width: 1 });

  return { ...part, closed: false, fill: null, line };
}

// ---------------------------------------------------------------------------------------------
// New parts. Each comes with its own paints, whatever group it is added to.

export type NewPart = "oval" | "line";
export type NewInside = "teeth" | "tongue";

/** A new part in the middle of the screen: an ink dot of four smooth nodes, or an ink line of three. */
export function newPart(kind: NewPart): Part {
  const mx = SCREEN.width / 2;
  const my = SCREEN.height / 2;

  return kind === "oval"
    ? { nodes: ovalNodes(mx, my, 4, 4), closed: true, fill: "ink", line: null }
    : { nodes: splineNodes([[mx - 8, my], [mx, my + 4], [mx + 8, my]], false), closed: false, fill: null, line: { paint: "ink", width: 2.2 } };
}

/** The box a part's drawing stands in, read from points along its segments. */
function boundsOf(part: Part): { left: number; right: number; top: number; bottom: number } {
  const points = segmentsOf(part).flatMap((segment) => Array.from({ length: 9 }, (_, step) => pointOn(segment, step / 8)));
  const xs = points.map(([x]) => x);
  const ys = points.map(([, y]) => y);

  return { left: Math.min(...xs), right: Math.max(...xs), top: Math.min(...ys), bottom: Math.max(...ys) };
}

const corner = (anchor: Point, coming: Point = [0, 0], leaving: Point = [0, 0]): PathNode => ({ anchor: onScreen(anchor), coming, leaving, kind: "corner" });

/**
 * A new part to stand inside `outer`, sized to its opening: the upper teeth band along the top,
 * reaching past the sides to be cut off at the outline; or the tongue, a dome rising from below the
 * bottom edge, a little to the left.
 */
export function newInside(kind: NewInside, outer: Part): Part {
  const { left, right, top, bottom } = boundsOf(outer);
  const width = right - left;
  const height = bottom - top;

  if (kind === "teeth") {
    const lower = top + height * 0.3;
    const pull = round(width / 4);

    return {
      nodes: [corner([left - 2, top - 2]), corner([right + 2, top - 2]), corner([right + 2, lower], [0, 0], [-pull, 0.6]), corner([left - 2, lower], [pull, 0.6])],
      closed: true,
      fill: "light",
      line: null,
    };
  }

  const [from, to] = [onScreen([left + width * 0.15, bottom + 2]), onScreen([left + width * 0.65, bottom + 2])];
  const peak = onScreen([(from[0] + to[0]) / 2, bottom - height * 0.4]);
  const rise = round((peak[1] - from[1]) * 0.55);
  const reach = round(((to[0] - from[0]) / 2) * 0.55);

  return {
    nodes: [corner(from, [0, 0], [0, rise]), { anchor: peak, coming: [-reach, 0], leaving: [reach, 0], kind: "smooth" }, corner(to, [0, rise])],
    closed: true,
    fill: "tongue",
    line: null,
  };
}

/** The part with a new part inside it; an open part, or one holding as many as it may, is left as it stands. */
export function addInside(part: Part, kind: NewInside): Part {
  const inside = part.inside ?? [];

  if (!part.closed || inside.length >= LIMITS.partsInside) {
    return part;
  }

  return { ...part, inside: [...inside, newInside(kind, part)] };
}

/** The part with one of the parts inside it taken out. */
export function removeInside(part: Part, index: number): Part {
  const inside = (part.inside ?? []).filter((_, place) => place !== index);

  if (inside.length === 0) {
    const { inside: _gone, ...rest } = part;

    return rest;
  }

  return { ...part, inside };
}

// ---------------------------------------------------------------------------------------------
// Stages and saving.

/** The mouth stages with one added after the given one, a copy of it; none past the limit. */
export function addStage(document: FaceDocument, after: number): FaceDocument {
  if (document.mouths.length >= LIMITS.maxStages) {
    return document;
  }

  const mouths = [...document.mouths];

  mouths.splice(after + 1, 0, structuredClone(mouths[after] ?? []));

  return { ...document, mouths };
}

/** The mouth stages with one removed; never fewer than the least. */
export function removeStage(document: FaceDocument, stage: number): FaceDocument {
  if (document.mouths.length <= LIMITS.minStages) {
    return document;
  }

  return { ...document, mouths: document.mouths.filter((_, index) => index !== stage) };
}

/** What a save sends: the document without its version, and the version it was edited from. */
export function saveBody(document: FaceDocument, basedOn: number): { document: Omit<FaceDocument, "savedAt">; basedOn: number } {
  const { savedAt: _version, ...rest } = document;

  return { document: rest, basedOn };
}

/** Whether the edited document differs from the one saved. */
export function changed(edited: FaceDocument, saved: FaceDocument): boolean {
  return JSON.stringify(saveBody(edited, 0).document) !== JSON.stringify(saveBody(saved, 0).document);
}

/** Why the edited document would be refused, checked here before it is sent; null for one that passes. */
export function refusalOf(document: FaceDocument): string | null {
  const checked = checkFaceDocument(saveBody(document, 0).document);

  return "error" in checked ? checked.error : null;
}
