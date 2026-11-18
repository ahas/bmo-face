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
     * A sound to speak, first heard at `start`: `{ "heard": [what `hear` answers], "cues": [{ "from",
     * "to", "text" }], "faces": [{ "at", "face" }] }`, showing `face`.
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
 * What a caller needs to know of the document, the clock and the hearing: the form, the screen, the
 * limits, the names, the paints, the eyes' answer to the voice, the tuning's declaration, the longest a
 * rest stage may wait, the frame rate, the lead and the spectrum's bands.
 */
export function facts(): string;

/**
 * What the face hears of a sound, one channel of samples at `rate` a second: a reading a frame.
 */
export function hear(samples: Float32Array, rate: number): string;

/**
 * Where a drawing stands among the talking set's, measured from itself: `{ "at": [open, wide], "teeth" }`,
 * or null for a drawing of nothing.
 */
export function measureAnchor(parts: string): string;

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
 * What the face does at each frame of a sound under a tuning, for the editor to draw beneath its
 * preview: `[{ "voice", "open", "round", "teeth", "tongue", "grow", "squint", "shine" }]`.
 */
export function poses(heard: string, tuning: string): string;

/**
 * The voice sampled for previews, at `rate` samples a second.
 */
export function sampleVoice(rate: number): Float32Array;

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
    readonly hear: (a: number, b: number, c: number) => [number, number];
    readonly measureAnchor: (a: number, b: number) => [number, number, number, number];
    readonly migrateDocument: (a: number, b: number) => [number, number, number, number];
    readonly mirrorParts: (a: number, b: number) => [number, number, number, number];
    readonly newInside: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly newPart: (a: number, b: number) => [number, number, number, number];
    readonly partsDrawing: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly poses: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly sampleVoice: (a: number) => [number, number];
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
