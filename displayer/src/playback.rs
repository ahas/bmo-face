//! A sound played on an ALSA device so that its first sample is heard at a chosen moment, and the moment
//! being heard read back after every write (docs/face.md, 「Speaking」): the playback clock the frame
//! clock follows while a briefing is heard. It runs on a thread of its own while it plays, since a write
//! blocks until the device takes it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::alsa::{Pcm, Written};
use crate::sound::Sound;

/// Where the sound stands: at an instant, how far into it is being heard, in seconds. Before the first
/// sample it is negative.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Heard {
  pub at: Instant,
  pub seconds: f64,
}

/// The hand a playing sound is held by: it stops it, and reads where it stands.
#[derive(Clone)]
pub struct Playing {
  stop: Arc<AtomicBool>,
  heard: Arc<Mutex<Option<Heard>>>,
}

impl Playing {
  /// Stops it at once, dropping what the device still holds.
  pub fn stop(&self) {
    self.stop.store(true, Ordering::Relaxed);
  }

  /// Where the sound stood when the device last said.
  pub fn heard(&self) -> Option<Heard> {
    *self.heard.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
  }
}

/// How far into a sound is heard at an instant: the samples given to the device, less the ones it
/// still holds, past the silence written before it.
pub fn heard_seconds(given: usize, delay: usize, rate: u32) -> f64 {
  (given as f64 - delay as f64) / f64::from(rate)
}

/// How many frames of silence still to give before the sound so its first sample is heard at
/// `target`, when the device holds `delay` frames at `now`; negative where it is already late by that
/// many.
pub fn silence_before(target: Instant, now: Instant, delay: usize, rate: u32) -> i64 {
  let ahead = match target.checked_duration_since(now) {
    Some(ahead) => ahead.as_secs_f64(),
    None => -now.duration_since(target).as_secs_f64(),
  };
  (ahead * f64::from(rate)).round() as i64 - delay as i64
}

/// Plays a sound on an open device, its first sample heard at `target`, and calls `ended` once it has
/// played out, been stopped or failed.
pub fn play(
  mut pcm: Pcm,
  sound: Sound,
  target: Instant,
  ended: impl FnOnce(Result<(), String>) + Send + 'static,
) -> Playing {
  let playing = Playing {
    stop: Arc::new(AtomicBool::new(false)),
    heard: Arc::new(Mutex::new(None)),
  };
  let hand = playing.clone();
  thread::spawn(move || {
    let result = run(&mut pcm, &sound, target, &hand);
    pcm.halt();
    ended(result);
  });
  playing
}

fn run(pcm: &mut Pcm, sound: &Sound, target: Instant, hand: &Playing) -> Result<(), String> {
  let channels = pcm.channels();
  let rate = pcm.rate();
  let period = pcm.period();
  let stopped = || hand.stop.load(Ordering::Relaxed);
  let silence = vec![0_i16; period * channels];
  // Silence enough to start the device, then as much more as puts the first sample on its moment.
  let first =
    silence_before(target, Instant::now(), 0, rate).clamp(period as i64, pcm.buffer().max(period) as i64);
  write_all(pcm, &silence[..], first as usize, &stopped)?;
  pcm.start()?;
  let skip = loop {
    if stopped() {
      return Ok(());
    }
    let left = silence_before(target, Instant::now(), pcm.delay().unwrap_or(0), rate);
    if left <= 0 {
      break (-left) as usize;
    }
    write_all(pcm, &silence[..], (left as usize).min(period), &stopped)?;
  };
  let frames = sound.frames();
  let mut given = skip.min(frames);
  while given < frames {
    if stopped() {
      return Ok(());
    }
    let until = (given + period).min(frames);
    match pcm.write(&sound.samples[given * channels..until * channels])? {
      Written::Frames(taken) => given += taken,
      Written::Recovered => {}
    }
    if let Some(delay) = pcm.delay() {
      *hand.heard.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Heard {
        at: Instant::now(),
        seconds: heard_seconds(given, delay, rate),
      });
    }
  }
  // Played out once the device holds nothing more, looked at a period at a time so a stop still holds.
  while !stopped() && pcm.is_running() && pcm.delay().is_some_and(|delay| delay > 0) {
    thread::sleep(Duration::from_secs_f64(period as f64 / f64::from(rate)));
  }
  Ok(())
}

/// Writes `count` frames out of a run of silence, as many writes as it takes.
fn write_all(
  pcm: &mut Pcm,
  silence: &[i16],
  count: usize,
  stopped: &dyn Fn() -> bool,
) -> Result<(), String> {
  let channels = pcm.channels();
  let mut left = count;
  while left > 0 && !stopped() {
    let now = left.min(silence.len() / channels);
    match pcm.write(&silence[..now * channels])? {
      Written::Frames(taken) => left -= taken.min(left),
      Written::Recovered => {}
    }
  }
  Ok(())
}
