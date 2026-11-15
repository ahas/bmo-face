import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { FaceScene, FACTS } from "../src/core";
import { checkFaceDocument, DEFAULT_FACES, EMOTIONS, FACE_NAMES, faceFor, FALLBACK_FACE, PAINTS, SIZES, STATES } from "../src/faces";
import { sourceHash } from "../scripts/core";

// The expression document as the screen holds it, and the core it hands on (docs/face.md, 「The face's
// core」). What the core answers is held by the core's own tests; these hold the screen's statement of the
// document to the core's, and the core's WebAssembly build to the sources it was made from.

describe("the core's WebAssembly build", () => {
  test("is made from the core's sources as they stand", () => {
    const standing = readFileSync(join(import.meta.dir, "..", "src", "wasm", "sources.sha256"), "utf8").trim();

    expect(standing === sourceHash(), "src/wasm/ is not built from the core as it stands; run bun scripts/core.ts").toBe(true);
  });
});

describe("the document's names", () => {
  test("are the core's: the faces, the fallback, the paints and the sizes", () => {
    expect<string[]>([...EMOTIONS]).toEqual(FACTS.emotions);
    expect<string[]>([...STATES]).toEqual(FACTS.states);
    expect<string>(FALLBACK_FACE).toBe(FACTS.fallback);
    expect<string[]>([...PAINTS]).toEqual(FACTS.paints);
    expect<string[]>([...SIZES]).toEqual(FACTS.sizes);
  });
});

describe("the default document", () => {
  test("holds a face for every name, passes the check as it stands and starts unsaved", () => {
    expect(Object.keys(DEFAULT_FACES.faces)).toEqual([...FACE_NAMES]);
    expect(DEFAULT_FACES.savedAt).toBe(0);

    const { savedAt: _version, ...unversioned } = DEFAULT_FACES;

    expect(checkFaceDocument(DEFAULT_FACES)).toEqual({ ok: unversioned });
  });

  test("falls back to the fallback face for a face it lacks", () => {
    const { idle: _gone, ...rest } = DEFAULT_FACES.faces;

    expect(faceFor({ ...DEFAULT_FACES, faces: rest }, "idle")).toBe(DEFAULT_FACES.faces[FALLBACK_FACE]!);
  });
});

describe("a refusal", () => {
  test("comes back as a code and the place it was met at", () => {
    const document = structuredClone(DEFAULT_FACES);

    document.faces[FALLBACK_FACE]!.mouth[0]!.line!.width = 20;

    expect(checkFaceDocument(document)).toEqual({
      refusal: { code: "line-width", at: [{ face: FALLBACK_FACE }, { element: "mouth" }, { part: 1 }], least: 0.2, most: 10 },
    });
  });
});

describe("the scene", () => {
  test("hands the same drawings to the same seed and events, held between the frames it names", () => {
    const run = () => {
      const scene = new FaceScene(DEFAULT_FACES, 5, 0);

      scene.speak({ loudness: Array.from({ length: 96 }, (_, frame) => (frame % 12 < 6 ? -30 : -80)), seed: 5 }, EMOTIONS[1], 10);

      return Array.from({ length: 140 }, (_, frame) => scene.drawingText(frame));
    };
    const drawings = run();

    expect(run()).toEqual(drawings);
    expect(new Set(drawings).size).toBeGreaterThan(10);
    expect(JSON.parse(drawings[50]!).paint.length).toBeGreaterThan(0);
  });
});
