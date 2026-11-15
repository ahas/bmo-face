//! The binding the screen's WebAssembly build exports (docs/face.md, 「The face's core」): the core's
//! answers handed across as JSON text and plain numbers, and its scene as an object the screen holds. It
//! converts at the boundary and holds no logic of its own.

use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

use crate::check::{self, check, check_colors};
use crate::clock::FPS;
use crate::document::{Colors, FORM, Paint, Part, SCREEN, Size, Voice, limits, parts_json};
use crate::drawing::Drawing;
use crate::edit::{self, NewInside, NewPart};
use crate::migrate::migrate;
use crate::names::{EMOTIONS, FALLBACK, STATES};
use crate::scene::Scene;
use crate::talk::{self, LEAD, Speech};
use crate::text::Cue;

fn read(text: &str) -> Result<Value, JsError> {
  serde_json::from_str(text).map_err(|fault| JsError::new(&format!("not JSON: {fault}")))
}

fn read_part(text: &str) -> Result<Part, JsError> {
  check::part(&read(text)?).map_err(|refusal| JsError::new(&refusal.describe()))
}

fn read_colors(text: &str) -> Result<Colors, JsError> {
  let value = read(text)?;
  let colors = value.as_object().ok_or_else(|| JsError::new("the colours are no object"))?;
  check_colors(colors).map_err(|refusal| JsError::new(&refusal.describe()))
}

/// What a caller needs to know of the document and the clock: the form, the screen, the limits, the
/// names, the paints, the sizes and the sounds, the frame rate and the lead.
#[wasm_bindgen]
pub fn facts() -> String {
  json!({
    "form": FORM,
    "screen": { "width": SCREEN[0], "height": SCREEN[1] },
    "limits": {
      "parts": limits::PARTS,
      "inside": limits::INSIDE,
      "minNodes": limits::MIN_NODES,
      "maxNodes": limits::MAX_NODES,
      "minWidth": limits::MIN_WIDTH,
      "maxWidth": limits::MAX_WIDTH,
      "maxReach": limits::MAX_REACH,
      "families": limits::FAMILIES,
      "mouths": limits::MOUTHS,
    },
    "emotions": EMOTIONS,
    "states": STATES,
    "fallback": FALLBACK,
    "paints": Paint::ALL.map(Paint::name),
    "sizes": Size::ALL.map(Size::name),
    "voices": Voice::ALL.map(Voice::name),
    "fps": FPS,
    "lead": LEAD,
  })
  .to_string()
}

/// The default document, its version nought.
#[wasm_bindgen(js_name = defaultDocument)]
pub fn default_document() -> String {
  let mut document = crate::default::document().to_json();
  document["savedAt"] = json!(0);
  document.to_string()
}

/// The check's verdict on a document: `{ "ok": document }` as it may be kept, or `{ "refusal": … }`.
#[wasm_bindgen(js_name = checkDocument)]
pub fn check_document(text: &str) -> Result<String, JsError> {
  Ok(
    match check(&read(text)?) {
      Ok(document) => json!({ "ok": document.to_json() }),
      Err(refusal) => json!({ "refusal": refusal.to_json() }),
    }
    .to_string(),
  )
}

/// A document of an older form carried into this one, unchecked.
#[wasm_bindgen(js_name = migrateDocument)]
pub fn migrate_document(text: &str) -> Result<String, JsError> {
  Ok(migrate(&read(text)?).to_string())
}

/// Parts drawn in order on a document's colours, as the editor's canvas draws what it edits.
#[wasm_bindgen(js_name = partsDrawing)]
pub fn parts_drawing(parts: &str, colors: &str) -> Result<String, JsError> {
  let parts = check::parts(&read(parts)?).map_err(|refusal| JsError::new(&refusal.describe()))?;
  Ok(Drawing::of_parts(&parts, &read_colors(colors)?).to_json().to_string())
}

/// A node added after the one at `index`, the drawing unchanged: `{ "part": …, "chosen": index }`.
#[wasm_bindgen(js_name = addNode)]
pub fn add_node(part: &str, index: usize) -> Result<String, JsError> {
  let (part, chosen) = edit::add_node(&read_part(part)?, index);
  Ok(json!({ "part": part.to_json(), "chosen": chosen }).to_string())
}

/// The handles a node takes made smooth from nothing, coming then leaving; null for no such node.
#[wasm_bindgen(js_name = smoothHandles)]
pub fn smooth_handles(part: &str, index: usize) -> Result<String, JsError> {
  Ok(
    edit::smooth_handles(&read_part(part)?, index)
      .map_or(Value::Null, |(coming, leaving)| json!([coming, leaving]))
      .to_string(),
  )
}

/// A new part of the editor's own: `oval` or `line`.
#[wasm_bindgen(js_name = newPart)]
pub fn new_part(kind: &str) -> Result<String, JsError> {
  let kind = match kind {
    "oval" => NewPart::Oval,
    "line" => NewPart::Line,
    other => return Err(JsError::new(&format!("no new part {other}"))),
  };
  Ok(edit::new_part(kind).to_json().to_string())
}

/// A new part to stand inside `outer`: `teeth`, `lowerTeeth`, `tongue`, `tongueLight` or `shine`.
#[wasm_bindgen(js_name = newInside)]
pub fn new_inside(kind: &str, outer: &str) -> Result<String, JsError> {
  let kind = match kind {
    "teeth" => NewInside::Teeth,
    "lowerTeeth" => NewInside::LowerTeeth,
    "tongue" => NewInside::Tongue,
    "tongueLight" => NewInside::TongueLight,
    "shine" => NewInside::Shine,
    other => return Err(JsError::new(&format!("no new inside part {other}"))),
  };
  Ok(edit::new_inside(kind, &read_part(outer)?).to_json().to_string())
}

/// Parts reflected across the screen's middle, as a mirrored side is drawn.
#[wasm_bindgen(js_name = mirrorParts)]
pub fn mirror_parts(parts: &str) -> Result<String, JsError> {
  let parts = check::parts(&read(parts)?).map_err(|refusal| JsError::new(&refusal.describe()))?;
  Ok(parts_json(&parts.iter().map(Part::mirrored).collect::<Vec<_>>()).to_string())
}

/// A sound's loudness sampled for a preview, in dBFS a frame.
#[wasm_bindgen(js_name = sampleLoudness)]
pub fn sample_loudness(seed: f64, frames: usize) -> Vec<f64> {
  talk::sample_loudness(seed as u64, frames)
}

/// The face from frame to frame, as the display and the editor's preview hold it.
#[wasm_bindgen]
pub struct FaceScene {
  scene: Scene,
}

fn document_of(text: &str) -> Result<crate::document::Document, JsError> {
  check(&read(text)?).map_err(|refusal| JsError::new(&refusal.describe()))
}

#[wasm_bindgen]
impl FaceScene {
  /// A scene of a document, its chance drawn from `seed`, its presence idle from `frame`.
  #[wasm_bindgen(constructor)]
  pub fn new(document: &str, seed: f64, frame: f64) -> Result<FaceScene, JsError> {
    Ok(FaceScene {
      scene: Scene::new(document_of(document)?, seed as u64, frame as i64),
    })
  }

  #[wasm_bindgen(js_name = setDocument)]
  pub fn set_document(&mut self, document: &str) -> Result<(), JsError> {
    self.scene.set_document(document_of(document)?);
    Ok(())
  }

  #[wasm_bindgen(js_name = setCalm)]
  pub fn set_calm(&mut self, calm: bool) {
    self.scene.set_calm(calm);
  }

  #[wasm_bindgen(js_name = setBlinking)]
  pub fn set_blinking(&mut self, blinking: bool) {
    self.scene.set_blinking(blinking);
  }

  /// A face shown whatever else is heard; none lets what is heard choose again.
  pub fn pin(&mut self, face: Option<String>) {
    self.scene.pin(face.as_deref());
  }

  pub fn presence(&mut self, state: &str, since: f64) {
    self.scene.presence(state, since as i64);
  }

  /// A sound to speak, first heard at `start`: `{ "loudness": [dBFS a frame], "cues": [{ "from", "to",
  /// "text" }], "seed", "faces": [{ "at", "face" }] }`, showing `face`.
  pub fn speak(&mut self, speech: &str, face: &str, start: f64) -> Result<(), JsError> {
    let speech = read(speech)?;
    let numbers = |key: &str| speech[key].as_array().cloned().unwrap_or_default();
    let loudness = numbers("loudness").iter().map(|level| level.as_f64().unwrap_or(-120.0)).collect();
    let cues = numbers("cues")
      .iter()
      .filter_map(|cue| {
        Some(Cue {
          from: cue["from"].as_i64()?,
          to: cue["to"].as_i64()?,
          text: cue["text"].as_str()?.to_owned(),
        })
      })
      .collect();
    let faces = numbers("faces")
      .iter()
      .filter_map(|change| Some((change["at"].as_i64()?, change["face"].as_str()?.to_owned())))
      .collect();
    let seed = speech["seed"].as_f64().unwrap_or(0.0) as u64;
    self.scene.speak(
      &Speech {
        loudness,
        cues,
        seed,
      },
      face,
      faces,
      start as i64,
    );
    Ok(())
  }

  pub fn stop(&mut self, frame: f64) {
    self.scene.stop(frame as i64);
  }

  pub fn gaze(&mut self, x: f64, y: f64, frame: f64) {
    self.scene.gaze([x, y], frame as i64);
  }

  /// The name of the face that shows at a frame.
  #[wasm_bindgen(js_name = faceAt)]
  pub fn face_at(&self, frame: f64) -> String {
    self.scene.face_at(frame as i64)
  }

  /// The drawing a frame shows, as JSON.
  pub fn drawing(&mut self, frame: f64) -> String {
    self.scene.drawing(frame as i64).to_json().to_string()
  }

  /// The next frame after this one where the drawing may change; none until something is told.
  #[wasm_bindgen(js_name = nextChange)]
  pub fn next_change(&mut self, frame: f64) -> Option<f64> {
    self.scene.next_change(frame as i64).map(|at| at as f64)
  }
}
