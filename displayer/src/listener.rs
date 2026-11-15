//! What BMO's listener stream says, read as the web display reads it (docs/face.md, 「What BMO sends,
//! and by which road」): each event told apart, the briefing's face picked, the cursor a reconnecting
//! stream resumes from, whether a briefing is still fresh, and how long to wait before trying again.

use std::time::Duration;

use bmo_face_core::names::{EMOTIONS, FALLBACK, is_state};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::Value;

/// A stream that has said nothing, not even its keep frame, for this long is taken as gone.
pub const SILENCE: Duration = Duration::from_secs(20);
/// A briefing heard of later than this after it was sent is not played.
pub const FRESH_MS: f64 = 2.0 * 60_000.0;
/// The longest of the growing waits.
const LAST_RETRY: Duration = Duration::from_secs(60);

/// A briefing as the displayer needs it: which message it is, the face it shows and when it was sent.
#[derive(Clone, Debug, PartialEq)]
pub struct Briefing {
  pub chat_id: i64,
  pub message_id: i64,
  pub face: String,
  pub sent_at: f64,
}

impl Briefing {
  pub fn key(&self) -> (i64, i64) {
    (self.chat_id, self.message_id)
  }
}

/// What one event tells the displayer.
#[derive(Clone, Debug, PartialEq)]
pub enum Heard {
  Run(String),
  Briefing(Briefing),
  /// What BMO is doing, since a moment on the console's clock in milliseconds.
  Presence {
    state: String,
    since: f64,
  },
  Stopped {
    chat_id: i64,
    message_id: i64,
  },
  Keep,
  Other,
}

/// The face a briefing names: its expression where it carries one this display knows, else its
/// emotion, else the fallback.
fn face_named(word: Option<&str>) -> String {
  word.filter(|word| EMOTIONS.contains(word)).unwrap_or(FALLBACK).to_owned()
}

/// What an event of the stream says, by its name and its data.
pub fn heard(event: Option<&str>, data: Option<&str>) -> Heard {
  let event = event.unwrap_or("message");
  if event == "keep" {
    return Heard::Keep;
  }
  let Some(data) = data.and_then(|data| serde_json::from_str::<Value>(data).ok()).filter(Value::is_object)
  else {
    return Heard::Other;
  };
  match event {
    "run" => data["run"].as_str().map_or(Heard::Other, |run| Heard::Run(run.to_owned())),
    "presence" => match (data["state"].as_str(), data["since"].as_f64()) {
      (Some(state), Some(since)) if is_state(state) => Heard::Presence {
        state: state.to_owned(),
        since,
      },
      _ => Heard::Other,
    },
    "briefing-stopped" => match (data["chatId"].as_i64(), data["messageId"].as_i64()) {
      (Some(chat_id), Some(message_id)) => Heard::Stopped {
        chat_id,
        message_id,
      },
      _ => Heard::Other,
    },
    "bot" if data["type"] == "briefing" => {
      let news = &data["briefing"];
      match (data["chatId"].as_i64(), data["messageId"].as_i64(), news["sentAt"].as_f64()) {
        (Some(chat_id), Some(message_id), Some(sent_at)) => Heard::Briefing(Briefing {
          chat_id,
          message_id,
          // The face the face's own entry answered for the briefing, else the emotion it was spoken with.
          face: face_named(news["expression"].as_str().or(news["emotion"].as_str())),
          sent_at,
        }),
        _ => Heard::Other,
      }
    }
    _ => Heard::Other,
  }
}

/// Whether a briefing sent at `sent_at` is still fresh enough to play at `now`, both in milliseconds.
pub fn is_fresh(sent_at: f64, now: f64) -> bool {
  now - sent_at <= FRESH_MS
}

/// The cursor a reconnecting stream resumes from: the last numbered event read, against the run it was
/// read in. A new run of the bot numbers its events afresh, so a cursor from another run is dropped.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cursor {
  pub after: u64,
  pub run: Option<String>,
}

impl Cursor {
  pub fn on_run(&mut self, run: &str) {
    if self.run.as_deref().is_some_and(|known| known != run) {
      self.after = 0;
    }
    self.run = Some(run.to_owned());
  }

  pub fn on_id(&mut self, id: Option<&str>) {
    if let Some(value) = id.and_then(|id| id.parse::<u64>().ok()) {
      self.after = self.after.max(value);
    }
  }

  /// The query a stream opens with.
  pub fn query(&self) -> String {
    let run = self
      .run
      .as_deref()
      .map_or(String::new(), |run| format!("&run={}", utf8_percent_encode(run, NON_ALPHANUMERIC)));
    format!("?after={}{run}", self.after)
  }
}

/// The growing waits between tries: the first one given, doubling up to a minute, for as long as it
/// takes.
#[derive(Clone, Debug)]
pub struct Backoff {
  first: Duration,
  failures: u32,
}

impl Backoff {
  pub fn new(first: Duration) -> Backoff {
    Backoff { first, failures: 0 }
  }

  /// The wait before the next try, counting this failure.
  pub fn next_wait(&mut self) -> Duration {
    let wait = self.first.saturating_mul(1 << self.failures.min(16)).min(LAST_RETRY.max(self.first));
    self.failures = self.failures.saturating_add(1);
    wait
  }

  /// A try that held: the next failure waits the first wait again.
  pub fn reset(&mut self) {
    self.failures = 0;
  }
}
