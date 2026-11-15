//! An expression change (docs/face.md, 「Moving between faces」): as the animation changes a face, a cut
//! about a third of the time, otherwise one to three in-between drawings blended by correspondence, each
//! held on twos, now and then after one drawing of anticipation, the face it leaves squashed a little
//! before it moves. Nothing is eased.

use crate::clock::ON_TWOS;
use crate::look::Look;
use crate::random::Random;

/// How often a change is a cut.
const CUT_SHARE: f64 = 1.0 / 3.0;

/// How often a change of one, two or three in-betweens is drawn, as the animation's held in-betweens
/// of two, four and six frames are.
const IN_BETWEENS: [f64; 3] = [0.4, 0.35, 0.25];

/// How often a change starts with a drawing of anticipation, and how far that drawing squashes the face.
const ANTICIPATION_SHARE: f64 = 0.3;
const ANTICIPATION: f64 = 0.85;

/// The drawings shown between one look and the next, each held `ON_TWOS` frames; none for a cut. With
/// `calm`, every change is a cut. A drawing that would cross itself is left out.
pub fn plan(from: &Look, to: &Look, random: &mut Random, calm: bool) -> Vec<Look> {
  let cut = random.chance(CUT_SHARE);
  let count = random.weighted(&IN_BETWEENS).unwrap_or(0) + 1;
  let anticipate = random.chance(ANTICIPATION_SHARE);
  if calm || cut || from == to {
    return Vec::new();
  }
  let anticipation = anticipate.then(|| from.squashed(ANTICIPATION));
  let in_betweens = (1..=count).map(|step| from.blend(to, step as f64 / (count + 1) as f64));
  anticipation
    .into_iter()
    .chain(in_betweens)
    .filter(|look| !look.crosses_itself())
    .collect()
}

/// How many frames a change's in-betweens are shown for.
pub fn frames(drawings: &[Look]) -> i64 {
  drawings.len() as i64 * ON_TWOS
}
