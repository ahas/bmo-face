//! Which connector and mode the panel is set to (docs/face.md, 「Drawing」), and how the settings name
//! them (「Its settings」).

use bmo_face_displayer::config::{Config, ModeWish, Sound};
use bmo_face_displayer::panel::{ConnectorOffer, ModeOffer, choose_connector, choose_mode};

fn connector(name: &str, connected: bool) -> ConnectorOffer {
  ConnectorOffer {
    name: name.to_owned(),
    connected,
  }
}

fn mode(width: u16, height: u16, refresh: u32, preferred: bool) -> ModeOffer {
  ModeOffer {
    width,
    height,
    refresh,
    preferred,
  }
}

#[test]
fn the_connector_is_the_one_named_else_the_first_connected() {
  let offers = [connector("DP-1", false), connector("HDMI-A-1", true), connector("HDMI-A-2", true)];
  assert_eq!(choose_connector(&offers, None), Some(1));
  assert_eq!(choose_connector(&offers, Some("HDMI-A-2")), Some(2));
  assert_eq!(choose_connector(&offers, Some("DP-1")), None, "named but not connected");
  assert_eq!(choose_connector(&[connector("DP-1", false)], None), None);
}

#[test]
fn the_mode_is_the_one_wished_else_the_preferred_else_the_first() {
  let offers = [
    mode(1920, 1080, 60, false),
    mode(1024, 600, 50, false),
    mode(1024, 600, 60, true),
    mode(800, 480, 60, false),
  ];
  assert_eq!(choose_mode(&offers, None), Some(2));
  assert_eq!(choose_mode(&offers[..2], None), Some(0));
  assert_eq!(
    choose_mode(&offers, ModeWish::parse("1024x600")),
    Some(2),
    "the highest refresh of that size"
  );
  assert_eq!(choose_mode(&offers, ModeWish::parse("1024x600@50")), Some(1));
  assert_eq!(choose_mode(&offers, ModeWish::parse("640x480")), None);
  assert_eq!(choose_mode(&[], None), None);
}

#[test]
fn the_settings_default_to_what_a_machine_needs() {
  let config =
    Config::from_lookup(|name| (name == "HOME").then(|| "/home/user".to_owned())).expect("settings");
  assert_eq!(config.face_url, "http://127.0.0.1:3217/bmo-face");
  assert_eq!(config.data, std::path::Path::new("/home/user/.local/share/bmo/data/face-displayer"));
  assert_eq!(config.gaze, config.data.join("gaze.sock"));
  assert_eq!(config.sound, Sound::Device("default".to_owned()));
  assert_eq!((config.card, config.connector, config.mode), (None, None, None));
}

#[test]
fn the_settings_name_the_sound_the_mode_and_the_gaze() {
  let set = |name: &str| match name {
    "HOME" => Some("/home/user".to_owned()),
    "XDG_RUNTIME_DIR" => Some("/run/user/7".to_owned()),
    "BMO_FACE_URL" => Some("http://127.0.0.1:9/face/".to_owned()),
    "BMO_FACE_DISPLAYER_SOUND" => Some("off".to_owned()),
    "BMO_FACE_DISPLAYER_MODE" => Some("1024x600@60".to_owned()),
    _ => None,
  };
  let config = Config::from_lookup(set).expect("settings");
  assert_eq!(config.face_url, "http://127.0.0.1:9/face");
  assert_eq!(config.sound, Sound::Off);
  assert_eq!(
    config.mode,
    Some(ModeWish {
      width: 1024,
      height: 600,
      refresh: Some(60)
    })
  );
  assert_eq!(config.gaze, std::path::Path::new("/run/user/7/bmo-face-gaze.sock"));
  let wrong =
    |name: &str| (name == "BMO_FACE_DISPLAYER_MODE").then(|| "wide".to_owned()).or_else(|| set(name));
  assert!(Config::from_lookup(wrong).is_err());
}
