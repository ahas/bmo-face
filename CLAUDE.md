# Working in this repository

This repository is BMO's face, `bmo-face`: the face's core, the display, the editor, the server behind
both and the `face/expression` tool entry, and the editor again as a console bundle. It knows no other unit's code.
BMO's accounts are read from its archive at the address a setting names, and its listener routes are
reached at the console's address; nothing of BMO's is imported.

## Rules

- **`docs/` is canonical.** Read the document covering a change before changing code. A change that
  would make it wrong or incomplete repairs it first, stating what is true now, and the code is then
  written to match. `docs/face.md` states the display, the held clock and its drivers, the expression
  document, the face's core and its interface, the native displayer, the editor and every route, the console screen among
  them; `docs/expressions.md` the design the face is drawn from; `docs/deploy.md` states the release, the unit, the tool
  entry and the native displayer's system unit.
- **`.graph/` is written only with `bmo graph`.** It holds the repository's feature nodes and the
  boundary nodes for the APIs it exposes and calls; no other command and no hand edit changes it, and
  a change to code brings the node it touches up to date in the checkout. Its area files are generated
  and untracked, so a fresh checkout builds them again, while `settings.toml` is the repository's
  configuration and stays tracked. The format is `docs/graph.md` in the bmo-graph repository.
- **Two base libraries, each for the console screen alone.** bmo-ui's release, the one bmo-chat's
  `shared-versions.toml` pins, is vendored under `client/vendor/` and named in the screen's manifest;
  the bundle is built on it, while the standalone screens do not extend it. bmo-chat's
  `bmo-console-face` crate serves the console face from the server, and the server's test reads the
  manifest it serves through that crate's reading. BMO is reached over the address a setting names.
  The testbed's dependency sweep fails on anything else.
- **English in code, comments, tests, documents and commit messages.** Every word a person reads on
  the screen stands in `client/i18n/locales/ko.yaml`, named by key. The Korean the code still holds is
  the debt `korean-ledger.json` records file by file; a file it does not name is allowed none, and the
  ledger only shrinks.
- **No personal data in code, comments, documents, test fixtures or commit messages.**
  `docs/PERSONAL-DATA.md` states what that forbids, the values it allows and where a value belongs; a
  test's data is a neutral name and value, and `tests/personal-data.test.ts` is the pattern check.
- **Tests live in `tests/`:** the core's in `core/tests/`, the server's integration tests in
  `server/tests/`, the native displayer's in `displayer/tests/`, the screen's in `client/tests/`, and the expression document's statement's, the
  core's WebAssembly build's and the development copy's in the root `tests/`, with their support under
  `tests/support/`. A unit test of a private helper stands beside it
  in the crate.
- **One Cargo workspace at the root**, whose members are `core/`, `displayer/` and `server/`. The root Bun project
  holds `src/`, `scripts/` and `tests/`; `client/` is a Bun project of its own, with its own manifest and
  lockfile.
- **The face is drawn once, in the core.** The geometry, the blend, the drivers and the document's
  check stand in `core/` alone; the screen draws what its WebAssembly build answers and keeps only the
  SVG, the sound's decoding and the editor's handling. After a change under `core/`, `bun
  scripts/core.ts` builds `src/wasm/` again, which needs the `wasm32-unknown-unknown` target and the
  `wasm-bindgen` command at the version the core locks; the root's test fails until it is built.
- **Documents state what is true now**: no change history, no quoted source and no file-and-line pins.

## Checks

A change moves through three stages in order:

1. Writing: `cargo check --workspace --all-targets`, and `bun run check:types` for the expression
   document and the scripts.
2. Review: read the whole diff against what the change is for and the document governing it, then
   `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` (108 columns,
   2-space indentation, as `rustfmt.toml` sets).
3. Verification: the test targets the change touched, then the suite `testbed.toml` declares once.

The suite runs the crates' tests, the root Bun test files, and the screen's check script, which is its
typecheck and its domain tests. The version is the repository's, and `VERSION` holds it. It moves only at a release the owner declares, as bmo-chat's `docs/conventions.md` 「Versions」 states; a commit, a deploy, a fix or a contract change moves none, and a deployment is told apart by its commit.
