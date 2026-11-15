//! The seeded chance every driver draws from (docs/face.md, 「The face's core」): one seed gives one
//! sequence, so a caller that hands the same seed and the same events is handed the same drawings.

/// SplitMix64, a small generator whose every seed gives a well-spread sequence.
#[derive(Clone, Debug)]
pub struct Random(u64);

impl Random {
  pub fn new(seed: u64) -> Random {
    Random(seed)
  }

  /// A generator of its own for one use of a seed, so two drivers on one seed do not draw alike.
  pub fn for_use(seed: u64, purpose: u64) -> Random {
    let mut mixed = Random(seed ^ purpose.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    mixed.draw();
    mixed
  }

  pub fn draw(&mut self) -> u64 {
    self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = self.0;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
  }

  /// A number from nought up to one.
  pub fn unit(&mut self) -> f64 {
    (self.draw() >> 11) as f64 / (1u64 << 53) as f64
  }

  /// Whether a thing of chance `share` happens.
  pub fn chance(&mut self, share: f64) -> bool {
    self.unit() < share
  }

  /// A whole number from `low` to `high`, both included.
  pub fn between(&mut self, low: i64, high: i64) -> i64 {
    low + (self.unit() * (high - low + 1) as f64).floor() as i64
  }

  /// An index picked by weight; none where no weight is above nought.
  pub fn weighted(&mut self, weights: &[f64]) -> Option<usize> {
    let total: f64 = weights.iter().filter(|weight| **weight > 0.0).sum();
    if total <= 0.0 {
      return None;
    }
    let mut left = self.unit() * total;
    for (index, weight) in weights.iter().enumerate() {
      if *weight <= 0.0 {
        continue;
      }
      if left < *weight {
        return Some(index);
      }
      left -= weight;
    }
    weights.iter().rposition(|weight| *weight > 0.0)
  }
}
