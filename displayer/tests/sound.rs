//! A briefing's sound made ready (docs/face.md, 「Speaking」): an Ogg Opus stream decoded with its
//! trimming, and its first channel heard by the core.

use bmo_face_core::clock::FPS;
use bmo_face_core::hearing::hear;
use bmo_face_displayer::sound::decode;

const RATE: u32 = 48_000;
const FIXTURE: &[u8] = include_bytes!("fixtures/tone-then-silence.ogg");

#[test]
fn an_ogg_opus_sound_is_decoded_trimmed_and_heard_by_the_core() {
  let sound = decode(FIXTURE.to_vec()).expect("the fixture decodes");
  assert_eq!((sound.rate, sound.channels), (RATE, 1));
  // One second exactly, the encoder's pre-skip and end padding trimmed away.
  assert_eq!(sound.frames(), RATE as usize);
  // What is heard is the core's hearing of the samples kept, a reading a frame of the held clock.
  let first: Vec<f32> =
    sound.samples.iter().map(|sample| f32::from(*sample) / f32::from(i16::MAX)).collect();
  assert_eq!(sound.heard.len(), FPS.ceil() as usize);
  assert_eq!(sound.heard.len(), hear(&first, RATE).len());
  // The tone, a sine at a sixteenth of the full scale, then silence.
  let tone = 20.0 * (1.0 / 16.0 / 2.0_f64.sqrt()).log10();
  assert!(
    sound.heard[2..10].iter().all(|frame| (frame.level - tone).abs() < 0.5),
    "{:?}",
    &sound.heard[2..10]
  );
  assert!(sound.heard[15..].iter().all(|frame| frame.level < -60.0));
}

#[test]
fn what_is_not_an_ogg_stream_is_refused() {
  assert!(decode(b"not a sound".to_vec()).is_err());
}
