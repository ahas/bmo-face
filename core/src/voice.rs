//! A voice sampled for the editor's preview, the shots and the tests (docs/face.md, 「Hearing the
//! sound」): about five seconds of the five vowels, hissed and hummed syllables and the pauses between,
//! made by a small formant synthesiser — a falling sawtooth voice through three resonances for a vowel,
//! noise through a double difference for a hiss — so a sound whose every stretch is known can be heard
//! and drawn without a recording.

use crate::random::Random;

/// The first three resonances of a vowel, in hertz, and their bandwidths.
type Formants = [f64; 3];
const BANDWIDTHS: [f64; 3] = [80.0, 100.0, 150.0];

const A: Formants = [750.0, 1_200.0, 2_500.0];
const E: Formants = [500.0, 1_900.0, 2_550.0];
const I: Formants = [300.0, 2_300.0, 3_000.0];
const O: Formants = [500.0, 900.0, 2_400.0];
const U: Formants = [320.0, 800.0, 2_300.0];
/// The lips closed on a hum, as on M.
const M: Formants = [250.0, 1_100.0, 2_300.0];

/// One stretch of the sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stretch {
  Pause,
  /// A vowel, or a hum when it is the lips' own.
  Voiced(Formants),
  /// A hiss, as on S.
  Hiss,
}

/// What the sample says, and for how many milliseconds each stretch: a, i, u, e, o one by one, then
/// "sa", "ma", "so" and "simu".
pub const SAID: [(Stretch, f64); 25] = [
  (Stretch::Pause, 200.0),
  (Stretch::Voiced(A), 320.0),
  (Stretch::Pause, 110.0),
  (Stretch::Voiced(I), 280.0),
  (Stretch::Pause, 110.0),
  (Stretch::Voiced(U), 300.0),
  (Stretch::Pause, 110.0),
  (Stretch::Voiced(E), 280.0),
  (Stretch::Pause, 110.0),
  (Stretch::Voiced(O), 320.0),
  (Stretch::Pause, 260.0),
  (Stretch::Hiss, 150.0),
  (Stretch::Voiced(A), 280.0),
  (Stretch::Pause, 130.0),
  (Stretch::Voiced(M), 90.0),
  (Stretch::Voiced(A), 300.0),
  (Stretch::Pause, 130.0),
  (Stretch::Hiss, 150.0),
  (Stretch::Voiced(O), 300.0),
  (Stretch::Pause, 260.0),
  (Stretch::Hiss, 120.0),
  (Stretch::Voiced(I), 220.0),
  (Stretch::Voiced(M), 80.0),
  (Stretch::Voiced(U), 260.0),
  (Stretch::Pause, 250.0),
];

/// How loud a hum and a hiss stand beside a vowel, and how long every stretch fades in and out.
const HUM: f64 = 0.3;
const HISS: f64 = 0.12;
const FADE_MS: f64 = 25.0;

/// The voice's pitch, falling from the first to the last of it, and the sample's peak.
const PITCH: (f64, f64) = (140.0, 115.0);
const PEAK: f64 = 0.5;

/// A two-pole resonance at unit gain for the lowest frequencies.
#[derive(Clone, Copy, Default)]
struct Resonance {
  gain: f64,
  first: f64,
  second: f64,
  last: f64,
  before: f64,
}

impl Resonance {
  fn tune(&mut self, hertz: f64, bandwidth: f64, rate: f64) {
    let radius = libm::exp(-std::f64::consts::PI * bandwidth / rate);
    self.first = 2.0 * radius * libm::cos(2.0 * std::f64::consts::PI * hertz / rate);
    self.second = -radius * radius;
    self.gain = 1.0 - self.first - self.second;
  }

  fn pass(&mut self, input: f64) -> f64 {
    let output = self.gain * input + self.first * self.last + self.second * self.before;
    (self.before, self.last) = (self.last, output);
    output
  }
}

/// The sample at `rate` samples a second, one channel, its peak at half the full scale.
pub fn sample_voice(rate: u32) -> Vec<f32> {
  let rate_f = f64::from(rate);
  let whole: f64 = SAID.iter().map(|(_, ms)| ms).sum();
  let mut noise = Random::for_use(0, 5);
  let mut resonances = [Resonance::default(); 3];
  let (mut phase, mut last_noise, mut last_step) = (0.0, 0.0, 0.0);
  let mut samples: Vec<f64> = Vec::new();
  let mut elapsed = 0.0;
  for (stretch, ms) in SAID {
    let count = libm::round(ms * rate_f / 1_000.0) as usize;
    if let Stretch::Voiced(formants) = stretch {
      for ((resonance, hertz), bandwidth) in resonances.iter_mut().zip(formants).zip(BANDWIDTHS) {
        resonance.tune(hertz, bandwidth, rate_f);
      }
    }
    for at in 0..count {
      let moment = at as f64 * 1_000.0 / rate_f;
      let fade = (moment / FADE_MS).min((ms - moment) / FADE_MS).clamp(0.0, 1.0);
      let pitch = PITCH.0 + (PITCH.1 - PITCH.0) * (elapsed + moment) / whole;
      phase = (phase + pitch / rate_f).fract();
      let value = match stretch {
        Stretch::Pause => 0.0,
        Stretch::Voiced(formants) => {
          let loud = if formants == M { HUM } else { 1.0 };
          let mut voice = 2.0 * phase - 1.0;
          for resonance in &mut resonances {
            voice = resonance.pass(voice);
          }
          voice * loud * fade
        }
        Stretch::Hiss => {
          // White noise differenced twice leaves its high frequencies.
          let white = noise.unit() * 2.0 - 1.0;
          let step = white - last_noise;
          let hiss = step - last_step;
          (last_noise, last_step) = (white, step);
          hiss * HISS * fade
        }
      };
      samples.push(value);
    }
    elapsed += ms;
  }
  let peak = samples.iter().fold(0.0_f64, |most, value| most.max(value.abs()));
  let scale = if peak > 0.0 { PEAK / peak } else { 0.0 };
  samples.into_iter().map(|value| (value * scale) as f32).collect()
}

/// Where each stretch of the sample starts and ends, in frames of the held clock, with what it is.
pub fn stretches() -> Vec<(i64, i64, Stretch)> {
  let mut from = 0.0;
  SAID
    .iter()
    .map(|(stretch, ms)| {
      let span = (crate::clock::frame_at(from), crate::clock::frame_at(from + ms));
      from += ms;
      (span.0, span.1, *stretch)
    })
    .collect()
}
