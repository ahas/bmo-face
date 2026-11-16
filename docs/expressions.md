# BMO's expressions

BMO's face as the animated series draws it, measured frame by frame, and what the face's shapes, its
default faces and its drivers take from those measurements (`docs/face.md`, 「The faces」 and 「The
mouth while speaking」).

## The study

- **What was read:** 26,135 frames in which BMO's eyes and mouth could be read, from 199 studio lines
  over 48 episodes and from 12 whole episodes: 5,450 mouth drawings in 40 clusters, 5,462 eye samples
  in 28 clusters, 199 blinks, 142 expression changes, and the sound under every talking frame.
- **The unit is the screen.** BMO's screen, read in each frame, is 100 wide; its height stands at 1.33
  of the width as the median (1.25 to 1.41), so the document's 4:3 screen of 100 by 75 holds, and
  every size here is in its units.
- **Nothing of the series enters the repository.** The frames and the sheets read from them stay
  where the study ran. What the face takes is numbers, and shapes redrawn clean at the measured sizes
  and proportions; where the study fitted exact control points it reads them as noise of the fit and
  takes the size, the proportion and the roundness instead.

## The common design

- **Flat drawing on a pale mint screen.** Flat fills and near-black lines; no glow, no gradient.
- **Two small dot eyes, far apart and high:** 6 wide and 6.5 high, their centres 53.6 apart (49.3 to
  58 as the middle half), 24.6 down the screen. One pair of eyes in five is drawn unalike, so each
  eye is a side of its own, mirrored by default.
- **The mouth stands 23.8 below the eyes,** centre to centre, at 48.4 down the screen. It is one value
  of the eye and mouth sets, which a later change sets.
- **Lines are thin and round.** A mouth's outline is 0.5 to 1.0 wide in the animation; the face draws
  0.8, 0.7 around a small opening and 0.9 for a closed eye. Every line has round caps and round joins.
- **No sharp corner on a mouth.** An open mouth's ends are turned on a radius of 0.36 of its height as
  the median (0.28 to 0.46), half its height on a round O.
- **An open mouth is a rounded bean:** a level top sagging a little toward the middle, round ends, a
  full round bottom; inside, the dark green cavity, a white band of upper teeth hanging from the top,
  thickest in the middle and stopping short of the corners, a two-tone tongue rising from the bottom
  a little right of the middle, and now and then a band of lower teeth. 33 % of read frames show
  teeth, 46 % a tongue.
- **A line mouth bows a little:** 63 % of them bow upward in the middle.
- **Other elements:** brows on 9 % of frames, a stroke above each eye; a white shine inside an eye on
  3 %; pink blush under the eyes on 3 %; tears, sweat and the like as marks.

## The colours

The daylight palette, the screen colour of 65 % of the sampled frames, each colour the median of its
pixels there.

| Paint | Colour | Drawn as |
| --- | --- | --- |
| screen | `#cdfeda` | the screen |
| ink | `#000201` | every line, the eyes, the outlines |
| light | `#fcfbfc` | the teeth, an eye's shine |
| mouth | `#2c843b` | the cavity of an open mouth |
| tongue | `#50dc66` | the tongue |
| tongueLight | `#64ea80` | the tongue's lighter tone, a rim along its top |
| cheek | `#fecbc0` | the blush |
| tear | `#74f1f0` | kept for the editor; no default face draws tears |

The body around the screen, `#5fbb9c`, is no measurement of the animation; it is the teal of the
official figure the face drew before.

## How a shape is drawn

- **An eye** is a filled ink oval of four smooth nodes; a lid half down is that oval with its top cut
  away and the cut drawn straight; a closed eye is a stroke through three or five points.
- **An open mouth** is six smooth segments: a quarter turn at each end on the end radius, the level
  top in two halves meeting at the middle, and the round bottom in two halves, each reaching a share
  of its half (0.55 an ellipse, more a squarer bowl). Turned upside down it is a wail.
- **The teeth band** hangs from above the outline, to be cut off at it, its lower edge falling from
  its ends to its middle; the lower band is the same, risen from below.
- **The tongue** is a dome rising from below the outline's bottom, its peak right of the middle; its
  light tone is the same dome raised by 0.45, drawn under it, so it shows as a rim along its top.
- **A line mouth** is a stroke through three points: its ends and its middle, the middle bowed.

## The eye set

The eyes the animation draws, each redrawn as the left eye around its centre, the right its
reflection. The reach is how far each moves with the gaze; the voice how far it may grow, squint and
shimmer its shine while BMO speaks, each a share of its own size, a larger eye less so that it stays
the eye it is (`docs/face.md`, 「The eyes while speaking」). A closed eye has neither.

| Name | Shape | Size (wide × high) | Reach | Voice (grow, squint, shine) |
| --- | --- | --- | --- | --- |
| `dot` | filled oval | 6 × 6.5 | 2.2 × 1.6 | 0.1, 0.15, 0 |
| `dot-small` | filled oval | 4.8 × 6.2 | 2.2 × 1.6 | 0.1, 0.15, 0 |
| `oval` | tall oval | 5.8 × 9.6 | 1.8 × 1.2 | 0.08, 0.12, 0 |
| `oval-tall` | taller oval | 6.8 × 13 | 1.6 × 1.0 | 0.06, 0.1, 0 |
| `oval-wide` | large oval | 8.2 × 11.2 | 1.6 × 1.0 | 0.06, 0.1, 0 |
| `surprise` | surprise oval | 8 × 17.2 | 1.4 × 0.8 | 0.05, 0.08, 0 |
| `shining` | big eye, two white shines inside | 10.2 × 9.8 | 1.6 × 1.2 | 0.06, 0.1, 0.25 |
| `half-lid-small` | lid half down | 5.2 × 3.75 | 1.6 × 0.6 | 0.04, 0.1, 0 |
| `half-lid` | lid half down | 6.2 × 5 | 1.8 × 0.6 | 0.04, 0.1, 0 |
| `half-lid-wide` | lid half down | 10 × 5.5 | 1.2 × 0.4 | 0.04, 0.1, 0 |
| `closed-arc` | shallow arc ‿ | 10 × 1.9 | — | — |
| `closed-cap` | arc ⌒ | 9 × 3.4 | — | — |
| `closed-u` | shallow U | 11 × 4.5 | — | — |
| `closed-u-deep` | deep U | 6.8 × 7.8 | — | — |
| `squeezed` | `>` of two straight strokes | 5.8 × 5.6 | — | — |

## The talking set

The 23 mouth drawings that occur twenty times or more inside the studied lines, in the animation's
families, are the talking set's anchors (`docs/face.md`, 「The talking set」). Sizes are wide × high;
inside, the upper teeth band's depth (T), the lower band's (L) and the tongue's rise (G), each as a share
of the height. Each stands where its own size measures it, how open and how wide; the slit of clenched
teeth and the grimace stand on the plane of the teeth together.

| Family | Share of the talking frames | Drawings, at their places (open, wide) |
| --- | --- | --- |
| bean | 0.528 | small: 14.9 × 7.2 G .38 (.357, .234); 20.5 × 7.8 T .22 G .34 (.387, .382); 21.8 × 9.8 T .24 G .36 (.486, .416) — medium: 28.2 × 8.5 T .3, top bowed up (.454, .584); 22 × 11.2 G .55 (.559, .421); 27 × 12.2 T .24 G .4 (.605, .553); 23.8 × 13.8 T .24 G .36 (.688, .468); 32.2 × 12.2 T .22 G .4 (.605, .689) — large: 28.8 × 15.8 T .22 G .38 (.787, .6); 35.2 × 14.2 T .24 G .4 (.704, .768); 32.5 × 15.5 T .2 G .42 (.772, .697); 42.1 × 18.2 T .2 G .38 (.905, .95) |
| line | 0.139 | 16.8 wide, bowed 0.5 (0, .284); 27.5, bowed 0.9 (0, .566); 30.2, bowed 3.2 (0, .637) |
| slit | 0.110 | 15.5 × 4.8 G .4 (.238, .25); 21.1 × 5.8 T .45, teeth together (.287, .397) |
| round | 0.085 | 15.5 × 10.5 G .34 (.52, .25); 17.2 × 13.8 T .16 G .32 (.683, .295); 19.8 × 17.8 T .16 G .3 (.881, .363); 26.5 × 20.2 T .18 G .28 (1, .539) |
| grimace | 0.068 | 34.8 × 10.5 T .44 L .44, teeth together (.52, .758) |
| small-o | 0.051 | 7.2 × 8, a dark oval with nothing inside (.396, .032) |

## How the animation moves

- **The talking mouth is drawn on twos:** a new drawing every two frames at 23.976 a second in 72 %
  of holds, four frames in 11 %, one frame in 10 %. Drawings cut from one to the next; the same
  drawing follows itself in 11 % of steps. The face follows its sound every frame by default, and on
  twos where its talking set asks for this look.
- **The mouth leads the sound by two frames:** its openness follows the loudness best two frames
  ahead.
- **Loudness opens the mouth, weakly** (Spearman's ρ 0.21): the animation's mouth is drawn, not
  read off the sound, and the face reads it off the sound more closely than the animation does. By tenths of the loudness, quietest first:

  | Tenth | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |
  | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
  | Openness | .47 | .47 | .53 | .54 | .60 | .60 | .68 | .72 | .72 | .75 |
  | Closed | 22 % | 17 % | 13 % | 11 % | 9 % | 9 % | 6 % | 8 % | 6 % | 4 % |

- **By sound** (English audio, a tendency): on P, B and M the mouth is closed a third of the time and
  widest and flattest; on rounded vowels narrowest, most often the small dark o; on open vowels most
  open; in pauses between words closed a quarter of the time.
- **Lines start and end small:** across the start of a line the mouth's opening grows from about 80
  to about 200 square units, and across its end it shrinks from about 243 to about 94.
- **The blink:** open, half-lid for one frame, a closed shallow arc for two, half-lid for one, open —
  four frames in 52 % of blinks; 10 to 15 a minute; 14 % come as doubles.
- **A change of expression** is a cut (13 of 40 eye changes, 29 of 102 mouth changes) or one to three
  held in-between drawings on twos, some with anticipation or overshoot; never a smooth ease.
- **Idle, the drawing is held;** only the blink moves, and the face does not drift.

## What each default face draws

Every face stands on the common design: its eyes at the eyes' places, its mouth centred under them.

| Face | Eyes | Brows | Mouth |
| --- | --- | --- | --- |
| `기본` | `dot` | — | a line 14 wide, smiling 1 deep |
| `기쁨` | `dot` | — | the large bean 28.8 × 15.8, teeth and tongue |
| `슬픔` | `oval` | raised toward the middle | a small wail 18 × 9, its round side up, a tongue |
| `놀람` | `oval-wide`, a little higher | — | a round O 15 × 14, teeth and tongue |
| `화남` | `dot`, a little lower | slanted down toward the middle | a wail 24 × 10 under its teeth band |
| `졸림` | `half-lid` | — | a level line 9 wide |
| `폭소` | `closed-u-deep` | — | the widest bean 38 × 18, teeth and tongue |
| `걱정` | `dot-small` | raised toward the middle | a slit of clenched teeth, both rows |
| `아픔` | `x x` of two straight strokes each | — | a slit 24 × 7 under a deep teeth band |
| `안간힘` | `squeezed` | — | the grimace 34.8 × 10.5, both rows of teeth |
| `writing` | `dot`, a little lower | level | a level line 8 wide |
| `thinking` | `dot`, up and to the left | the left one raised | a line rising to the right |
| `idle` | `dot` | — | a level line 10 wide |

No default face draws blush, tears or a shine; the set holds them for the editor.

## Where the face still differs from the animation

- **The animation draws its mouth fresh each time;** the face blends its 23 drawings into the mouth
  each moment of the sound calls for, so it passes through every one of them and through every mouth
  between, and repeats a mouth only where the sound repeats itself.
- **Expressions in the animation change with the scene;** the face's expressions are its thirteen
  faces, chosen by BMO's emotion and presence.
- **The talking mouth does not lean, squash or tilt** with the head, as the animation's does when BMO
  moves; the face has no head to move.
- **The animation's sound was measured on English voices,** and its mouth by loudness alone; the face
  reads the formants of any voice, Korean's as well, and BMO's briefings carry no timed text today.
