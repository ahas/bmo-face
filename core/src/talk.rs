//! The speaking driver (docs/face.md, 「The mouth while speaking」): the mouth drawings a sound is spoken
//! with, planned once over the whole sound before it plays. Each drawing is held for whole frames — two
//! as a rule, one or four now and then — and cut to the next, never morphed. A drawing is picked for the
//! sound two frames after it shows, as the animation's mouth leads its voice: the loudness of those
//! frames picks the family and size by the measured relation, a timed text's sound steers the family,
//! and within what is left the pick is seeded chance that never draws the same mouth twice running.

use crate::clock::ON_TWOS;
use crate::document::{Size, TalkFamily, Voice};
use crate::random::Random;
use crate::text::{Cue, voice_at};

/// How many frames a drawing shows before the sound it is drawn for.
pub const LEAD: i64 = 2;

/// How long drawings are held, and how often each: on twos, now and then on ones or fours.
const HOLDS: [(i64, f64); 3] = [(1, 0.1), (ON_TWOS, 0.8), (4, 0.1)];

/// The loudness at or below which a frame is silent: this far below the sound's loudest frame, or
/// quieter than the floor.
const QUIET_BELOW_PEAK: f64 = 35.0;
const QUIET_FLOOR: f64 = -50.0;

/// What the animation's mouth does at each tenth of its voice's loudness, quietest first: how open it
/// stands, as the size scale reads it, and how often it is closed.
const BY_LOUDNESS: [(f64, f64); 10] = [
  (0.472, 0.216),
  (0.469, 0.172),
  (0.534, 0.13),
  (0.539, 0.108),
  (0.602, 0.086),
  (0.597, 0.086),
  (0.679, 0.064),
  (0.718, 0.079),
  (0.718, 0.055),
  (0.754, 0.04),
];

/// How often a frame whose text closes the lips is drawn closed.
const CLOSING_SHARE: f64 = 0.7;

/// How widely the size of a pick spreads around the openness aimed at: wide, as the animation's openness
/// follows its loudness only weakly.
const SIZE_SPREAD: f64 = 0.25;

/// The same spread where a timed text names an open vowel, which draws the mouth wide, and at the edge
/// of a line, where the mouth is small.
const OPEN_SPREAD: f64 = 0.1;
const EDGE_SPREAD: f64 = 0.05;

/// The most a drawing at the edge of a line may open: the first after a pause and the last before one
/// are small, as the animation's opening grows from a line's first drawing and shrinks into its last.
const EDGE_OPENNESS: f64 = Size::Small.openness();

/// A sound to speak: its loudness in dBFS at each frame from its first, the timed text it says where it
/// is known, and the seed its picks are drawn from.
#[derive(Clone, Debug, PartialEq)]
pub struct Speech {
  pub loudness: Vec<f64>,
  pub cues: Vec<Cue>,
  pub seed: u64,
}

/// One drawing of the plan: from which frame of the sound it is drawn for, for how many frames, and which
/// mouth of the set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
  pub at: i64,
  pub hold: i64,
  pub family: usize,
  pub mouth: usize,
}

/// What the coming frames call for.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Call {
  voice: Voice,
  /// The tenth of the sound's loudness the frames stand at; none where they are silent.
  tenth: Option<usize>,
}

/// The loudness of a sound read for its calls: which frames are silent, and the tenth each voiced
/// frame's loudness stands at among the voiced frames.
struct Loudness {
  tenths: Vec<Option<usize>>,
}

impl Loudness {
  fn new(loudness: &[f64]) -> Loudness {
    let peak = loudness.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let quiet = (peak - QUIET_BELOW_PEAK).max(QUIET_FLOOR);
    let mut voiced: Vec<f64> = loudness.iter().copied().filter(|level| *level > quiet).collect();
    voiced.sort_by(f64::total_cmp);
    let tenths = loudness
      .iter()
      .map(|level| {
        (*level > quiet).then(|| {
          let below = voiced.partition_point(|other| other < level);
          (below * 10 / voiced.len().max(1)).min(9)
        })
      })
      .collect();
    Loudness { tenths }
  }

  fn silent(&self, frame: i64) -> bool {
    usize::try_from(frame)
      .ok()
      .and_then(|frame| self.tenths.get(frame))
      .is_none_or(Option::is_none)
  }

  /// The tenth a run of frames stands at, their middle one's, or none where most of them are silent.
  fn tenth(&self, from: i64, to: i64) -> Option<usize> {
    let tenths: Vec<usize> = (from..to)
      .filter_map(|frame| {
        usize::try_from(frame).ok().and_then(|frame| self.tenths.get(frame).copied().flatten())
      })
      .collect();
    if tenths.len() * 2 < (to - from) as usize {
      return None;
    }
    let mut tenths = tenths;
    tenths.sort_unstable();
    tenths.get(tenths.len() / 2).copied()
  }
}

/// The plan of drawings over a whole sound, the last one closing the mouth as the sound ends.
pub fn plan(speech: &Speech, set: &[TalkFamily]) -> Vec<Step> {
  let length = speech.loudness.len() as i64;
  let loudness = Loudness::new(&speech.loudness);
  let mut random = Random::for_use(speech.seed, 1);
  let mut steps: Vec<Step> = Vec::new();
  let mut at = 0;
  let mut edge = true;
  while at < length {
    let weights: Vec<f64> = HOLDS.iter().map(|(_, share)| *share).collect();
    let hold = HOLDS[random.weighted(&weights).unwrap_or(1)].0;
    let tenth = loudness.tenth(at, at + hold);
    let voice = match tenth {
      None => Voice::Silence,
      Some(_) => (at..at + hold).find_map(|frame| voice_at(&speech.cues, frame)).unwrap_or(Voice::Other),
    };
    let last_before_pause = loudness.silent(at + hold) && loudness.silent(at + hold + 1);
    let at_edge = tenth.is_some() && (edge || last_before_pause);
    let previous = steps.last().map(|step| (step.family, step.mouth));
    if let Some((family, mouth)) = pick(set, Call { voice, tenth }, at_edge, previous, &mut random) {
      steps.push(Step {
        at,
        hold,
        family,
        mouth,
      });
    }
    edge = tenth.is_none();
    at += hold;
  }
  let previous = steps.last().map(|step| (step.family, step.mouth));
  if let Some((family, mouth)) = pick(
    set,
    Call {
      voice: Voice::Silence,
      tenth: None,
    },
    false,
    previous,
    &mut random,
  ) {
    steps.push(Step {
      at,
      hold: ON_TWOS,
      family,
      mouth,
    });
  }
  steps
}

/// The mouth one drawing shows: its family by the call, its size by the loudness, by chance among what
/// is left, never the mouth shown just before.
fn pick(
  set: &[TalkFamily],
  call: Call,
  at_edge: bool,
  previous: Option<(usize, usize)>,
  random: &mut Random,
) -> Option<(usize, usize)> {
  let (aim, closed) = match call.tenth {
    None => (Size::Small.openness(), 1.0),
    Some(tenth) => BY_LOUDNESS[tenth],
  };
  let voice = match call.voice {
    Voice::Silence => Voice::Silence,
    Voice::Closing if random.chance(CLOSING_SHARE) => Voice::Closing,
    _ if random.chance(closed) => Voice::Closing,
    Voice::Closing => Voice::Other,
    other => other,
  };
  let aim = match voice {
    Voice::Open => aim.max(Size::Large.openness() - 0.03),
    _ => aim,
  };
  let aim = if at_edge { aim.min(EDGE_OPENNESS) } else { aim };
  let spread = if at_edge {
    EDGE_SPREAD
  } else if voice == Voice::Open {
    OPEN_SPREAD
  } else {
    SIZE_SPREAD
  };
  let candidates = |voiced: &dyn Fn(&TalkFamily) -> bool| -> Vec<((usize, usize), f64)> {
    set
      .iter()
      .enumerate()
      .filter(|(_, family)| voiced(family))
      .flat_map(|(index, family)| {
        let sizes = |size: Size| family.mouths.iter().filter(|mouth| mouth.size == size).count() as f64;
        family.mouths.iter().enumerate().map(move |(place, mouth)| {
          let near = ((mouth.size.openness() - aim) / spread).powi(2);
          ((index, place), family.weight * (-near).exp() / sizes(mouth.size))
        })
      })
      .filter(|(which, _)| Some(*which) != previous)
      .collect()
  };
  // A sound no text names is any voiced sound: every family drawn for one may draw it.
  let drawn_for = |family: &TalkFamily| match voice {
    Voice::Other => family.voices.iter().any(|own| !matches!(own, Voice::Silence | Voice::Closing)),
    named => family.voices(named),
  };
  let mut found = candidates(&drawn_for);
  if found.is_empty() {
    found = candidates(&|_: &TalkFamily| true);
  }
  let weights: Vec<f64> = found.iter().map(|(_, weight)| *weight).collect();
  random.weighted(&weights).map(|index| found[index].0)
}

/// The plan's step showing at a frame of the sound, counted from its first frame: the drawing a frame
/// shows is the one drawn for the sound `LEAD` frames later.
pub fn step_at(steps: &[Step], frame: i64) -> Option<&Step> {
  let sound = frame + LEAD;
  let index = steps.partition_point(|step| step.at <= sound);
  steps
    .get(index.checked_sub(1)?)
    .filter(|step| sound < step.at + step.hold || index == steps.len())
}

/// A sound's loudness sampled for a preview, `frames` long: words of a few syllables, each syllable a
/// rise and fall, with pauses between, as a voice moves.
pub fn sample_loudness(seed: u64, frames: usize) -> Vec<f64> {
  let mut random = Random::for_use(seed, 2);
  let mut loudness = Vec::with_capacity(frames);
  while loudness.len() < frames {
    for _ in 0..random.between(1, 4) {
      let syllable = random.between(4, 7);
      let peak = -32.0 + random.unit() * 18.0;
      for step in 0..syllable {
        let rise = (std::f64::consts::PI * (step as f64 + 0.5) / syllable as f64).sin();
        loudness.push(peak - 18.0 * (1.0 - rise));
      }
    }
    for _ in 0..random.between(4, 12) {
      loudness.push(-90.0);
    }
  }
  loudness.truncate(frames);
  loudness
}
