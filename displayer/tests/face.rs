//! The face the displayer holds, told what is heard (docs/face.md, 「What it hears」 and 「Speaking」):
//! which face stands, which briefing plays and which waits, the stops, when the first sample is heard,
//! and the document taken.

use std::time::{Duration, Instant};

use bmo_face_core::clock::FPS;
use bmo_face_core::hearing::{BANDS, Heard};
use bmo_face_core::names::{ASLEEP, EMOTIONS, FALLBACK, IDLE};
use bmo_face_core::talk::LEAD;
use bmo_face_displayer::face::Face;
use bmo_face_displayer::gaze::direction;
use bmo_face_displayer::listener::{Briefing, Played};
use serde_json::json;

/// The engine's clock now, in milliseconds.
const NOW: f64 = 1_800_000_000_000.0;

fn briefing(id: i64, chat_id: i64, face: &str, ended_at: f64) -> Briefing {
  Briefing {
    id,
    chat_id,
    played: Played::Summary,
    face: face.to_owned(),
    ended_at,
  }
}

/// A sound `frames` long, every frame of it heard as a steady voice at −20 dBFS.
fn voice(frames: usize) -> Vec<Heard> {
  let steady = Heard {
    level: -20.0,
    bands: [-30.0; BANDS],
    centroid: 700.0,
    flux: 0.0,
    voiced: true,
  };
  vec![steady; frames]
}

fn frame() -> Duration {
  Duration::from_secs_f64(1.0 / FPS)
}

/// The briefing a play speaks, by its id.
fn spoken(face: &Face, play: u64) -> Option<i64> {
  face.briefing_of(play).map(|briefing| briefing.id)
}

#[test]
fn a_fresh_briefing_plays_once_and_a_late_one_not_at_all() {
  let origin = Instant::now();
  let mut face = Face::new(1, origin);
  assert_eq!(face.briefing(briefing(9, 1, EMOTIONS[1], NOW - 120_001.0), NOW), None, "late");
  let play = face.briefing(briefing(10, 1, EMOTIONS[1], NOW - 5_000.0), NOW).expect("fresh");
  assert_eq!(spoken(&face, play), Some(10));
  face.ended(play, origin);
  assert_eq!(face.briefing(briefing(10, 1, EMOTIONS[1], NOW), NOW), None, "heard before");
}

#[test]
fn a_briefing_heard_while_another_speaks_waits_and_follows_it() {
  let origin = Instant::now();
  let mut face = Face::new(1, origin);
  let first = face.briefing(briefing(10, 1, EMOTIONS[1], NOW), NOW).expect("fresh");
  assert_eq!(face.briefing(briefing(11, 1, EMOTIONS[2], NOW), NOW), None, "waits");
  assert_eq!(face.briefing(briefing(12, 2, EMOTIONS[3], NOW), NOW), None, "waits");
  assert_eq!(face.waiting(), [11, 12]);
  assert_eq!(spoken(&face, first), Some(10), "the first still speaks");
  let second = face.ended(first, origin).expect("the next follows");
  assert_eq!(spoken(&face, second), Some(11));
  assert_eq!(face.ended(first, origin), None, "a play already ended ends nothing more");
  let third = face.ended(second, origin).expect("the last follows");
  assert_eq!(spoken(&face, third), Some(12));
  assert_eq!(face.ended(third, origin), None);
}

#[test]
fn the_sound_is_heard_the_lead_after_the_next_frame_or_later_where_the_device_needs() {
  let origin = Instant::now();
  let mut face = Face::new(1, origin);
  let id = face.briefing(briefing(10, 1, EMOTIONS[2], NOW), NOW).expect("fresh");
  let at = origin + Duration::from_secs(3);
  let target = face.speak(id, voice(48), 0.0, at).expect("still the one to speak");
  let ahead = target - at;
  assert!(ahead > frame() * LEAD as u32 && ahead <= frame() * (LEAD as u32 + 1), "{ahead:?}");
  assert_eq!(face.face_at(target), EMOTIONS[2]);

  face.briefing(briefing(11, 1, EMOTIONS[3], NOW), NOW);
  let id = face.ended(id, at).expect("the next follows");
  let target = face.speak(id, voice(48), 0.5, at).expect("still the one to speak");
  assert!(target - at >= Duration::from_millis(500), "the device's delay is waited out");
}

#[test]
fn a_stop_silences_its_room_and_what_of_it_ended_before() {
  let origin = Instant::now();
  let mut face = Face::new(1, origin);
  let id = face.briefing(briefing(10, 1, EMOTIONS[1], NOW - 1_000.0), NOW).expect("fresh");
  face.speak(id, voice(480), 0.0, origin);
  face.briefing(briefing(11, 1, EMOTIONS[1], NOW - 900.0), NOW);
  face.briefing(briefing(12, 2, EMOTIONS[1], NOW - 800.0), NOW);

  // Another room's stop leaves this room's briefings.
  assert_eq!(face.silence(Some(9), NOW - 500.0, NOW, origin), None);
  assert_eq!(spoken(&face, id), Some(10));

  // The room's stop ends the one speaking and takes away the one of it waiting; the other room's follows.
  let next = face.silence(Some(1), NOW - 500.0, NOW, origin).expect("the other room's follows");
  assert_eq!(spoken(&face, id), None);
  assert_eq!(face.speak(id, voice(48), 0.0, origin), None, "a play given up speaks nothing");
  assert_eq!(spoken(&face, next), Some(12));
  assert!(face.waiting().is_empty());

  // A briefing of the room that ended before its stop is never spoken; one that ended after it is.
  face.ended(next, origin);
  assert_eq!(
    face.briefing(briefing(13, 1, EMOTIONS[1], NOW - 600.0), NOW),
    None,
    "ended before the stop"
  );
  assert!(
    face.briefing(briefing(14, 1, EMOTIONS[1], NOW - 400.0), NOW).is_some(),
    "ended after it"
  );
}

#[test]
fn a_stop_of_every_room_silences_them_all() {
  let origin = Instant::now();
  let mut face = Face::new(1, origin);
  let id = face.briefing(briefing(10, 1, EMOTIONS[1], NOW - 1_000.0), NOW).expect("fresh");
  face.briefing(briefing(11, 2, EMOTIONS[1], NOW - 900.0), NOW);
  assert_eq!(face.silence(None, NOW - 500.0, NOW, origin), None);
  assert_eq!(spoken(&face, id), None);
  assert!(face.waiting().is_empty());
  assert_eq!(
    face.briefing(briefing(12, 3, EMOTIONS[1], NOW - 600.0), NOW),
    None,
    "ended before the stop"
  );
}

#[test]
fn no_presence_is_heard_so_the_face_rests_goes_blank_and_falls_asleep_as_the_core_reads_a_rest() {
  let origin = Instant::now();
  let mut face = Face::new(1, origin);
  assert_eq!(face.face_at(origin + Duration::from_secs(60)), FALLBACK);
  assert_eq!(face.face_at(origin + Duration::from_secs(11 * 60)), IDLE);
  assert_eq!(face.face_at(origin + Duration::from_secs(31 * 60)), ASLEEP);
  // A briefing still speaks with its face, and the fallback where it names none.
  let id = face.briefing(briefing(10, 1, FALLBACK, NOW), NOW).expect("fresh");
  let at = origin + Duration::from_secs(31 * 60);
  let target = face.speak(id, voice(48), 0.0, at).expect("still the one to speak");
  assert_eq!(face.face_at(target), FALLBACK);
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
