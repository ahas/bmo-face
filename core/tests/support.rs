//! What the tests share: the default document and its looks.

use bmo_face_core::default;
use bmo_face_core::document::Document;
use bmo_face_core::look::Look;

pub fn document() -> Document {
  default::document()
}

/// Every default face's look, by its name.
pub fn looks() -> Vec<(String, Look)> {
  document().faces.iter().map(|(name, face)| (name.clone(), Look::of(face))).collect()
}
