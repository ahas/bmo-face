//! The face server as the displayer calls it (docs/face.md, 「The server」): the expression document and
//! its saves stream, both read openly.

use serde_json::Value;

use crate::http::{Events, Fault, Service};

/// The face server at its address.
#[derive(Clone)]
pub struct FaceServer {
  service: Service,
}

impl FaceServer {
  /// The server whose address, base path included, is `base`.
  pub fn new(base: &str) -> FaceServer {
    FaceServer {
      service: Service::new(base),
    }
  }

  /// The expression document, as the server keeps it.
  pub async fn document(&self) -> Result<Value, Fault> {
    self.service.json("/api/faces", None).await
  }

  /// The stream of the document's saves.
  pub async fn saves(&self) -> Result<Events, Fault> {
    self.service.events("/api/faces/events", None).await
  }
}
