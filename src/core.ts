// The face's core, built to WebAssembly (docs/face.md, 「The face's core」), compiled and instantiated
// the first time this module is imported, so nothing is awaited before a call. The module's bytes are a
// JavaScript module's text, which Bun and every bundler read as they are. This module types what the
// core answers and holds no reading of its own.

import * as glue from "./wasm/bmo_face_core.js";
import { bytes } from "./wasm/bytes.js";

import type { FaceColors, FaceDocument, Part, Point, Refusal } from "./faces";

function decoded(base64: string): Uint8Array {
  const binary = atob(base64);
  const decoded = new Uint8Array(binary.length);

  for (let at = 0; at < binary.length; at += 1) {
    decoded[at] = binary.charCodeAt(at);
  }

  return decoded;
}

glue.initSync({ module: decoded(bytes) });

/** What the core states of the document and the clock. */
export interface Facts {
  form: number;
  screen: { width: number; height: number };
  limits: {
    parts: number;
    inside: number;
    minNodes: number;
    maxNodes: number;
    minWidth: number;
    maxWidth: number;
    maxReach: number;
    families: number;
    mouths: number;
  };
  emotions: string[];
  states: string[];
  fallback: string;
  paints: string[];
  sizes: string[];
  voices: string[];
  /** Frames a second on the held clock. */
  fps: number;
  /** How many frames a talking mouth shows before the sound it is drawn for. */
  lead: number;
}

export const FACTS: Facts = JSON.parse(glue.facts());

/** A path as the core hands it: where it starts, each segment's two control points and end, and whether it closes. */
export interface DrawnPath {
  start: Point;
  curves: [number, number, number, number, number, number][];
  closed: boolean;
}

/** One thing laid down: a path filled, or stroked with round caps and joins; cut off at a clip where it names one. */
export type DrawnPaint =
  | { fill: string; path: DrawnPath; clip: number | null }
  | { stroke: string; width: number; path: DrawnPath; clip: number | null };

/** A drawing: the screen's size and colours, the outlines that clip, and the paint in the order it is laid down. */
export interface Drawing {
  width: number;
  height: number;
  body: string;
  screen: string;
  clips: DrawnPath[];
  paint: DrawnPaint[];
}

export function defaultDocument(): FaceDocument {
  return JSON.parse(glue.defaultDocument());
}

export function checkDocument(value: unknown): { ok: Omit<FaceDocument, "savedAt"> } | { refusal: Refusal } {
  return JSON.parse(glue.checkDocument(JSON.stringify(value ?? null)));
}

/** Parts drawn in order on a document's colours, as the editor's canvas draws what it edits. */
export function partsDrawing(parts: Part[], colors: FaceColors): Drawing {
  return JSON.parse(glue.partsDrawing(JSON.stringify(parts), JSON.stringify(colors)));
}

/** A node added after the one at `index`, the drawing unchanged, and the index of the new node. */
export function addNode(part: Part, index: number): { part: Part; chosen: number } {
  return JSON.parse(glue.addNode(JSON.stringify(part), index));
}

/** The handles a node takes when it is made smooth from nothing: coming, then leaving. */
export function smoothHandles(part: Part, index: number): [Point, Point] | null {
  return JSON.parse(glue.smoothHandles(JSON.stringify(part), index));
}

export type NewPart = "oval" | "line";
export type NewInside = "teeth" | "lowerTeeth" | "tongue" | "tongueLight" | "shine";

export function newPart(kind: NewPart): Part {
  return JSON.parse(glue.newPart(kind));
}

/** A new part to stand inside `outer`, sized to it. */
export function newInside(kind: NewInside, outer: Part): Part {
  return JSON.parse(glue.newInside(kind, JSON.stringify(outer)));
}

/** Parts reflected across the screen's middle, as a mirrored side is drawn. */
export function mirrorParts(parts: Part[]): Part[] {
  return JSON.parse(glue.mirrorParts(JSON.stringify(parts)));
}

/** A sound's loudness sampled for a preview, in dBFS a frame. */
export function sampleLoudness(seed: number, frames: number): number[] {
  return Array.from(glue.sampleLoudness(seed, frames));
}

/** A sound to speak: its loudness in dBFS a frame, the timed text it says where known, its seed, and the faces it moves to at frames of the sound. */
export interface Speech {
  loudness: number[];
  cues?: { from: number; to: number; text: string }[];
  seed: number;
  faces?: { at: number; face: string }[];
}

/**
 * The face from frame to frame: told the document, what BMO is doing, a sound being spoken and where the
 * eyes look, and asked the drawing each frame of the held clock shows and the next frame it may change.
 */
export class FaceScene {
  private readonly scene: glue.FaceScene;

  constructor(document: Omit<FaceDocument, "savedAt">, seed: number, frame: number) {
    this.scene = new glue.FaceScene(JSON.stringify(document), seed, frame);
  }

  setDocument(document: Omit<FaceDocument, "savedAt">): void {
    this.scene.setDocument(JSON.stringify(document));
  }

  /** Expression changes as cuts alone, for a viewer who asks for less motion. */
  setCalm(calm: boolean): void {
    this.scene.setCalm(calm);
  }

  setBlinking(blinking: boolean): void {
    this.scene.setBlinking(blinking);
  }

  /** A face shown whatever else is heard; null lets what is heard choose again. */
  pin(face: string | null): void {
    this.scene.pin(face);
  }

  presence(state: string, since: number): void {
    this.scene.presence(state, since);
  }

  /** A sound to speak, first heard at frame `start`, showing `face`. */
  speak(speech: Speech, face: string, start: number): void {
    this.scene.speak(JSON.stringify(speech), face, start);
  }

  stop(frame: number): void {
    this.scene.stop(frame);
  }

  /** Where the eyes look from a frame on: −1 to 1 each way. */
  gaze(x: number, y: number, frame: number): void {
    this.scene.gaze(x, y, frame);
  }

  /** The name of the face that shows at a frame. */
  faceAt(frame: number): string {
    return this.scene.faceAt(frame);
  }

  /** The drawing a frame shows, as the core writes it; the same text is the same drawing. */
  drawingText(frame: number): string {
    return this.scene.drawing(frame);
  }

  drawing(frame: number): Drawing {
    return JSON.parse(this.drawingText(frame));
  }

  /** The next frame after this one where the drawing may change; null until something is told. */
  nextChange(frame: number): number | null {
    return this.scene.nextChange(frame) ?? null;
  }

  free(): void {
    this.scene.free();
  }
}

/** The frame standing `ms` milliseconds after frame nought, and the moment a frame begins. */
export function frameAt(ms: number): number {
  return Math.floor((ms * FACTS.fps) / 1000 + 1e-6);
}

export function msAt(frame: number): number {
  return (frame * 1000) / FACTS.fps;
}
