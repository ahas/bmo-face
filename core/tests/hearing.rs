//! What the face hears of a sound (docs/face.md, 「Hearing the sound」): a reading a frame of the held
//! clock — its level, its spectrum in sixteen bands, its centroid, its flux and whether it is voiced — the
//! same for the same samples, and written beside the WebAssembly build's test as its fixture.

use bmo_face_core::clock::FPS;
use bmo_face_core::hearing::{BAND_EDGES, BANDS, Heard, SILENCE_DB, hear, window_length};
use bmo_face_core::random::Random;
use bmo_face_core::voice::sample_voice;
use serde_json::{Value, json};

const RATE: u32 = 48_000;

/// A sine of an amplitude at a frequency, `seconds` long.
pub fn sine(hertz: f64, amplitude: f64, seconds: f64, rate: u32) -> Vec<f32> {
  let count = (seconds * f64::from(rate)) as usize;
  (0..count)
    .map(|at| (amplitude * (2.0 * std::f64::consts::PI * hertz * at as f64 / f64::from(rate)).sin()) as f32)
    .collect()
}

/// Noise differenced twice, which leaves its high frequencies: a hiss.
pub fn hiss(amplitude: f64, seconds: f64) -> Vec<f32> {
  let mut random = Random::new(11);
  let (mut last, mut step) = (0.0, 0.0);
  (0..(seconds * f64::from(RATE)) as usize)
    .map(|_| {
      let white = random.unit() * 2.0 - 1.0;
      let next = white - last;
      let out = next - step;
      (last, step) = (white, next);
      (out * amplitude / 4.0) as f32
    })
    .collect()
}

fn band_of(hertz: f64) -> usize {
  BAND_EDGES.windows(2).position(|edges| edges[0] <= hertz && hertz < edges[1]).unwrap()
}

#[test]
fn silence_is_heard_as_silence_every_frame_it_reaches_into() {
  let heard = hear(&vec![0.0; RATE as usize], RATE);
  assert_eq!(heard.len(), FPS.ceil() as usize);
  assert!(heard.iter().all(|frame| *frame == Heard::silent()));
  assert!(hear(&[], RATE).is_empty());
}

#[test]
fn a_frame_reads_its_level_from_the_samples_its_start_rounds_to_up_to_the_next() {
  // A loud first frame and a silent second: the boundary at 48000 / 23.976 rounds to 2002.
  let boundary = (f64::from(RATE) / FPS).round() as usize;
  assert_eq!(boundary, 2_002);
  let mut samples = vec![1.0_f32; boundary];
  samples.extend(vec![0.0_f32; boundary]);
  let heard = hear(&samples, RATE);
  assert_eq!(heard[0].level, 0.0);
  assert_eq!(heard[1].level, SILENCE_DB);
}

#[test]
fn a_tone_is_heard_at_its_level_in_its_own_band_and_voiced() {
  let heard = hear(&sine(600.0, 0.5, 1.0, RATE), RATE);
  let level = 20.0 * (0.5 / 2.0_f64.sqrt()).log10();
  let band = band_of(600.0);
  for frame in &heard[2..20] {
    assert!((frame.level - level).abs() < 0.05, "{}", frame.level);
    assert!((frame.bands[band] - level).abs() < 0.5, "{:?}", frame.bands);
    for (other, reading) in frame.bands.iter().enumerate() {
      if other.abs_diff(band) >= 3 {
        assert!(*reading < level - 40.0, "band {other}: {reading}");
      }
    }
    assert!((frame.centroid - 600.0).abs() < 30.0, "{}", frame.centroid);
    assert!(frame.voiced);
    assert!(frame.flux < 0.05, "{}", frame.flux);
  }
  // Its start is sharp.
  assert!(heard[0].flux > 0.9);
}

#[test]
fn a_hiss_is_heard_high_and_unvoiced() {
  let heard = hear(&hiss(0.3, 1.0), RATE);
  for frame in &heard[2..20] {
    assert!(!frame.voiced);
    assert!(frame.centroid > 5_000.0, "{}", frame.centroid);
    assert!(frame.bands[BANDS - 1] > frame.bands[band_of(600.0)] + 20.0);
  }
}

#[test]
fn a_tone_reads_alike_at_every_rate_with_a_window_near_its_frame() {
  assert_eq!((window_length(48_000), window_length(44_100), window_length(16_000)), (2_048, 2_048, 512));
  let band = band_of(600.0);
  let at = |rate: u32| hear(&sine(600.0, 0.5, 1.0, rate), rate)[10].bands[band];
  assert!((at(48_000) - at(16_000)).abs() < 1.0);
  assert!((at(48_000) - at(44_100)).abs() < 1.0);
  // A band above half the rate stands silent.
  assert_eq!(hear(&sine(600.0, 0.5, 1.0, 16_000), 16_000)[10].bands[BANDS - 1], SILENCE_DB);
}

/// The fixture the WebAssembly build's test reads: the first second and a half of the sampled voice at
/// 16 kHz as 16-bit samples, and what the native build hears of them.
const FIXTURE: &str = "tests/fixtures/heard.json";
const FIXTURE_RATE: u32 = 16_000;

fn fixture_samples() -> Vec<i16> {
  sample_voice(FIXTURE_RATE)
    .iter()
    .take(3 * FIXTURE_RATE as usize / 2)
    .map(|sample| (sample * 32_768.0).round().clamp(-32_768.0, 32_767.0) as i16)
    .collect()
}

#[test]
fn the_fixture_the_webassembly_build_is_held_to_is_what_this_build_hears() {
  let samples = fixture_samples();
  let floats: Vec<f32> = samples.iter().map(|sample| f32::from(*sample) / 32_768.0).collect();
  let heard: Vec<Value> = hear(&floats, FIXTURE_RATE).iter().map(Heard::to_json).collect();
  let path = format!("{}/{FIXTURE}", env!("CARGO_MANIFEST_DIR"));
  if std::env::var_os("BMO_FACE_WRITE_FIXTURE").is_some() {
    let written = json!({ "rate": FIXTURE_RATE, "samples": samples, "heard": heard });
    std::fs::write(&path, format!("{}\n", serde_json::to_string(&written).unwrap())).unwrap();
  }
  let fixture: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
  assert_eq!(fixture["rate"], json!(FIXTURE_RATE));
  assert_eq!(fixture["samples"], json!(samples), "the sampled voice changed: write the fixture again");
  assert_eq!(fixture["heard"], json!(heard));
}
