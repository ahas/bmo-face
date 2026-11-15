// The expression document as the screen reads and edits it (docs/face.md, 「The expression document」):
// its shape as types, the names it is keyed by, and what the core answers of it — the default, the
// check, the limits. The core stated in Rust is the one implementation; this module types it and hands
// on its answers, so the screen keeps no copy of either.

import { checkDocument, defaultDocument, FACTS } from "./core";

/** The emotions BMO speaks a briefing with; one face each. None named reads as the fallback face. */
export const EMOTIONS = ["기본", "기쁨", "슬픔", "놀람", "화남", "졸림", "폭소", "걱정", "아픔", "안간힘"] as const;
export type Emotion = (typeof EMOTIONS)[number];

/** What BMO is doing, as its presence names it; one face each. */
export const STATES = ["writing", "thinking", "idle"] as const;
export type FaceState = (typeof STATES)[number];

export const FACE_NAMES = [...EMOTIONS, ...STATES] as const;
export type FaceName = Emotion | FaceState;

/** The face every missing face falls back to, which a document must therefore hold. */
export const FALLBACK_FACE: FaceName = EMOTIONS[0];

/** A face's elements, in the order the editor offers them; the first three are pairs. */
export const ELEMENTS = ["eyes", "brows", "blush", "mouth", "marks"] as const;
export type Element = (typeof ELEMENTS)[number];
export type PairElement = "eyes" | "brows" | "blush";
export type ListElement = "mouth" | "marks";
export const PAIR_ELEMENTS: readonly Element[] = ["eyes", "brows", "blush"];

export const SIDES = ["left", "right"] as const;
export type Side = (typeof SIDES)[number];

/** The paints a part is filled and lined with; each is a colour of the document. */
export const PAINTS = ["ink", "light", "mouth", "tongue", "tongueLight", "cheek", "tear"] as const;
export type Paint = (typeof PAINTS)[number];

/** How open a talking mouth is, among the mouths of its family. */
export const SIZES = ["small", "medium", "large"] as const;
export type Size = (typeof SIZES)[number];

/** The sounds a family of talking mouths is drawn for. */
export type Voice = "silence" | "closing" | "rounded" | "open" | "other";

/** The screen's own coordinates: every anchor and every handle end stands inside this box. */
export const SCREEN = FACTS.screen;

/** The limits a document is held to, as the core states them. */
export const LIMITS = FACTS.limits;

export type Point = [number, number];

/**
 * One node of a path: the anchor it passes through and its two handles, each an offset from the
 * anchor — the one the path comes in by and the one it leaves by. A smooth node's handles stand on
 * one line through the anchor, which the editor keeps; the drawing reads only the handles.
 */
export interface PathNode {
  anchor: Point;
  coming: Point;
  leaving: Point;
  kind: "smooth" | "corner";
}

export interface PartLine {
  paint: Paint;
  width: number;
}

/** One path of cubic Bézier segments, with its own paints and, when closed, the parts cut off inside it. */
export interface Part {
  nodes: PathNode[];
  closed: boolean;
  fill: Paint | null;
  line: PartLine | null;
  inside?: Part[];
}

/** An element drawn once a side: the left's parts, and the right's, or null where the right mirrors the left. */
export interface Pair {
  left: Part[];
  right: Part[] | null;
}

/** The eyes, and how far each moves each way as the gaze turns. */
export interface Eyes extends Pair {
  reach: Point;
}

export interface Face {
  eyes: Eyes;
  brows: Pair;
  blush: Pair;
  mouth: Part[];
  marks: Part[];
}

export interface TalkMouth {
  size: Size;
  parts: Part[];
}

/** A family of talking mouths: its name, the sounds it is drawn for, how often it is drawn, its drawings. */
export interface TalkFamily {
  name: string;
  voices: Voice[];
  weight: number;
  mouths: TalkMouth[];
}

export type FaceColors = { body: string; screen: string } & Record<Paint, string>;

export interface FaceDocument {
  form: number;
  colors: FaceColors;
  faces: Partial<Record<FaceName, Face>>;
  /** The talking mouth set. */
  talk: TalkFamily[];
  /** When it was last saved, in epoch milliseconds; also its version. */
  savedAt: number;
}

/** Where in a document a refusal was met, from the outside in, each place counted from one. */
export type RefusalPlace =
  | { color: string }
  | { face: string }
  | { element: string }
  | { side: Side }
  | { part: number }
  | { inside: number }
  | { node: number }
  | { family: number }
  | { mouth: number };

/** Why a document is refused, as a code the screen words from its language pack, and where. */
export interface Refusal {
  code: string;
  at: RefusalPlace[];
  least?: number;
  most?: number;
  name?: string;
}

export function isFaceName(word: unknown): word is FaceName {
  return typeof word === "string" && (FACE_NAMES as readonly string[]).includes(word);
}

/** The face drawn for a name: its own, or the fallback's where the document holds none. */
export function faceFor(document: FaceDocument, name: FaceName): Face {
  return document.faces[name] ?? (document.faces[FALLBACK_FACE] as Face);
}

/** The document as it may be saved, or the refusal saying where and why not. `savedAt` is not checked here. */
export function checkFaceDocument(value: unknown): { ok: Omit<FaceDocument, "savedAt"> } | { refusal: Refusal } {
  return checkDocument(value);
}

/** The default document, which a new data root starts from. */
export const DEFAULT_FACES: FaceDocument = defaultDocument();
