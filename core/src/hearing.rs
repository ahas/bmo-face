//! What the face hears of a sound (docs/face.md, 「Hearing the sound」): a sound's samples read once a
//! frame of the held clock into its level, its spectrum in sixteen bands over the voice's range, where
//! its energy centres, how sharply it has just risen and whether it is voiced. Every caller hands its
//! decoded samples here, so the display, the editor and the native displayer hear one sound alike.
//!
//! Each frame's level is the root mean square of its own samples. Its spectrum is the mean of two
//! Hann-windowed transforms, each about one frame long, centred a quarter and three quarters of the way
//! through it, so the windows overlap by half and every sample weighs alike. Nothing is smoothed here:
//! the speaking driver smooths what it draws from these readings.

use rustfft::algorithm::Radix4;
use rustfft::num_complex::Complex;
use rustfft::{Fft, FftDirection};
use serde_json::{Value, json};

use crate::clock::FPS;
use crate::json::number;

/// How many bands the spectrum is read in.
pub const BANDS: usize = 16;

/// The bands' edges in hertz: sixteen bands of seven sixteenths of an octave each, from 80 Hz to 10,240
/// Hz, rounded to the hertz.
pub const BAND_EDGES: [f64; BANDS + 1] = [
  80.0, 108.0, 147.0, 199.0, 269.0, 364.0, 494.0, 668.0, 905.0, 1_226.0, 1_660.0, 2_248.0, 3_044.0,
  4_123.0, 5_583.0, 7_561.0, 10_240.0,
];

/// The quietest level written: silence.
pub const SILENCE_DB: f64 = -120.0;

/// A frame is voiced when it is louder than this, more of its energy stands below the low split than
/// above the high one, and it crosses nought fewer times a second than this.
const VOICED_FLOOR: f64 = -60.0;
const LOW_SPLIT: f64 = BAND_EDGES[8];
const HIGH_SPLIT: f64 = BAND_EDGES[12];
const VOICED_CROSSINGS: f64 = 3_000.0;

/// What one frame of a sound holds, as the face hears it.
#[derive(Clone, Debug, PartialEq)]
pub struct Heard {
  /// The root mean square of the frame's samples, in dBFS; silence at `SILENCE_DB`.
  pub level: f64,
  /// Each band's share of the sound as the mean square it would have alone, in dBFS.
  pub bands: [f64; BANDS],
  /// The frequency the energy between the first and the last band's edge centres on, in hertz; nought
  /// for silence.
  pub centroid: f64,
  /// How much the spectrum's magnitude rose since the frame before, as a share of its own: nought to one.
  pub flux: f64,
  pub voiced: bool,
}

impl Heard {
  pub fn silent() -> Heard {
    Heard {
      level: SILENCE_DB,
      bands: [SILENCE_DB; BANDS],
      centroid: 0.0,
      flux: 0.0,
      voiced: false,
    }
  }

  /// The mean square of the sound between two frequencies, from the bands: each band counted by the
  /// share of its width, in octaves, that stands between them.
  pub fn power_between(&self, from: f64, to: f64) -> f64 {
    (0..BANDS)
      .map(|band| {
        let (low, high) = (BAND_EDGES[band], BAND_EDGES[band + 1]);
        let overlap = (libm::log2(to.min(high)) - libm::log2(from.max(low))).max(0.0);
        let share = overlap / (libm::log2(high) - libm::log2(low));
        if share > 0.0 { power(self.bands[band]) * share } else { 0.0 }
      })
      .sum()
  }

  /// The frequency the energy between two frequencies centres on, by the bands' middles in octaves;
  /// none where nothing stands between them.
  pub fn centre_between(&self, from: f64, to: f64) -> Option<f64> {
    let mut weighed = 0.0;
    let mut total = 0.0;
    for band in 0..BANDS {
      let (low, high) = (BAND_EDGES[band].max(from), BAND_EDGES[band + 1].min(to));
      if high <= low {
        continue;
      }
      let share = self.power_between(low, high);
      weighed += share * (libm::log2(low) + libm::log2(high)) / 2.0;
      total += share;
    }
    (total > 0.0).then(|| libm::exp2(weighed / total))
  }

  pub fn to_json(&self) -> Value {
    json!({
      "level": number(self.level),
      "bands": self.bands.iter().map(|band| number(*band)).collect::<Vec<_>>(),
      "centroid": number(self.centroid),
      "flux": number(self.flux),
      "voiced": self.voiced,
    })
  }

  /// A frame as `to_json` writes it; none where it is not one.
  pub fn from_json(value: &Value) -> Option<Heard> {
    let bands: Vec<f64> = value["bands"].as_array()?.iter().map(Value::as_f64).collect::<Option<_>>()?;
    Some(Heard {
      level: value["level"].as_f64()?,
      bands: bands.try_into().ok()?,
      centroid: value["centroid"].as_f64()?,
      flux: value["flux"].as_f64()?,
      voiced: value["voiced"].as_bool()?,
    })
  }
}

/// A mean square in dBFS, silence held at the quietest written.
pub fn decibels(power: f64) -> f64 {
  if power > 0.0 { (10.0 * libm::log10(power)).max(SILENCE_DB) } else { SILENCE_DB }
}

/// The mean square a level in dBFS stands for; nought for silence.
pub fn power(decibels: f64) -> f64 {
  if decibels <= SILENCE_DB { 0.0 } else { libm::pow(10.0, decibels / 10.0) }
}

/// The window's length at a rate: the power of two nearest one frame's samples, 2,048 at 48 kHz.
pub fn window_length(rate: u32) -> usize {
  let frame = f64::from(rate) / FPS;
  let above = (frame.max(64.0) as usize).next_power_of_two();
  if above as f64 - frame <= frame - (above / 2) as f64 { above } else { above / 2 }
}

/// The periodic Hann window of a length.
fn hann(length: usize) -> Vec<f32> {
  (0..length)
    .map(|at| (0.5 - 0.5 * libm::cos(2.0 * std::f64::consts::PI * at as f64 / length as f64)) as f32)
    .collect()
}

/// The sample of the frame where the frame `frame` starts: the one its moment rounds to.
fn frame_start(frame: usize, rate: f64) -> usize {
  libm::round(frame as f64 * rate / FPS) as usize
}

/// Reads a sound, one channel of samples at `rate` per second, into one reading for each frame of the
/// held clock it reaches into from its first sample, the last one partial.
pub fn hear(samples: &[f32], rate: u32) -> Vec<Heard> {
  let rate_f = f64::from(rate);
  let frames = libm::ceil(samples.len() as f64 / rate_f * FPS) as usize;
  let length = window_length(rate);
  let window = hann(length);
  let squares: f64 = window.iter().map(|weight| f64::from(*weight) * f64::from(*weight)).sum();
  // Two transforms' powers are averaged; one side of the spectrum stands for both.
  let scale = 2.0 / (length as f64 * squares);
  let fft = Radix4::<f32>::new(length, FftDirection::Forward);
  let mut scratch = vec![Complex::new(0.0, 0.0); fft.get_inplace_scratch_len()];
  let mut buffer = vec![Complex::new(0.0, 0.0); length];
  let bin = rate_f / length as f64;
  // The bins between the first and the last band's edge, below the half the rate, each with its band.
  let bins: Vec<(usize, usize)> = (1..=length / 2)
    .filter_map(|index| {
      let hertz = index as f64 * bin;
      let band = BAND_EDGES.windows(2).position(|edges| edges[0] <= hertz && hertz < edges[1])?;
      (hertz < rate_f / 2.0).then_some((index, band))
    })
    .collect();
  let sample = |at: i64| usize::try_from(at).ok().and_then(|at| samples.get(at)).copied().unwrap_or(0.0);
  let mut before = vec![0.0_f64; bins.len()];
  let mut heard = Vec::with_capacity(frames);

  for frame in 0..frames {
    let start = frame_start(frame, rate_f).min(samples.len());
    let end = frame_start(frame + 1, rate_f).min(samples.len());
    let own = &samples[start..end];
    let sum: f64 = own.iter().map(|value| f64::from(*value) * f64::from(*value)).sum();
    let level = if own.is_empty() { SILENCE_DB } else { decibels(sum / own.len() as f64) };
    let crossings = (start.max(1)..end).filter(|at| (samples[at - 1] < 0.0) != (samples[*at] < 0.0)).count();
    let crossings = if own.is_empty() { 0.0 } else { crossings as f64 * rate_f / own.len() as f64 };

    let mut powers = vec![0.0_f64; length / 2 + 1];
    for quarter in [1.0, 3.0] {
      let centre = start as f64 + libm::round((end - start) as f64 * quarter / 4.0);
      let from = centre as i64 - (length / 2) as i64;
      for (at, slot) in buffer.iter_mut().enumerate() {
        *slot = Complex::new(sample(from + at as i64) * window[at], 0.0);
      }
      fft.process_with_scratch(&mut buffer, &mut scratch);
      for (power, value) in powers.iter_mut().zip(&buffer) {
        *power += f64::from(value.norm_sqr()) / 2.0;
      }
    }

    let mut bands = [0.0_f64; BANDS];
    let (mut weighed, mut total, mut low, mut high) = (0.0, 0.0, 0.0, 0.0);
    let (mut rise, mut magnitude) = (0.0, 0.0);
    for ((index, band), last) in bins.iter().zip(before.iter_mut()) {
      let power = powers[*index] * scale;
      let hertz = *index as f64 * bin;
      bands[*band] += power;
      weighed += hertz * power;
      total += power;
      if hertz < LOW_SPLIT {
        low += power;
      } else if hertz >= HIGH_SPLIT {
        high += power;
      }
      let now = power.sqrt();
      rise += (now - *last).max(0.0);
      magnitude += now;
      *last = now;
    }

    heard.push(Heard {
      level,
      bands: bands.map(decibels),
      centroid: if total > 0.0 && level > SILENCE_DB { weighed / total } else { 0.0 },
      flux: if magnitude > 0.0 { (rise / magnitude).min(1.0) } else { 0.0 },
      voiced: level > VOICED_FLOOR && low > high && crossings < VOICED_CROSSINGS,
    });
  }
  heard
}
