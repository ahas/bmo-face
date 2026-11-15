//! A drawing of the core laid down in pixels (docs/face.md, 「Drawing」): the body over the whole panel,
//! the screen's rounded rectangle fitted and centred on it, and each paint in order — filled with the
//! non-zero rule or stroked with round caps and joins, anti-aliased, cut off at its clip — as the web
//! display lays the same drawing down in SVG.

use bmo_face_core::document::SCREEN;
use bmo_face_core::drawing::{Drawing, Paint as Painted};
use bmo_face_core::geometry::{Cubic, Path as CorePath};
use tiny_skia::{
  Color, FillRule, LineCap, LineJoin, Mask, Paint, Path, PathBuilder, Pixmap, Stroke, Transform,
};

/// How round the screen's corners stand, in the document's units, as the web display draws them.
const SCREEN_ROUND: f32 = 6.0;
/// How far a cubic's handle reaches to draw a quarter circle.
const QUARTER_HANDLE: f32 = 0.552_284_8;

/// Where the screen stands on the panel: its scale in pixels a unit and its top left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
  pub width: u32,
  pub height: u32,
  pub scale: f32,
  pub left: f32,
  pub top: f32,
}

impl Layout {
  /// The screen as large as the panel allows, centred, its pixels a unit whole where any fit.
  pub fn fit(width: u32, height: u32) -> Layout {
    let [screen_width, screen_height] = SCREEN;
    let exact = (f64::from(width) / screen_width).min(f64::from(height) / screen_height);
    let scale = if exact >= 1.0 { exact.floor() } else { exact } as f32;
    Layout {
      width,
      height,
      scale,
      left: ((width as f32) - screen_width as f32 * scale) / 2.0,
      top: ((height as f32) - screen_height as f32 * scale) / 2.0,
    }
  }

  /// The document's units on the panel's pixels.
  pub fn transform(&self) -> Transform {
    Transform::from_row(self.scale, 0.0, 0.0, self.scale, self.left, self.top)
  }
}

/// A `#rrggbb` colour, the one form the document's check lets through; anything else is black.
pub fn colour(hex: &str) -> Color {
  let channel =
    |at: usize| hex.get(at..at + 2).and_then(|pair| u8::from_str_radix(pair, 16).ok()).unwrap_or(0);
  Color::from_rgba8(channel(1), channel(3), channel(5), 255)
}

fn paint_of(hex: &str) -> Paint<'static> {
  let mut paint = Paint::default();
  paint.set_color(colour(hex));
  paint.anti_alias = true;
  paint
}

/// A path of the core as the rasteriser takes it: its start, each cubic, closed where it closes.
fn path_of(path: &CorePath) -> Option<Path> {
  let [x, y] = path.segments.first().map(Cubic::start)?;
  let mut builder = PathBuilder::new();
  builder.move_to(x as f32, y as f32);
  for Cubic([_, c1, c2, end]) in &path.segments {
    builder.cubic_to(c1[0] as f32, c1[1] as f32, c2[0] as f32, c2[1] as f32, end[0] as f32, end[1] as f32);
  }
  if path.closed {
    builder.close();
  }
  builder.finish()
}

/// The screen's rectangle with its corners turned, in the document's units.
fn screen_outline() -> Option<Path> {
  let [width, height] = SCREEN.map(|side| side as f32);
  let (r, k) = (SCREEN_ROUND, SCREEN_ROUND * QUARTER_HANDLE);
  let mut builder = PathBuilder::new();
  builder.move_to(r, 0.0);
  builder.line_to(width - r, 0.0);
  builder.cubic_to(width - r + k, 0.0, width, r - k, width, r);
  builder.line_to(width, height - r);
  builder.cubic_to(width, height - r + k, width - r + k, height, width - r, height);
  builder.line_to(r, height);
  builder.cubic_to(r - k, height, 0.0, height - r + k, 0.0, height - r);
  builder.line_to(0.0, r);
  builder.cubic_to(0.0, r - k, r - k, 0.0, r, 0.0);
  builder.close();
  builder.finish()
}

/// The panel's picture, drawn again whole for each drawing.
pub struct Canvas {
  layout: Layout,
  pixmap: Pixmap,
  screen: Option<Path>,
}

impl Canvas {
  pub fn new(width: u32, height: u32) -> Option<Canvas> {
    Some(Canvas {
      layout: Layout::fit(width, height),
      pixmap: Pixmap::new(width, height)?,
      screen: screen_outline(),
    })
  }

  pub fn layout(&self) -> Layout {
    self.layout
  }

  pub fn pixmap(&self) -> &Pixmap {
    &self.pixmap
  }

  /// Lays a drawing down over the whole panel.
  pub fn draw(&mut self, drawing: &Drawing) {
    let transform = self.layout.transform();
    self.pixmap.fill(colour(&drawing.body));
    if let Some(screen) = &self.screen {
      self
        .pixmap
        .fill_path(screen, &paint_of(&drawing.screen), FillRule::Winding, transform, None);
    }
    let clips: Vec<Option<Mask>> = drawing.clips.iter().map(|clip| self.mask(clip, transform)).collect();
    for painted in &drawing.paint {
      let (path, colour, clip) = match painted {
        Painted::Fill { path, colour, clip }
        | Painted::Stroke {
          path, colour, clip, ..
        } => (path, colour, clip),
      };
      let Some(path) = path_of(path) else { continue };
      let mask = clip.and_then(|index| clips.get(index)).and_then(Option::as_ref);
      match painted {
        Painted::Fill { .. } => {
          self.pixmap.fill_path(&path, &paint_of(colour), FillRule::Winding, transform, mask);
        }
        Painted::Stroke { width, .. } => {
          let stroke = Stroke {
            width: *width as f32,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
          };
          self.pixmap.stroke_path(&path, &paint_of(colour), &stroke, transform, mask);
        }
      }
    }
  }

  /// A clip's outline as a mask the size of the panel.
  fn mask(&self, clip: &CorePath, transform: Transform) -> Option<Mask> {
    let mut mask = Mask::new(self.pixmap.width(), self.pixmap.height())?;
    mask.fill_path(&path_of(clip)?, FillRule::Winding, true, transform);
    Some(mask)
  }
}
