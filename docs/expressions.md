# BMO's expressions

BMO's face as its finished official design draws it, measured from a set of reference pictures, and
what each face and mouth stage of the default expression document (`docs/face.md`, 「The expression
document」) takes from them.

## The references

- **Only finished official pictures are references:** the pictures the owner gave, official product
  illustrations and packaging, photographs of official figures and plush, and official sticker art.
  Fan wikis' stills, galleries, storyboards and concept sketches are drafts and are not references.
- **The owner's three pictures come first.** Where the others disagree with them, the three decide.
- The pictures and a file of where each came from are kept on this worker's workbench, under
  `references/` — `img/` the pictures, `sources.tsv` their addresses, `crops/` each screen cut out,
  `measurements.txt` what was measured on them. They are not committed: they are other people's
  pictures, kept only to draw against.

| Id | Picture | Kind | Source |
| --- | --- | --- | --- |
| r1 | smiling, arms up | flat illustration | the owner, message 9110 |
| r2 | smiling, beside a skateboard | photograph of the official figure | the owner, message 9111 |
| r3 | calm smile, walking | line art | the owner, message 9112 |
| r4 | laughing, open case | photograph of Kidrobot's official 8" art figure | `https://www.kidrobot.com/products/adventure-time-bmo-art-figure-by-kidrobot` |
| r5 | smiling | the front of Kidrobot's official art-figure box | `https://www.kidrobot.com/products/adventure-time-bmo-glow-in-the-dark-vinyl-art-figure-game-over-limited-edition-con-exclusive` |
| r7 | smiling | photograph of Kidrobot's official plush | `https://www.kidrobot.com/products/adventure-time-bmo-12-glow-in-the-dark-interactive-plush` |
| r8 | smiling, dancing | sticker of the official LINE pack 「Animated Adventure Time」 (Turner Japan) | `https://store.line.me/stickershop/product/5331/en` |
| r9 | crying into its hands | a sticker of the same pack | `https://store.line.me/stickershop/product/5331/en` |

## How the face is measured

- On each picture the screen is the pale region around the face, and its box is the unit: the
  screen's width is 100, as in the expression document.
- The ink is every dark pixel inside it; each eye and each mouth is one blob of it, and its box and
  centre are read off in those units.
- r3 is drawn at an angle and r8 turned, so their distances are foreshortened and they speak for the
  shapes rather than the numbers.
- The colours are sampled from the flat pictures (r1, r3, r5, r8), whose paint no lighting has moved.

## The common design

What every reference agrees on, as the default draws it on the screen of 100 by 75.

- **Flat drawing on a pale mint screen.** Flat fills and near-black lines; no glow, no gradient.
- **The screen** is wider than tall: 1.28 (r2, r4) to 1.64 (r7) wide to one high, 1.39 as the
  middle of them. The document's 4:3 stands inside that range.
- **Two small black dot eyes, far apart and high:**
  - centre to centre 53 of the width — 50 (r2), 52.5 (r5), 53 (r4), 53.6 (r7), 57.6 (r1) — so each
    eye stands about a quarter of the way in from its side: at 23.5 and 76.5;
  - a little taller than wide: 6.3 wide and 7.8 high as the middle of 5.5 to 6.9 wide and 6 to 10.4
    high;
  - about a third of the way down the screen — 29 % (r1) to 37 % (r4, r5): at 25 of 75.
- **The mouth is small, centred between the eyes and just below them.** Its top stands about 10
  below the eyes' centres — 4.6 (r2) to 17.5 (r1) — so the eyes and the mouth read as one small
  face high on the screen, with the screen's lower third left empty.
- **The smiling mouth — the face BMO wears in nearly every reference** — is one open bowl:
  - 32 wide (29 to 40) and 15.5 high (13 to 17.4), a little under half as high as wide;
  - its top edge sags gently toward the middle and meets the round bottom in two corners;
  - an ink outline;
  - three layers inside it, cut off at the outline: the dark green inside; one white band of upper
    teeth along the top edge, thickest in the middle and thinning into the corners, never separate
    teeth; and a light green tongue rising from the bottom across most of the width, its peak right
    of the middle and a little above half the mouth's height.
- **The calm mouth** is one thin ink arc (r3): 20 wide, 4 deep, its ends about 12 below the eyes.
- **A feeling changes the parts, not the proportions.** Eyes stay at the eyes' places and the size
  of a dot; a mouth stays centred under them and no wider than the laughing mouth.

## The colours

| Paint | Colour | Sampled from |
| --- | --- | --- |
| body | `#5fbb9c` | r1 (`#56bc9e` to `#5fbd9c`) |
| screen | `#cef7d8` | r1 (`#d0e9d2`), r3 (`#d9ffe8`), r8 (`#c1fcce`) |
| ink | `#000a00` | the dark lines of every picture |
| light | `#fcfcfc` | the teeth band, r8 (`#fefefe`), r1 (`#eef9ef`) |
| mouth | `#417640` | the inside of the mouth, r1 (`#385d37`), r5 (`#417936`), r8 (`#43794a`) |
| tongue | `#66b567` | r1 (`#7aaf6d`), r5 (`#6ca46c`), r8 (`#50c862`) |
| cheek | `#f4a3b4` | no reference draws cheeks; the paint is kept for the editor |
| tear | `#74f1f0` | no reference draws tears; the paint is kept for the editor |

## What the default draws, and after which reference

Every face stands on the common design above: its eyes at the eyes' places, its mouth centred under
them. Where a reference shows the feeling itself, the face follows it; where none does, the face is
drawn from the common design alone, and says so.

| Face | Eyes | Brows | Mouth | Follows |
| --- | --- | --- | --- | --- |
| `기본` | dots | — | the calm arc | r3 |
| `기쁨` | dots | — | the smiling bowl with teeth band and tongue | r1, r2, r5, r8, r7 |
| `폭소` | closed arcs bowed upward | — | a larger bowl, 40 wide and 18 high | r4 |
| `슬픔` | closed arcs bowed upward, higher at the inner end | — | a small open wail: an arched top, the teeth band along it, the tongue below | r9 |
| `놀람` | larger dots | — | a small round opening, teeth band and tongue | the common design alone |
| `화남` | dots | heavy, slanted down toward the middle | an open wail wider than `슬픔`'s | the common design alone |
| `걱정` | dots | raised toward the middle | a small gritted strip of teeth, filled `light` and lined `ink` | the common design alone |
| `아픔` | `x x` of two crossed strokes each | — | a thin crescent opening under the teeth band | the common design alone |
| `안간힘` | squeezed flat dots | — | a long lozenge, upper teeth above and lower teeth below | the common design alone |
| `졸림` | closed arcs bowed downward | — | a short flat line | the common design alone |
| `writing` | dots a little lower, looking down | level | a short flat line | the common design alone |
| `thinking` | dots up and to the left | one, raised on the left | a short line rising to the right | the common design alone |
| `idle` | dots | — | a short flat line | the common design alone |

- **No face draws cheeks or tears,** since no reference does.

## The mouth stages

The mouths while speaking, from closed to widest, all of one family: the calm arc, then the smiling
bowl of the references at four sizes, each with the teeth band and the tongue inside it, so a
briefing's mouth is BMO's own smile opening and closing.

1. the calm arc of `기본` (r3);
2. the bowl 16 wide and 6 high;
3. 22 wide and 9.5 high;
4. 27 wide and 12.5 high;
5. 32 wide and 15.5 high — the smiling mouth of `기쁨` (r1, r2, r5, r8).
