import { afterAll } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

// A directory a test may write in, removed once the file that asked for it is done, whatever
// became of its tests. Only a test calls this: the removal is registered with the test runner.
export function scratchDir(prefix: string): string {
  const dir = mkdtempSync(join(tmpdir(), prefix));

  afterAll(() => {
    rmSync(dir, { recursive: true, force: true });
  });

  return dir;
}
