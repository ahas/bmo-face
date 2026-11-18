//! The speaking driver (docs/face.md, 「The mouth while speaking」 and 「The eyes while speaking」): what
//! the face does at each frame of a sound, read from what is heard there, planned once over the whole
//! sound before it plays. The level opens the mouth, the first formant opens it further, a low second
//! formant rounds it, a hiss shows the teeth and an open voiced sound the tongue; a timed text bends it
//! toward round or wide where one is given. Each of these follows its aim with a light smoothing, quick to
//! rise and slower to fall, and the eyes follow the level, its sharp starts, its long loud stretches and
//! its hiss. The same sound gives the same face: nothing is drawn by chance.

use crate::clock::{FPS, ON_TWOS};
use crate::hearing::{Heard, decibels};
use crate::mouth::Shape;
use crate::text::{Cue, Voice, voice_at};
use crate::tuning::Tuning;

/// How many frames a drawing shows before the sound it is drawn for.
pub const LEAD: i64 = 2;

/// The share of the sound's frames above the floor that are quieter than its loud level.
const LOUD_SHARE: f64 = 0.95;

/// How much a full hiss closes the mouth, toward the slit its teeth stand in.
const HISS_CLOSES: f64 = 0.7;

/// How far above the voiced bands, in dB, the hiss band must stand to show the teeth fully.
const HISS_SPAN: f64 = 6.0;

/// How far below the energy under it, in dB, the band above a rounded vowel's formants stands where the
/// lips start to round, and how much further where they are fully round.
const ROUND_FROM: f64 = -18.0;
const ROUND_SPAN: f64 = 12.0;

/// Where an open voiced sound starts to show the tongue, and where it shows it whole, in openness.
const TONGUE_FROM: f64 = 0.45;
const TONGUE_SPAN: f64 = 0.35;

/// How fast an eye's start widening falls away, and how long a loud stretch is averaged over before
/// the eyes squint, in milliseconds; and how loud on average a stretch is before they start to.
const ONSET_FALL: f64 = 250.0;
const SUSTAIN: f64 = 1_500.0;
const SQUINT_FROM: f64 = 0.5;

/// A sound to speak: what is heard at each frame from its first, and the timed text it says where it is
/// known.
#[derive(Clone, Debug, PartialEq)]
pub struct Speech {
  pub heard: Vec<Heard>,
  pub cues: Vec<Cue>,
}

/// What the face does at one frame of a sound: how far the talking mouth stands in for the face's
/// resting mouth, the talking mouth's shape, the shape of the last frame heard above quiet, which a face
/// whose own mouth is open closes to, and how far the eyes grow, squint and shimmer, each from nought to
/// one.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
  pub voice: f64,
  pub shape: Shape,
  pub last: Shape,
  pub grow: f64,
  pub squint: f64,
  pub shine: f64,
}

impl Pose {
  /// The pose to a thousandth, so two frames that would draw alike are equal.
  fn settled(self) -> Pose {
    let unit = |value: f64| (value.clamp(0.0, 1.0) * 1000.0).round() / 1000.0;
    let shape = |shape: Shape| Shape {
      open: unit(shape.open),
      round: unit(shape.round),
      teeth: unit(shape.teeth),
      tongue: unit(shape.tongue),
    };
    Pose {
      voice: unit(self.voice),
      shape: shape(self.shape),
      last: shape(self.last),
      grow: unit(self.grow),
      squint: unit(self.squint),
      shine: unit(self.shine),
    }
  }
}

/// Where a frequency stands between two, from nought to one, in octaves.
fn between(hertz: f64, [from, to]: [f64; 2]) -> f64 {
  ((libm::log2(hertz) - libm::log2(from)) / (libm::log2(to) - libm::log2(from))).clamp(0.0, 1.0)
}

/// How a sound's level is read: the level from which the mouth opens, and the sound's own loud level,
/// the one most of its frames above the floor stay under.
struct Scale {
  quiet: f64,
  loud: f64,
}

impl Scale {
  fn of(heard: &[Heard], tuning: &Tuning) -> Scale {
    let mut levels: Vec<f64> =
      heard.iter().map(|one| one.level).filter(|level| *level > tuning.floor).collect();
    levels.sort_by(f64::total_cmp);
    let loud = match levels.len() {
      0 => tuning.floor,
      count => levels[((count - 1) as f64 * LOUD_SHARE).round() as usize],
    };
    Scale {
      quiet: tuning.floor.max(loud - tuning.range),
      loud,
    }
  }

  /// A level's place between quiet and loud, from nought to one.
  fn of_level(&self, level: f64) -> f64 {
    if self.loud <= self.quiet {
      0.0
    } else {
      ((level - self.quiet) / (self.loud - self.quiet)).clamp(0.0, 1.0)
    }
  }
}

/// What one frame calls for, before it is smoothed.
#[derive(Clone, Copy, Debug, Default)]
struct Aim {
  voice: f64,
  level: f64,
  shape: Shape,
  onset: f64,
}

/// What a frame calls for: the mouth's openness from its level and its first formant, its roundness from
/// how little stands above a rounded vowel's formants — held from the frame before where it is heard
/// but not voiced, let go in silence — its teeth from the hiss
/// band standing over the voiced ones, its tongue from an open voiced sound, all bent by a timed text's
/// sound where one is given.
fn aim(heard: &Heard, scale: &Scale, tuning: &Tuning, held_round: f64, voice: Option<Voice>) -> Aim {
  let level = scale.of_level(heard.level);
  let low = heard.power_between(tuning.low_hz[0], tuning.low_hz[1]);
  let mid = heard.power_between(tuning.mid_hz[0], tuning.mid_hz[1]);
  let high = heard.power_between(tuning.high_hz[0], tuning.high_hz[1]);
  let jaw = heard
    .centre_between(tuning.low_hz[0], tuning.low_hz[1])
    .map_or(0.5, |first| between(first, tuning.jaw_hz));
  let round = if heard.voiced {
    let under = heard.power_between(tuning.low_hz[0], tuning.round_hz[0]);
    let over = heard.power_between(tuning.round_hz[0], tuning.round_hz[1]);
    let lips = ((ROUND_FROM - (decibels(over) - decibels(under))) / ROUND_SPAN).clamp(0.0, 1.0);
    (tuning.round * lips).clamp(0.0, 1.0)
  } else if level > 0.0 {
    held_round
  } else {
    0.0
  };
  let hiss = if high > 0.0 {
    ((decibels(high) - decibels(low.max(mid))) / HISS_SPAN * tuning.teeth).clamp(0.0, 1.0)
  } else {
    0.0
  };
  let mut open = tuning.open * level * (1.0 - tuning.jaw + tuning.jaw * jaw) * (1.0 - HISS_CLOSES * hiss);
  let mut round = round;
  match voice {
    Some(Voice::Rounded) => round += (1.0 - round) * tuning.cues,
    Some(Voice::Open) => round -= round * tuning.cues,
    Some(Voice::Closing) => open -= open * tuning.cues,
    _ => {}
  }
  let open = open.clamp(0.0, 1.0);
  let tongue = if heard.voiced {
    ((open - TONGUE_FROM) / TONGUE_SPAN * tuning.tongue).clamp(0.0, 1.0)
  } else {
    0.0
  };
  Aim {
    voice: if heard.level > scale.quiet { 1.0 } else { 0.0 },
    level,
    shape: Shape {
      open,
      round,
      teeth: hiss,
      tongue,
    },
    onset: if level > 0.0 { heard.flux } else { 0.0 },
  }
}

/// One step of a value following its aim: rising over `rise` milliseconds and falling over `fall`, as a
/// one-pole filter a frame long; at nought it stands on its aim at once.
fn follow(value: f64, aim: f64, rise: f64, fall: f64) -> f64 {
  let span = if aim > value { rise } else { fall };
  if span <= 0.0 {
    return aim;
  }
  let frame = 1_000.0 / FPS;
  value + (aim - value) * (1.0 - libm::exp(-frame / span))
}

/// The poses of a whole sound, one a frame from its first.
pub fn plan(speech: &Speech, tuning: &Tuning) -> Vec<Pose> {
  let scale = Scale::of(&speech.heard, tuning);
  let mut now = Pose::default();
  let (mut level, mut onset, mut sustain, mut hiss, mut held_round) = (0.0, 0.0, 0.0, 0.0, 0.0);
  speech
    .heard
    .iter()
    .enumerate()
    .map(|(frame, heard)| {
      let wanted = aim(heard, &scale, tuning, held_round, voice_at(&speech.cues, frame as i64));
      held_round = wanted.shape.round;
      level = follow(level, wanted.level, tuning.attack, tuning.release);
      onset = follow(onset, wanted.onset, 0.0, ONSET_FALL);
      sustain = follow(sustain, wanted.level, SUSTAIN, SUSTAIN);
      hiss = follow(hiss, wanted.shape.teeth, tuning.glide, tuning.glide);
      let shape = Shape {
        open: follow(now.shape.open, wanted.shape.open, tuning.attack, tuning.release),
        round: follow(now.shape.round, wanted.shape.round, tuning.glide, tuning.glide),
        teeth: follow(now.shape.teeth, wanted.shape.teeth, tuning.glide, tuning.glide),
        tongue: follow(now.shape.tongue, wanted.shape.tongue, tuning.glide, tuning.glide),
      };
      now = Pose {
        voice: follow(now.voice, wanted.voice, tuning.glide, tuning.rest),
        shape,
        last: if wanted.voice > 0.0 { shape } else { now.last },
        grow: tuning.eye_pulse * level + tuning.eye_onset * onset,
        squint: tuning.eye_squint * ((sustain - SQUINT_FROM) / (1.0 - SQUINT_FROM)).max(0.0),
        shine: tuning.eye_shine * hiss,
      };
      now.settled()
    })
    .collect()
}

/// The frame of the sound whose pose a frame shows, counted from the sound's first frame: the one `LEAD`
/// frames later, or on twos the first of the pair it falls in.
pub fn shown_at(frame: i64, on_twos: bool) -> i64 {
  let sound = frame + LEAD;
  if on_twos { sound - sound.rem_euclid(ON_TWOS) } else { sound }
}

/// The pose a frame shows, counted from the sound's first frame; the last one past the sound's end, none
/// before its first drawing.
pub fn pose_at(poses: &[Pose], frame: i64, on_twos: bool) -> Option<&Pose> {
  let sound = usize::try_from(shown_at(frame, on_twos)).ok()?;
  poses.get(sound).or(poses.last())
}
