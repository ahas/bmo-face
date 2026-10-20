import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { fixtureCases, fixtureFiles } from "../src/fixtures";

// The documents the server's Rust check is held to are this project's check's own verdicts, so the
// two checks accept and refuse the same documents (docs/face.md, 「The expression document」).

describe("the shared fixtures", () => {
  test("stand as the expression document's default and check make them", () => {
    for (const [name, text] of Object.entries(fixtureFiles())) {
      const standing = readFileSync(join(import.meta.dir, "fixtures", name), "utf8");

      expect(standing === text, `tests/fixtures/${name} is not what the document makes; run bun scripts/fixtures.ts`).toBe(true);
    }
  });

  test("hold accepted and refused documents both", () => {
    const verdicts = fixtureCases().map((one) => ("ok" in (one.verdict as object) ? "ok" : "error"));

    expect(verdicts.filter((one) => one === "ok").length).toBeGreaterThan(4);
    expect(verdicts.filter((one) => one === "error").length).toBeGreaterThan(20);
  });
});
