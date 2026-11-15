//! The face the displayer holds, told what is heard (docs/face.md, 「What it hears」 and 「Speaking」):
//! which face stands, which briefing plays, when its first sample is heard, and the document taken.

use std::time::{Duration, Instant};

use bmo_face_core::clock::FPS;
use bmo_face_core::names::{ASLEEP, EMOTIONS};
use bmo_face_core::talk::LEAD;
use bmo_face_displayer::face::Face;
use bmo_face_displayer::gaze::direction;
use bmo_face_displayer::listener::Briefing;
use serde_json::json;

/// The console's clock now, in milliseconds.
const NOW: f64 = 1_800_000_000_000.0;

fn briefing(message_id: i64, face: &str, sent_at: f64) -> Briefing {
  Briefing {
    chat_id: 1,
    message_id,
    face: face.to_owned(),
    sent_at,
  }
}

fn frame() -> Duration {
  Duration::from_secs_f64(1.0 / FPS)
}

#[test]
fn a_fresh_briefing_plays_once_and_a_late_one_not_at_all() {
  let origin = Instant::now();
  let mut face = Face::new(1, origin);
  assert_eq!(face.briefing(briefing(9, EMOTIONS[1], NOW - 120_001.0), NOW, origin), None, "late");
  let id = face.briefing(briefing(10, EMOTIONS[1], NOW - 5_000.0), NOW, origin).expect("fresh");
  assert_eq!(face.briefing(briefing(10, EMOTIONS[1], NOW), NOW, origin), None, "heard before");
  assert_eq!(face.briefing_of(id).map(|briefing| briefing.message_id), Some(10));
}

#[test]
fn the_sound_is_heard_the_lead_after_the_next_frame_or_later_where_the_device_needs() {
  let origin = Instant::now();
  let mut face = Face::new(1, origin);
  let id = face.briefing(briefing(10, EMOTIONS[2], NOW), NOW, origin).expect("fresh");
  let at = origin + Duration::from_secs(3);
  let target = face.speak(id, vec![-20.0; 48], 0.0, at).expect("still the one to speak");
  let ahead = target - at;
  assert!(ahead > frame() * LEAD as u32 && ahead <= frame() * (LEAD as u32 + 1), "{ahead:?}");
  assert_eq!(face.face_at(target), EMOTIONS[2]);

  let id = face.briefing(briefing(11, EMOTIONS[3], NOW), NOW, at).expect("fresh");
  assert_eq!(face.briefing_of(id - 1), None, "the newer briefing takes over");
  let target = face.speak(id, vec![-20.0; 48], 0.5, at).expect("still the one to speak");
  assert!(target - at >= Duration::from_millis(500), "the device's delay is waited out");
}

#[test]
fn a_stop_heard_ends_the_briefing_it_names_alone() {
  let origin = Instant::now();
  let mut face = Face::new(1, origin);
  let id = face.briefing(briefing(10, EMOTIONS[1], NOW), NOW, origin).expect("fresh");
  face.speak(id, vec![-20.0; 480], 0.0, origin);
  face.stopped((1, 99), origin);
  assert!(face.briefing_of(id).is_some());
  face.stopped((1, 10), origin);
  assert!(face.briefing_of(id).is_none());
  assert_eq!(face.speak(id, vec![-20.0; 48], 0.0, origin), None, "a play given up speaks nothing");
}

#[test]
fn presence_is_told_from_the_moment_it_began() {
  let origin = Instant::now();
  let mut face = Face::new(1, origin);
  let at = origin + Duration::from_secs(1);
  face.presence("thinking", NOW + 60_000.0, NOW, at);
  assert_eq!(face.face_at(at), "thinking", "a moment ahead of this clock is now");
  face.presence("idle", NOW - 11.0 * 60_000.0, NOW, at);
  assert_eq!(face.face_at(at), ASLEEP, "idle since eleven minutes ago");
}

#[test]
fn a_document_is_taken_only_when_the_check_passes_it_and_it_names_its_version() {
  let mut face = Face::new(1, Instant::now());
  let mut document = bmo_face_core::default::document().to_json();
  assert!(face.document(&document).is_err(), "no version");
  document["savedAt"] = json!(5);
  face.document(&document).expect("taken");
  assert_eq!(face.saved_at(), Some(5.0));
  let mut broken = document.clone();
  broken["form"] = json!(99);
  broken["savedAt"] = json!(6);
  assert!(face.document(&broken).is_err());
  assert_eq!(face.saved_at(), Some(5.0), "the one drawn stays");
}

#[test]
fn a_gaze_line_is_a_json_object_of_two_numbers() {
  assert_eq!(direction(r#"{"x": -0.5, "y": 1}"#), Ok([-0.5, 1.0]));
  assert!(direction(r#"{"x": "left"}"#).is_err());
  assert!(direction("left").is_err());
}
