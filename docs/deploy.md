# Deployment

How a commit of this repository goes live. The testbed deploys it from the repository's own
declaration, `testbed.toml`; nothing here deploys itself.

## What a deployment does

- **The release:** the testbed builds the workspace's two binaries, `bmo-face` and
  `bmo-face-displayer`, into the release's `bin/`,
  installs the root and the screen's Bun packages (the declaration's setup), and, before the flip,
  generates the screen with `bun run generate` in `client/`, into `client/.output/public`, and then
  builds the console bundle with `bun run console`, into `client/.output/console`. The server serves
  both from there, reading the bundle once as it starts: its own defaults are those two folders
  beside the release's `bin/` (`docs/face.md`, 「The server」 and 「The console screen」). The screen
  is generated in Korean, from the Korean pack this repository holds under `client/i18n/locales/`, and the
  bundle's words are read from every pack there, one to a locale. Both take the face's core from the WebAssembly build the
  repository carries under `src/wasm/` (`docs/face.md`, 「The face's core」), so a release builds no
  WebAssembly and needs no toolchain for it; the server's binary is built with the core as a crate.
- **The unit:** `deploy/bmo-face.service`, kept by the testbed as a user unit. It runs
  `bin/bmo-face` in the release on loopback at the port the server's settings name as `BMO_FACE_PORT`,
  which an install writes into `env/face.env` with the port it chose for the face, and 3217 where they
  name none, with `BMO_FACE_DATA` naming the document and the session database under the shared data
  root, `face/`. Its health is `bmo-face health`, which reads the same settings and asks the server
  there for its document route and its generated screen's HTML. A stop sends the server SIGTERM, on which it drains: it takes
  no new connection, ends its open streams and lets the calls in hand finish within its own deadline
  (`docs/face.md`, 「The server」). The unit waits for that end with no stop timeout, so the server's
  deadline is the one that bounds a stop.
- **The console probe:** once the health answers, the testbed probes the console face against the
  console plugin contract, as the testbed's `docs/deploy.md` states in 「The console probe」, and a
  point that does not hold fails the deployment. The declaration's `[deploy.console]` reach is
  `bmo-face console-reach`, which prints from the same settings the server's loopback address under
  `/bmo-face` and the console's credential, and, for the service's own token, a value made new for each
  run: the face holds no
  token of its own (`docs/face.md`, 「The console screen」), so any credential but the console's
  stands for one, and the console's routes must refuse it.
- **After health:** the release's binary, `tool.toml` and the two schemas are placed as one folder
  and registered with the tool host as this service's own, under the repository's name. A failed
  registration leaves the service live and is said.
- **BMO:** the server reads `env/face.env` under the data root where it exists (`docs/face.md`, 「The
  server」), for the port it listens on, for the address of
  BMO's archive (the editor's login) and of BMO's console (the passed-through listener routes), and
  for `BMO_FACE_TOKEN`, the credential a console calls the console routes with, which that console's
  own settings hold beside the face's address. All are this machine's settings, never the
  repository's.
- **The proxy:** the two `/bmo-face` locations of the host's server block are a settings file the
  install writes, `proxy/bmo-face.conf` under the data root (`install.toml`): the bare path
  redirected to the one with the slash, and everything under it passed to the face's port as the
  install chose it, with the request's host and scheme handed on and nothing buffered, so a stream
  passes as it comes. The web server includes it once, `include <data root>/proxy/*.conf;` inside the
  server block that serves the console, written by a person with root; the install's verification
  says where it is missing. No basic auth stands in front: the display signs in with a BMO account
  and the editor with its own session. A deployment never touches the proxy, and the repository
  holds no host's proxy configuration.

## The native displayer

- **Not a unit the testbed keeps.** The displayer (`docs/face.md`, 「The native displayer」) needs a
  logind session on a virtual terminal of its own, which only the system's service manager opens, so
  its unit is a system unit, `deploy/system/bmo-face-displayer@.service`, laid down once by a person
  with the PAM service it names. The instance is the user it runs as, the user the testbed deploys as.
- **It runs the current release.** The unit starts the binary through the deploy link,
  `~/.local/share/bmo-face/deploy/bin/bmo-face-displayer` under that user's home. A deployment
  restarts the server, the displayer's saves stream opens again, it sees that the deploy link has
  moved and ends with success, and the unit starts it again from the new release. No deployment step
  and no privilege is needed for that.
- **It starts at boot and comes back from any end:** `Restart=always`, wanted by `graphical.target`.
- **Its lines** stand in the journal under its own identifier, read with
  `journalctl -t bmo-face-displayer` (`docs/face.md`, 「What it says」): it runs in a logind session's
  scope, so the unit's name finds only its starts and stops.
- **Its settings** are set as `Environment=` lines in the unit's drop-in (`systemctl edit`), never in
  the repository (`docs/face.md`, 「Its settings」).
- **It hears briefings from the voice engine,** on the loopback, with the engine's own token
  (`docs/face.md`, 「What it hears」): the drop-in names the engine's address, the file its token stands
  in and the rooms whose summaries are spoken; a speech is spoken from every room. The token file is the voice's own, the file under the
  shared data root's `env/` that the voice's settings name as `token_file` (bmo-voice's
  `docs/boundary-api.md`, 「Settings」); the unit's user reads it where it stands, and the drop-in names
  its path, never its value. The displayer signs in nowhere and keeps no token of its own.

### Setting it up once

Run by a person with root, in this order; `<user>` is the user the testbed deploys as, and `tty8`
stands for a VT no getty or display manager uses.

1. The user may use the panel and the sound devices when its session is not in the foreground of the
   seat as well: `sudo usermod -aG video,render,audio <user>` — `video` is enough for the card where
   logind hands it over; `audio` lets a briefing that arrives while the VT is in the background open
   the sound device.
2. The PAM service the unit names, an account check and a logind session with no password asked:
   `sudo install -m 0644 deploy/system/pam.d/bmo-face-displayer /etc/pam.d/bmo-face-displayer`.
3. The unit: `sudo install -m 0644 deploy/system/bmo-face-displayer@.service /etc/systemd/system/` and
   `sudo systemctl daemon-reload`.
4. Its settings, in a drop-in: `sudo systemctl edit bmo-face-displayer@<user>.service` with
   `Environment=` lines (`docs/face.md`, 「Its settings」):
   - `BMO_FACE_DISPLAYER_VOICE=<the voice engine's address>`, the `address` the voice's settings name;
   - `BMO_FACE_DISPLAYER_VOICE_TOKEN_FILE=<the shared data root>/env/<the voice's token file>`;
   - `BMO_FACE_DISPLAYER_ROOMS=<the owner's private room>`, the room's number as the engine keeps it, the one
     room whose summaries are spoken;
   - and any other where its default does not hold, for example `BMO_FACE_DISPLAYER_SOUND=off` where
     nothing should be heard.
5. Starting it now and at every boot: `sudo systemctl enable --now bmo-face-displayer@<user>.service`;
   a unit already running takes changed settings with `sudo systemctl restart` of the same name.

It brings its VT to the foreground as it starts; the greeter stays running behind it. `Ctrl+Alt+F1`
— or whichever VT the greeter stands on — brings the greeter back, and the face's VT key brings the
face back.

## Where things stand

- **The deploy tree** is the testbed's, `~/.local/share/bmo-face/deploy`, under the root
  `testbed.toml` names; the unit runs from there and never from a checkout.
- **The data root** is the shared one, `~/.local/share/bmo/data`; the face keeps its document and
  session database under `face/` (`docs/face.md`, 「The server」).
- **A failed step** stops the deployment before the unit is restarted, so what is served has not
  moved.
