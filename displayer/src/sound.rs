//! A briefing's sound made ready to speak (docs/face.md, 「Speaking」 and 「The mouth while speaking」):
//! the Ogg Opus voice message demuxed by Symphonia's Ogg reader and decoded by libopus at 48 kHz, the
//! stream's pre-skip and end trimming applied, kept as 16-bit samples to play, and its first channel
//! heard by the core (「Hearing the sound」), as the web display hears it.

use std::io::Cursor;

use bmo_face_core::hearing::{Heard, hear};
use symphonia_core::formats::{FormatOptions, FormatReader};
use symphonia_core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia_format_ogg::OggReader;

use crate::opus::{self, Decoder};

/// A decoded sound, ready to play and to speak with.
#[derive(Clone, Debug, PartialEq)]
pub struct Sound {
  pub rate: u32,
  pub channels: usize,
  /// Interleaved 16-bit samples.
  pub samples: Vec<i16>,
  /// What the face hears of each frame of the held clock from the first sample, as the core reads it.
  pub heard: Vec<Heard>,
}

impl Sound {
  /// How many samples each channel holds.
  pub fn frames(&self) -> usize {
    self.samples.len() / self.channels.max(1)
  }

  /// How long it plays, in seconds.
  pub fn seconds(&self) -> f64 {
    self.frames() as f64 / f64::from(self.rate)
  }
}

/// A sample as 16 bits, held to the range.
fn quantised(sample: f32) -> i16 {
  (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16
}

/// An Ogg Opus stream decoded whole.
pub fn decode(bytes: Vec<u8>) -> Result<Sound, String> {
  let source = MediaSourceStream::new(Box::new(Cursor::new(bytes)), MediaSourceStreamOptions::default());
  let mut reader = OggReader::try_new(source, FormatOptions::default())
    .map_err(|fault| format!("the sound is no Ogg stream: {fault}"))?;
  let (track, channels, delay) = reader
    .tracks()
    .iter()
    .find_map(|track| {
      let channels = track.codec_params.as_ref()?.audio()?.channels.as_ref()?.count();
      Some((track.id, channels, track.delay.unwrap_or(0) as usize))
    })
    .ok_or("the Ogg stream holds no Opus track")?;
  let mut decoder = Decoder::new(channels)?;
  let mut first = Vec::new();
  let mut pcm = Vec::new();
  let mut samples = Vec::new();
  // The stream's pre-skip: decoded frames at its start that are no part of the sound.
  let mut skip = delay;
  while let Some(packet) = reader.next_packet().map_err(|fault| format!("the Ogg stream broke: {fault}"))? {
    if packet.track_id != track {
      continue;
    }
    let decoded = decoder.decode(&packet.data, &mut pcm)?;
    let from = (packet.trim_start.get() as usize).max(skip).min(decoded);
    skip -= skip.min(decoded);
    let to = decoded.saturating_sub(packet.trim_end.get() as usize).max(from);
    let kept = &pcm[from * channels..to * channels];
    first.extend(kept.iter().step_by(channels).copied());
    samples.extend(kept.iter().copied().map(quantised));
  }
  Ok(Sound {
    rate: opus::RATE,
    channels,
    samples,
    heard: hear(&first, opus::RATE),
  })
}
