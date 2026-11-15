//! The drawing the core hands its caller (docs/face.md, 「The drawing」): the screen's size and colours,
//! the outlines that clip, and the paint to lay down in order — each a filled or stroked path of cubic
//! segments, in the document's units, with its colour, its width and the clip it is cut off at.

use serde_json::{Value, json};

use crate::document::{Colors, Part, SCREEN};
use crate::geometry::{Cubic, Path};
use crate::json::rounded;
use crate::look::Look;

/// One thing laid down: a path filled, or a path stroked with round caps and round joins.
#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
  Fill {
    path: Path,
    colour: String,
    clip: Option<usize>,
  },
  Stroke {
    path: Path,
    colour: String,
    width: f64,
    clip: Option<usize>,
  },
}

/// A drawing: the screen it stands on, the clips, and the paint in the order it is laid down.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawing {
  pub body: String,
  pub screen: String,
  pub clips: Vec<Path>,
  pub paint: Vec<Paint>,
}

impl Drawing {
  fn new(colors: &Colors) -> Drawing {
    Drawing {
      body: colors.body.clone(),
      screen: colors.screen.clone(),
      clips: Vec::new(),
      paint: Vec::new(),
    }
  }

  /// A part as it is drawn: its fill, then what stands inside it cut off at its outline, then its line
  /// over the cut edge.
  fn add(&mut self, part: &Part, colors: &Colors, clip: Option<usize>) {
    let path = part.path();
    if path.segments.is_empty() {
      return;
    }
    if let (Some(fill), true) = (part.fill, part.closed) {
      self.paint.push(Paint::Fill {
        path: path.clone(),
        colour: colors.paint(fill).to_owned(),
        clip,
      });
    }
    if !part.inside.is_empty() && part.closed {
      self.clips.push(path.clone());
      let inner = Some(self.clips.len() - 1);
      for inside in &part.inside {
        self.add(inside, colors, inner);
      }
    }
    if let Some(line) = &part.line {
      self.paint.push(Paint::Stroke {
        path,
        colour: colors.paint(line.paint).to_owned(),
        width: line.width,
        clip,
      });
    }
  }

  /// A look drawn whole, element by element.
  pub fn of(look: &Look, colors: &Colors) -> Drawing {
    Drawing::of_parts(look.parts(), colors)
  }

  /// Parts drawn in order, as the editor draws the parts it edits.
  pub fn of_parts<'a>(parts: impl IntoIterator<Item = &'a Part>, colors: &Colors) -> Drawing {
    let mut drawing = Drawing::new(colors);
    for part in parts {
      drawing.add(part, colors, None);
    }
    drawing
  }

  pub fn to_json(&self) -> Value {
    let paint: Vec<Value> = self
      .paint
      .iter()
      .map(|paint| match paint {
        Paint::Fill { path, colour, clip } => {
          json!({ "fill": colour, "path": path_json(path), "clip": clip })
        }
        Paint::Stroke {
          path,
          colour,
          width,
          clip,
        } => {
          json!({ "stroke": colour, "width": rounded(*width), "path": path_json(path), "clip": clip })
        }
      })
      .collect();
    json!({
      "width": SCREEN[0],
      "height": SCREEN[1],
      "body": self.body,
      "screen": self.screen,
      "clips": self.clips.iter().map(path_json).collect::<Vec<_>>(),
      "paint": paint,
    })
  }
}

/// A path as a caller reads it: where it starts, each segment's two control points and end as six
/// numbers, and whether it closes.
fn path_json(path: &Path) -> Value {
  let start = path.segments.first().map_or([0.0, 0.0], Cubic::start);
  let curves: Vec<Value> = path
    .segments
    .iter()
    .map(|Cubic([_, c1, c2, end])| json!([c1[0], c1[1], c2[0], c2[1], end[0], end[1]].map(rounded)))
    .collect();
  json!({ "start": start.map(rounded), "curves": curves, "closed": path.closed })
}
