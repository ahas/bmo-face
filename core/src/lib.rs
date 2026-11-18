//! BMO's face, drawn once (docs/face.md, 「The face's core」): the expression document read, checked and
//! carried from its older forms, the eye and talking sets it is drawn from, the geometry of its paths,
//! the blend of one drawing into another, the hearing of a sound, and the drivers that move the face on
//! the held clock — speaking, blinking, changing expression and gazing. A caller hands it the document, the events it
//! hears and a frame number, and is handed back the drawing to show: filled and stroked Bézier paths in
//! the document's units. It does no I/O and keeps no clock of its own.

pub mod blend;
pub mod blink;
pub mod change;
pub mod check;
pub mod clock;
pub mod default;
pub mod document;
pub mod drawing;
pub mod edit;
pub mod eyes;
pub mod gaze;
pub mod geometry;
pub mod hearing;
pub mod idle;
pub mod json;
pub mod look;
pub mod migrate;
pub mod mouth;
pub mod names;
pub mod random;
pub mod scene;
pub mod sets;
pub mod shapes;
pub mod talk;
pub mod text;
pub mod tuning;
pub mod voice;

#[cfg(feature = "wasm")]
mod wasm;
