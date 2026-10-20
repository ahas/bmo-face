import { checkFaceDocument, DEFAULT_FACES, ovalNodes } from "./faces";

import type { FaceDocument, Part } from "./faces";

// The documents the server's Rust check is held to (docs/face.md, 「The expression document」): the
// default as the server writes it, and every case below with this module's own check's verdict on it.
// `bun scripts/fixtures.ts` writes them into `tests/fixtures/`; the test beside this module fails while what
// stands there is not what this module makes.

/** The expression document as the server writes it to its file. */
export const documentText = (document: unknown) => `${JSON.stringify(document, null, 2)}\n`;

const dot: Part = { nodes: ovalNodes(30, 30, 3, 3), closed: true, fill: "ink", line: null };
const stroke: Part = { nodes: [{ anchor: [40, 40], coming: [0, 0], leaving: [0, 0], kind: "corner" }, { anchor: [60, 40], coming: [0, 0], leaving: [0, 0], kind: "corner" }], closed: false, fill: null, line: { paint: "ink", width: 2 } };

/** A small document each case changes one thing of, so a case holds little beyond what it is about. */
const base = () =>
  structuredClone({
    colors: DEFAULT_FACES.colors,
    faces: { 기본: { eyes: [dot], brows: [], cheeks: [], mouth: [stroke], marks: [] }, },
    mouths: [[stroke], [dot]],
    savedAt: 0,
  }) as unknown as Record<string, unknown> & FaceDocument;

/** A copy of the default with one face's eyes, or the first mouth stage, changed. */
function withEyes(parts: unknown): unknown {
  const document = base();
  (document.faces.기본 as unknown as Record<string, unknown>).eyes = parts;
  return document;
}

const CASES: [string, () => unknown][] = [
  ["a small document", () => base()],
  ["colours in capitals are kept in small letters", () => ({ ...base(), colors: { ...DEFAULT_FACES.colors, body: "#5FBB9C", ink: "#000A00" } })],
  ["fields the check does not know are dropped, and an empty inside with them", () => withEyes([
    { ...dot, label: "왼눈", nodes: dot.nodes.map((node) => ({ ...node, weight: 2 })), inside: [] },
  ])],
  ["a group a face does not hold is an empty one", () => {
    const document = base();
    delete (document.faces.기본 as unknown as Record<string, unknown>).marks;
    (document.faces.기본 as unknown as Record<string, unknown>).brows = null;
    return document;
  }],
  ["a handle ending on the screen's edge, within the last bit of floating point", () => withEyes([
    { ...stroke, nodes: [{ anchor: [99.5, 40], coming: [0, 0], leaving: [0.5000000001, 0], kind: "corner" }, stroke.nodes[1]] },
  ])],
  ["an open mouth with a teeth band and an open divider inside", () => withEyes([
    { ...dot, line: { paint: "ink", width: 1.6 }, inside: [{ ...dot, fill: "light" }, stroke] },
  ])],
  ["a part of the old ellipse shape", () => withEyes([{ shape: "ellipse", cx: 30, cy: 30, rx: 3, ry: 3, filled: true, width: 1 }])],
  ["not a document", () => ({ colors: {}, faces: {} })],
  ["no document at all", () => null],
  ["a colour that is not #rrggbb", () => ({ ...base(), colors: { ...DEFAULT_FACES.colors, ink: "black" } })],
  ["a paint's colour missing", () => {
    const document = base();
    delete (document.colors as Record<string, unknown>).tear;
    return document;
  }],
  ["a face of a name nobody sends", () => ({ ...base(), faces: { ...base().faces, 심심: base().faces.기본 } })],
  ["no 기본 face", () => {
    const document = base();
    delete (document.faces as Record<string, unknown>).기본;
    return document;
  }],
  ["a face that is no face", () => ({ ...base(), faces: { ...base().faces, 기쁨: [] } })],
  ["too few mouth stages", () => ({ ...base(), mouths: [[stroke]] })],
  ["too many mouth stages", () => ({ ...base(), mouths: Array.from({ length: 9 }, () => [stroke]) })],
  ["a mouth stage that is no list", () => ({ ...base(), mouths: [[stroke], { parts: [] }] })],
  ["more parts in a group than it may hold", () => withEyes(Array.from({ length: 17 }, () => dot))],
  ["a part that is no part", () => withEyes(["dot"])],
  ["a part not saying whether it is closed", () => withEyes([{ ...dot, closed: "yes" }])],
  ["one node", () => withEyes([{ ...stroke, nodes: [stroke.nodes[0]] }])],
  ["too many nodes", () => withEyes([{ ...stroke, nodes: Array.from({ length: 33 }, () => stroke.nodes[0]) }])],
  ["a node that is not an anchor and two handles", () => withEyes([{ ...stroke, nodes: [stroke.nodes[0], { anchor: [1, 2], coming: [0, 0], kind: "corner" }] }])],
  ["a node of a kind the drawing does not know", () => withEyes([{ ...stroke, nodes: [stroke.nodes[0], { ...stroke.nodes[1], kind: "sharp" }] }])],
  ["an anchor outside the screen", () => withEyes([{ ...stroke, nodes: [stroke.nodes[0], { ...stroke.nodes[1], anchor: [101, 40] }] }])],
  ["a handle whose end falls outside the screen, though its anchor is on it", () => withEyes([{ ...stroke, nodes: [stroke.nodes[0], { ...stroke.nodes[1], anchor: [60, 74], leaving: [0, 2] }] }])],
  ["a fill of a paint the document does not hold", () => withEyes([{ ...dot, fill: "gold" }])],
  ["a part saying nothing of its fill", () => {
    const { fill: _fill, ...rest } = dot;
    return withEyes([rest]);
  }],
  ["a part saying nothing of its line", () => {
    const { line: _line, ...rest } = dot;
    return withEyes([rest]);
  }],
  ["a line of a paint the document does not hold", () => withEyes([{ ...stroke, line: { paint: "gold", width: 2 } }])],
  ["a line too thin", () => withEyes([{ ...stroke, line: { paint: "ink", width: 0.1 } }])],
  ["a line too thick", () => withEyes([{ ...stroke, line: { paint: "ink", width: 10.5 } }])],
  ["a part of neither a fill nor a line", () => withEyes([{ ...dot, fill: null }])],
  ["an open part that is filled", () => withEyes([{ ...stroke, fill: "ink" }])],
  ["parts inside a part inside", () => withEyes([{ ...dot, inside: [{ ...dot, inside: [dot] }] }])],
  ["parts inside an open part", () => withEyes([{ ...stroke, inside: [dot] }])],
  ["more parts inside one than it may hold", () => withEyes([{ ...dot, inside: Array.from({ length: 9 }, () => dot) }])],
  ["an inside that is no list", () => withEyes([{ ...dot, inside: null }])],
  ["a part inside that is refused", () => withEyes([{ ...dot, inside: [{ ...stroke, fill: "ink" }] }])],
];

/** Each case with the document checked and the check's verdict on it. */
export function fixtureCases(): { name: string; document: unknown; verdict: unknown }[] {
  return CASES.map(([name, make]) => {
    const document = make();

    return { name, document, verdict: checkFaceDocument(document) };
  });
}

/** Every fixture file by its name under `tests/fixtures/`, as it should stand. */
export function fixtureFiles(): Record<string, string> {
  return {
    "default-faces.json": documentText(DEFAULT_FACES),
    // One case to a line: the cases are many and each is small, so a line of each reads best.
    "checks.json": `[\n${fixtureCases().map((one) => JSON.stringify(one)).join(",\n")}\n]\n`,
  };
}
