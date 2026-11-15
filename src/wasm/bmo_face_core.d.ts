/* tslint:disable */
/* eslint-disable */

/**
 * The face from frame to frame, as the display and the editor's preview hold it.
 */
export class FaceScene {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The drawing a frame shows, as JSON.
     */
    drawing(frame: number): string;
    /**
     * The name of the face that shows at a frame.
     */
    faceAt(frame: number): string;
    gaze(x: number, y: number, frame: number): void;
    /**
     * A scene of a document, its chance drawn from `seed`, its presence idle from `frame`.
     */
    constructor(document: string, seed: number, frame: number);
    /**
     * The next frame after this one where the drawing may change; none until something is told.
     */
    nextChange(frame: number): number | undefined;
    /**
     * A face shown whatever else is heard; none lets what is heard choose again.
     */
    pin(face?: string | null): void;
    presence(state: string, since: number): void;
    setBlinking(blinking: boolean): void;
    setCalm(calm: boolean): void;
    setDocument(document: string): void;
    /**
     * A sound to speak, first heard at `start`: `{ "loudness": [dBFS a frame], "cues": [{ "from", "to",
     * "text" }], "seed", "faces": [{ "at", "face" }] }`, showing `face`.
     */
    speak(speech: string, face: string, start: number): void;
    stop(frame: number): void;
}

/**
 * A node added after the one at `index`, the drawing unchanged: `{ "part": …, "chosen": index }`.
 */
export function addNode(part: string, index: number): string;

/**
 * The check's verdict on a document: `{ "ok": document }` as it may be kept, or `{ "refusal": … }`.
 */
export function checkDocument(text: string): string;

/**
 * The default document, its version nought.
 */
export function defaultDocument(): string;

/**
 * What a caller needs to know of the document and the clock: the form, the screen, the limits, the
 * names, the paints, the sizes and the sounds, the frame rate and the lead.
 */
export function facts(): string;

/**
 * A document of an older form carried into this one, unchecked.
 */
export function migrateDocument(text: string): string;

/**
 * Parts reflected across the screen's middle, as a mirrored side is drawn.
 */
export function mirrorParts(parts: string): string;

/**
 * A new part to stand inside `outer`: `teeth`, `lowerTeeth`, `tongue`, `tongueLight` or `shine`.
 */
export function newInside(kind: string, outer: string): string;

/**
 * A new part of the editor's own: `oval` or `line`.
 */
export function newPart(kind: string): string;

/**
 * Parts drawn in order on a document's colours, as the editor's canvas draws what it edits.
 */
export function partsDrawing(parts: string, colors: string): string;

/**
 * A sound's loudness sampled for a preview, in dBFS a frame.
 */
export function sampleLoudness(seed: number, frames: number): Float64Array;

/**
 * The handles a node takes made smooth from nothing, coming then leaving; null for no such node.
 */
export function smoothHandles(part: string, index: number): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_facescene_free: (a: number, b: number) => void;
    readonly addNode: (a: number, b: number, c: number) => [number, number, number, number];
    readonly checkDocument: (a: number, b: number) => [number, number, number, number];
    readonly defaultDocument: () => [number, number];
    readonly facescene_drawing: (a: number, b: number) => [number, number];
    readonly facescene_faceAt: (a: number, b: number) => [number, number];
    readonly facescene_gaze: (a: number, b: number, c: number, d: number) => void;
    readonly facescene_new: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly facescene_nextChange: (a: number, b: number) => [number, number];
    readonly facescene_pin: (a: number, b: number, c: number) => void;
    readonly facescene_presence: (a: number, b: number, c: number, d: number) => void;
    readonly facescene_setBlinking: (a: number, b: number) => void;
    readonly facescene_setCalm: (a: number, b: number) => void;
    readonly facescene_setDocument: (a: number, b: number, c: number) => [number, number];
    readonly facescene_speak: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly facescene_stop: (a: number, b: number) => void;
    readonly facts: () => [number, number];
    readonly migrateDocument: (a: number, b: number) => [number, number, number, number];
    readonly mirrorParts: (a: number, b: number) => [number, number, number, number];
    readonly newInside: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly newPart: (a: number, b: number) => [number, number, number, number];
    readonly partsDrawing: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly sampleLoudness: (a: number, b: number) => [number, number];
    readonly smoothHandles: (a: number, b: number, c: number) => [number, number, number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
