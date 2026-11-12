# Deployment

How a commit of this repository goes live. The testbed deploys it from the repository's own
declaration, `testbed.toml`; nothing here deploys itself.

## What a deployment does

- **The release:** the testbed builds the workspace's binary, `bmo-face`, into the release's `bin/`,
  installs the root and the screen's Bun packages (the declaration's setup), and, before the flip,
  generates the screen with `bun run generate` in `client/`, into `client/.output/public`, and then
  builds the console bundle with `bun run console`, into `client/.output/console`. The server serves
  both from there, reading the bundle once as it starts: its own defaults are those two folders beside the release's `bin/`
  (`docs/face.md`, 「The server」 and 「The console screen」). The screen is generated in Korean, from
  the pack this repository holds, `client/i18n/locales/ko.yaml`, and the bundle's words are read from
  the same pack.
- **The unit:** `deploy/bmo-face.service`, kept by the testbed as a user unit. It runs
  `bin/bmo-face` in the release on 127.0.0.1:3217, with `BMO_FACE_DATA` naming the document and the
  session database under the shared data root, `face/`. Its health is read from the document route
  and from the generated screen's HTML. A stop sends the server SIGTERM, on which it drains: it takes
  no new connection, ends its open streams and lets the calls in hand finish within its own deadline
  (`docs/face.md`, 「The server」). The unit waits for that end with no stop timeout, so the server's
  deadline is the one that bounds a stop.
- **After health:** the release's binary, `tool.toml` and the two schemas are placed as one folder
  and registered with the tool host as this service's own, under the repository's name. A failed
  registration leaves the service live and is said.
- **BMO:** the unit reads `env/face.env` under the data root where it exists, for the address of
  BMO's archive (the editor's login) and of BMO's console (the passed-through listener routes), and
  for `BMO_FACE_TOKEN`, the credential a console calls the console routes with, which that console's
  own settings hold beside the face's address. All are this machine's settings, never the
  repository's.
- **The proxy:** `deploy/nginx.conf` holds the two `/bmo-face` locations of the host's server block:
  the bare path redirected to the one with the slash, and everything under it passed to port 3217
  through the shared proxy snippet, which does not buffer, so a stream passes as it comes. No basic
  auth stands in front: the display signs in with a BMO account and the editor with its own
  session. A deployment never touches the proxy; the path and the port are fixed.

## Where things stand

- **The deploy tree** is the testbed's, `~/.local/share/bmo-face/deploy`, under the root
  `testbed.toml` names; the unit runs from there and never from a checkout.
- **The data root** is the shared one, `~/.local/share/bmo/data`; the face keeps its document and
  session database under `face/` (`docs/face.md`, 「The server」).
- **A failed step** stops the deployment before the unit is restarted, so what is served has not
  moved.
