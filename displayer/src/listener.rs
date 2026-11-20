//! What the voice engine's event stream says, read as a listener reads it (docs/face.md, 「What it hears」;
//! bmo-voice's `docs/boundary-api.md`, 「Events」 and 「Briefings」): the briefings a listener plays by
//! itself, the rooms each is heard from, and the face each shows, the stops, the cursor a reconnecting stream resumes from, whether a
//! briefing is still fresh, and how long to wait before trying again.

use std::time::Duration;

use bmo_face_core::names::{EMOTIONS, FALLBACK};
use serde_json::Value;

/// The face server's saves stream says something, a keep frame at least, every five seconds; one silent
/// for this long is taken as gone.
pub const SILENCE: Duration = Duration::from_secs(20);
/// The voice engine writes a comment line every fifteen seconds; a stream that carries nothing at all for
/// this long is taken as gone.
pub const ENGINE_SILENCE: Duration = Duration::from_secs(40);
/// A briefing heard of later than this after it ended is not played.
pub const FRESH_MS: f64 = 2.0 * 60_000.0;
/// The kinds of the engine's events the displayer reads.
pub const KINDS: &str = "briefing.ended,briefings.stopped";
/// The longest of the growing waits.
const LAST_RETRY: Duration = Duration::from_secs(60);

/// Which briefing a listener plays by itself: a summary of a line, heard from the rooms the settings name, or
/// an agent's speech, the agent speaking as itself in a room, heard from every room, since the face speaks
/// whenever its agent does. A sound the engine makes that is no briefing is never heard here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Played {
  Summary,
  Speech,
}

impl Played {
  /// The briefing kind the engine names, where a listener plays it by itself.
  pub fn of(kind: &str) -> Option<Self> {
    match kind {
      "summary" => Some(Self::Summary),
      "speech" => Some(Self::Speech),
      _ => None,
    }
  }

  /// Whether a briefing of this kind, spoken for `chat_id`, is heard where the settings name `rooms`, every
  /// room's where they name none.
  pub fn heard_from(self, rooms: &[i64], chat_id: i64) -> bool {
    match self {
      Self::Summary => rooms.is_empty() || rooms.contains(&chat_id),
      Self::Speech => true,
    }
  }
}

/// A briefing as the displayer needs it: which one it is, the room it was spoken for, which kind a listener
/// plays it as, the face it shows and when it ended, on the engine's clock in milliseconds.
#[derive(Clone, Debug, PartialEq)]
pub struct Briefing {
  pub id: i64,
  pub chat_id: i64,
  pub played: Played,
  pub face: String,
  pub ended_at: f64,
}

impl Briefing {
  /// Whether the briefing is heard where the settings name `rooms`.
  pub fn heard_from(&self, rooms: &[i64]) -> bool {
    self.played.heard_from(rooms, self.chat_id)
  }
}

/// What one event tells the displayer.
#[derive(Clone, Debug, PartialEq)]
pub enum Heard {
  /// A briefing ended that a listener plays by itself.
  Briefing(Briefing),
  /// The briefings of a room were stopped, or of every room where none is named, at a moment on the
  /// engine's clock.
  Stopped {
    chat_id: Option<i64>,
    at: f64,
  },
  Other,
}

/// The face a briefing shows: the face of the emotion it was spoken with, where this display knows it,
/// else the fallback. The face's expression entry answers each emotion with its own face, and the engine's
/// briefing carries the emotion alone.
fn face_named(emotion: Option<&str>) -> String {
  emotion.filter(|word| EMOTIONS.contains(word)).unwrap_or(FALLBACK).to_owned()
}

/// The kind a listener plays a briefing as, as the engine tells it, where it plays it by itself: a summary or a
/// speech, done, which the engine spoke and which plays by itself.
fn plays_by_itself(briefing: &Value) -> Option<Played> {
  let played = Played::of(briefing["kind"].as_str()?)?;

  (briefing["state"] == "done" && briefing["plays"] != false && !briefing["synthesisId"].is_null())
    .then_some(played)
}

/// What an event of the engine's stream says, by its name and its data.
pub fn heard(event: Option<&str>, data: Option<&str>) -> Heard {
  let Some(data) = data.and_then(|data| serde_json::from_str::<Value>(data).ok()).filter(Value::is_object)
  else {
    return Heard::Other;
  };
  let Some(at) = data["at"].as_f64() else { return Heard::Other };
  let member = &data["data"];
  match event.or(data["kind"].as_str()) {
    Some("briefing.ended") => {
      match (plays_by_itself(member), member["id"].as_i64(), member["chatId"].as_i64()) {
        (Some(played), Some(id), Some(chat_id)) => Heard::Briefing(Briefing {
          id,
          chat_id,
          played,
          face: face_named(member["emotion"].as_str()),
          ended_at: member["finishedAt"].as_f64().unwrap_or(at),
        }),
        _ => Heard::Other,
      }
    }
    Some("briefings.stopped") => match &member["chatId"] {
      Value::Null => Heard::Stopped { chat_id: None, at },
      room => room.as_i64().map_or(Heard::Other, |room| Heard::Stopped {
        chat_id: Some(room),
        at,
      }),
    },
    _ => Heard::Other,
  }
}

/// Whether a briefing that ended at `ended_at` is still fresh enough to play at `now`, both in
/// milliseconds.
pub fn is_fresh(ended_at: f64, now: f64) -> bool {
  now - ended_at <= FRESH_MS
}

/// The cursor a reconnecting stream resumes from: the last event number read. With none, the stream reads
/// on from the newest event the engine keeps, the live ones alone.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cursor {
  pub after: Option<i64>,
}

impl Cursor {
  pub fn on_id(&mut self, id: Option<&str>) {
    if let Some(value) = id.and_then(|id| id.trim().parse::<i64>().ok()) {
      self.after = Some(self.after.map_or(value, |after| after.max(value)));
    }
  }

  /// The engine keeps the events past the cursor no longer: the stream reads on from the live ones.
  pub fn gone(&mut self) {
    self.after = None;
  }

  /// The query a stream opens with.
  pub fn query(&self) -> String {
    let after = self.after.map_or(String::new(), |after| format!("&after={after}"));
    format!("?kinds={KINDS}{after}")
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
