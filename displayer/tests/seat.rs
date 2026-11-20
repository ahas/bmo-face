//! Which cards the displayer tries for the panel (docs/face.md, 「How it takes the panel」): the cards the
//! kernel reads a connected connector on, in their numbers' order, read before any card is asked for.

use std::fs;
use std::path::Path;

use bmo_face_displayer::seat::cards_with_connected;

fn device(devices: &Path, name: &str) {
  fs::write(devices.join(name), "").expect("a device stand-in");
}

fn connector(classes: &Path, name: &str, status: &str) {
  let folder = classes.join(name);
  fs::create_dir_all(&folder).expect("a connector's folder");
  fs::write(folder.join("status"), format!("{status}\n")).expect("a connector's status");
}

#[test]
fn only_cards_with_a_connected_connector_are_tried_in_their_numbers_order() {
  let scratch = tempfile::tempdir().expect("a scratch folder");
  let (devices, classes) = (scratch.path().join("dri"), scratch.path().join("drm"));
  fs::create_dir_all(&devices).expect("the devices' folder");
  fs::create_dir_all(classes.join("card1")).expect("a card's own folder");
  for name in ["card1", "card2", "card10", "card11", "renderD128"] {
    device(&devices, name);
  }
  // A card whose connectors are all disconnected, as one another session holds may be.
  connector(&classes, "card1-DP-1", "disconnected");
  connector(&classes, "card1-HDMI-A-2", "unknown");
  connector(&classes, "card2-DP-2", "disconnected");
  connector(&classes, "card2-HDMI-A-1", "connected");
  // card10's connector is not card1's, though its name starts with card1.
  connector(&classes, "card10-DP-3", "connected");
  connector(&classes, "card11-eDP-1", "connected");

  assert_eq!(
    cards_with_connected(&devices, &classes),
    [devices.join("card2"), devices.join("card10"), devices.join("card11")]
  );
}

#[test]
fn no_card_is_tried_where_nothing_is_connected_or_nothing_can_be_read() {
  let scratch = tempfile::tempdir().expect("a scratch folder");
  let (devices, classes) = (scratch.path().join("dri"), scratch.path().join("drm"));
  fs::create_dir_all(&devices).expect("the devices' folder");
  device(&devices, "card1");
  connector(&classes, "card1-DP-1", "disconnected");

  assert!(cards_with_connected(&devices, &classes).is_empty());
  assert!(cards_with_connected(&scratch.path().join("none"), &classes).is_empty());
}
