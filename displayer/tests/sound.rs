//! A briefing's sound made ready (docs/face.md, 「Speaking」): its loudness read once a held frame, and
//! an Ogg Opus stream decoded with its trimming.

use bmo_face_core::clock::FPS;
use bmo_face_displayer::sound::{LoudnessMeter, SILENCE_DB, decibels, decode};

const RATE: u32 = 48_000;
const FIXTURE: &[u8] = include_bytes!("fixtures/tone-then-silence.ogg");

fn meter(samples: &[f32], chunk: usize) -> Vec<f64> {
  let mut meter = LoudnessMeter::new(RATE, FPS);
  for part in samples.chunks(chunk) {
    meter.push(part.iter().copied());
  }
  meter.finish()
}

#[test]
fn every_frame_the_sound_reaches_into_has_one_reading() {
  // Two seconds: the frames of the held clock it reaches into, the last one partial.
  let samples = vec![0.5_f32; 2 * RATE as usize];
  let readings = meter(&samples, 1_000);
  assert_eq!(readings.len(), (2.0 * FPS).ceil() as usize);
  assert!(readings.iter().all(|level| (level - decibels(0.5)).abs() < 1e-9));
  assert!(meter(&[], 1).is_empty());
}

#[test]
fn a_frame_reads_the_samples_its_start_rounds_to_up_to_the_next() {
  // A loud first frame and a silent second: the boundary at 48000 / 23.976 rounds to 2002.
  let boundary = (f64::from(RATE) / FPS).round() as usize;
  assert_eq!(boundary, 2002);
  let mut samples = vec![1.0_f32; boundary];
  samples.extend(vec![0.0_f32; boundary]);
  let readings = meter(&samples, 7);
  assert_eq!(readings[0], 0.0);
  assert_eq!(readings[1], SILENCE_DB);
}

#[test]
fn the_reading_does_not_turn_on_how_the_samples_arrive() {
  let samples: Vec<f32> =
    (0..30_000).map(|at| ((at as f32) * 0.01).sin() * (at as f32 / 30_000.0)).collect();
  assert_eq!(meter(&samples, 1), meter(&samples, 4_096));
}

#[test]
fn loudness_is_in_dbfs_with_silence_held() {
  assert_eq!(decibels(1.0), 0.0);
  assert!((decibels(0.5) + 6.0206).abs() < 1e-3);
  assert_eq!(decibels(0.0), SILENCE_DB);
  assert_eq!(decibels(1e-9), SILENCE_DB);
}

#[test]
fn an_ogg_opus_sound_is_decoded_trimmed_and_read() {
  let sound = decode(FIXTURE.to_vec()).expect("the fixture decodes");
  assert_eq!((sound.rate, sound.channels), (RATE, 1));
  // One second exactly, the encoder's pre-skip and end padding trimmed away.
  assert_eq!(sound.frames(), RATE as usize);
  assert_eq!(sound.loudness.len(), FPS.ceil() as usize);
  // The tone, a sine at a sixteenth of the full scale, then silence.
  let tone = &sound.loudness[2..10];
  assert!(
    tone.iter().all(|level| (level - decibels(1.0 / 16.0 / 2.0_f64.sqrt())).abs() < 0.5),
    "{tone:?}"
  );
  let quiet = &sound.loudness[15..];
  assert!(quiet.iter().all(|level| *level < -60.0), "{quiet:?}");
}

#[test]
fn what_is_not_an_ogg_stream_is_refused() {
  assert!(decode(b"not a sound".to_vec()).is_err());
}
