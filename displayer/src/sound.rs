//! A briefing's sound made ready to speak (docs/face.md, 「Speaking」 and 「The mouth while speaking」):
//! the Ogg Opus voice message demuxed by Symphonia's Ogg reader and decoded by libopus at 48 kHz, the
//! stream's pre-skip and end trimming applied, kept as 16-bit samples to play, and its loudness read once
//! a frame of the held clock — the root mean square of the first channel, in dBFS — as the web display
//! reads it.

use std::io::Cursor;

use bmo_face_core::clock::FPS;
use symphonia_core::formats::{FormatOptions, FormatReader};
use symphonia_core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia_format_ogg::OggReader;

use crate::opus::{self, Decoder};

/// The quietest loudness written for a frame: silence.
pub const SILENCE_DB: f64 = -120.0;

/// A decoded sound, ready to play and to speak with.
#[derive(Clone, Debug, PartialEq)]
pub struct Sound {
  pub rate: u32,
  pub channels: usize,
  /// Interleaved 16-bit samples.
  pub samples: Vec<i16>,
  /// The loudness of each frame of the held clock from the first sample, in dBFS.
  pub loudness: Vec<f64>,
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

/// A loudness in dBFS, silence held at the quietest written.
pub fn decibels(level: f64) -> f64 {
  if level > 0.0 { (20.0 * level.log10()).max(SILENCE_DB) } else { SILENCE_DB }
}

/// Reads a sound's loudness one held frame at a time, as its samples come: each frame's samples are
/// those from the sample its start rounds to up to the one the next frame's start rounds to.
#[derive(Clone, Debug)]
pub struct LoudnessMeter {
  rate: f64,
  fps: f64,
  /// Samples read so far.
  read: u64,
  /// The frame being read, and where it ends.
  frame: u64,
  ends_at: u64,
  sum: f64,
  count: u64,
  loudness: Vec<f64>,
}

impl LoudnessMeter {
  pub fn new(rate: u32, fps: f64) -> LoudnessMeter {
    let mut meter = LoudnessMeter {
      rate: f64::from(rate),
      fps,
      read: 0,
      frame: 0,
      ends_at: 0,
      sum: 0.0,
      count: 0,
      loudness: Vec::new(),
    };
    meter.ends_at = meter.start_of(1);
    meter
  }

  /// The sample a frame's start rounds to.
  fn start_of(&self, frame: u64) -> u64 {
    (frame as f64 * self.rate / self.fps).round() as u64
  }

  /// The frame read is closed and the next begun.
  fn close_frame(&mut self) {
    let level = if self.count == 0 { 0.0 } else { (self.sum / self.count as f64).sqrt() };
    self.loudness.push(decibels(level));
    self.frame += 1;
    self.ends_at = self.start_of(self.frame + 1);
    self.sum = 0.0;
    self.count = 0;
  }

  /// The next samples of the one channel read.
  pub fn push(&mut self, samples: impl IntoIterator<Item = f32>) {
    for sample in samples {
      while self.read >= self.ends_at {
        self.close_frame();
      }
      self.sum += f64::from(sample) * f64::from(sample);
      self.count += 1;
      self.read += 1;
    }
  }

  /// One reading for each frame the sound reaches into, the last one partial.
  pub fn finish(mut self) -> Vec<f64> {
    let frames = (self.read as f64 / self.rate * self.fps).ceil() as u64;
    while (self.loudness.len() as u64) < frames {
      self.close_frame();
    }
    self.loudness
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
  let mut meter = LoudnessMeter::new(opus::RATE, FPS);
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
    meter.push(kept.iter().step_by(channels).copied());
    samples.extend(kept.iter().copied().map(quantised));
  }
  Ok(Sound {
    rate: opus::RATE,
    channels,
    samples,
    loudness: meter.finish(),
  })
}
