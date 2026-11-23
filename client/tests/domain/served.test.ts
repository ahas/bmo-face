import { describe, expect, test } from "bun:test";

import { DEFAULT_FACES } from "#faces";

import { keptDocument, shouldReload } from "../../app/domain/served";

// What a display takes of what the server serves (docs/face.md, 「Showing on any screen」).
describe("a display reloads", () => {
  test("when the served build is not the one it runs", () => {
    expect(shouldReload("38221513", { id: "5c0ffee1", timestamp: 1 })).toBe(true);
  });

  test("never when the served build is its own, as on the bench", () => {
    expect(shouldReload("dev", { id: "dev", timestamp: 1 })).toBe(false);
  });

  test("never when the manifest names no build, or is not there to read", () => {
    expect(shouldReload("dev", {})).toBe(false);
    expect(shouldReload("dev", { id: "" })).toBe(false);
    expect(shouldReload("dev", null)).toBe(false);
    expect(shouldReload("dev", "<!doctype html>")).toBe(false);
  });
});

describe("the document a display draws", () => {
  const saved = { ...DEFAULT_FACES, savedAt: 7 };

  test("is what arrived when its own check passes it", () => {
    expect(keptDocument(null, saved)).toEqual(saved);
    expect(keptDocument(DEFAULT_FACES, saved)).toEqual(saved);
  });

  test("stays the last good one when what arrived is a shape it does not know", () => {
    const old = { ...saved, faces: { neutral: { eyes: [{ shape: "ellipse", cx: 33, cy: 30, rx: 3, ry: 3, filled: true, width: 1 }] } } };

    expect(keptDocument(DEFAULT_FACES, old)).toBe(DEFAULT_FACES);
    expect(keptDocument(null, old)).toBeNull();
  });

  test("stays the last good one when what arrived names no version", () => {
    const { savedAt: _gone, ...unversioned } = saved;

    expect(keptDocument(DEFAULT_FACES, unversioned)).toBe(DEFAULT_FACES);
  });
});
