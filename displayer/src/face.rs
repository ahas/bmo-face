//! The face the displayer holds (docs/face.md, 「What it hears」 and 「Speaking」): the core's scene on the
//! displayer's held clock, told what is heard — the document, the briefings to speak one at a time and
//! the stops that silence them, where the eyes look — and asked the drawing of the frame standing and
//! when the next may change.

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use bmo_face_core::check::check;
use bmo_face_core::clock::{FPS, ms_at};
use bmo_face_core::document::Document;
use bmo_face_core::drawing::Drawing;
use bmo_face_core::geometry::Point;
use bmo_face_core::scene::Scene;
use bmo_face_core::talk::{LEAD, Speech};
use serde_json::Value;

use crate::listener::{Briefing, FRESH_MS, is_fresh};
use crate::playback::Playing;

/// How many frames the first drawing of a briefing waits after its sound is ready, so its own frame
/// is not already past.
const SETTLE_FRAMES: i64 = 1;

/// A briefing being made ready or spoken.
struct Speaking {
  id: u64,
  briefing: Briefing,
  /// The frame its first sample is heard at, once it is ready.
  start: Option<i64>,
  playing: Option<Playing>,
}

pub struct Face {
  scene: Scene,
  clock: crate::clock::HeldClock,
  /// The version of the document drawn; none while it is the core's default.
  saved_at: Option<f64>,
  /// The briefings already taken, by their ids, with when each ended, kept while one could still be fresh.
  heard: HashMap<i64, f64>,
  speaking: Option<Speaking>,
  /// The briefings taken while another spoke, in the order they came.
  waiting: VecDeque<Briefing>,
  /// The latest stop of every room's briefings, and of each room's, on the engine's clock: a briefing
  /// that ended at or before the stop covering its room is never spoken.
  every_room_stopped: f64,
  room_stopped: HashMap<i64, f64>,
  plays: u64,
}

impl Face {
  /// The face of the core's default document, its chance drawn from `seed`, counted from `origin`.
  pub fn new(seed: u64, origin: Instant) -> Face {
    let mut clock = crate::clock::HeldClock::new(origin);
    let frame = clock.frame(origin);
    Face {
      scene: Scene::new(bmo_face_core::default::document(), seed, frame),
      clock,
      saved_at: None,
      heard: HashMap::new(),
      speaking: None,
      waiting: VecDeque::new(),
      every_room_stopped: f64::NEG_INFINITY,
      room_stopped: HashMap::new(),
      plays: 0,
    }
  }

  /// The version drawn, as the saves stream names it.
  pub fn saved_at(&self) -> Option<f64> {
    self.saved_at
  }

  /// A document the server answered: drawn at once where the core's check passes it and it names its
  /// version, and left where it does not, the one drawn staying, with the reason.
  pub fn document(&mut self, value: &Value) -> Result<(), String> {
    let saved_at = value["savedAt"].as_f64().ok_or("the document names no version")?;
    let document: Document = check(value).map_err(|refusal| refusal.describe())?;
    self.scene.set_document(document);
    self.saved_at = Some(saved_at);
    Ok(())
  }

  /// A briefing heard when this machine's clock read `now`, in milliseconds: the play it starts now, where
  /// it is fresh, not heard before and not stopped, and nothing else speaks. One heard while another
  /// speaks waits behind it.
  pub fn briefing(&mut self, briefing: Briefing, now: f64) -> Option<u64> {
    if self.heard.contains_key(&briefing.id) || !is_fresh(briefing.ended_at, now) || self.stopped(&briefing)
    {
      return None;
    }
    self.heard.retain(|_, ended_at| is_fresh(*ended_at, now));
    self.heard.insert(briefing.id, briefing.ended_at);
    self.waiting.push_back(briefing);
    self.next()
  }

  /// Whether a stop of its room or of every room came at or after the briefing ended.
  fn stopped(&self, briefing: &Briefing) -> bool {
    let room = self.room_stopped.get(&briefing.chat_id).copied().unwrap_or(f64::NEG_INFINITY);
    briefing.ended_at <= self.every_room_stopped.max(room)
  }

  /// The briefing waiting first, as a play, where nothing speaks.
  fn next(&mut self) -> Option<u64> {
    if self.speaking.is_some() {
      return None;
    }
    let briefing = self.waiting.pop_front()?;
    self.plays += 1;
    self.speaking = Some(Speaking {
      id: self.plays,
      briefing,
      start: None,
      playing: None,
    });
    Some(self.plays)
  }

  /// The briefings waiting behind the one speaking, by their ids, in the order they will be spoken.
  pub fn waiting(&self) -> Vec<i64> {
    self.waiting.iter().map(|briefing| briefing.id).collect()
  }

  /// The briefing a play is of, while it is still the one to speak.
  pub fn briefing_of(&self, id: u64) -> Option<&Briefing> {
    self
      .speaking
      .as_ref()
      .filter(|speaking| speaking.id == id)
      .map(|speaking| &speaking.briefing)
  }

  /// A play's sound is ready, its device `latency` seconds behind: the scene is told to speak it, and
  /// the moment its first sample must be heard is answered. Nothing where the play was given up.
  pub fn speak(&mut self, id: u64, loudness: Vec<f64>, latency: f64, at: Instant) -> Option<Instant> {
    let speaking = self.speaking.as_mut().filter(|speaking| speaking.id == id)?;
    let now = self.clock.frame(at);
    let start = (now + SETTLE_FRAMES + LEAD).max(now + SETTLE_FRAMES + (latency * FPS).ceil() as i64);
    let speech = Speech {
      loudness,
      cues: Vec::new(),
      seed: speaking.briefing.id.unsigned_abs(),
    };
    self.scene.speak(&speech, &speaking.briefing.face, Vec::new(), start);
    speaking.start = Some(start);
    Some(self.clock.instant_of(start))
  }

  /// The hand a play's sound is held by, once it plays.
  pub fn playing(&mut self, id: u64, playing: Playing) {
    match self.speaking.as_mut().filter(|speaking| speaking.id == id) {
      Some(speaking) => speaking.playing = Some(playing),
      None => playing.stop(),
    }
  }

  /// A play ended: its sound ran out, failed or was stopped. Answers the play that follows it.
  pub fn ended(&mut self, id: u64, at: Instant) -> Option<u64> {
    if self.speaking.as_ref().is_some_and(|speaking| speaking.id == id) {
      self.stop(at);
    }
    self.next()
  }

  /// The engine stopped the briefings of a room, or of every room where none is named, at `stopped_at` on
  /// its clock, heard when this machine's clock read `now`: the one of that room speaking stops, those of
  /// it waiting go, and none of it that ended at or before the stop is spoken afterwards. Answers the play
  /// that follows.
  pub fn silence(&mut self, chat_id: Option<i64>, stopped_at: f64, now: f64, at: Instant) -> Option<u64> {
    match chat_id {
      None => self.every_room_stopped = self.every_room_stopped.max(stopped_at),
      Some(room) => {
        let known = self.room_stopped.entry(room).or_insert(f64::NEG_INFINITY);
        *known = known.max(stopped_at);
      }
    }
    // A stop older than a briefing can be and still play reaches nothing more.
    self.room_stopped.retain(|_, stopped_at| now - *stopped_at <= FRESH_MS);
    let reaches = |briefing: &Briefing| chat_id.is_none_or(|room| room == briefing.chat_id);
    self.waiting.retain(|briefing| !reaches(briefing));
    if self.speaking.as_ref().is_some_and(|speaking| reaches(&speaking.briefing)) {
      self.stop(at);
    }
    self.next()
  }

  /// Stops what speaks, the sound with it.
  fn stop(&mut self, at: Instant) {
    if let Some(speaking) = self.speaking.take() {
      if let Some(playing) = speaking.playing {
        playing.stop();
      }
      let frame = self.clock.frame(at);
      self.scene.stop(frame);
    }
  }

  /// Where the eyes look from now on.
  pub fn gaze(&mut self, gaze: Point, at: Instant) {
    let frame = self.clock.frame(at);
    self.scene.gaze(gaze, frame);
  }

  /// The frame clock moved to where the sound being heard says it stands.
  fn follow(&mut self) {
    let Some(speaking) = &self.speaking else { return };
    if let (Some(start), Some(heard)) = (speaking.start, speaking.playing.as_ref().and_then(Playing::heard))
    {
      self.clock.follow(heard.at, ms_at(start) + heard.seconds * 1_000.0);
    }
  }

  /// The drawing standing at an instant.
  pub fn drawing(&mut self, at: Instant) -> Drawing {
    self.follow();
    let frame = self.clock.frame(at);
    self.scene.drawing(frame)
  }

  /// The name of the face standing at an instant.
  pub fn face_at(&mut self, at: Instant) -> String {
    let frame = self.clock.frame(at);
    self.scene.face_at(frame)
  }

  /// When the drawing may next change after an instant; none until something is heard.
  pub fn next_change(&mut self, at: Instant) -> Option<Instant> {
    self.follow();
    let frame = self.clock.frame(at);
    self.scene.next_change(frame).map(|next| self.clock.instant_of(next))
  }
}
