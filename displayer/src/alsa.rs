//! An ALSA playback device through the system's libasound, loaded as the program runs (docs/face.md,
//! 「Speaking」), so a build needs no audio development files. Only what one interleaved 16-bit stream
//! needs is reached: opening by name, the simple parameter call, writing, the device's delay, recovery
//! from an underrun, and closing.

use std::ffi::{CStr, CString, c_char, c_int, c_long, c_uint, c_ulong, c_void};

use libloading::Library;

/// A function of a loaded library, by its nul-ended name.
///
/// # Safety
///
/// `T` must be the function's exact C signature.
pub(crate) unsafe fn symbol<T: Copy>(library: &Library, name: &[u8]) -> Result<T, String> {
  // SAFETY: the caller states the signature.
  unsafe { library.get::<T>(name) }
    .map(|symbol| *symbol)
    .map_err(|fault| fault.to_string())
}

/// The library's name as the system's loader finds it.
const LIBRARY: &str = "libasound.so.2";
const STREAM_PLAYBACK: c_int = 0;
const FORMAT_S16_LE: c_int = 2;
const ACCESS_RW_INTERLEAVED: c_int = 3;
const STATE_PREPARED: c_int = 2;
const STATE_RUNNING: c_int = 3;
/// How far behind the device is asked to play at most: room for a few writes, little delay.
const LATENCY_MICROSECONDS: c_uint = 60_000;

type Open =
  unsafe extern "C" fn(pcm: *mut *mut c_void, name: *const c_char, stream: c_int, mode: c_int) -> c_int;
type SetParams = unsafe extern "C" fn(
  pcm: *mut c_void,
  format: c_int,
  access: c_int,
  channels: c_uint,
  rate: c_uint,
  soft_resample: c_int,
  latency: c_uint,
) -> c_int;
type GetParams =
  unsafe extern "C" fn(pcm: *mut c_void, buffer: *mut c_ulong, period: *mut c_ulong) -> c_int;
type Writei = unsafe extern "C" fn(pcm: *mut c_void, buffer: *const c_void, frames: c_ulong) -> c_long;
type Delay = unsafe extern "C" fn(pcm: *mut c_void, delay: *mut c_long) -> c_int;
type Recover = unsafe extern "C" fn(pcm: *mut c_void, error: c_int, silent: c_int) -> c_int;
type Simple = unsafe extern "C" fn(pcm: *mut c_void) -> c_int;
type Error = unsafe extern "C" fn(error: c_int) -> *const c_char;

struct Calls {
  set_params: SetParams,
  get_params: GetParams,
  writei: Writei,
  delay: Delay,
  recover: Recover,
  start: Simple,
  state: Simple,
  drop: Simple,
  close: Simple,
  error: Error,
}

/// An open playback device.
pub struct Pcm {
  pcm: *mut c_void,
  calls: Calls,
  channels: usize,
  rate: u32,
  buffer: usize,
  period: usize,
  // Held so the calls above stay loaded for as long as the device is open.
  _library: Library,
}

/// What a write left the device doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Written {
  /// This many frames were taken.
  Frames(usize),
  /// The device ran dry and was made ready again; nothing was taken.
  Recovered,
}

impl Pcm {
  /// The device named, set for `channels` channels of 16-bit samples at `rate`.
  pub fn open(name: &str, channels: usize, rate: u32) -> Result<Pcm, String> {
    // SAFETY: loading libasound runs only its own initialisers.
    let library =
      unsafe { Library::new(LIBRARY) }.map_err(|fault| format!("{LIBRARY} is not loaded: {fault}"))?;
    // SAFETY: each symbol is libasound's public function with the C signature its type states.
    let (open, calls) = unsafe {
      let open: Open = symbol(&library, b"snd_pcm_open\0")?;
      let calls = Calls {
        set_params: symbol(&library, b"snd_pcm_set_params\0")?,
        get_params: symbol(&library, b"snd_pcm_get_params\0")?,
        writei: symbol(&library, b"snd_pcm_writei\0")?,
        delay: symbol(&library, b"snd_pcm_delay\0")?,
        recover: symbol(&library, b"snd_pcm_recover\0")?,
        start: symbol(&library, b"snd_pcm_start\0")?,
        state: symbol(&library, b"snd_pcm_state\0")?,
        drop: symbol(&library, b"snd_pcm_drop\0")?,
        close: symbol(&library, b"snd_pcm_close\0")?,
        error: symbol(&library, b"snd_strerror\0")?,
      };
      (open, calls)
    };
    let device = CString::new(name).map_err(|_| format!("a device name holding a nul: {name:?}"))?;
    let mut pcm = std::ptr::null_mut();
    // SAFETY: `pcm` is a valid place for the handle and `device` a nul-ended name.
    let opened = unsafe { open(&mut pcm, device.as_ptr(), STREAM_PLAYBACK, 0) };
    let mut device = Pcm {
      pcm,
      calls,
      channels,
      rate,
      buffer: 0,
      period: 0,
      _library: library,
    };
    if opened < 0 {
      device.pcm = std::ptr::null_mut();
      return Err(format!("the device {name} did not open: {}", device.describe(opened)));
    }
    let wanted = c_uint::try_from(channels).map_err(|_| format!("{channels} channels"))?;
    // SAFETY: the device is open; the format, access and counts are plain values libasound checks.
    let set = unsafe {
      (device.calls.set_params)(
        device.pcm,
        FORMAT_S16_LE,
        ACCESS_RW_INTERLEAVED,
        wanted,
        rate,
        1,
        LATENCY_MICROSECONDS,
      )
    };
    if set < 0 {
      return Err(format!("the device {name} took no 16-bit stream: {}", device.describe(set)));
    }
    let (mut buffer, mut period): (c_ulong, c_ulong) = (0, 0);
    // SAFETY: the device is open and both places are valid.
    let got = unsafe { (device.calls.get_params)(device.pcm, &mut buffer, &mut period) };
    if got < 0 {
      return Err(format!("the device {name} said nothing of its buffer: {}", device.describe(got)));
    }
    device.buffer = buffer as usize;
    device.period = (period as usize).max(1);
    Ok(device)
  }

  pub fn channels(&self) -> usize {
    self.channels
  }

  pub fn rate(&self) -> u32 {
    self.rate
  }

  /// The frames one write moves at a time.
  pub fn period(&self) -> usize {
    self.period
  }

  /// How far behind the device plays when its buffer is full, in frames.
  pub fn buffer(&self) -> usize {
    self.buffer
  }

  fn describe(&self, error: c_int) -> String {
    // SAFETY: snd_strerror answers a static nul-ended string for any code.
    let text = unsafe { (self.calls.error)(error) };
    if text.is_null() {
      return format!("error {error}");
    }
    // SAFETY: not null, and static for the library's life.
    unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned()
  }

  /// Interleaved frames written, blocking until the device takes some.
  pub fn write(&mut self, samples: &[i16]) -> Result<Written, String> {
    let frames = samples.len() / self.channels;
    // SAFETY: the device is open and `samples` holds `frames` whole interleaved frames.
    let wrote = unsafe { (self.calls.writei)(self.pcm, samples.as_ptr().cast(), frames as c_ulong) };
    if wrote >= 0 {
      return Ok(Written::Frames(wrote as usize));
    }
    // SAFETY: the device is open; recovering is what libasound asks after a failed write.
    let recovered = unsafe { (self.calls.recover)(self.pcm, wrote as c_int, 1) };
    if recovered < 0 {
      return Err(format!("the device failed a write: {}", self.describe(wrote as c_int)));
    }
    Ok(Written::Recovered)
  }

  /// Starts a device holding frames but not yet playing.
  pub fn start(&mut self) -> Result<(), String> {
    // SAFETY: the device is open.
    if unsafe { (self.calls.state)(self.pcm) } != STATE_PREPARED {
      return Ok(());
    }
    // SAFETY: the device is open and prepared.
    let started = unsafe { (self.calls.start)(self.pcm) };
    if started < 0 {
      Err(format!("the device did not start: {}", self.describe(started)))
    } else {
      Ok(())
    }
  }

  pub fn is_running(&self) -> bool {
    // SAFETY: the device is open.
    unsafe { (self.calls.state)(self.pcm) == STATE_RUNNING }
  }

  /// The frames between the next one written and the one being heard: how long until it is heard.
  pub fn delay(&self) -> Option<usize> {
    let mut delay: c_long = 0;
    // SAFETY: the device is open and `delay` is a valid place.
    let got = unsafe { (self.calls.delay)(self.pcm, &mut delay) };
    (got >= 0).then(|| delay.max(0) as usize)
  }

  /// Drops what is queued, at once.
  pub fn halt(&mut self) {
    // SAFETY: the device is open.
    unsafe { (self.calls.drop)(self.pcm) };
  }
}

impl Drop for Pcm {
  fn drop(&mut self) {
    if !self.pcm.is_null() {
      // SAFETY: the device was opened by snd_pcm_open and is closed once, here.
      unsafe { (self.calls.close)(self.pcm) };
    }
  }
}

// SAFETY: an ALSA PCM handle has no thread affinity; it is used by one owner at a time.
unsafe impl Send for Pcm {}
