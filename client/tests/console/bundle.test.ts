import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { LAYER_VERSION } from "bmo-ui/shared/console-manifest";

import { CLIENT, run, startServer } from "../support/server";

import type { FaceServer } from "../support/server";

// The console bundle as a deployment builds it (docs/face.md, 「The console screen」), served by the
// face's own binary: the console face the server builds as it starts takes it, and it names the layer
// and the words its manifest needs. The manifest's reading, the files' paths and the guard are held by
// the console face's own tests and the server's; the shared libraries left out, by bmo-ui's bundle build.

const scratch = mkdtempSync(join(tmpdir(), "bmo-face-console-"));
const consoleDir = join(scratch, "console");
const TOKEN = "console-token";
const credential = { authorization: `Bearer ${TOKEN}` };

let server: FaceServer | null = null;
let base = "";

beforeAll(async () => {
  // The bundle, built into the scratch folder as a deployment builds it into the release.
  run(["bun", "scripts/console.ts"], CLIENT, { BMO_FACE_OUTPUT_DIR: scratch });

  mkdirSync(join(scratch, "public"));
  writeFileSync(join(scratch, "public/index.html"), "<!doctype html><title>face</title>");

  server = await startServer({ scratch, screen: join(scratch, "public"), console: consoleDir, token: TOKEN });
  base = server.base;
}, 600_000);

afterAll(async () => {
  await server?.stop();
  rmSync(scratch, { recursive: true, force: true });
});

async function manifest(): Promise<Record<string, unknown>> {
  const answer = await fetch(`${base}/console-manifest`, { headers: credential });

  expect(answer.status).toBe(200);

  return (await answer.json()) as Record<string, unknown>;
}

describe("the bundle as built", () => {
  test("is taken by the console face, which names the layer of the release it was built on", async () => {
    expect((await manifest()).layer).toBe(LAYER_VERSION);
  });

  test("takes the layer's island, layers, create button and select where the screens have copies", async () => {
    const module = await Bun.file(join(consoleDir, "bundle.js")).text();
    const taken = [...module.matchAll(/from ?"(bmo-ui\/[^"]+)"/g)].map((found) => found[1]);

    expect(taken).toEqual(expect.arrayContaining(["bmo-ui/app/domain/island", "bmo-ui/app/domain/layers", "bmo-ui/app/components/CreateButton.vue", "bmo-ui/app/components/ChoiceSelect.vue"]));
    expect(module).not.toContain("common.placeholder.search");
  });

  test("names its menu by a word the bundle's words hold", async () => {
    const read = await manifest();
    const answer = await fetch(`${base}${read.words as string}`, { headers: credential });
    const words = (await answer.json()) as { ko: Record<string, Record<string, unknown>> };

    expect(Object.keys(words)).toEqual(["ko"]);
    expect(Object.keys(words.ko)).toEqual([read.name as string]);
    expect(words.ko[read.name as string]?.title).toBeString();
  });
});
