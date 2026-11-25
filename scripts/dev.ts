// The development bench: the face's server on 127.0.0.1:3218 against the development copy of the
// data root and BMO's development console, and the screen's Nuxt dev server on 127.0.0.1:3219, which
// passes the server's routes through to it (docs/development.md). When either ends, the other is
// ended too, so whatever runs this starts both again together.
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";

import { devDataRoot, FACES_FILE, realDataRoot, refreshDevData } from "../src/dev-data";

const root = resolve(import.meta.dir, "..");
const API_PORT = 3218;
const SCREEN_PORT = 3219;

if (!existsSync(join(devDataRoot, FACES_FILE))) {
  const report = refreshDevData(realDataRoot, devDataRoot);

  console.log(`dev: made the development copy at ${devDataRoot} from the ${report.document} document`);
}

const env = {
  ...process.env,
  BMO_FACE_DATA: devDataRoot,
  // A data root holding no env/face.env, so the live install's settings never reach the bench.
  BMO_DATA_DIR: devDataRoot,
  BMO_FACE_PORT: String(API_PORT),
  BMO_DB_PATH: process.env.BMO_DB_PATH ?? join(homedir(), ".local/share/bmo/data/workbench/worker/bmo.db"),
  BMO_CONSOLE_URL: process.env.BMO_CONSOLE_URL ?? "http://127.0.0.1:3211/bmo-dev",
  BMO_FACE_DEV_API: `http://127.0.0.1:${API_PORT}`,
  // The console bundle last built in the checkout; Cargo's binary stands under target/, not beside it.
  BMO_FACE_CONSOLE: process.env.BMO_FACE_CONSOLE ?? join(root, "client/.output/console"),
};

// The server built and started by Cargo; a change to it is taken up by starting the bench again.
const server = Bun.spawn(["cargo", "run", "--locked", "--bin", "bmo-face"], { cwd: root, env, stdout: "inherit", stderr: "inherit" });
const screen = Bun.spawn(["bun", "--bun", "nuxi", "dev", "--host", "127.0.0.1", "--port", String(SCREEN_PORT)], {
  cwd: join(root, "client"),
  env,
  stdout: "inherit",
  stderr: "inherit",
});

const stop = () => {
  server.kill();
  screen.kill();
};

process.on("SIGINT", stop);
process.on("SIGTERM", stop);

const first = await Promise.race([server.exited.then((code) => ["server", code] as const), screen.exited.then((code) => ["screen", code] as const)]);

console.error(`dev: the ${first[0]} ended (${first[1]}); ending the other`);
stop();
await Promise.all([server.exited, screen.exited]);
process.exit(first[1] === 0 ? 1 : first[1]);
