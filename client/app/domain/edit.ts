import { addNode as coreAddNode, measureAnchor, mirrorParts, newInside as coreNewInside, newPart as coreNewPart, smoothHandles } from "#core";
import { checkFaceDocument, FALLBACK_FACE, LIMITS, PAIR_ELEMENTS, REST_MOST, SCREEN } from "#faces";

import type { NewInside, NewPart } from "#core";
import type { Anchor, Element, EyeVoice, Face, FaceDocument, FaceName, IdleStages, Pair, PairElement, Part, PathNode, Point, Refusal, Side, Tuning } from "#faces";

// The editor's own logic, kept apart from its screen (docs/face.md, 「The editor」): which parts are
// being edited, the handles each part is dragged by, and every change as a new part or a new
// document, so the screen only ever swaps one for the next. The geometry of a change — a node added,
// a smooth node's handles, a new part — is the face's core's.

export type { NewInside, NewPart };

/** What is being edited: one side of a face's paired element or its other element, or one anchor of the talking set. */
export type EditTarget = { tab: "faces"; face: FaceName; element: Element; side: Side } | { tab: "talk"; anchor: number };

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

export const isPair = (element: Element): element is PairElement => PAIR_ELEMENTS.includes(element);

/** The face a target edits, made from the fallback face where the document lacks it. */
function faceOf(document: FaceDocument, name: FaceName): Face {
  return document.faces[name] ?? structuredClone(document.faces[FALLBACK_FACE] as Face);
}

/** Whether the target is a right side drawn as the left reflected, which is not edited on its own. */
export function isMirrored(document: FaceDocument, target: EditTarget): boolean {
  if (target.tab !== "faces" || !isPair(target.element)) {
    return false;
  }

  return faceOf(document, target.face)[target.element].right === null;
}

export function partsOf(document: FaceDocument, target: EditTarget): Part[] {
  if (target.tab === "talk") {
    return document.talk.anchors[target.anchor]?.parts ?? [];
  }

  const face = document.faces[target.face];

  if (face === undefined) {
    return [];
  }

  if (!isPair(target.element)) {
    return face[target.element as "mouth" | "marks"];
  }

  const pair: Pair = face[target.element];

  return target.side === "left" ? pair.left : (pair.right ?? mirrorParts(pair.left));
}

/** The document with the target's parts replaced. A face the document lacks is made from the fallback first. */
export function withParts(document: FaceDocument, target: EditTarget, parts: Part[]): FaceDocument {
  if (target.tab === "talk") {
    return withAnchor(document, target.anchor, (anchor) => ({ ...anchor, parts }));
  }

  const face = faceOf(document, target.face);
  const element = isPair(target.element) ? { ...face[target.element], [target.side]: parts } : parts;

  return { ...document, faces: { ...document.faces, [target.face]: { ...face, [target.element]: element } } };
}

/** The document with a face's paired element mirrored, the right dropped; or drawn apart, the right the left reflected to edit on its own. */
export function setMirrored(document: FaceDocument, name: FaceName, element: PairElement, mirrored: boolean): FaceDocument {
  const face = faceOf(document, name);
  const pair = face[element];
  const right = mirrored ? null : (pair.right ?? mirrorParts(pair.left));

  return { ...document, faces: { ...document.faces, [name]: { ...face, [element]: { ...pair, right } } } };
}

/** The document with how far a face's eyes move with the gaze set, each way held to the limit. */
export function setReach(document: FaceDocument, name: FaceName, reach: Point): FaceDocument {
  const face = faceOf(document, name);
  const held = reach.map((value) => within(value, 0, LIMITS.maxReach)) as Point;

  return { ...document, faces: { ...document.faces, [name]: { ...face, eyes: { ...face.eyes, reach: held } } } };
}

/** The document with how far a face's eyes answer the voice set, each held between nought and the most an eye may. */
export function setEyeVoice(document: FaceDocument, name: FaceName, voice: EyeVoice): FaceDocument {
  const face = faceOf(document, name);
  const held = Object.fromEntries(Object.entries(voice).map(([key, value]) => [key, within(value, 0, LIMITS.maxEyeVoice)])) as EyeVoice;

  return { ...document, faces: { ...document.faces, [name]: { ...face, eyes: { ...face.eyes, voice: held } } } };
}

/** The document with how far a face's talking mouth lifts its corners set, held from −1 to 1. */
export function setCorners(document: FaceDocument, name: FaceName, corners: number): FaceDocument {
  return { ...document, faces: { ...document.faces, [name]: { ...faceOf(document, name), corners: within(corners, -1, 1) } } };
}

function within(value: number, least: number, most: number): number {
  return Math.min(most, Math.max(least, value));
}

/** A face's parts in the order they are drawn, both sides of a pair drawn out, leaving out one element's side or a whole element. */
export function faceParts(face: Face, except: { element: Element; side: Side | null } | null = null): Part[] {
  const order: Element[] = ["blush", "eyes", "brows", "mouth", "marks"];

  return order.flatMap((element) => {
    if (!isPair(element)) {
      return except?.element === element ? [] : face[element as "mouth" | "marks"];
    }

    const pair: Pair = face[element];
    const sides: [Side, Part[]][] = [
      ["left", pair.left],
      ["right", pair.right ?? mirrorParts(pair.left)],
    ];

    return sides.flatMap(([side, parts]) => (except?.element === element && (except.side === null || except.side === side) ? [] : parts));
  });
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
 * with both handles at nothing takes the handles of the smooth run through its part's anchors.
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
      const [coming, leaving] = smoothHandles(part, index) ?? [
        [0, 0],
        [0, 0],
      ];

      changedNode = { ...changedNode, coming: along(anchor, coming, length(coming)), leaving: along(anchor, leaving, length(leaving)) };
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
  if (part.nodes.length >= LIMITS.maxNodes || index < 0 || index >= part.nodes.length) {
    return { part, chosen: index };
  }

  return coreAddNode(part, index);
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
// New parts, drawn by the core, each with its own paints whatever element it is added to.

/** A new part in the middle of the screen: an ink dot the size of an eye, or an ink line of three smooth nodes. */
export function newPart(kind: NewPart): Part {
  return coreNewPart(kind);
}

/** A new part to stand inside `outer`, sized to its opening. */
export function newInside(kind: NewInside, outer: Part): Part {
  return coreNewInside(kind, outer);
}

/** The part with a new part inside it; an open part, or one holding as many as it may, is left as it stands. */
export function addInside(part: Part, kind: NewInside): Part {
  const inside = part.inside ?? [];

  if (!part.closed || inside.length >= LIMITS.inside) {
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
// The talking mouth set and saving.

/** The document with one anchor of the talking set changed. */
function withAnchor(document: FaceDocument, index: number, change: (anchor: Anchor) => Anchor): FaceDocument {
  return { ...document, talk: { ...document.talk, anchors: document.talk.anchors.map((anchor, place) => (place === index ? change(anchor) : anchor)) } };
}

/** How much more open a copy of an anchor stands than the anchor it copies. */
const COPY_STEP = 0.05;

/** A place on the talking set's plane held to its square and to a thousandth. */
function placeOf(at: Point): Point {
  return at.map((value) => Math.round(within(value, 0, 1) * 1000) / 1000) as Point;
}

/**
 * The talking set with an anchor added after the given one: a copy of it standing a twentieth more open,
 * or less where that would pass the top (docs/face.md, 「The editor」); none past the limit.
 */
export function addAnchor(document: FaceDocument, after: number): FaceDocument {
  const { anchors } = document.talk;
  const copied = anchors[after];

  if (copied === undefined || anchors.length >= LIMITS.anchors) {
    return document;
  }

  const open = copied.at[0] + COPY_STEP > 1 ? copied.at[0] - COPY_STEP : copied.at[0] + COPY_STEP;
  const added: Anchor = { ...structuredClone(copied), at: placeOf([open, copied.at[1]]) };

  return { ...document, talk: { ...document.talk, anchors: anchors.toSpliced(after + 1, 0, added) } };
}

/** The talking set with an anchor removed; never its last. */
export function removeAnchor(document: FaceDocument, index: number): FaceDocument {
  const { anchors } = document.talk;

  return anchors.length <= 1 ? document : { ...document, talk: { ...document.talk, anchors: anchors.filter((_, place) => place !== index) } };
}

/** The talking set with where an anchor stands set, how open and how wide, each held from nought to one. */
export function setAnchorPlace(document: FaceDocument, index: number, at: Point): FaceDocument {
  return withAnchor(document, index, (anchor) => ({ ...anchor, at: placeOf(at) }));
}

/** The talking set with whether an anchor's teeth stand together set. */
export function setAnchorTeeth(document: FaceDocument, index: number, teeth: boolean): FaceDocument {
  return withAnchor(document, index, (anchor) => ({ ...anchor, teeth }));
}

/** The talking set with an anchor placed where its drawing measures (docs/face.md, 「The talking set」); unchanged for a drawing of nothing. */
export function placeMeasured(document: FaceDocument, index: number): FaceDocument {
  const measured = measureAnchor(document.talk.anchors[index]?.parts ?? []);

  return measured === null ? document : withAnchor(document, index, (anchor) => ({ ...anchor, at: measured.at, teeth: measured.teeth }));
}

/** The talking set with one tuning value set: a number, a span of two, or the switch. */
export function setTuning(document: FaceDocument, key: string, value: Tuning[string]): FaceDocument {
  return { ...document, talk: { ...document.talk, tuning: { ...document.talk.tuning, [key]: value } } };
}

/** A rest stage the editor sets, by its key in the document's `idle`. */
export type RestStage = keyof IdleStages;

/** The longest a rest stage may wait, in the minutes the editor shows. */
export const REST_MOST_MINUTES = REST_MOST / 60;

/** A rest stage as the editor shows it, in minutes. */
export function restMinutes(document: FaceDocument, stage: RestStage): number {
  return document.idle[stage] / 60;
}

/**
 * The document with one rest stage set from minutes, kept as the whole second nearest them within the
 * core's bounds: the blank face from nought, sleep from the blank face on. A blank face set past sleep
 * carries sleep up with it.
 */
export function setRestMinutes(document: FaceDocument, stage: RestStage, minutes: number): FaceDocument {
  const { blank, asleep } = document.idle;

  if (stage === "asleep") {
    return { ...document, idle: { blank, asleep: within(Math.round(minutes * 60), blank, REST_MOST) } };
  }

  const held = within(Math.round(minutes * 60), 0, REST_MOST);

  return { ...document, idle: { blank: held, asleep: Math.max(asleep, held) } };
}

/** What a save sends: the document without its version, and the version it was edited from. */
export function saveBody(document: FaceDocument, basedOn: number): { document: Omit<FaceDocument, "savedAt">; basedOn: number } {
  const { savedAt: _version, ...rest } = document;

  return { document: rest, basedOn };
}

/**
 * A value with every object's keys in one order, so two documents written by different hands are
 * compared by what they hold. The server hands a saved document back with its keys in the order
 * its check writes them, which is not always the order a screen built one in.
 */
function ordered(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(ordered);
  }

  if (value !== null && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value as Record<string, unknown>)
        .sort()
        .map((key) => [key, ordered((value as Record<string, unknown>)[key])]),
    );
  }

  return value;
}

/** Whether the edited document differs from the one saved. */
export function changed(edited: FaceDocument, saved: FaceDocument): boolean {
  return JSON.stringify(ordered(saveBody(edited, 0).document)) !== JSON.stringify(ordered(saveBody(saved, 0).document));
}

/** Why the edited document would be refused, checked here before it is sent; null for one that passes. */
export function refusalOf(document: FaceDocument): Refusal | null {
  const checked = checkFaceDocument(saveBody(document, 0).document);

  return "refusal" in checked ? checked.refusal : null;
}

/** The words of a refusal: where it was met, from the outside in, and why, each from the language pack. */
export function refusalWords(refusal: Refusal, t: (key: string, values?: Record<string, unknown>) => string): string {
  const place = (at: Refusal["at"][number]): string => {
    if ("face" in at) return t(`face.name.${at.face}`);
    if ("element" in at) return t(`face.edit.element.${at.element}`);
    if ("side" in at) return t(`face.edit.side.${at.side}`);
    if ("color" in at) return t("face.edit.place.color", { name: at.color });
    if ("tuning" in at) return t(`face.edit.tuning.key.${at.tuning}`);
    if ("idle" in at) return t(`face.edit.place.idle.${at.idle}`);

    const [kind, n] = Object.entries(at)[0] as [string, number];

    return t(`face.edit.place.${kind}`, { n });
  };
  const why = t(`face.edit.refusal.${refusal.code}`, { least: refusal.least, most: refusal.most, name: refusal.name });

  return refusal.at.length === 0 ? why : `${refusal.at.map(place).join(" · ")}: ${why}`;
}
