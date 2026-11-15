//! BMO's listener stream as the displayer reads it (docs/face.md, 「What BMO sends, and by which road」
//! and 「What it hears」): each event told apart, the cursor, freshness and the growing waits.

use std::time::Duration;

use bmo_face_core::names::{EMOTIONS, FALLBACK};
use bmo_face_displayer::listener::{Backoff, Briefing, Cursor, Heard, heard, is_fresh};
use serde_json::json;

fn briefing(news: serde_json::Value) -> Heard {
  let data = json!({ "type": "briefing", "chatId": 3, "messageId": 41, "briefing": news }).to_string();
  heard(Some("bot"), Some(&data))
}

#[test]
fn a_briefing_shows_its_expression_else_its_emotion_else_the_fallback() {
  let (sad, joy) = (EMOTIONS[2], EMOTIONS[1]);
  let expect = |face: &str| {
    Heard::Briefing(Briefing {
      chat_id: 3,
      message_id: 41,
      face: face.to_owned(),
      sent_at: 1_000.0,
    })
  };
  assert_eq!(briefing(json!({ "sentAt": 1000, "expression": sad, "emotion": joy })), expect(sad));
  assert_eq!(briefing(json!({ "sentAt": 1000, "emotion": joy })), expect(joy));
  assert_eq!(briefing(json!({ "sentAt": 1000, "emotion": "unknown" })), expect(FALLBACK));
  assert_eq!(briefing(json!({ "sentAt": 1000 })), expect(FALLBACK));
  assert_eq!(briefing(json!({ "emotion": joy })), Heard::Other, "no moment it was sent");
}

#[test]
fn presence_stops_runs_and_keeps_are_told_apart() {
  assert_eq!(
    heard(Some("presence"), Some(r#"{"state":"thinking","since":5}"#)),
    Heard::Presence {
      state: "thinking".to_owned(),
      since: 5.0
    }
  );
  assert_eq!(heard(Some("presence"), Some(r#"{"state":"dancing","since":5}"#)), Heard::Other);
  assert_eq!(
    heard(Some("briefing-stopped"), Some(r#"{"chatId":3,"messageId":41}"#)),
    Heard::Stopped {
      chat_id: 3,
      message_id: 41
    }
  );
  assert_eq!(heard(Some("run"), Some(r#"{"run":"r-2"}"#)), Heard::Run("r-2".to_owned()));
  assert_eq!(heard(Some("keep"), Some("{}")), Heard::Keep);
  assert_eq!(heard(Some("presence"), Some("not json")), Heard::Other);
  assert_eq!(heard(None, Some("{}")), Heard::Other);
}

#[test]
fn the_cursor_resumes_from_the_last_id_of_its_run_and_drops_another_runs() {
  let mut cursor = Cursor::default();
  assert_eq!(cursor.query(), "?after=0");
  cursor.on_run("run one");
  cursor.on_id(Some("7"));
  cursor.on_id(Some("5"));
  cursor.on_id(Some("not a number"));
  assert_eq!(cursor.query(), "?after=7&run=run%20one");
  cursor.on_run("run one");
  assert_eq!(cursor.after, 7);
  cursor.on_run("run-two");
  assert_eq!(cursor.query(), "?after=0&run=run%2Dtwo");
}

#[test]
fn a_briefing_more_than_two_minutes_old_is_late() {
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
