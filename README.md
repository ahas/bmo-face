# face

BMO's face, `bmo-face`: the full-screen display on the case's screen, the editor its faces are drawn
and corrected in, the server behind both, and the `face/expression` tool entry a briefing's face is
read through. It runs as one service on 127.0.0.1:3217 behind `/bmo-face/`. From BMO it borrows two
things: its web accounts, which the editor logs in against and a display signs in with, and its
listener stream, which carries every briefing with its emotion and BMO's presence.

## Layout

| Folder | Holds |
|---|---|
| `core/` | the face's core, a Rust crate: the expression document's check and its carrying from older forms, the eye and talking mouth sets, the geometry, the blend and the drivers of the held clock, with its tests under `tests/` and the old documents it carries under `tests/fixtures/` |
| `client/` | the screen, a Bun project of its own: the Nuxt display and editor on `@nuxt/ui`, every word it shows as a key into `i18n/locales/ko.yaml`, the console bundle's entry under `app/console/` and its build under `scripts/`, the vendored bmo-ui release under `vendor/`, and its tests under `tests/` |
| `deploy/` | the user unit `bmo-face.service`, which a deployment keeps, the proxy's two `/bmo-face` locations, and under `system/` the native displayer's system unit and PAM service, laid down once by a person |
| `displayer/` | the native displayer's Rust crate, on the core: the panel taken through logind and drawn on the CPU, the face server's document and the voice engine's briefings it hears, the sound it plays and its gaze socket, with its tests and their fixture under `tests/` |
| `docs/` | the canonical statements: the face and its server (`face.md`), the design it is drawn from (`expressions.md`), the deployment (`deploy.md`) and the development bench (`development.md`) |
| `scripts/` | the development bench's commands — the copy of the data root and the bench itself — and the core's WebAssembly build |
| `server/` | the server's Rust crate, on the core: the expression document's preparing, the editor's sessions, the server's routes, the pass-through, and the tool entry, with its tests under `tests/` |
| `src/` | the expression document's TypeScript statement, which the screen imports as `#faces`; the core's WebAssembly build under `wasm/` and its typed module, which the screen imports as `#core`; and the development copy's module |
| `tests/` | the tests of the expression document's statement, of the core's WebAssembly build and of the development copy |

At the root stand the Cargo workspace manifest and its lockfile, whose members are `core/`, `displayer/` and `server/`; the
root Bun project's manifest, lockfile and `tsconfig.json`, which hold `src/`, `scripts/` and `tests/`;
`VERSION`; `testbed.toml`; `tool.toml` with its two schemas; `console-manifest.json`, the console
face's declaration, the manifest's fields the face decides; `rustfmt.toml`; `korean-ledger.json`;
`LICENSE`; and `CLAUDE.md`. `docs/PERSONAL-DATA.md` names the check `tests/personal-data.test.ts`,
which the suite runs with the rest.

## Documents

| Document | States |
|---|---|
| `docs/face.md` | what the display shows and when, what BMO sends it and by which road, how a screen signs in, the held clock and the drivers, the expression document and how a path is written and blended, the face's core and its interface, the native displayer, the editor, the server's routes and the console screen |
| `docs/expressions.md` | BMO's face as the animation draws it: what was measured frame by frame, the colours, how a shape is drawn, the eye set, the talking mouth set, how the animation moves, and what each default face draws |
| `docs/deploy.md` | how a commit goes live: the release, the unit, the generated screen and console bundle, the tool entry and the proxy, and the native displayer's system unit with its one-time setup |
| `docs/development.md` | the development bench: the copy of the data root, the server and the screen run against it, BMO's development console behind it, and how a login is made for it |
| `docs/PERSONAL-DATA.md` | the personal data no file may hold, the values allowed, and the pattern check that keeps it out (`tests/personal-data.test.ts`) |

## Build and test

```sh
bmo-testbed cargo build --workspace
bmo-testbed cargo test --workspace
bmo-testbed cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
bun run check:types
bun test ./tests
(cd client && bun run check)
```

The Cargo verbs run through `bmo-testbed`, which holds the shared build directory; a plain `cargo`
would compile the workspace cold into the checkout's own `target/`. The crates' format and lint need
no declared step: the testbed's Cargo workspace check runs them over every member wherever a change
marks a crate, and the change check compiles the crates it marks. The screen is seen working on the
bench (`docs/development.md`) before it is reported.

## Version

The version stands in `VERSION`, in the Cargo workspace's package and in the screen's manifest, which
hold the same number. It moves only at a release the owner declares, as bmo-chat's `docs/conventions.md`
「Versions」 states; a contract change moves none, and a deployment is told apart by its commit.

## License

MIT, as `LICENSE` states.
