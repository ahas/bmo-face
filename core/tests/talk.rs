//! The speaking driver (docs/face.md, 「The mouth while speaking」): drawings held on twos, now and then
//! on ones and fours, cut from one to the next, never the same twice running, led by the loudness and
//! steered by a timed text.

use bmo_face_core::document::{Size, TalkFamily, Voice};
use bmo_face_core::talk::{LEAD, Speech, Step, plan, sample_loudness, step_at};
use bmo_face_core::text::{Cue, voice_at};

use crate::support::document;

fn speech(loudness: Vec<f64>, cues: Vec<Cue>) -> Speech {
  Speech {
    loudness,
    cues,
    seed: 7,
  }
}

fn family<'a>(set: &'a [TalkFamily], step: &Step) -> &'a TalkFamily {
  &set[step.family]
}

#[test]
fn drawings_are_held_on_twos_now_and_then_on_ones_and_fours() {
  let set = document().talk;
  let steps = plan(&speech(sample_loudness(3, 24_000), Vec::new()), &set);
  let count = |hold: i64| steps.iter().filter(|step| step.hold == hold).count() as f64 / steps.len() as f64;
  assert!((count(2) - 0.8).abs() < 0.03, "{}", count(2));
  assert!((count(1) - 0.1).abs() < 0.02 && (count(4) - 0.1).abs() < 0.02);
  // Each drawing starts where the one before ends.
  for pair in steps.windows(2) {
    assert_eq!(pair[0].at + pair[0].hold, pair[1].at);
  }
}

#[test]
fn the_same_mouth_is_never_drawn_twice_running() {
  let set = document().talk;
  let steps = plan(&speech(sample_loudness(5, 6_000), Vec::new()), &set);
  for pair in steps.windows(2) {
    assert_ne!((pair[0].family, pair[0].mouth), (pair[1].family, pair[1].mouth));
  }
}

#[test]
fn silence_closes_the_mouth_and_loudness_opens_it() {
  let set = document().talk;
  let quiet: Vec<f64> = (0..2_400).map(|frame| if frame % 48 < 24 { -90.0 } else { -40.0 }).collect();
  let steps = plan(&speech(quiet.clone(), Vec::new()), &set);
  for step in &steps {
    if (step.at..step.at + step.hold)
      .all(|frame| quiet.get(frame as usize).is_none_or(|level| *level < -60.0))
    {
      assert!(family(&set, step).voices(Voice::Silence), "silent at {}", step.at);
    }
  }
  // Over a sound of every loudness, the loudest tenth draws larger mouths than the quietest.
  let loudness = sample_loudness(9, 24_000);
  let steps = plan(&speech(loudness.clone(), Vec::new()), &set);
  let mut sorted: Vec<f64> = loudness.iter().copied().filter(|level| *level > -60.0).collect();
  sorted.sort_by(f64::total_cmp);
  let (low, high) = (sorted[sorted.len() / 10], sorted[sorted.len() * 9 / 10]);
  let openness = |keep: &dyn Fn(f64) -> bool| {
    let sizes: Vec<f64> = steps
      .iter()
      .filter(|step| {
        loudness.get(step.at as usize).is_some_and(|level| keep(*level))
          && !family(&set, step).voices(Voice::Closing)
      })
      .map(|step| set[step.family].mouths[step.mouth].size.openness())
      .collect();
    sizes.iter().sum::<f64>() / sizes.len() as f64
  };
  assert!(openness(&|level| level >= high) > openness(&|level| level > -60.0 && level <= low) + 0.05);
}

#[test]
fn a_drawing_shows_two_frames_before_the_sound_it_is_drawn_for() {
  let steps = vec![
    Step {
      at: 0,
      hold: 2,
      family: 0,
      mouth: 0,
    },
    Step {
      at: 2,
      hold: 2,
      family: 0,
      mouth: 1,
    },
  ];
  assert_eq!(LEAD, 2);
  assert_eq!(step_at(&steps, -3), None);
  assert_eq!(step_at(&steps, -2), Some(&steps[0]));
  assert_eq!(step_at(&steps, 0), Some(&steps[1]));
}

#[test]
fn a_timed_text_steers_the_family_by_the_sound_it_says() {
  let set = document().talk;
  let loud = vec![-30.0; 480];
  let said = |text: &str| {
    plan(
      &speech(
        loud.clone(),
        vec![Cue {
          from: 0,
          to: 480,
          text: text.repeat(40),
        }],
      ),
      &set,
    )
  };
  let share = |steps: &[Step], voice: Voice| {
    steps.iter().filter(|step| family(&set, step).voices(voice)).count() as f64 / steps.len() as f64
  };
  assert!(share(&said("mmmm"), Voice::Closing) > 0.6);
  assert!(share(&said("oooo"), Voice::Rounded) > 0.6);
  let open = said("aaaa");
  let large = open
    .iter()
    .filter(|step| set[step.family].mouths[step.mouth].size == Size::Large)
    .count();
  assert!(large as f64 / open.len() as f64 > 0.4);
}

#[test]
fn korean_is_read_by_its_letters() {
  let syllable = |initial: u32, medial: u32, last: u32| {
    char::from_u32(0xac00 + initial * 588 + medial * 28 + last).unwrap()
  };
  // mieum with a, then the vowel o alone, then ga ending in mieum.
  let text: String = [syllable(6, 0, 0), syllable(11, 8, 0), syllable(0, 0, 16)].iter().collect();
  let cues = [Cue {
    from: 0,
    to: 9,
    text,
  }];
  let voices: Vec<Voice> = (0..9).map(|frame| voice_at(&cues, frame).unwrap()).collect();
  assert_eq!(voices[0], Voice::Closing);
  assert_eq!(voices[1], Voice::Open);
  assert_eq!(voices[4], Voice::Rounded);
  assert_eq!(voices[8], Voice::Closing);
  assert_eq!(voice_at(&cues, 9), None);
}
