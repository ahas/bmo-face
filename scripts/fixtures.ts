// Writes the documents the server's Rust check is held to into `tests/fixtures/`, from the expression
// document's own default and check (docs/face.md, 「The expression document」). Run after the
// default or the check changes; the test beside `src/fixtures.ts` says when it is due.
import { mkdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { fixtureFiles } from "../src/fixtures";

const folder = resolve(import.meta.dir, "..", "tests", "fixtures");

mkdirSync(folder, { recursive: true });

for (const [name, text] of Object.entries(fixtureFiles())) {
  writeFileSync(join(folder, name), text);
  console.log(`fixtures: ${join(folder, name)}`);
}
