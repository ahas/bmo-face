//! The sound a timed text says at each frame (docs/face.md, 「The mouth while speaking」), for the
//! speaking driver to steer the family of the coming mouth by: the lips meeting, a rounded vowel, an open
//! vowel, a pause.
//!
//! A cue is a run of text and the frames of the sound it is spoken over; its sounds are spread evenly
//! across them, one unit a syllable of Korean or a letter of English, a space a pause. English is read
//! as the animation was measured: P, B and M close the lips, O, U and W round them, A opens them. Korean
//! is read the same way by extrapolation, never measured: a syllable whose first consonant is mieum,
//! bieup, ssangbieup or pieup starts closed, one whose last consonant is mieum, bieup or pieup ends
//! closed, the vowels o, u and their compounds are rounded, and a and ya are open.

use crate::document::Voice;

/// A run of text and the frames of the sound it is spoken over, from the sound's first frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Cue {
  pub from: i64,
  pub to: i64,
  pub text: String,
}

/// The sound of one unit of text: as it starts, through its middle, and as it ends.
type Unit = [Voice; 3];

/// Where Korean's syllables are stored in Unicode, and how each is composed of its letters.
const SYLLABLES: std::ops::RangeInclusive<u32> = 0xac00..=0xd7a3;
const PER_INITIAL: u32 = 21 * 28;
const PER_MEDIAL: u32 = 28;

/// Initial consonants that close the lips: mieum, bieup, ssangbieup, pieup.
const CLOSING_INITIALS: [u32; 4] = [6, 7, 8, 17];
/// Final consonants that close the lips: rieul-mieum, mieum, bieup, bieup-sios, pieup.
const CLOSING_FINALS: [u32; 5] = [10, 16, 17, 18, 26];
/// Vowels that round the lips: o, wa, wae, oe, yo, u, wo, we, wi, yu.
const ROUNDED_MEDIALS: std::ops::RangeInclusive<u32> = 8..=17;
/// Vowels that open the mouth wide: a, ya.
const OPEN_MEDIALS: [u32; 2] = [0, 2];

fn syllable(code: u32) -> Unit {
  let index = code - SYLLABLES.start();
  let (initial, medial, last) =
    (index / PER_INITIAL, (index % PER_INITIAL) / PER_MEDIAL, index % PER_MEDIAL);
  let vowel = if ROUNDED_MEDIALS.contains(&medial) {
    Voice::Rounded
  } else if OPEN_MEDIALS.contains(&medial) {
    Voice::Open
  } else {
    Voice::Other
  };
  let start = if CLOSING_INITIALS.contains(&initial) { Voice::Closing } else { vowel };
  let end = if CLOSING_FINALS.contains(&last) { Voice::Closing } else { vowel };
  [start, vowel, end]
}

fn letter(one: char) -> Voice {
  match one.to_ascii_lowercase() {
    'p' | 'b' | 'm' => Voice::Closing,
    'o' | 'u' | 'w' => Voice::Rounded,
    'a' => Voice::Open,
    _ => Voice::Other,
  }
}

/// The units a text is spoken as; what is neither a letter, a syllable nor a space says nothing.
fn units(text: &str) -> Vec<Unit> {
  text
    .chars()
    .filter_map(|one| {
      let code = one as u32;
      if SYLLABLES.contains(&code) {
        Some(syllable(code))
      } else if one.is_ascii_alphabetic() {
        Some([letter(one); 3])
      } else if one.is_whitespace() {
        Some([Voice::Silence; 3])
      } else {
        None
      }
    })
    .collect()
}

/// The sound the cues say at a frame of the sound; none where no cue stands over it.
pub fn voice_at(cues: &[Cue], frame: i64) -> Option<Voice> {
  let cue = cues.iter().find(|cue| cue.from <= frame && frame < cue.to)?;
  let units = units(&cue.text);
  if units.is_empty() {
    return None;
  }
  // Each unit is spoken over an equal share of the cue's frames, in thirds.
  let thirds = (frame - cue.from) * units.len() as i64 * 3 / (cue.to - cue.from);
  let unit = units[(thirds / 3) as usize];
  Some(unit[(thirds % 3) as usize])
}
