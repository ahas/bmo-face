//! The face as it stands from frame to frame (docs/face.md, 「The face's core」 and 「Which face
//! shows」): the document, what the caller has told it — what BMO is doing, a sound being spoken, where
//! the eyes look — and the drawing every frame shows. Frames are the caller's, counted on the held clock
//! and asked in order; given one seed and one run of events, every caller is handed the same drawings,
//! and the speaking turns on what is heard alone.

use crate::blink::{Blinks, Lid, lidded};
use crate::change;
use crate::clock::{ON_TWOS, frames_in};
use crate::document::{Document, Part};
use crate::drawing::Drawing;
use crate::eyes::voiced;
use crate::gaze::{GazeTrack, gazed};
use crate::geometry::Point;
use crate::look::Look;
use crate::mouth::{Mouths, Shape, between};
use crate::names::{ASLEEP, FALLBACK, IDLE};
use crate::random::Random;
use crate::talk::{self, LEAD, Pose, Speech};

/// How long a briefing's face stays after its sound ends, whatever BMO is doing.
pub const LINGER_SECONDS: f64 = 4.0;

/// The openness under which a talking mouth gives way to the closed mouth a face rests on, in
/// proportion, so a talking mouth that has shut draws as the closed one.
const CLOSING_OPEN: f64 = 0.1;

/// A briefing being spoken: the face it shows and when, and what the face does at each frame of its
/// sound.
#[derive(Clone, Debug)]
struct Speaking {
  /// The frame the sound's first frame is heard at.
  start: i64,
  /// The frame the sound ends, or was stopped, at.
  end: i64,
  /// The face it shows from its start, and the faces it moves to at frames of the sound.
  face: String,
  faces: Vec<(i64, String)>,
  poses: Vec<Pose>,
  on_twos: bool,
}

impl Speaking {
  /// Whether it is showing at a frame: from its first drawing, `LEAD` frames before its sound, to its end.
  fn shows(&self, frame: i64) -> bool {
    self.start - LEAD <= frame && frame < self.end
  }

  fn face_at(&self, frame: i64) -> &str {
    self
      .faces
      .iter()
      .rev()
      .find(|(at, _)| self.start + at <= frame)
      .map_or(&self.face, |(_, face)| face)
  }

  /// The shape of the last frame heard above quiet, as a frame shows it.
  fn last_at(&self, frame: i64) -> Shape {
    talk::pose_at(&self.poses, frame - self.start, self.on_twos)
      .map_or_else(Shape::default, |pose| pose.last)
  }

  /// The pose a frame shows, while it shows.
  fn pose_at(&self, frame: i64) -> Option<&Pose> {
    if !self.shows(frame) {
      return None;
    }
    talk::pose_at(&self.poses, frame - self.start, self.on_twos)
  }

  /// The next frame after this one whose pose differs from its own, while it shows.
  fn next_pose(&self, frame: i64) -> Option<i64> {
    let now = self.pose_at(frame).copied();
    (frame + 1..self.end).find(|later| self.pose_at(*later).copied() != now)
  }
}

/// The briefing spoken last: the face it showed as it ended, the frame its sound ended at, and the shape
/// of its last frame heard above quiet, which a face whose own mouth is open stays closed to.
#[derive(Clone, Debug)]
struct Spoke {
  face: String,
  ended: i64,
  last: Shape,
}

/// A change being shown: the frame it began and its in-betweens.
#[derive(Clone, Debug)]
struct Changing {
  start: i64,
  drawings: Vec<Look>,
}

impl Changing {
  fn at(&self, frame: i64) -> Option<&Look> {
    let step = (frame - self.start).div_euclid(ON_TWOS);
    usize::try_from(step).ok().and_then(|step| self.drawings.get(step))
  }

  fn end(&self) -> i64 {
    self.start + change::frames(&self.drawings)
  }
}

pub struct Scene {
  document: Document,
  /// The talking set made ready to draw from.
  mouths: Mouths,
  /// Expression changes are cuts alone, for a viewer who asks for less motion.
  calm: bool,
  blinking: bool,
  /// The face a caller has pinned, which shows whatever else is heard: the editor's preview.
  pinned: Option<String>,
  presence: (String, i64),
  speaking: Option<Speaking>,
  /// The last briefing spoken, until a rest reaches the blank face.
  spoke: Option<Spoke>,
  /// The face shown, as a name and as its own look.
  shown: Option<(String, Look)>,
  /// The shape the mouth of the face shown closes to, where a briefing closed it.
  closing: Option<Shape>,
  changing: Option<Changing>,
  changes: Random,
  blinks: Blinks,
  gaze: GazeTrack,
}

impl Scene {
  /// A scene of a document, its chance drawn from `seed`, its presence idle from `frame`.
  pub fn new(document: Document, seed: u64, frame: i64) -> Scene {
    Scene {
      mouths: Mouths::new(&document.talk.anchors),
      document,
      calm: false,
      blinking: true,
      pinned: None,
      presence: (IDLE.to_owned(), frame),
      speaking: None,
      spoke: None,
      shown: None,
      closing: None,
      changing: None,
      changes: Random::for_use(seed, 4),
      blinks: Blinks::new(seed),
      gaze: GazeTrack::default(),
    }
  }

  /// A document saved since: the face shown is drawn from it at once, with no in-between.
  pub fn set_document(&mut self, document: Document) {
    self.mouths = Mouths::new(&document.talk.anchors);
    self.document = document;
    self.shown = None;
    self.changing = None;
  }

  /// Expression changes as cuts alone, or with their in-betweens again.
  pub fn set_calm(&mut self, calm: bool) {
    self.calm = calm;
  }

  pub fn set_blinking(&mut self, blinking: bool) {
    self.blinking = blinking;
  }

  /// A face shown whatever else is heard, or none to let what is heard choose again.
  pub fn pin(&mut self, face: Option<&str>) {
    self.pinned = face.map(str::to_owned);
  }

  /// What BMO is doing, since a frame. A rest that had reached the blank face by then forgets the last
  /// briefing.
  pub fn presence(&mut self, state: &str, since: i64) {
    if self
      .rested_at(since)
      .is_some_and(|rested| rested >= frames_in(self.document.idle.blank))
    {
      self.spoke = None;
    }
    self.presence = (state.to_owned(), since);
  }

  /// A sound to speak, first heard at `start`, with the face it shows and the faces it moves to at
  /// frames of the sound.
  pub fn speak(&mut self, speech: &Speech, face: &str, faces: Vec<(i64, String)>, start: i64) {
    let tuning = &self.document.talk.tuning;
    let poses = talk::plan(speech, tuning);
    let end = start + speech.heard.len() as i64;
    if let Some(earlier) = self.speaking.take() {
      let ended = start.min(earlier.end);
      self.spoke = Some(Spoke {
        face: earlier.face_at(ended).to_owned(),
        ended,
        last: earlier.last_at(ended - 1),
      });
    }
    self.speaking = Some(Speaking {
      start,
      end,
      face: face.to_owned(),
      faces,
      poses,
      on_twos: tuning.on_twos,
    });
  }

  /// The sound stopped at a frame, before it ran out.
  pub fn stop(&mut self, frame: i64) {
    if let Some(speaking) = &mut self.speaking {
      speaking.end = speaking.end.min(frame.max(speaking.start - LEAD));
    }
  }

  /// Where the eyes look from a frame on: −1 to 1 each way, the middle at nought.
  pub fn gaze(&mut self, gaze: Point, frame: i64) {
    self.gaze.set(frame, gaze);
  }

  /// The name of the face that shows at a frame (docs/face.md, 「The faces」).
  pub fn face_at(&self, frame: i64) -> String {
    self.choice_at(frame).0
  }

  /// How many frames BMO has rested by a frame, while it is idle: from the later of going idle and the
  /// last briefing's end.
  fn rested_at(&self, frame: i64) -> Option<i64> {
    let (state, since) = &self.presence;
    let from = self.spoke.as_ref().map_or(*since, |spoke| spoke.ended.max(*since));
    (state == IDLE).then_some(frame - from)
  }

  /// The face that shows at a frame, and the shape its mouth closes to where a briefing closed it. First
  /// match wins: a pinned face; a briefing being spoken; the briefing just spoken, for a while; what BMO
  /// is doing at work; and at rest, the face held, then the blank face, then sleep.
  fn choice_at(&self, frame: i64) -> (String, Option<Shape>) {
    let speaking = self.speaking.as_ref().filter(|speaking| speaking.shows(frame));
    let closing = speaking.map(|speaking| speaking.last_at(frame));
    if let Some(face) = &self.pinned {
      return (face.clone(), closing);
    }
    if let Some(speaking) = speaking {
      return (speaking.face_at(frame).to_owned(), closing);
    }
    let spoke = self.spoke.as_ref();
    if let Some(spoke) = spoke.filter(|spoke| frame - spoke.ended < frames_in(LINGER_SECONDS)) {
      return (spoke.face.clone(), Some(spoke.last));
    }
    let Some(rested) = self.rested_at(frame) else {
      return (self.presence.0.clone(), None);
    };
    let idle = self.document.idle;
    if rested >= frames_in(idle.asleep) {
      return (ASLEEP.to_owned(), None);
    }
    if rested >= frames_in(idle.blank) {
      return (IDLE.to_owned(), None);
    }
    match spoke.filter(|spoke| Look::of(self.document.face(&spoke.face)).blinks()) {
      Some(spoke) => (spoke.face.clone(), Some(spoke.last)),
      None => (FALLBACK.to_owned(), None),
    }
  }

  /// Moves what the scene holds on to a frame: a briefing over is one just spoken, and a face newly
  /// called for begins its change from what showed.
  fn advance(&mut self, frame: i64) {
    if let Some(speaking) = self.speaking.as_ref().filter(|speaking| frame >= speaking.end) {
      self.spoke = Some(Spoke {
        face: speaking.face_at(speaking.end - 1).to_owned(),
        ended: speaking.end,
        last: speaking.last_at(speaking.end - 1),
      });
      self.speaking = None;
    }
    let (name, closing) = self.choice_at(frame);
    let own = Look::of(self.document.face(&name));
    match &self.shown {
      None => self.shown = Some((name, own)),
      Some((shown, _)) if *shown != name => {
        let from = self.base_at(frame);
        let target = self.rested(&name, own.clone(), closing);
        let drawings = change::plan(&from, &target, &mut self.changes, self.calm);
        self.changing = (!drawings.is_empty()).then_some(Changing {
          start: frame,
          drawings,
        });
        self.shown = Some((name, own));
      }
      Some(_) => {}
    }
    self.closing = closing;
    if self.changing.as_ref().is_some_and(|changing| frame >= changing.end()) {
      self.changing = None;
    }
  }

  /// A face's look at rest: its own, with the closed mouth of a shape in place of its own mouth where a
  /// briefing closed it and its own is open.
  fn rested(&self, name: &str, mut look: Look, closing: Option<Shape>) -> Look {
    if let Some(closed) = closing.and_then(|last| self.closed_mouth(name, last, look.corners)) {
      look.mouth = closed;
    }
    look
  }

  /// The closed mouth a face rests on after a shape, where its own mouth is open and the talking set
  /// draws one.
  fn closed_mouth(&self, name: &str, last: Shape, corners: f64) -> Option<Vec<Part>> {
    let open = self.document.face(name).mouth.iter().any(Part::is_solid);
    open
      .then(|| self.mouths.closed(last, corners * self.document.talk.tuning.corners))
      .flatten()
  }

  /// Whether the face shown rests on a closed mouth in place of its own.
  fn rests_closed(&self) -> bool {
    matches!((&self.shown, self.closing), (Some((name, look)), Some(last))
      if self.closed_mouth(name, last, look.corners).is_some())
  }

  /// The look at a frame before the talking mouth, the lids and the gaze: an in-between, or the face
  /// shown at rest.
  fn base_at(&self, frame: i64) -> Look {
    if let Some(look) = self.changing.as_ref().and_then(|changing| changing.at(frame)) {
      return look.clone();
    }
    match &self.shown {
      Some((name, look)) => self.rested(name, look.clone(), self.closing),
      None => {
        let (name, closing) = self.choice_at(frame);
        self.rested(&name, Look::of(self.document.face(&name)), closing)
      }
    }
  }

  /// The pose a frame shows, while a briefing is spoken.
  fn pose_at(&self, frame: i64) -> Option<Pose> {
    self.speaking.as_ref().and_then(|speaking| speaking.pose_at(frame)).copied()
  }

  /// The drawing a frame shows.
  pub fn drawing(&mut self, frame: i64) -> Drawing {
    let look = self.look(frame);
    Drawing::of(&look, &self.document.colors)
  }

  /// The look a frame shows, the talking mouth, the eyes' answer to the voice, the lids and the gaze
  /// laid over it. The talking mouth stands in for the resting mouth as far as the pose's voice says,
  /// and a blink, while it plays, wins over the eyes' answer.
  pub fn look(&mut self, frame: i64) -> Look {
    self.advance(frame);
    let mut look = self.base_at(frame);
    let pose = self.pose_at(frame);
    if let Some(pose) = &pose {
      let talking = self.mouths.draw(pose.shape, look.corners * self.document.talk.tuning.corners);
      let share = if self.rests_closed() {
        pose.voice * (pose.shape.open / CLOSING_OPEN).min(1.0)
      } else {
        pose.voice
      };
      look.mouth = between(std::mem::take(&mut look.mouth), &talking, share);
    }
    let blink = (self.blinking && self.changing.is_none() && look.blinks())
      .then(|| self.blinks.at(frame))
      .flatten();
    let gaze = self.gaze.at(frame, blink.map(|(_, start)| start));
    let lid = blink.map_or(Lid::Open, |(lid, _)| lid);
    let (reach, voice) = (look.reach, look.voice);
    for side in &mut look.eyes {
      for eye in side.iter_mut() {
        let answered = match (&pose, blink) {
          (Some(pose), None) => voiced(eye, voice, pose),
          _ => eye.clone(),
        };
        *eye = lidded(&gazed(&answered, reach, gaze), lid);
      }
    }
    look
  }

  /// The next frame after this one where the drawing may change, so a caller can wait until then; none
  /// where nothing will change until it is told something.
  pub fn next_change(&mut self, frame: i64) -> Option<i64> {
    let mut next: Vec<i64> = Vec::new();
    if let Some(changing) = &self.changing {
      next.push(frame + ON_TWOS - (frame - changing.start).rem_euclid(ON_TWOS));
    }
    if let Some(speaking) = &self.speaking {
      let sound = frame - speaking.start + LEAD;
      next.extend(speaking.next_pose(frame));
      next.extend(speaking.faces.iter().map(|(at, _)| speaking.start + at).find(|at| *at > frame));
      next.push(if sound < 0 { speaking.start - LEAD } else { speaking.end });
    }
    if let Some(spoke) = &self.spoke {
      next.push(spoke.ended + frames_in(LINGER_SECONDS));
    }
    if let Some(rested) = self.rested_at(frame) {
      let idle = self.document.idle;
      next.extend([idle.blank, idle.asleep].map(|after| frame - rested + frames_in(after)));
    }
    if self.blinking && self.base_at(frame).blinks() {
      next.push(self.blinks.next_move(frame));
    }
    next.extend(self.gaze.next_move(frame));
    next.into_iter().filter(|at| *at > frame).min()
  }
}
