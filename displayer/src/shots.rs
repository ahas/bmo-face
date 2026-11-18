//! The shots (docs/face.md, 「Frames and shots」): from the core's default document and with no server,
//! every default face, a blink frame by frame, a talking sequence over the core's sampled voice or a
//! sound named, as the core hears it, by the resting face and by an open-mouthed one, and the rest after
//! it at each stage, each written as a PNG at the panel's size, to be looked at beside the web display's
//! drawing of the same core.

use std::path::Path;

use bmo_face_core::clock::frames_in;
use bmo_face_core::default;
use bmo_face_core::hearing::{Heard, hear};
use bmo_face_core::names::{EMOTIONS, FALLBACK, face_names};
use bmo_face_core::scene::Scene;
use bmo_face_core::talk::Speech;
use bmo_face_core::voice::sample_voice;

use crate::render::Canvas;
use crate::run::FRAMES_SIZE;
use crate::sound::decode;

/// The seed every shot is drawn from, so two runs write the same pictures.
const SEED: u64 = 7;
/// How many frames the blink's shots take, from the frame before it.
const BLINK_FRAMES: i64 = 7;
/// The frame the talking sequence's sound is first heard at, and the rate its voice is sampled at.
const TALK_START: i64 = 20;
const TALK_RATE: u32 = 48_000;
/// The open-mouthed face the rest sequence speaks with, and how many frames past a stage's start its
/// shot is taken, so a change into it has ended.
const REST_FACE: &str = EMOTIONS[1];
const SETTLED: i64 = 24;

fn write(canvas: &mut Canvas, scene: &mut Scene, frame: i64, path: &Path) -> Result<(), String> {
  canvas.draw(&scene.drawing(frame));
  canvas.pixmap().save_png(path).map_err(|fault| format!("{}: {fault}", path.display()))
}

/// What the talking sequence hears: an Ogg Opus sound decoded as a briefing's is, or the sampled voice.
fn talk_heard(sound: Option<&Path>) -> Result<Vec<Heard>, String> {
  match sound {
    Some(path) => {
      let bytes = std::fs::read(path).map_err(|fault| format!("{}: {fault}", path.display()))?;
      Ok(decode(bytes).map_err(|fault| format!("{}: {fault}", path.display()))?.heard)
    }
    None => Ok(hear(&sample_voice(TALK_RATE), TALK_RATE)),
  }
}

/// Writes every shot into a folder, answering how many; the talking sequence speaks `sound` where one is
/// named.
pub fn write_all(folder: &Path, sound: Option<&Path>) -> Result<usize, String> {
  std::fs::create_dir_all(folder).map_err(|fault| format!("{}: {fault}", folder.display()))?;
  let (width, height) = FRAMES_SIZE;
  let mut canvas = Canvas::new(width, height).ok_or("no picture of the panel's size")?;
  let mut written = 0;

  for (index, name) in face_names().enumerate() {
    let mut scene = Scene::new(default::document(), SEED, 0);
    scene.pin(Some(name));
    scene.set_blinking(false);
    write(&mut canvas, &mut scene, 0, &folder.join(format!("face-{:02}-{name}.png", index + 1)))?;
    written += 1;
  }

  // The first blink an idle face makes, from the frame before it closes until it is open again.
  let mut scene = Scene::new(default::document(), SEED, 0);
  let mut quiet = scene.drawing(0);
  let mut frame = 0;
  let first = loop {
    let next = scene.next_change(frame).ok_or("an idle face that never blinks")?;
    let drawing = scene.drawing(next);
    if drawing != quiet {
      break next;
    }
    (quiet, frame) = (drawing, next);
  };
  let mut scene = Scene::new(default::document(), SEED, 0);
  for frame in first - 1..first - 1 + BLINK_FRAMES {
    write(&mut canvas, &mut scene, frame, &folder.join(format!("blink-{frame:04}.png")))?;
    written += 1;
  }

  // A sampled voice spoken by the resting face, every frame from the first drawing on.
  let mut scene = Scene::new(default::document(), SEED, 0);
  scene.set_blinking(false);
  let speech = Speech {
    heard: talk_heard(sound)?,
    cues: Vec::new(),
  };
  let frames = speech.heard.len() as i64;
  scene.speak(&speech, FALLBACK, Vec::new(), TALK_START);
  for frame in 0..TALK_START + frames {
    write(&mut canvas, &mut scene, frame, &folder.join(format!("talk-{frame:04}.png")))?;
    written += 1;
  }

  // The same voice spoken by an open-mouthed face, every frame, then the rest after it at each stage.
  let mut scene = Scene::new(default::document(), SEED, 0);
  scene.set_blinking(false);
  scene.speak(&speech, REST_FACE, Vec::new(), TALK_START);
  for frame in 0..TALK_START + frames {
    write(&mut canvas, &mut scene, frame, &folder.join(format!("open-{frame:04}.png")))?;
    written += 1;
  }
  let ended = TALK_START + frames;
  let idle = default::document().idle;
  // Each stage from the frame it starts at, and the frame its shot is taken at.
  let (blank, asleep) = (ended + frames_in(idle.blank), ended + frames_in(idle.asleep));
  let stages = [
    ("spoken", ended, ended),
    ("held", blank - 1, blank - 1),
    ("blank", blank, blank + SETTLED),
    ("asleep", asleep, asleep + SETTLED),
  ];
  for (index, (stage, starts, shot)) in stages.into_iter().enumerate() {
    scene.drawing(starts);
    write(&mut canvas, &mut scene, shot, &folder.join(format!("rest-{}-{stage}.png", index + 1)))?;
    written += 1;
  }
  Ok(written)
}
