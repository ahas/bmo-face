# The development bench

How the face is run and checked while it is being changed, without touching the live document or
the real BMO.

## The copy of the data root

- **Where:** `~/.local/share/bmo/data/workbench/bmo-face/data`, under BMO's workbench, never
  inside the data root it copies.
- **What it holds:** `faces.json`, copied from the live data root. The session database is not
  copied; a bench login makes its own.
- **Making and refreshing it:** `bun scripts/dev-data.ts`. A refresh replaces the copy's document
  with the live one, dropping whatever was saved on the bench. `--from` and `--to` name another
  source and copy. With no live document yet, the copy starts from the project's default.

## The bench

- `bun scripts/dev.ts` runs two processes and makes the copy first if there is none:
  - **The server** on 127.0.0.1:3218, the workspace's `bmo-face` binary built and started by Cargo
    against the copy (`BMO_FACE_DATA`). A change to the server is taken up by starting the bench
    again; the screen reloads on its own. It
    checks editor logins against the workbench copy of BMO's archive at
    `~/.local/share/bmo/data/workbench/worker/bmo.db` unless `BMO_DB_PATH` names another, and it
    passes the display's listener routes to BMO's development console at
    `http://127.0.0.1:3211/bmo-dev` unless `BMO_CONSOLE_URL` names another. When the bench is started
    with `BMO_FACE_TOKEN`, it answers the console routes to that token with the console bundle
    `bun run console` in `client/` built into `client/.output/console`, read once as the server starts.
    Nuxt's dev server empties `client/.output` as it starts, so the bundle is built once the bench
    stands and the bench is then started again for the server to read it.
  - **The screen**, Nuxt's dev server on 127.0.0.1:3219 under `/bmo-face/`, with hot reload. It
    passes `/bmo-face/api` and `/bmo-face/bmo` through to 3218 and keeps the request's Host header,
    since the server matches a write's Origin against it.
- When either process ends the other is ended too, so whatever runs the bench starts both again
  together.
- **No pane keeps it standing.** A job that needs the bench starts `bun scripts/dev.ts` in the
  background itself, reads its output from a file, and takes it down before it ends. Its ports are
  fixed, so one bench runs at a time on this machine.
- **Nothing real is reached.** BMO's development console reads its own archive copy, and its control
  port is the bench's stand-in bot, so the stream the display hears carries only what a check hands
  that stand-in. A briefing handed that way is a real voice message of the copy, so the display
  plays a real voice of BMO's.
- **A login** — for the editor, and for signing the display in — is an account of BMO's archive,
  which the bench reads as its copy. Accounts are made the platform's own way, through the
  console's user script in the console's app (`client/apps/web`): `bun scripts/user.ts set <name>
  <password>`, against the database `WEB_DATABASE_URL` names (bmo-web's `docs/web.md`, 「Making an
  account and resetting a password」).
