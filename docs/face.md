# BMO's face

What the face display shows, what BMO sends it and by which road, how a screen signs in, the
expression document both the display and the editor read, and the server that serves them.

## What stands here

- **The display** — one full-screen web page, `/bmo-face/`, that draws BMO's face in SVG and CSS on
  a teal body and moves it: the mouth with the sound BMO speaks, the eyes and brows with the emotion
  it speaks with, and the whole face with what BMO is doing.
- **The editor** — `/bmo-face/edit`, where a person draws and corrects the faces and the mouth
  stages and saves them. What it saves is what every display shows, with no rebuild and no reload.
- **The server** — serves both pages, keeps the expression document, logs the editor in, passes
  a display's requests through to BMO's listener routes, and answers a console for the console screen.
- **The console screen** — the editor again, as a console bundle: a console that keeps bmo-ui's
  console plugin contract finds it by the face's manifest and opens it under `/face`, behind the
  console's own sign-in (「The console screen」).
- **Any screen with a browser.** The case's own screen is not settled, so nothing here assumes a
  board, a resolution or an aspect ratio. A choice that turns on the device is asked of the owner as a
  numbered question rather than guessed.

## What BMO sends, and by which road

- **The road is BMO's own listener stream** — the one every listener hears briefings on. Nothing
  new is opened on BMO's side for the face; the face is one more listener.
- **What the stream carries that the face uses:**
  - `briefing` — a voice briefing went out: the voice message it is, the words, the length, when it
    was sent, the emotion it was spoken with (`기본`, `기쁨`, `슬픔`, `놀람`, `화남`, `졸림`, `폭소`,
    `걱정`, `아픔`, `안간힘`), and the expression it shows, which is what this face's own entry
    answered for that emotion (「The expression entry」).
    - The display shows the expression the event carries; an event carrying none shows its emotion,
      and none of either, or one this display does not know, reads as `기본`.
  - `presence` — what BMO is doing, one state for all of it: `writing` (답 쓰는 중), `thinking`
    (생각 중) or `idle` (쉬는 중), with when it began. It comes once as soon as the stream opens and
    again whenever it moves.
  - `briefing-stopped` — somebody pressed 그만듣기 on any listener; a briefing playing here stops.
  - `keep` — every five seconds, so a quiet stream is told from a dead one.
  - `run` — the name of the bot's run the cursor counts against.
- **The sound** is the voice message itself, fetched from BMO's media route by its chat and message.
  The display plays it and moves the mouth from it; nothing about the mouth is sent.
- **The same-origin pass-through.** A page cannot read another origin's stream with a token header,
  so the display never talks to BMO directly. This server passes exactly four of BMO's routes
  through, under `/bmo-face/bmo/`:
  - `POST /bmo-face/bmo/api/app/login` — signing a screen in;
  - `GET /bmo-face/bmo/api/briefing/events` — the listener stream, with its cursor;
  - `POST /bmo-face/bmo/api/briefing/stop` — 그만듣기;
  - `GET /bmo-face/bmo/api/media/audio/<chat>/<message>` — a briefing's sound, ranges included.
  - Anything else under `/bmo-face/bmo/` is refused here and never reaches BMO.
  - The request's `Authorization` header goes through untouched, with its body, `Content-Type`,
    `Range` and `Accept`; no cookie of this site goes through, and none of BMO's comes back.
  - The answer comes back as it streams, with its status, type, length and range headers; a stream
    is not buffered on the way.
  - A refusal comes back with BMO's status and this server's own one-line answer, never BMO's body,
    which names addresses inside this machine.
  - Where BMO's console is comes from `BMO_CONSOLE_URL` — the live console's address with its base
    path, or the development console's on the bench.
  - A BMO that does not answer is a 502 to the display, which tries again on its own.

## The expression entry

- **`tool.toml` at the repository's root declares the face as a tool of BMO's:** one entry,
  `face/expression`, the expression a briefing shows, for the system alone. It is never shown to a
  session; the voice calls it through BMO's tool host, on the call protocol's first version.
- **The road from the entry to the display is the listener stream:** BMO's briefing runner, as it
  files a briefing's event, calls the entry with the briefing's emotion on the host's control socket
  and puts the expression it answered on the event. The display reads it there like the rest of the
  event; nothing else is opened between BMO and the face.
- **What it is asked and answers:** the emotion a briefing is spoken with, one of the briefing's set,
  or none; it answers the face that emotion shows, `기본` where none was named.
  - The schema holds the emotion to the set a briefing launch is checked against, so the host refuses
    any other before the entry runs.
  - The schema also names `기본` as the emotion's default, and the executable answers that default
    read off the schema placed beside it, so the set and the default are stated once, in the schema.
- **Its executable is the workspace's binary,** started as `bmo-face tool`. It reads the call on
  standard input and answers it on standard output, in the protocol's first version, and reads the
  default off `expression.json` in the folder it is started in.
- **What is placed:** the deployment places the binary with the manifest and its two schemas alone,
  as one folder; nothing else of the face is placed, and a call reads nothing of the face's data
  root.
- **Tested:** the test beside the crate calls the entry with each emotion and with none, and holds
  the schema to the briefing's set.

## How a screen signs in

- **The display signs in once, the way the Android app does.** The first time a screen opens the
  page it asks for a BMO web account's name and password and posts them to BMO's app login through
  the pass-through. BMO answers with a device token, shown once.
- **The token is kept in that browser's own storage** and sent as a bearer token on every stream,
  sound and stop request. A screen whose token BMO refuses — revoked on BMO's account screen, where
  the screen stands listed by the name it signed in with — forgets it and asks to sign in again.
- **This server keeps no token and no password** of any screen's. It never sees a token except as a
  header it passes on.
- **The editor signs in with a BMO web account too,** checked here against BMO's `web_users`, which
  is opened read-only for that and nothing else. What follows the check is this server's own: a
  session in its own database, the `bmo_face` cookie on `/bmo-face` for seven days, and at most
  twenty failed logins from one address in five minutes.
- **The display itself needs no editor session:** the expression document is read openly, since it
  is drawings and nothing else.
- **The console screen signs in nowhere here.** Whoever the console lets onto it edits; the face
  holds the console's credential and the reader's role to account instead (「The console screen」).

## The faces

- **Ten emotion faces**, one per emotion BMO can speak with: `기본`, `기쁨`, `슬픔`, `놀람`, `화남`,
  `졸림`, `폭소`, `걱정`, `아픔`, `안간힘`.
- **Three state faces**, one per presence: `writing`, `thinking`, `idle`.
- **Which face shows**, first match wins:
  1. **Speaking** — a briefing is playing: the face the briefing shows, with the mouth taken from
     the mouth stages as the sound moves it.
  2. **Just spoke** — for a few seconds after a briefing ends, that face with its own mouth,
     so the feeling does not snap away with the last syllable.
  3. **Asleep** — BMO has been `idle` for a long stretch: the `졸림` face.
  4. **Otherwise** — the face of the current presence.
- **A face that is missing** from the document falls back to `기본`, so a document with fewer faces
  still draws.
- **Moving between faces** is a short transition of the parts that differ, not a cut.
  - A part turns into the part standing at the same place in the same group of the next face, and
    a part inside another into the one at the same place inside it.
  - Every part is drawn as the same run of commands — one move and then a fixed number of cubic
    segments, as many as a path can hold, the authored segments split evenly to make up the count
    and a closed path returning to its start by a segment rather than a close — so any part
    morphs into any other: a dot eye into an arc, an open mouth into a line.
  - A paint changes colour over the same transition; a fill or a line that appears or goes away
    does so at once. A part with no counterpart appears or goes away at once.
  - The mouth moving with the sound is never eased.
- **What each default face draws** is taken from the reference pictures of BMO's finished official
  design, measured in `docs/expressions.md`: dot eyes far apart and high, and a small mouth centred
  just below them.

## The mouth while speaking

- **The mouth stages** are an ordered list of mouths, from closed to widest open. Speaking draws
  one of them at a time over the emotion face in place of its own mouth.
- **A stage is parts like any group,** so an open stage is its opening with the teeth and the tongue
  drawn inside it (「The expression document」). By default the first stage is a closed line and
  every stage after it is BMO's smiling mouth at a larger size, with the upper teeth band and the
  tongue inside it, as in the reference pictures (`docs/expressions.md`).
- **The sound picks the stage.** The display decodes the briefing into the page's own audio
  graph and reads its loudness many times a second:
  - the loudness is smoothed, so a single spike does not flash the mouth open;
  - it is mapped onto the stages, silence to the first and full voice to the last;
  - a stage changes only when the loudness moves clearly past the boundary, so the mouth does not
    flutter between two neighbours on a steady vowel.
- **Heard or silent.** By default the display plays the sound. With `?sound=off` in its address it
  plays nothing and moves the mouth from the same decoded sound, for a case whose sound comes from
  somewhere else.
- **A briefing that is late** — sent more than two minutes before the display hears of it, as after
  a reconnect — is not played, the same rule every listener keeps.
- **그만듣기** stands on the display while it speaks. It stops the sound here and posts the stop, so
  every other listener stops too; a stop from anywhere else stops it here.

## The expression document

- **One JSON document** in the data root, `faces.json`, which the display reads and the editor
  writes. A server starting with none writes the default drawn into this project.
- **What it holds:**
  - `colors` — the body and the screen, and the six paints a part is drawn in: `ink` (the lines,
    eyes and outlines), `light` (the teeth and the white of an eye), `mouth` (the inside of an open
    mouth), `tongue`, `cheek` and `tear`. Each is a `#rrggbb` colour.
  - `faces` — one face per name above. A face is its parts in five groups, drawn in this order, each
    over the ones before it: `eyes`, `brows`, `cheeks`, `mouth`, and `marks` — tears, sweat and
    whatever else is drawn over the face. A group may be empty.
  - `mouths` — the mouth stages, in order, each a group of parts.
  - `savedAt` — when it was last saved, which is also its version.
- **A part** is one path, in the screen's own coordinates, a box one hundred wide and seventy-five
  high:
  - its **nodes**, two to thirty-two of them. A node is an anchor the path passes through and two
    handles held as offsets from the anchor: the one the path comes in by and the one it leaves by.
  - between two nodes the path is the cubic Bézier curve from the first anchor, pulled by the first
    node's leaving handle and the second node's coming handle, to the second anchor. A handle of no
    length pulls nothing, so a node with both handles at nothing is a sharp corner and two such
    nodes are joined by a straight line.
  - every node is **smooth** or a **corner**. A smooth node's two handles stand on one line through
    the anchor, which the editor keeps as either is dragged; a corner's handles move apart. The
    drawing reads only the handles.
  - **open or closed**: a closed path runs on from its last node back to its first.
  - its **fill** — a paint, or none; only a closed path is filled.
  - its **line** — a paint, or none, and its width, drawn with round ends and round joins. A part
    has a fill, a line or both.
  - what stands **inside** it — only for a closed path: parts drawn within its outline and cut off
    at it. They are drawn after its fill and before its line, in order, so the outline stays over
    the edge where they are cut. A part inside may be open or closed, and holds nothing inside it.
    This is how an open mouth shows its teeth and its tongue, a laughing grin the lines between its
    teeth, and an eye a pupil or a shine.
- **The paint is the part's own.** A part names the paint it is filled and lined with; nothing takes
  a colour from the group a part stands in.
- **Saving is checked.** A document the editor sends is refused whole unless every face, stage and
  part is well formed and within the screen — every anchor and the end of every handle — and it must
  name the version it was edited from: one saved in between is a conflict, refused, and the editor
  says another window saved first. A group holds at most sixteen parts, a part at most eight inside
  it, and a line is between 0.2 and 10 wide.
- **One check, in two languages.** The screen's editor checks with `src/faces.ts` and the server
  with its own Rust copy of the same rules. The two are held to one set of documents in `tests/fixtures/`
  — what each accepts, what it hands back, and the sentence it refuses each other one with — which
  both test suites read, so neither can drift from the other unseen.
  - **The default is drawn once,** in `src/faces.ts`. `tests/fixtures/default-faces.json` is that default
    as the server writes it; the server reads its default from there, and a test fails while the two
    differ. `bun scripts/fixtures.ts` writes it and the shared documents again after the default or the check
    is changed.
- **The document is prepared once, when the server starts,** before it answers any request, so a
  display and the editor only ever receive a document the check passes. After that a read only
  reads: nothing a display asks for writes the document, and only a save does.
  - no document: the default drawn into this project is written;
  - a document never saved from the editor — `savedAt` of zero — is the default drawn into this
    project, and is replaced by the default drawn now;
  - a saved document holding no face for a name the default draws is given the default's face for
    it, written in place with its version unchanged, so a face added here reaches a document saved
    before it rather than falling back to `기본`;
  - a document in the point-and-ellipse shape is turned into this one and written in place, with
    its version unchanged:
    - a curve through points becomes a path whose nodes are those points, each smooth: a node's
      leaving handle is a sixth of the step from the point before it to the point after it, and its
      coming handle the same step reversed — the end of an open curve standing for its own missing
      neighbour, a closed curve's neighbours running round. That is the very spline the points
      drew, except where a handle had to be shortened to the screen's edge below, and a two-point
      open curve stays a straight line.
    - an ellipse becomes a closed path of four smooth nodes at the ends of its radii, each handle
      along the ellipse and 4/3·(√2 − 1) ≈ 0.5523 of its radius long, which strays from the ellipse
      by less than 0.03 percent of the radius;
    - a filled ellipse is filled and has no line; an unfilled one has a line and no fill; a curve
      has a line, and a fill of the same paint when it is closed and filled;
    - a part in `cheeks` is painted `cheek` and every other part `ink`; `marks` is empty. This is
      the one rule anywhere that reads a group's name, and it stands in the server alone: the
      screen never carries it.
    - a handle whose end falls outside the screen is shortened along itself to the screen's edge,
      and the colours a document does not hold are taken from the default;
    - the handles are kept as computed, not rounded.
  - a document the check still refuses is moved aside, beside it, as `faces.json.broken-` and the
    time in milliseconds — never deleted — and the default is written in its place. The server says
    so on its error output.
  - whatever preparing writes, the server says on its output which of these it was; a document
    left as it stood is not mentioned.
- **Every open display hears a save** on a small stream of this server's own,
  `/bmo-face/api/faces/events`, and draws the new document at once.

## The editor

- **`/bmo-face/edit`, behind the editor's login.** Without a session it shows the login and nothing
  else.
- **Two tabs, edited apart:**
  - `감정 표정` — one face at a time, chosen by name: the ten emotions and the three states. A face
    is edited in its five groups — eyes, brows, cheeks, the resting mouth and the marks — one group
    at a time.
  - `입 모양 단계` — one stage at a time, from closed to widest open. Stages are added and removed
    here, between two and eight of them.
  - Both tabs edit parts the same way, parts inside a part included.
- **The canvas** is the screen at the display's own proportions, with the face or stage being edited
  on it. The parts of the group being edited carry handles; the rest of the face stands dimmed
  behind them for reference, each part in its own paints.
  - Every node of a part has a handle on its anchor. The chosen node also shows the ends of its two
    handles, each on a thin line to its anchor; a handle of no length shows none.
  - Dragging an anchor moves the node with its handles. Dragging a handle's end moves that handle;
    on a smooth node the other handle turns with it to stay on the same line, keeping its own
    length. Nothing is dragged outside the screen.
  - Every anchor and handle end the editor moves lands on a tenth of a screen unit, and a handle is
    kept as its landed end less its landed anchor.
  - Pressing a part chooses it and its handles stand larger; pressing an anchor chooses that node.
- **Parts inside a part.** A chosen closed part lists what stands inside it. Choosing one of those
  edits it on the canvas with the same handles, the outline around it standing dimmed as the edge it
  is cut off at; going back chooses the outline again.
- **The chosen part's numbers** stand beside the canvas, and editing a number moves the part as
  dragging does:
  - the part: open or closed, the fill's paint or none, the line's paint or none, the line width.
    The fill is offered only for a closed part. Opening a part drops its fill — a part with no line
    takes its fill's paint as a line, so it still shows — and a part with anything inside it cannot
    be opened until what is inside is removed.
  - the chosen node: its anchor, its two handles as offsets from the anchor, and smooth or corner.
    A handle can be pulled in to nothing, which makes a sharp corner.
  - each number is a box with its two step buttons, each naming itself in a tooltip, laid out as
    BMO's console lays its own: on a narrow screen, under 640px wide, − at the box's start and + at
    its end; on a wider one an up and a down arrow stacked at its end.
  - making a node smooth:
    - when its leaving handle has a length, its coming handle turns to stand opposite it, keeping
      its own length, or taking the leaving handle's length when it had none;
    - when both handles are nothing, the node takes the spline's handles: its leaving handle a sixth
      of the step from the node before it to the node after it, its coming handle the same step
      reversed, the end of an open path standing for its own missing neighbour. This is how a sharp
      corner is pulled out again on the canvas, where a handle of no length shows no end to drag.
- **Parts are added and removed** in the group being edited: an oval (a closed path of four smooth
  nodes) or a line (an open path of three). Inside a chosen closed part: an upper teeth band along
  the top of the opening, or a tongue rising from its bottom.
  - A new part comes with its own paints, whatever group it is added to: an oval filled `ink` with
    no line, a line in `ink` 2.2 wide, a teeth band filled `light`, a tongue filled `tongue`.
  - A node is added after the chosen one by splitting the segment it leaves by at its middle, which
    leaves the drawing as it was — at the end of an open path, the segment coming into it.
  - Removing a node removes the chosen one, down to two.
- **The preview** beside the canvas is the display's own drawing of what is being edited. In the
  mouth stage tab, and for a face, it can play through the mouth stages on a loop, as speaking
  would.
- **Saving** sends the whole document with the version it was edited from. A save refused by the
  check says why beside the save button; one refused because another window saved first says so and
  offers to load the saved document, dropping what was edited here.
- **Changes not yet saved** are marked beside the save button, and the button is only pressable
  while there are some.
- **On a phone** the canvas takes the screen and the numbers and the preview open from one button,
  in a drawer from the bottom.

## Showing on any screen

- **The face fits the screen it is on.** The body fills the whole viewport in any aspect ratio, and
  the screen with the face keeps its own proportions, centred and as large as the body allows.
- **Full screen** is asked for on the first tap, which a browser allows only after one.
- **The browser may refuse to play sound until the screen is touched.** The display then shows one
  prompt, 눌러서 시작, and plays from the first touch on. A kiosk browser started with Chromium's
  `--autoplay-policy=no-user-gesture-required` and `--kiosk` never shows it.
- **The screen is kept awake** where the browser offers a wake lock.
- **A display runs the screen being served.** Whenever its stream of saves opens again — as it does
  after the server restarts on a deploy — a display reads the build manifest of the screen the
  server serves now (「The server」) and reloads itself when its build is not the one it is running.
  Where there is no manifest, or it names no build, it does not reload; on the development bench
  the manifest names the running development build, so a restart there reloads nothing. A display is never
  left drawing a document whose shape its own drawing does not know, and a document its own check
  refuses is not drawn: the last one it drew stays.

## The screens' shared style

- **One focus ring.** A control reached by the keyboard draws the theme's accent, 2px wide, in a
  small gap outside the control, so the ring follows the control's own corners. The ring is stated
  once as tokens and no component writes a colour or width of its own. A control focused by a mouse
  press or a finger draws none.
- **A field shows focus on its own edge.** An input, a textarea or a select takes the accent on the
  edge it already draws, with no ring outside it.
- **A region with a fixed surface states the ring's step.** The display paints the face on a
  surface of its own whatever the theme, and the message island is fixed dark in both themes, so
  each names the accent step that reads at 3:1 on that surface.
- **A toolbar is one row of equal cells.** An icon-only button in a toolbar is a square as tall as
  the toolbar's row, so the row reads as one band and a finger has the whole height to aim at.
- **Every tooltip is one component.** A screen gives it the words and whether a press on what it
  stands on does something else; how it opens, how wide it grows and how a finger reaches it are the
  component's. On a touch screen a held finger opens it and the lift presses nothing, and the next
  touch anywhere puts it away. Its words wrap inside the room a phone has rather than being cut a
  second time.
- **One long press.** A finger resting on a point is read by one shared mechanism. It is cancelled
  the moment the finger slides beyond the platform's touch distance from where it landed, the page
  scrolls, a second finger lands or the finger lifts, so it is never taken for a scroll or a drag.
  Once it has fired, what it opened stays.
- **Text cut short is shown whole.** One layer over the whole interface finds the line cut short
  under the pointer, the keyboard's focus or a held finger, and shows its whole text in a tooltip.
  Whether it is cut is measured as the tooltip is about to open, so a wider box needs no tooltip and
  a narrower one does. Text that fits carries none, and a line that already has a tooltip of its own
  is left to that tooltip.
- **The theme's colours are measured.** Every badge's words read at 4.5:1 and the focus ring at
  3:1 against every surface they are drawn on, in every theme and every badge shape, and a browser
  test keeps the measurement.

## The server

- **It is a Rust program,** the workspace's one crate in `server/`. It starts on `BMO_FACE_PORT`
  (127.0.0.1:3217 live) and answers everything under `/bmo-face`:
  - the generated screen, and its page for every path that is not the server's;
  - the generated screen's build manifest, `/bmo-face/_nuxt/builds/latest.json`, whose build id a
    display compares with its own;
  - `GET /api/faces`, openly, and `PUT /api/faces` with an editor session;
  - `GET /api/faces/events`, the saves as they happen, with a keep frame every five seconds;
  - `POST /api/login`, `POST /api/logout` and `GET /api/me` for the editor;
  - the four passed-through routes under `/bmo`;
  - `GET /console-manifest` and the routes under `/console`, for a console alone (「The console
    screen」).
- **A write must come from this site.** A `POST` or `PUT` whose `Origin` is not this host is refused
  before anything else is read. The console's routes are the exception: a console's server makes
  those calls, and its credential guards them in place of the origin.
- **The data root** is the one `BMO_FACE_DATA` names: the expression document and the session
  database. The unit names `~/.local/share/bmo/data/face/`, under the data root this machine shares
  with BMO's other units; a process without the variable uses `~/.local/share/bmo-face/`. It is live
  data: never committed, never written by a test, and never opened by development work, which uses a
  copy (`docs/development.md`).
- **What it reads from its environment:** `BMO_FACE_DATA`, `BMO_FACE_PORT`, `BMO_FACE_STATIC` (the
  generated screen, `client/.output/public` beside the binary's project by default),
  `BMO_FACE_CONSOLE` (the built console bundle, `client/.output/console` beside it by default),
  `BMO_FACE_TOKEN` (the console's credential), BMO's archive for the editor's login and
  `BMO_CONSOLE_URL`. The archive is the address `BMO_ARCHIVE_URL` names,
  else the file `BMO_DB_PATH` names, else the address the data root's `env/archive-url` holds, else
  its `bmo.db`. `BMO_FACE_KEEP_MS` shortens the keep frame's beat, for a test that cannot spend
  seconds on it. A port of 0 is one the system chooses, named in the line the service prints as it
  starts listening; its contract test starts it so, and reads the port from that line.
- **A request body is at most 2,000,000 bytes;** a larger one is answered `413` before it is read.
- **The editor's password** is checked against the Argon2 hash BMO keeps in `web_users`, the one form
  BMO writes there.
- **Its check** is the crate's tests: the document's check and its preparing against the shared
  fixtures, the tool entry, and a contract test that stands the binary on a free port over a scratch
  data root, a scratch archive and a stand-in for BMO's console. The screen's own test stands the
  same binary up once more and reads its console manifest through bmo-ui's `consoleManifest`.

## The console screen

- **The contract is bmo-ui's console plugin contract** (bmo-ui's `docs/console-bundles.md`), at the
  layer version this repository vendors under `client/vendor/`. The face is one more service a
  console lists by its address and credential and finds by its manifest; nothing in a console is
  written for the face, and nothing here names a console.
- **What the screen is:** the editor (「The editor」) whole, under the screen path `/face`, without
  its login and its log-out: the console's sign-in stands in their place. What it saves is the one
  document every display draws, heard by them as any save is.
- **The service's API is everything under `/bmo-face`,** so a console's setting names the face at
  its address with that path, and every path below is written from there.
- **The manifest**, `GET /console-manifest`:
  - `name` `face`, its words' namespace; `title` `face.title`; `icon` `i-material-symbols-face-outline`;
    `screenPath` `/face`; `serviceName` `face`;
  - `apiPrefix` `/console`: everything the screen calls stands under it;
  - `role` `member`: an administrator or a member opens it;
  - `ownerKey` false: the expression document is the machine's one document, no owner's own, so
    nothing here reads who the reader is;
  - `events` false and `websocket` false: the editor reads the document when it opens and after a
    conflict, and listens for nothing; `pushKinds` none;
  - `bundle`, `stylesheet` and `words`: the module's, the stylesheet's and the words' paths below.
  - What does not follow from the built bundle — every field but those three paths — stands once, in
    `console-manifest.json` at the repository's root. The server answers it with the three paths
    added, and the bundle reads its screen path, service name and API prefix from the same file.
- **The routes under `/console`:**
  - `GET /console/bundle.<hash>.js` — the module; `GET /console/bundle.<hash>.css` — the
    stylesheet. The hash is the first sixteen hexadecimal digits of the SHA-256 of the file's bytes,
    so a path changes whenever its bytes do. A path naming bytes that are not the ones built is a
    404, and the ones built are answered as kept for good.
  - `GET /console/words` — the words.
  - `GET /console/faces` — the expression document, as `GET /api/faces` answers it.
  - `PUT /console/faces` — a save, taking and answering what `PUT /api/faces` does: the same check,
    the same version it must name, the same conflict.
- **The guard:**
  - Every console route — the manifest among them — needs `Authorization: Bearer` with the token
    `BMO_FACE_TOKEN` names. Without it, or with any other, the answer is a 401; a server with no
    token set refuses every one of them so, and the display and the editor are untouched by it.
  - The manifest and the words need nothing more, as the contract has it.
  - The module, the stylesheet and the document also need `Owner-Role` naming a role that allows
    the manifest's: `administrator` or `member`. A role missing or any other is a 403. Where the
    manifest asks for the owner key, a call without `Owner-Key` is a 403 too; this manifest does not
    ask for it.
- **No bundle built, no manifest.** Where the built folder holds no module, the manifest is a 404,
  so a console leaves the face out rather than load a screen that is not there.
- **The bundle** is built by `bun run console` in `client/`, into `client/.output/console`: the
  module `bundle.js`, the stylesheet `bundle.css` and the words `words.json`. The server reads that
  folder on every request, so a bundle built again is answered under its new paths at once.
  - **The module** is an ECMAScript module whose default export is `{ routes }`: one route at the
    screen path, the editor's page, handed the road to the console routes through bmo-ui's
    `relayAddress` for the service's name.
  - **It leaves unbundled** every import of `vue`, `vue-router`, `vue-i18n`, `nuxt/app`, `@nuxt/ui`
    and `bmo-ui`, written as the package's own path: a Nuxt UI component as
    `@nuxt/ui/components/<Name>.vue`, a composable Nuxt makes available on its own as `nuxt/app`.
    Everything else of the screen it carries.
  - **The island and the back gesture are the console's.** The bundle takes bmo-ui's `island` and
    `layers` modules where the screen has its own copies, so the editor's notices and questions stand
    on the console's island and inside a part is left by the console's one history of layers.
  - **The stylesheet** is the utility classes the screen's pages and components write, built against
    the theme's names without the theme, the base or the library's own classes, and the components'
    own styles. The theme, the focus ring and the fonts are the console's.
  - **The words** are the `face` tree of the pack, under `ko`. Every word the screens show stands
    under `face` in `client/i18n/locales/ko.yaml`, beside the `common.*` words a console answers
    itself, so the standalone screens and the bundle read one pack by the same keys.
- **Tested:** the crate's contract test asks every console route without the credential, without a
  role and with one, and writes the module twice to see its path follow its bytes; the screen's
  test reads the manifest the binary answers through bmo-ui's `consoleManifest`, and the module's
  imports against the libraries it may leave unbundled.
