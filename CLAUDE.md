# Working in this repository

This repository is BMO's face, `bmo-face`: the display, the editor, the server behind both and the
`face/expression` tool entry. It knows no other unit's code. BMO's accounts are read from its archive
at the address a setting names, and its listener routes are reached at the console's address; nothing
of BMO's is imported.

## Rules

- **`docs/` is canonical.** Read the document covering a change before changing code. A change that
  would make it wrong or incomplete repairs it first, stating what is true now, and the code is then
  written to match. `docs/face.md` states the display, the expression document, the editor and every
  route; `docs/deploy.md` states the release, the unit and the tool entry.
- **No dependency from another repository.** The base libraries are none; BMO is reached over the
  address a setting names. The testbed's dependency sweep fails on anything else.
- **English in code, comments, tests, documents and commit messages.** Every word a person reads on
  the screen stands in `client/i18n/locales/ko.yaml`, named by key. The Korean the code still holds is
  the debt `korean-ledger.json` records file by file; a file it does not name is allowed none, and the
  ledger only shrinks.
- **No owner names, agent names, host names, home paths or private network addresses** in code,
  documents or commits. A test's data is a neutral name and value.
- **Tests live in `tests/`:** the crate's integration tests in `server/tests/`, the screen's in
  `client/tests/`, and the expression document's, its fixtures' and the development copy's in the root
  `tests/`, with their support under `tests/support/`. A unit test of a private helper stands beside it
  in the crate.
- **One Cargo workspace at the root**, whose one member is `server/`. The root Bun project holds
  `src/`, `scripts/` and `tests/`; `client/` is a Bun project of its own, with its own manifest and
  lockfile.
- **Documents state what is true now**: no change history, no quoted source and no file-and-line pins.

## Checks

A change moves through three stages in order:

1. Writing: `cargo check --workspace --all-targets`, and `bun run check:types` for the expression
   document and the scripts.
2. Review: read the whole diff against what the change is for and the document governing it, then
   `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` (108 columns,
   2-space indentation, as `rustfmt.toml` sets).
3. Verification: the test targets the change touched, then the suite `testbed.toml` declares once.

The suite runs the crate's tests, the root Bun test files, and the screen's check script, which is its
typecheck and its domain tests. The version is the repository's, in semver, and `VERSION` holds it.
