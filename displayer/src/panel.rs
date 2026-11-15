//! The panel through the kernel's mode setting (docs/face.md, 「Drawing」): a connector and its mode
//! chosen by the settings or by what the card offers, two dumb buffers that take turns, and each new
//! picture written into the one not shown and swapped in with a page flip.

use std::io;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd};

use drm::buffer::{Buffer, DrmFourcc};
use drm::control::dumbbuffer::DumbBuffer;
use drm::control::{
  self, Device as ControlDevice, Event, ModeTypeFlags, PageFlipFlags, connector, crtc, framebuffer,
};
use tiny_skia::Pixmap;
use tokio::io::unix::AsyncFd;

use crate::config::ModeWish;

/// A card opened for mode setting, by logind or by the displayer itself.
pub struct Card(OwnedFd);

impl Card {
  pub fn new(fd: OwnedFd) -> Card {
    Card(fd)
  }
}

impl From<Card> for OwnedFd {
  fn from(card: Card) -> OwnedFd {
    card.0
  }
}

impl AsFd for Card {
  fn as_fd(&self) -> BorrowedFd<'_> {
    self.0.as_fd()
  }
}

impl AsRawFd for Card {
  fn as_raw_fd(&self) -> RawFd {
    self.0.as_raw_fd()
  }
}

impl drm::Device for Card {}
impl ControlDevice for Card {}

/// A connector as the choice reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectorOffer {
  pub name: String,
  pub connected: bool,
}

/// A mode as the choice reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModeOffer {
  pub width: u16,
  pub height: u16,
  pub refresh: u32,
  pub preferred: bool,
}

/// The connector named, connected; else the first connected one.
pub fn choose_connector(offers: &[ConnectorOffer], named: Option<&str>) -> Option<usize> {
  match named {
    Some(name) => offers.iter().position(|offer| offer.connected && offer.name == name),
    None => offers.iter().position(|offer| offer.connected),
  }
}

/// The mode wished for, its refresh the highest where none is named; else the preferred mode; else the
/// first offered.
pub fn choose_mode(offers: &[ModeOffer], wish: Option<ModeWish>) -> Option<usize> {
  let indexed = || offers.iter().enumerate();
  match wish {
    Some(wish) => indexed()
      .filter(|(_, offer)| offer.width == wish.width && offer.height == wish.height)
      .filter(|(_, offer)| wish.refresh.is_none_or(|refresh| offer.refresh == refresh))
      .max_by_key(|(_, offer)| offer.refresh)
      .map(|(index, _)| index),
    None => indexed()
      .find(|(_, offer)| offer.preferred)
      .map(|(index, _)| index)
      .or((!offers.is_empty()).then_some(0)),
  }
}

/// A connector's kernel name, such as `HDMI-A-1`.
fn connector_name(info: &connector::Info) -> String {
  format!("{}-{}", info.interface().as_str(), info.interface_id())
}

fn mode_offer(mode: &control::Mode) -> ModeOffer {
  let (width, height) = mode.size();
  ModeOffer {
    width,
    height,
    refresh: mode.vrefresh(),
    preferred: mode.mode_type().contains(ModeTypeFlags::PREFERRED),
  }
}

/// Whether a card has a connector with something on it, for picking a card when none is named.
pub fn has_connected(card: &Card) -> bool {
  card.resource_handles().is_ok_and(|resources| {
    resources.connectors().iter().any(|handle| {
      card
        .get_connector(*handle, false)
        .is_ok_and(|info| info.state() == connector::State::Connected)
    })
  })
}

/// One of the two pictures that take turns.
struct Surface {
  buffer: DumbBuffer,
  framebuffer: framebuffer::Handle,
}

/// The panel being drawn on.
pub struct Panel {
  card: AsyncFd<Card>,
  crtc: crtc::Handle,
  surfaces: Vec<Surface>,
  /// The surface on the panel, and whether a flip to it is still waiting for the vertical blank.
  shown: usize,
  flipping: bool,
  width: u32,
  height: u32,
  name: String,
}

fn fault(what: &str) -> impl Fn(io::Error) -> String + '_ {
  move |error| format!("{what}: {error}")
}

impl Panel {
  /// Sets the chosen connector to its mode on a free display controller, showing a dark picture.
  pub fn new(card: Card, named: Option<&str>, wish: Option<ModeWish>) -> Result<Panel, String> {
    let resources = card.resource_handles().map_err(fault("the card's resources"))?;
    let infos: Vec<connector::Info> = resources
      .connectors()
      .iter()
      .filter_map(|handle| card.get_connector(*handle, false).ok())
      .collect();
    let offers: Vec<ConnectorOffer> = infos
      .iter()
      .map(|info| ConnectorOffer {
        name: connector_name(info),
        connected: info.state() == connector::State::Connected,
      })
      .collect();
    let chosen = choose_connector(&offers, named).ok_or_else(|| match named {
      Some(name) => format!("no connected connector {name}"),
      None => "no connected connector".to_owned(),
    })?;
    let info = &infos[chosen];
    let modes: Vec<ModeOffer> = info.modes().iter().map(mode_offer).collect();
    let mode =
      info.modes()[choose_mode(&modes, wish).ok_or("the connector offers no mode it was asked for")?];
    let crtc = info
      .current_encoder()
      .into_iter()
      .chain(info.encoders().iter().copied())
      .filter_map(|encoder| card.get_encoder(encoder).ok())
      .find_map(|encoder| {
        encoder
          .crtc()
          .or_else(|| resources.filter_crtcs(encoder.possible_crtcs()).first().copied())
      })
      .ok_or("no display controller drives the connector")?;
    let (width, height) = mode.size();
    let (width, height) = (u32::from(width), u32::from(height));
    let mut surfaces = Vec::with_capacity(2);
    for _ in 0..2 {
      let buffer = card
        .create_dumb_buffer((width, height), DrmFourcc::Xrgb8888, 32)
        .map_err(fault("a dumb buffer"))?;
      let framebuffer = card.add_framebuffer(&buffer, 24, 32).map_err(fault("a framebuffer"))?;
      surfaces.push(Surface {
        buffer,
        framebuffer,
      });
    }
    card
      .set_crtc(crtc, Some(surfaces[0].framebuffer), (0, 0), &[info.handle()], Some(mode))
      .map_err(fault("setting the mode"))?;
    let name = format!("{} at {width}x{height}@{}", offers[chosen].name, mode.vrefresh());
    // SAFETY: the card owns its descriptor, which stays open and the same until the card is dropped.
    let card =
      unsafe { AsyncFd::register(card) }.map_err(|fault| format!("watching the card: {}", fault))?;
    Ok(Panel {
      card,
      crtc,
      surfaces,
      shown: 0,
      flipping: false,
      width,
      height,
      name,
    })
  }

  pub fn size(&self) -> (u32, u32) {
    (self.width, self.height)
  }

  /// The connector and mode shown on, for the output.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Waits until the flip asked last has happened.
  async fn flipped(&mut self) -> Result<(), String> {
    while self.flipping {
      let mut ready = self.card.readable().await.map_err(fault("waiting on the card"))?;
      match ready.try_io(|card| card.get_ref().receive_events()) {
        Ok(events) => {
          for event in events.map_err(fault("the card's events"))? {
            if let Event::PageFlip(flip) = event
              && flip.crtc == self.crtc
            {
              self.flipping = false;
            }
          }
        }
        Err(_would_block) => continue,
      }
    }
    Ok(())
  }

  /// Shows a picture the panel's size: written into the surface not shown, then flipped in.
  pub async fn show(&mut self, picture: &Pixmap) -> Result<(), String> {
    self.flipped().await?;
    let next = 1 - self.shown;
    let card = self.card.get_ref();
    let surface = &mut self.surfaces[next];
    let pitch = surface.buffer.pitch() as usize;
    let mut mapping = card.map_dumb_buffer(&mut surface.buffer).map_err(fault("mapping a dumb buffer"))?;
    let width = picture.width() as usize;
    for (row, pixels) in picture.data().chunks_exact(width * 4).enumerate().take(self.height as usize) {
      let line = &mut mapping[row * pitch..row * pitch + width * 4];
      // The rasteriser's pixels are opaque RGBA; the panel's are XRGB in the card's little-endian words.
      for (out, [red, green, blue, _]) in
        line.as_chunks_mut::<4>().0.iter_mut().zip(pixels.as_chunks::<4>().0)
      {
        *out = [*blue, *green, *red, 0xff];
      }
    }
    drop(mapping);
    card
      .page_flip(self.crtc, self.surfaces[next].framebuffer, PageFlipFlags::EVENT, None)
      .map_err(fault("flipping the page"))?;
    self.shown = next;
    self.flipping = true;
    Ok(())
  }
}

impl Drop for Panel {
  fn drop(&mut self) {
    // The card's file may outlive this process's copy of it, held by logind, so what was made on it is
    // taken down here rather than left until it closes.
    let card = self.card.get_ref();
    for surface in self.surfaces.drain(..) {
      let _ = card.destroy_framebuffer(surface.framebuffer);
      let _ = card.destroy_dumb_buffer(surface.buffer);
    }
  }
}
