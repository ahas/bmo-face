import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { Database } from "bun:sqlite";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import { LAYER_VERSION } from "bmo-ui/shared/console-manifest";

import type { Subprocess } from "bun";

// The console bundle as a deployment builds it (docs/face.md, 「The console screen」), served by the
// face's own binary: the console face the server builds as it starts takes it, and it names the layer
// and the words its manifest needs. The manifest's reading, the files' paths and the guard are held by
// the console face's own tests and the server's; the shared libraries left out, by bmo-ui's bundle build.

const client = resolve(import.meta.dir, "../..");
const repository = resolve(client, "..");
const scratch = mkdtempSync(join(tmpdir(), "bmo-face-console-"));
const consoleDir = join(scratch, "console");
const TOKEN = "console-token";
const credential = { authorization: `Bearer ${TOKEN}` };

let server: Subprocess<"ignore", "pipe", "inherit"> | null = null;
let base = "";

function run(command: string[], cwd: string, env: Record<string, string> = {}): string {
  const done = Bun.spawnSync(command, { cwd, env: { ...process.env, ...env }, stdout: "pipe", stderr: "pipe" });

  if (done.exitCode !== 0) {
    throw new Error(`${command.join(" ")} failed: ${done.stderr.toString()}${done.stdout.toString()}`);
  }

  return done.stdout.toString();
}

// The server's binary as Cargo builds it, wherever the build directory stands.
function serverBinary(): string {
  const said = run(["cargo", "build", "--quiet", "--locked", "--bin", "bmo-face", "--message-format=json"], repository);
  const built = said
    .split("\n")
    .filter((line) => line.startsWith("{"))
    .map((line) => JSON.parse(line) as { reason?: string; executable?: string | null; target?: { name?: string } })
    .find((message) => message.reason === "compiler-artifact" && message.target?.name === "bmo-face" && message.executable);

  if (!built?.executable) {
    throw new Error("cargo built no bmo-face binary");
  }

  return built.executable;
}

beforeAll(async () => {
  // The bundle, built into the scratch folder as a deployment builds it into the release.
  run(["bun", "scripts/console.ts"], client, { BMO_FACE_OUTPUT_DIR: scratch });
  const binary = serverBinary();

  mkdirSync(join(scratch, "data"));
  mkdirSync(join(scratch, "public"));
  writeFileSync(join(scratch, "public/index.html"), "<!doctype html><title>face</title>");
  const archive = new Database(join(scratch, "archive.db"), { create: true });

  archive.run("CREATE TABLE web_users (name TEXT PRIMARY KEY, password_hash TEXT NOT NULL)");
  archive.close();

  server = Bun.spawn([binary], {
    env: {
      ...process.env,
      BMO_FACE_DATA: join(scratch, "data"),
      BMO_FACE_PORT: "0",
      BMO_FACE_STATIC: join(scratch, "public"),
      BMO_FACE_CONSOLE: consoleDir,
      BMO_FACE_TOKEN: TOKEN,
      BMO_DB_PATH: join(scratch, "archive.db"),
      BMO_ARCHIVE_URL: "",
      BMO_CONSOLE_URL: "http://[::1]:9/bmo",
    },
    stdin: "ignore",
    stdout: "pipe",
    stderr: "inherit",
  });

  // The server names the port it chose as it starts answering.
  const decoder = new TextDecoder();
  let said = "";

  for await (const chunk of server.stdout) {
    said += decoder.decode(chunk);
    const port = /BMO face: http:\/\/127\.0\.0\.1:(\d+)\//.exec(said)?.[1];

    if (port !== undefined) {
      base = `http://127.0.0.1:${port}/bmo-face`;
      break;
    }
  }

  if (base === "") {
    throw new Error(`the server never listened; it said ${said}`);
  }
}, 600_000);

afterAll(() => {
  server?.kill();
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

  test("names its menu by a word the bundle's words hold", async () => {
    const read = await manifest();
    const answer = await fetch(`${base}${read.words as string}`, { headers: credential });
    const words = (await answer.json()) as { ko: Record<string, Record<string, unknown>> };

    expect(Object.keys(words)).toEqual(["ko"]);
    expect(Object.keys(words.ko)).toEqual([read.name as string]);
    expect(words.ko[read.name as string]?.title).toBeString();
  });
});
