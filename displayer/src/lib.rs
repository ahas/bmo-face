//! BMO's face on a machine's own panel (docs/face.md, 「The native displayer」): the core's drawing
//! rasterised on the CPU and shown through DRM/KMS, moved by the face server's document and the voice
//! engine's briefings, with the voice played on an ALSA device and the gaze taken from a local socket.

pub mod alsa;
pub mod clock;
pub mod config;
pub mod engine;
pub mod face;
pub mod gaze;
pub mod http;
pub mod journal;
pub mod listener;
pub mod opus;
pub mod panel;
pub mod playback;
pub mod render;
pub mod run;
pub mod seat;
pub mod server;
pub mod shots;
pub mod sound;
