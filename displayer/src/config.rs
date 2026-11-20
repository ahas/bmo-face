//! The displayer's settings (docs/face.md, 「Its settings」), read once from its environment, each with
//! the default a machine needs when it names none.

use std::path::PathBuf;
use std::time::Duration;

/// Where the face server answers, its base path included.
const FACE_URL: &str = "http://127.0.0.1:3217/bmo-face";
/// The data root under the home folder when none is named: beside BMO's other units' data.
const DATA_UNDER_HOME: &str = ".local/share/bmo/data/face-displayer";
/// The gaze socket's file name, in the runtime folder or else the data root.
const GAZE_SOCKET: &str = "bmo-face-gaze.sock";
const DATA_GAZE_SOCKET: &str = "gaze.sock";
/// The first of the growing waits between tries.
const FIRST_RETRY: Duration = Duration::from_secs(1);

/// Where the sound of a briefing goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sound {
  /// Nothing is heard; the mouth moves from the decoded sound alone.
  Off,
  /// An ALSA PCM, by name.
  Device(String),
}

impl Sound {
  fn named(setting: Option<String>) -> Sound {
    match setting.as_deref().map(str::trim) {
      Some("off") => Sound::Off,
      Some(name) if !name.is_empty() => Sound::Device(name.to_owned()),
      _ => Sound::Device("default".to_owned()),
    }
  }
}

/// A mode asked for by its size, and its refresh where it matters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModeWish {
  pub width: u16,
  pub height: u16,
  pub refresh: Option<u32>,
}

impl ModeWish {
  /// `<width>x<height>` or `<width>x<height>@<hz>`.
  pub fn parse(text: &str) -> Option<ModeWish> {
    let (size, refresh) = match text.split_once('@') {
      Some((size, refresh)) => (size, Some(refresh.trim().parse().ok()?)),
      None => (text, None),
    };
    let (width, height) = size.trim().split_once('x')?;
    Some(ModeWish {
      width: width.parse().ok()?,
      height: height.parse().ok()?,
      refresh,
    })
  }
}

/// Everything the displayer is set to.
#[derive(Clone, Debug)]
pub struct Config {
  /// The face server's address with its base path, without a trailing slash.
  pub face_url: String,
  /// The voice engine's address, without a trailing slash; none hears no briefing.
  pub voice: Option<String>,
  /// The file the voice engine's token stands in.
  pub voice_token: Option<PathBuf>,
  /// The rooms whose summaries are spoken, every room's where none is named; a speech is spoken from every
  /// room.
  pub rooms: Vec<i64>,
  pub data: PathBuf,
  pub card: Option<PathBuf>,
  pub connector: Option<String>,
  pub mode: Option<ModeWish>,
  pub sound: Sound,
  pub gaze: PathBuf,
  pub first_retry: Duration,
}

impl Config {
  /// The settings this process was started with.
  pub fn from_env() -> Result<Config, String> {
    Config::from_lookup(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
  }

  /// The settings a lookup answers, as the environment would.
  pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Config, String> {
    let home = lookup("HOME").map(PathBuf::from);
    let data = match lookup("BMO_FACE_DISPLAYER_DATA") {
      Some(data) => PathBuf::from(data),
      None => home.ok_or("neither BMO_FACE_DISPLAYER_DATA nor HOME is set")?.join(DATA_UNDER_HOME),
    };
    let mode = match lookup("BMO_FACE_DISPLAYER_MODE") {
      Some(mode) => Some(
        ModeWish::parse(&mode)
          .ok_or_else(|| format!("BMO_FACE_DISPLAYER_MODE is not <width>x<height>[@<hz>]: {mode}"))?,
      ),
      None => None,
    };
    let gaze = match (lookup("BMO_FACE_DISPLAYER_GAZE"), lookup("XDG_RUNTIME_DIR")) {
      (Some(path), _) => PathBuf::from(path),
      (None, Some(runtime)) => PathBuf::from(runtime).join(GAZE_SOCKET),
      (None, None) => data.join(DATA_GAZE_SOCKET),
    };
    let rooms = match lookup("BMO_FACE_DISPLAYER_ROOMS") {
      Some(rooms) => rooms
        .split(',')
        .map(str::trim)
        .filter(|room| !room.is_empty())
        .map(|room| room.parse::<i64>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| format!("BMO_FACE_DISPLAYER_ROOMS is not a list of room numbers: {rooms}"))?,
      None => Vec::new(),
    };
    let first_retry = match lookup("BMO_FACE_DISPLAYER_RETRY_MS") {
      Some(ms) => Duration::from_millis(
        ms.parse()
          .map_err(|_| format!("BMO_FACE_DISPLAYER_RETRY_MS is not a count of milliseconds: {ms}"))?,
      ),
      None => FIRST_RETRY,
    };
    Ok(Config {
      face_url: lookup("BMO_FACE_URL")
        .unwrap_or_else(|| FACE_URL.to_owned())
        .trim_end_matches('/')
        .to_owned(),
      voice: lookup("BMO_FACE_DISPLAYER_VOICE")
        .map(|voice| voice.trim().trim_end_matches('/').to_owned())
        .filter(|voice| !voice.is_empty()),
      voice_token: lookup("BMO_FACE_DISPLAYER_VOICE_TOKEN_FILE").map(PathBuf::from),
      rooms,
      data,
      card: lookup("BMO_FACE_DISPLAYER_CARD").map(PathBuf::from),
      connector: lookup("BMO_FACE_DISPLAYER_CONNECTOR"),
      mode,
      sound: Sound::named(lookup("BMO_FACE_DISPLAYER_SOUND")),
      gaze,
      first_retry,
    })
  }
}
