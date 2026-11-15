//! The device token's file (docs/face.md, 「Signing in」): made private before anything is written,
//! replaced whole, and forgotten.

use std::fs;
use std::os::unix::fs::PermissionsExt;

use bmo_face_displayer::token::TokenFile;

fn mode(path: &std::path::Path) -> u32 {
  fs::metadata(path).expect("the path stands").permissions().mode() & 0o777
}

#[test]
fn the_token_is_kept_private_and_replaced_whole() {
  let root = tempfile::tempdir().expect("a scratch folder");
  let data = root.path().join("data");
  let file = TokenFile::new(&data);
  assert_eq!(file.read().expect("nothing to read is no fault"), None);

  file.write("first-token").expect("written");
  assert_eq!(mode(&data), 0o700);
  assert_eq!(mode(&file.path()), 0o600);
  assert_eq!(file.read().expect("read").as_deref(), Some("first-token"));

  file.write("second-token").expect("written again");
  assert_eq!(file.read().expect("read").as_deref(), Some("second-token"));
  assert_eq!(mode(&file.path()), 0o600);
  let names: Vec<String> = fs::read_dir(&data)
    .expect("listed")
    .map(|entry| entry.expect("an entry").file_name().to_string_lossy().into_owned())
    .collect();
  assert_eq!(names, ["token"], "no partial file is left beside it");
}

#[test]
fn a_folder_left_open_is_closed_and_a_forgotten_token_is_gone() {
  let root = tempfile::tempdir().expect("a scratch folder");
  fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).expect("opened");
  let file = TokenFile::new(root.path());
  file.write("a-token").expect("written");
  assert_eq!(mode(root.path()), 0o700);
  file.forget().expect("forgotten");
  assert_eq!(file.read().expect("read"), None);
  file.forget().expect("forgetting twice is no fault");
}
