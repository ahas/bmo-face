# face

BMO's face, `bmo-face`: the full-screen display on the case's screen, the editor its faces are drawn
and corrected in, the server behind both, and the `face/expression` tool entry a briefing's face is
read through. It runs as one service on 127.0.0.1:3217 behind `/bmo-face/`. From BMO it borrows two
things: its web accounts, which the editor logs in against and a display signs in with, and its
listener stream, which carries every briefing with its emotion and BMO's presence.

## Layout

| Folder | Holds |
|---|---|
| `.graph/` | the code graph: one file per area of features, the boundary nodes for the APIs this repository exposes and calls, and `settings.toml` with its coverage; it is written only with `bmo graph` |
| `client/` | the screen, a Bun project of its own: the Nuxt display and editor on `@nuxt/ui`, every word it shows as a key into `i18n/locales/ko.yaml`, and its domain tests under `tests/` |
| `deploy/` | the user unit `bmo-face.service`, which a deployment keeps, and the proxy's two `/bmo-face` locations |
| `docs/` | the canonical statements: the face and its server (`face.md`), the design it is drawn from (`expressions.md`), the deployment (`deploy.md`) and the development bench (`development.md`) |
| `scripts/` | the development bench's commands: the copy of the data root, the bench itself, and the fixtures' writer |
| `server/` | the Rust crate: the expression document's check and preparing, the editor's sessions, the server's routes, the pass-through, and the tool entry, with its tests under `tests/` |
| `src/` | the expression document's TypeScript statement, which the screen imports as `#faces`, and the development copy's module |
| `tests/` | the expression document's, its fixtures' and the development copy's tests, and under `tests/fixtures/` the documents both checks of the expression document are held to, the default the server writes among them |

At the root stand the Cargo workspace manifest and its lockfile, whose one member is `server/`; the
root Bun project's manifest, lockfile and `tsconfig.json`, which hold `src/`, `scripts/` and `tests/`;
`VERSION`; `testbed.toml`; `tool.toml` with its two schemas; `rustfmt.toml`; `korean-ledger.json`; and
`CLAUDE.md`.

## Documents

| Document | States |
|---|---|
| `docs/face.md` | what the display shows and when, what BMO sends it and by which road, how a screen signs in, the expression document, the mouth stages, the editor and the server's routes |
| `docs/expressions.md` | BMO's face as its finished official design draws it: the reference pictures, what was measured on them, the colours, and what each default face and mouth stage takes from them |
| `docs/deploy.md` | how a commit goes live: the release, the unit, the generated screen, the tool entry and the proxy |
| `docs/development.md` | the development bench: the copy of the data root, the server and the screen run against it, BMO's development console behind it, and how a login is made for it |

## Build and test

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
bun run check:types
bun test ./tests
(cd client && bun run check)
```

The screen is seen working on the bench (`docs/development.md`) before it is reported.

## Version

The version stands in `VERSION`, in the Cargo workspace's package and in the screen's manifest, which
hold the same number. A change to a contract a consumer reads, a route, a code or the tool's entry,
raises the major version.
