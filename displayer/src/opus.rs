//! The system's libopus, the reference Opus decoder, loaded as the program runs (docs/face.md,
//! 「Speaking」). A build needs no development files for it; the machine that plays needs the library,
//! which every Ubuntu with a browser or a media player carries. Only the three calls a decoder needs are
//! reached.

use std::ffi::{c_float, c_int, c_uchar, c_void};

use libloading::Library;

use crate::alsa::symbol;

/// The library's name as the system's loader finds it.
const LIBRARY: &str = "libopus.so.0";
/// Opus decodes at 48 kHz whatever it was encoded at.
pub const RATE: u32 = 48_000;
/// The most one packet decodes to, per channel: 120 ms.
pub const MAX_PACKET_FRAMES: usize = 5_760;

type Create = unsafe extern "C" fn(rate: i32, channels: c_int, error: *mut c_int) -> *mut c_void;
type DecodeFloat = unsafe extern "C" fn(
  decoder: *mut c_void,
  data: *const c_uchar,
  length: i32,
  pcm: *mut c_float,
  frames: c_int,
  fec: c_int,
) -> c_int;
type Destroy = unsafe extern "C" fn(decoder: *mut c_void);

/// One decoder of one stream.
pub struct Decoder {
  decoder: *mut c_void,
  channels: usize,
  decode: DecodeFloat,
  destroy: Destroy,
  // Held so the functions above stay loaded for as long as the decoder lives.
  _library: Library,
}

impl Decoder {
  /// A decoder at 48 kHz for a stream of `channels` channels.
  pub fn new(channels: usize) -> Result<Decoder, String> {
    // SAFETY: loading libopus runs no initialiser beyond the C runtime's own.
    let library =
      unsafe { Library::new(LIBRARY) }.map_err(|fault| format!("{LIBRARY} is not loaded: {fault}"))?;
    // SAFETY: the three symbols are libopus's public functions with these exact C signatures.
    let (create, decode, destroy) = unsafe {
      (
        symbol::<Create>(&library, b"opus_decoder_create\0")?,
        symbol::<DecodeFloat>(&library, b"opus_decode_float\0")?,
        symbol::<Destroy>(&library, b"opus_decoder_destroy\0")?,
      )
    };
    let wanted = c_int::try_from(channels).map_err(|_| format!("{channels} channels"))?;
    let mut error: c_int = 0;
    // SAFETY: `error` is a valid place for the status; the rate and count are checked by libopus.
    let decoder = unsafe { create(RATE as i32, wanted, &mut error) };
    if decoder.is_null() || error != 0 {
      return Err(format!("libopus refused a decoder of {channels} channels: error {error}"));
    }
    Ok(Decoder {
      decoder,
      channels,
      decode,
      destroy,
      _library: library,
    })
  }

  /// One packet decoded into `pcm`, interleaved: the frames it held per channel.
  pub fn decode(&mut self, packet: &[u8], pcm: &mut Vec<f32>) -> Result<usize, String> {
    pcm.resize(MAX_PACKET_FRAMES * self.channels, 0.0);
    let length = i32::try_from(packet.len()).map_err(|_| "a packet past two gigabytes".to_owned())?;
    // SAFETY: the decoder is live, `packet` is `length` bytes, and `pcm` holds MAX_PACKET_FRAMES frames
    // of `channels` samples, the most libopus writes for the frame count it is given.
    let frames = unsafe {
      (self.decode)(self.decoder, packet.as_ptr(), length, pcm.as_mut_ptr(), MAX_PACKET_FRAMES as c_int, 0)
    };
    usize::try_from(frames).map_err(|_| format!("libopus refused a packet: error {frames}"))
  }
}

impl Drop for Decoder {
  fn drop(&mut self) {
    // SAFETY: the decoder was made by `opus_decoder_create` and is destroyed once, here.
    unsafe { (self.destroy)(self.decoder) };
  }
}

// SAFETY: a libopus decoder has no thread affinity; it is used by one owner at a time.
unsafe impl Send for Decoder {}
