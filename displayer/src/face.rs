//! The face the displayer holds (docs/face.md, 「What it hears」 and 「Speaking」): the core's scene on the
//! displayer's held clock, told what is heard as the web display tells it — the document, what BMO is
//! doing, a briefing to speak and its stop, where the eyes look — and asked the drawing of the frame
//! standing and when the next may change.

use std::collections::HashSet;
use std::time::Instant;

use bmo_face_core::check::check;
use bmo_face_core::clock::{FPS, ms_at};
use bmo_face_core::document::Document;
use bmo_face_core::drawing::Drawing;
use bmo_face_core::geometry::Point;
use bmo_face_core::scene::Scene;
use bmo_face_core::talk::{LEAD, Speech};
use serde_json::Value;

use crate::listener::{Briefing, is_fresh};
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
  heard: HashSet<(i64, i64)>,
  speaking: Option<Speaking>,
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
      heard: HashSet::new(),
      speaking: None,
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

  /// What BMO is doing, since `since` on the console's clock, heard when this machine's clock read `now`
  /// (both in milliseconds); a moment ahead of this clock is read as now.
  pub fn presence(&mut self, state: &str, since: f64, now: f64, at: Instant) {
    let ago = (now - since).max(0.0);
    let frame = self.clock.frame(at) - (ago * FPS / 1_000.0).floor() as i64;
    self.scene.presence(state, frame);
  }

  /// A briefing heard: the play it starts, where it is fresh and not heard before. A briefing still
  /// speaking stops for it.
  pub fn briefing(&mut self, briefing: Briefing, now: f64, at: Instant) -> Option<u64> {
    if self.heard.contains(&briefing.key()) || !is_fresh(briefing.sent_at, now) {
      return None;
    }
    self.heard.insert(briefing.key());
    self.stop(at);
    self.plays += 1;
    self.speaking = Some(Speaking {
      id: self.plays,
      briefing,
      start: None,
      playing: None,
    });
    Some(self.plays)
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
      seed: speaking.briefing.message_id.unsigned_abs(),
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

  /// A play ended: its sound ran out, failed or was stopped.
  pub fn ended(&mut self, id: u64, at: Instant) {
    if self.speaking.as_ref().is_some_and(|speaking| speaking.id == id) {
      self.stop(at);
    }
  }

  /// `briefing-stopped` heard: the briefing it names stops if it is the one speaking.
  pub fn stopped(&mut self, key: (i64, i64), at: Instant) {
    if self.speaking.as_ref().is_some_and(|speaking| speaking.briefing.key() == key) {
      self.stop(at);
    }
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
