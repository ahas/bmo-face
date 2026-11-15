//! The shots (docs/face.md, 「Frames and shots」): from the core's default document and with no server,
//! every default face, a blink frame by frame and a talking sequence over a sampled voice, each written
//! as a PNG at the panel's size, to be looked at beside the web display's drawing of the same core.

use std::path::Path;

use bmo_face_core::default;
use bmo_face_core::names::{FALLBACK, face_names};
use bmo_face_core::scene::Scene;
use bmo_face_core::talk::{Speech, sample_loudness};

use crate::render::Canvas;
use crate::run::FRAMES_SIZE;

/// The seed every shot is drawn from, so two runs write the same pictures.
const SEED: u64 = 7;
/// How many frames the blink's shots take, from the frame before it.
const BLINK_FRAMES: i64 = 7;
/// The talking sequence: the frame its sound is first heard at, and how many frames it speaks.
const TALK_START: i64 = 20;
const TALK_FRAMES: usize = 72;

fn write(canvas: &mut Canvas, scene: &mut Scene, frame: i64, path: &Path) -> Result<(), String> {
  canvas.draw(&scene.drawing(frame));
  canvas.pixmap().save_png(path).map_err(|fault| format!("{}: {fault}", path.display()))
}

/// Writes every shot into a folder, answering how many.
pub fn write_all(folder: &Path) -> Result<usize, String> {
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
    loudness: sample_loudness(SEED, TALK_FRAMES),
    cues: Vec::new(),
    seed: SEED,
  };
  scene.speak(&speech, FALLBACK, Vec::new(), TALK_START);
  for frame in 0..TALK_START + TALK_FRAMES as i64 {
    write(&mut canvas, &mut scene, frame, &folder.join(format!("talk-{frame:04}.png")))?;
    written += 1;
  }
  Ok(written)
}
