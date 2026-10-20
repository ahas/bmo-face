import { expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { refreshDevData } from "../src/dev-data";
import { DEFAULT_FACES } from "../src/faces";
import { scratchDir } from "./support/scratch";

// The copy of the data root the bench works against (docs/development.md).
test("the copy holds the live document", () => {
  const root = join(scratchDir("bmo-face-real-"), "data");

  mkdirSync(root);
  writeFileSync(join(root, "faces.json"), '{"savedAt":7}');

  const copy = join(scratchDir("bmo-face-copy-"), "data");

  expect(refreshDevData(root, copy)).toEqual({ document: "live" });
  expect(readFileSync(join(copy, "faces.json"), "utf8")).toBe('{"savedAt":7}');
});

test("with no live document yet, the copy starts from the default", () => {
  const copy = join(scratchDir("bmo-face-copy-"), "data");

  expect(refreshDevData(join(scratchDir("bmo-face-none-"), "data"), copy)).toEqual({ document: "default" });
  expect(JSON.parse(readFileSync(join(copy, "faces.json"), "utf8"))).toEqual(DEFAULT_FACES);
});

test("the copy may not stand inside the data root it copies", () => {
  const root = scratchDir("bmo-face-real-");

  expect(() => refreshDevData(root, join(root, "copy"))).toThrow("inside the data root");
});
