//! The speaking driver (docs/face.md, 「The mouth while speaking」): the mouth read off the sound — closed
//! in silence, open on a low formant, rounded on a low second formant, the teeth on a hiss — followed
//! quickly up and more slowly down, two frames ahead of the sound, every frame or on twos, the same for
//! the same samples, and bent by a timed text where one is given.

use bmo_face_core::hearing::hear;
use bmo_face_core::talk::{LEAD, Pose, Speech, plan, pose_at, shown_at};
use bmo_face_core::text::{Cue, Voice, voice_at};
use bmo_face_core::tuning::Tuning;
use bmo_face_core::voice::{SAID, Stretch, sample_voice, stretches};

use crate::hearing::{hiss, sine};

const RATE: u32 = 48_000;

fn poses_of(samples: &[f32]) -> Vec<Pose> {
  poses_with(samples, Vec::new())
}

fn poses_with(samples: &[f32], cues: Vec<Cue>) -> Vec<Pose> {
  plan(
    &Speech {
      heard: hear(samples, RATE),
      cues,
    },
    &Tuning::default(),
  )
}

/// A sound of a stretch of silence, then `sound`, then silence again.
fn framed(sound: Vec<f32>) -> Vec<f32> {
  let quiet = vec![0.0; RATE as usize / 2];
  [quiet.clone(), sound, quiet].concat()
}

#[test]
fn silence_keeps_the_mouth_closed_and_the_face_its_own() {
  let poses = poses_of(&vec![0.0; RATE as usize]);
  assert!(poses.iter().all(|pose| *pose == Pose::default()));
}

#[test]
fn a_tone_in_the_low_formant_band_opens_the_mouth_and_its_end_closes_it() {
  let poses = poses_of(&framed(sine(700.0, 0.3, 1.0, RATE)));
  for pose in &poses[16..32] {
    assert!(pose.shape.open > 0.7, "{pose:?}");
    assert!(pose.voice > 0.95 && pose.shape.teeth < 0.05);
  }
  let last = poses.last().unwrap();
  assert!(last.shape.open < 0.01 && last.voice < 0.2, "{last:?}");
}

#[test]
fn a_hiss_alone_draws_the_slit_of_teeth_together() {
  let poses = poses_of(&framed(hiss(0.3, 0.5)));
  for pose in &poses[16..24] {
    assert!(pose.shape.teeth > 0.8, "{pose:?}");
    assert!(pose.shape.open < 0.35, "{pose:?}");
    assert_eq!(pose.shape.tongue, 0.0);
  }
}

#[test]
fn a_rising_level_opens_the_mouth_ever_wider() {
  // From −60 to −6 dBFS over three seconds at one pitch.
  let count = 3 * RATE as usize;
  let samples: Vec<f32> = sine(700.0, 1.0, 3.0, RATE)
    .iter()
    .enumerate()
    .map(|(at, sample)| sample * 10f32.powf((-60.0 + 54.0 * at as f32 / count as f32) / 20.0))
    .collect();
  let poses = poses_of(&samples);
  for pair in poses[1..poses.len() - 1].windows(2) {
    assert!(pair[1].shape.open >= pair[0].shape.open, "{:?} then {:?}", pair[0], pair[1]);
  }
  assert!(poses[poses.len() - 2].shape.open > 0.8);
}

#[test]
fn the_sampled_voice_draws_each_of_its_sounds_as_its_own_mouth() {
  let poses = poses_of(&sample_voice(RATE));
  let mut seen = std::collections::HashMap::<&str, usize>::new();
  for (from, to, stretch) in stretches() {
    // The middle of the stretch, past the smoothing of its start.
    let middle: Vec<&Pose> = poses[(from + 3) as usize..(to - 1).max(from + 4) as usize].iter().collect();
    let each = |test: &dyn Fn(&Pose) -> bool| middle.iter().all(|pose| test(pose));
    let name = match stretch {
      Stretch::Pause if to - from >= 6 => {
        let end = &poses[(to - 1) as usize];
        assert!(end.shape.open < 0.05 && end.voice < 0.7, "pause to {to}: {end:?}");
        "pause"
      }
      Stretch::Hiss => {
        assert!(each(&|pose| pose.shape.teeth > 0.6 && pose.shape.open < 0.35), "hiss {from}: {middle:?}");
        "hiss"
      }
      Stretch::Voiced([first, second, _]) if to - from >= 7 => {
        let open = |least: f64| each(&|pose| pose.shape.open > least);
        match (first, second) {
          (750.0, _) => assert!(open(0.75) && each(&|pose| pose.shape.round < 0.3), "a {from}: {middle:?}"),
          (300.0, 2_300.0) => assert!(open(0.4) && each(&|pose| pose.shape.round < 0.15), "i {from}"),
          (320.0, 800.0) => assert!(open(0.4) && each(&|pose| pose.shape.round > 0.8), "u {from}"),
          (500.0, 1_900.0) => assert!(open(0.55) && each(&|pose| pose.shape.round < 0.2), "e {from}"),
          (500.0, 900.0) => assert!(open(0.6) && each(&|pose| pose.shape.round > 0.35), "o {from}"),
          _ => {}
        }
        "vowel"
      }
      _ => "short",
    };
    *seen.entry(name).or_default() += 1;
  }
  assert_eq!(SAID.len(), seen.values().sum::<usize>());
  assert!(seen["pause"] >= 5 && seen["hiss"] == 3 && seen["vowel"] >= 8, "{seen:?}");
}

#[test]
fn the_mouth_opens_faster_than_it_closes() {
  let poses = poses_of(&framed(sine(700.0, 0.3, 1.0, RATE)));
  let on = (0..poses.len()).find(|frame| poses[*frame].shape.open > 0.0).unwrap();
  let off = (on..poses.len()).find(|frame| poses[*frame].shape.open < poses[frame - 1].shape.open - 0.1).unwrap();
  let full = poses[on + 4].shape.open;
  // A frame in, it has risen most of the way; a frame out, it has fallen half of it.
  assert!(poses[on + 1].shape.open > 0.8 * full);
  let fallen = (poses[off - 1].shape.open - poses[off].shape.open) / poses[off - 1].shape.open;
  assert!(fallen < 0.8, "{fallen}");
}

#[test]
fn the_same_samples_give_the_same_poses() {
  let samples = sample_voice(RATE);
  assert_eq!(poses_of(&samples), poses_of(&samples));
}

#[test]
fn a_frame_shows_the_pose_two_frames_ahead_every_frame_or_on_twos() {
  let poses: Vec<Pose> = (0..10)
    .map(|frame| Pose {
      voice: frame as f64 / 10.0,
      ..Pose::default()
    })
    .collect();
  assert_eq!(LEAD, 2);
  assert_eq!(pose_at(&poses, -3, false), None);
  assert_eq!(pose_at(&poses, -2, false), Some(&poses[0]));
  assert_eq!(pose_at(&poses, 3, false), Some(&poses[5]));
  assert_eq!(pose_at(&poses, 3, true), Some(&poses[4]));
  assert_eq!(pose_at(&poses, 40, false), poses.last());
  let shown: Vec<i64> = (-2..4).map(|frame| shown_at(frame, true)).collect();
  assert_eq!(shown, [0, 0, 2, 2, 4, 4]);
}

#[test]
fn a_timed_text_bends_the_mouth_round_or_wide_but_the_sound_still_opens_it() {
  let tone = framed(sine(700.0, 0.3, 1.0, RATE));
  let cue = |text: &str| Cue {
    from: 0,
    to: 60,
    text: text.repeat(20),
  };
  let plain = poses_of(&tone);
  let rounded = poses_with(&tone, vec![cue("oooo")]);
  let closing = poses_with(&tone, vec![cue("mmmm")]);
  for frame in 16..30 {
    assert!(rounded[frame].shape.round > plain[frame].shape.round + 0.3, "{frame}");
    assert!(closing[frame].shape.open < plain[frame].shape.open);
    assert!(closing[frame].shape.open > 0.2);
  }
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
