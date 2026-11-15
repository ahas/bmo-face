// Makes the core's WebAssembly build into `src/wasm/` (docs/face.md, 「The face's core」): the core crate
// compiled with its `wasm` feature under the workspace's `wasm` profile, its binding's glue and types
// written by `wasm-bindgen`, its bytes written into a JavaScript module `src/core.ts` instantiates on
// import, and the hash of the sources the build was made from. Run after the core changes; the test
// beside `src/core.ts` says when it is due.

import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative } from "node:path";

const root = join(import.meta.dir, "..");
const out = join(root, "src", "wasm");

/** The file the hash of the core's sources is kept in, beside the build made from them. */
export const sourceHashFile = join(out, "sources.sha256");

function filesUnder(folder: string): string[] {
  return readdirSync(join(root, folder), { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => relative(root, join(entry.parentPath, entry.name)));
}

/** What the build reads: the workspace's manifest and lockfile, and the core's manifest and sources. */
function sources(): string[] {
  return ["Cargo.toml", "Cargo.lock", "core/Cargo.toml", ...filesUnder("core/src")].sort();
}

/** The hash of what the build reads, each file by its path and its bytes. */
export function sourceHash(): string {
  const hash = createHash("sha256");

  for (const path of sources()) {
    hash.update(`${path}\0`);
    hash.update(readFileSync(join(root, path)));
    hash.update("\0");
  }

  return hash.digest("hex");
}

function run(command: string[]): string {
  const done = Bun.spawnSync(command, { cwd: root, stdout: "pipe", stderr: "inherit" });

  if (done.exitCode !== 0) {
    throw new Error(`${command.join(" ")} failed with exit code ${done.exitCode}.`);
  }

  return done.stdout.toString();
}

function build(): void {
  run(["cargo", "rustc", "-p", "bmo-face-core", "--lib", "--profile", "wasm", "--target", "wasm32-unknown-unknown", "--features", "wasm", "--crate-type", "cdylib"]);

  const target: string = JSON.parse(run(["cargo", "metadata", "--format-version", "1", "--no-deps"])).target_directory;
  const made = mkdtempSync(join(tmpdir(), "bmo-face-core-wasm-"));

  try {
    run(["wasm-bindgen", join(target, "wasm32-unknown-unknown", "wasm", "bmo_face_core.wasm"), "--out-dir", made, "--target", "web", "--omit-default-module-path"]);

    rmSync(out, { recursive: true, force: true });
    mkdirSync(out);
    copyFileSync(join(made, "bmo_face_core.js"), join(out, "bmo_face_core.js"));
    copyFileSync(join(made, "bmo_face_core.d.ts"), join(out, "bmo_face_core.d.ts"));

    const bytes = readFileSync(join(made, "bmo_face_core_bg.wasm")).toString("base64");

    writeFileSync(join(out, "bytes.js"), `// The core's WebAssembly module, written by scripts/core.ts.\nexport const bytes = "${bytes}";\n`);
    writeFileSync(join(out, "bytes.d.ts"), "// The core's WebAssembly module, in base64.\nexport declare const bytes: string;\n");
    writeFileSync(sourceHashFile, `${sourceHash()}\n`);
  } finally {
    rmSync(made, { recursive: true, force: true });
  }
}

if (import.meta.main) {
  build();
  console.log(`core: built into ${out}`);
}
