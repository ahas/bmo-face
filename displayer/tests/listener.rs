//! The voice engine's event stream as the displayer reads it (docs/face.md, 「What it hears」; bmo-voice's
//! `docs/boundary-api.md`, 「Events」 and 「Briefings」): which briefings a listener plays by itself and the
//! face each shows, the stops, the cursor, freshness and the growing waits. The events are written in the
//! shape the engine's `docs/openapi.json` states: `id`, `kind`, `at` and the member under `data`.

use std::time::Duration;

use bmo_face_core::names::{EMOTIONS, FALLBACK};
use bmo_face_displayer::listener::{Backoff, Briefing, Cursor, Heard, Played, heard, is_fresh};
use serde_json::{Value, json};

/// A summary the engine spoke and ended done, which plays by itself, as `GET /briefings/{briefingId}`
/// answers it.
fn summary() -> Value {
  json!({
    "id": 41, "chatId": 3, "messageId": 900, "kind": "summary", "title": "a title", "voiceName": "voice-a",
    "text": "a line", "emotion": null, "synthesisId": 77, "state": "done", "durationMs": 1200, "plays": true,
    "awaitsLine": false, "voiceMessageId": null, "startedAt": 1_000, "finishedAt": 2_000
  })
}

fn event(kind: &str, data: Value) -> Heard {
  let frame = json!({ "id": 12, "kind": kind, "at": 2_500, "data": data }).to_string();
  heard(Some(kind), Some(&frame))
}

fn ended(face: &str) -> Heard {
  Heard::Briefing(Briefing {
    id: 41,
    chat_id: 3,
    played: Played::Summary,
    face: face.to_owned(),
    ended_at: 2_000.0,
  })
}

#[test]
fn a_summary_ended_done_that_plays_by_itself_is_heard_with_the_face_of_its_emotion() {
  assert_eq!(event("briefing.ended", summary()), ended(FALLBACK), "no emotion");
  let (joy, sad) = (EMOTIONS[1], EMOTIONS[2]);
  let mut joyful = summary();
  joyful["emotion"] = json!(joy);
  assert_eq!(event("briefing.ended", joyful), ended(joy));
  let mut unknown = summary();
  unknown["emotion"] = json!("unknown");
  assert_eq!(event("briefing.ended", unknown), ended(FALLBACK));
  // An expression the briefing does not carry is not read: the emotion alone names the face.
  let mut named = summary();
  named["expression"] = json!(sad);
  assert_eq!(event("briefing.ended", named), ended(FALLBACK));
}

#[test]
fn a_briefing_that_ended_with_no_moment_of_its_own_ended_at_its_event() {
  let mut briefing = summary();
  briefing["finishedAt"] = Value::Null;
  let Heard::Briefing(heard) = event("briefing.ended", briefing) else { panic!("heard") };
  assert_eq!(heard.ended_at, 2_500.0);
}

#[test]
fn only_a_done_summary_the_engine_spoke_and_that_plays_by_itself_is_played() {
  for (field, value) in [
    ("kind", json!("full")),
    ("state", json!("failed")),
    ("state", json!("stopped")),
    ("plays", json!(false)),
    ("synthesisId", Value::Null),
  ] {
    let mut briefing = summary();
    briefing[field] = value.clone();
    assert_eq!(event("briefing.ended", briefing), Heard::Other, "{field} {value}");
  }
  assert_eq!(event("briefing.created", summary()), Heard::Other, "not ended");
  assert_eq!(event("briefing.changed", summary()), Heard::Other, "a line named");
}

// "an agent's speech ended done is heard from every room, and a summary only from the rooms the settings name"
#[test]
fn a_speech_is_heard_from_every_room_and_a_summary_from_the_rooms_named() {
  let mut speech = summary();
  speech["kind"] = json!("speech");
  speech["chatId"] = json!(-900);
  speech["emotion"] = json!(EMOTIONS[1]);
  let Heard::Briefing(spoken) = event("briefing.ended", speech) else {
    panic!("a speech is heard")
  };
  assert_eq!(spoken.played, Played::Speech);
  assert_eq!(spoken.face, EMOTIONS[1]);
  let mut far = summary();
  far["chatId"] = json!(-900);
  let Heard::Briefing(summarised) = event("briefing.ended", far) else {
    panic!("a summary is heard")
  };
  let named = [3];
  assert!(spoken.heard_from(&named), "a speech from a room not named");
  assert!(!summarised.heard_from(&named), "a summary from a room not named");
  assert!(summarised.heard_from(&[]), "a summary where no room is named");
  assert!(summarised.heard_from(&[-900]), "a summary from a room named");
}

// "a sound the engine made that is no briefing, a generated voice among them, never reaches the face"
#[test]
fn a_synthesis_is_never_heard() {
  let synthesis = json!({
    "id": 77, "voiceName": "voice-a", "text": "a generated voice", "spoken": "a generated voice", "emotion": null,
    "key": null, "state": "done", "wait": null, "waitingOn": null, "failure": null, "durationMs": 1200,
    "createdAt": 1_000
  });
  assert_eq!(event("synthesis.ended", synthesis.clone()), Heard::Other);
  assert_eq!(event("briefing.ended", synthesis), Heard::Other, "a member with no briefing kind");
}

#[test]
fn a_stop_names_its_room_or_every_room() {
  assert_eq!(
    event("briefings.stopped", json!({ "chatId": 3, "briefingIds": [41] })),
    Heard::Stopped {
      chat_id: Some(3),
      at: 2_500.0
    }
  );
  assert_eq!(
    event("briefings.stopped", json!({ "chatId": null, "briefingIds": [] })),
    Heard::Stopped {
      chat_id: None,
      at: 2_500.0
    }
  );
  assert_eq!(event("briefings.stopped", json!({ "chatId": "three" })), Heard::Other);
  assert_eq!(heard(Some("briefings.stopped"), Some("not json")), Heard::Other);
  assert_eq!(
    heard(Some("synthesis.ended"), Some(r#"{"id":1,"kind":"synthesis.ended","at":5,"data":{}}"#)),
    Heard::Other
  );
}

#[test]
fn the_cursor_resumes_after_the_last_event_and_reads_the_live_ones_once_gone() {
  let mut cursor = Cursor::default();
  assert_eq!(cursor.query(), "?kinds=briefing.ended,briefings.stopped");
  cursor.on_id(Some("7"));
  cursor.on_id(Some("5"));
  cursor.on_id(Some("not a number"));
  cursor.on_id(None);
  assert_eq!(cursor.query(), "?kinds=briefing.ended,briefings.stopped&after=7");
  cursor.gone();
  assert_eq!(cursor.query(), "?kinds=briefing.ended,briefings.stopped");
}

#[test]
fn a_briefing_more_than_two_minutes_past_its_end_is_late() {
  assert!(is_fresh(0.0, 120_000.0));
  assert!(!is_fresh(0.0, 120_001.0));
}

#[test]
fn the_waits_double_from_the_first_up_to_a_minute_and_start_again_after_a_hold() {
  let mut backoff = Backoff::new(Duration::from_secs(1));
  let waits: Vec<u64> = (0..9).map(|_| backoff.next_wait().as_secs()).collect();
  assert_eq!(waits, [1, 2, 4, 8, 16, 32, 60, 60, 60]);
  backoff.reset();
  assert_eq!(backoff.next_wait(), Duration::from_secs(1));
}
