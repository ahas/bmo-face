import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { Database } from "bun:sqlite";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import { consoleManifest } from "bmo-ui/shared/console-manifest";

import type { Subprocess } from "bun";

// The console screen as a console meets it (docs/face.md, 「The console screen」): the bundle built
// as a deployment builds it, served by the face's own binary, its manifest read through bmo-ui's
// reading and its module through the guard.

const client = resolve(import.meta.dir, "../..");
const repository = resolve(client, "..");
const scratch = mkdtempSync(join(tmpdir(), "bmo-face-console-"));
const consoleDir = join(scratch, "console");
const TOKEN = "console-token";
const credential = { authorization: `Bearer ${TOKEN}` };
const reader = { ...credential, "owner-role": "member" };

// What a console shares with every bundle, which the module may leave unbundled.
const SHARED = /^(?:vue|vue-router|vue-i18n|nuxt\/app|@nuxt\/ui|bmo-ui)(?:\/|$)/;

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

describe("the manifest", () => {
  test("keeps the contract, read through bmo-ui's own reading", async () => {
    const read = consoleManifest(await manifest());

    expect(read).toMatchObject({
      name: "face",
      title: "face.title",
      screenPath: "/face",
      serviceName: "face",
      apiPrefix: "/console",
      role: "member",
      ownerKey: false,
      events: false,
      websocket: false,
      pushKinds: [],
      calls: [],
      words: "/console/words",
    });
    expect(read.bundle).toMatch(/^\/console\/bundle\.[0-9a-f]{16}\.js$/);
    expect(read.stylesheet).toMatch(/^\/console\/bundle\.[0-9a-f]{16}\.css$/);
  });

  test("is answered to the console's credential alone", async () => {
    expect((await fetch(`${base}/console-manifest`)).status).toBe(401);
    expect((await fetch(`${base}/console-manifest`, { headers: { authorization: "Bearer other" } })).status).toBe(401);
  });

  test("names its menu by a word the bundle's words hold", async () => {
    const read = consoleManifest(await manifest());
    const answer = await fetch(`${base}${read.words}`, { headers: credential });
    const words = (await answer.json()) as { ko: Record<string, Record<string, unknown>> };

    expect(Object.keys(words)).toEqual(["ko"]);
    expect(Object.keys(words.ko)).toEqual([read.name]);
    expect(words.ko[read.name]?.title).toBeString();
  });
});

describe("the module", () => {
  test("is answered to a reader the manifest's role allows, and refused otherwise", async () => {
    const path = `${base}${consoleManifest(await manifest()).bundle}`;

    expect((await fetch(path)).status).toBe(401);
    expect((await fetch(path, { headers: credential })).status).toBe(403);
    expect((await fetch(path, { headers: { ...credential, "owner-role": "guest" } })).status).toBe(403);
    expect((await fetch(path, { headers: { ...credential, "owner-role": "administrator" } })).status).toBe(200);

    const answer = await fetch(path, { headers: reader });

    expect(answer.status).toBe(200);
    expect(answer.headers.get("content-type")).toStartWith("text/javascript");
  });

  test("leaves only what a console shares unbundled, and exports the routes", async () => {
    const source = await (await fetch(`${base}${consoleManifest(await manifest()).bundle}`, { headers: reader })).text();
    const scanned = new Bun.Transpiler({ loader: "js" }).scan(source);

    expect(scanned.imports.length).toBeGreaterThan(0);
    expect(scanned.imports.filter((each) => !SHARED.test(each.path))).toEqual([]);
    expect(scanned.exports).toEqual(["default"]);
  });

  test("is answered at a path that changes with its bytes", async () => {
    const first = consoleManifest(await manifest()).bundle;
    const module = join(consoleDir, "bundle.js");

    writeFileSync(module, `${readFileSync(module, "utf8")}\n`);

    const second = consoleManifest(await manifest()).bundle;

    expect(second).not.toBe(first);
    expect((await fetch(`${base}${first}`, { headers: reader })).status).toBe(404);
    expect((await fetch(`${base}${second}`, { headers: reader })).status).toBe(200);
  });
});
