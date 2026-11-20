//! The gaze socket (docs/face.md, 「The gaze socket」): a local Unix stream socket, reachable by the
//! displayer's own user alone, where a later camera program says where the eyes look, one JSON object a
//! line.

use std::fs::{self, Permissions};
use std::io;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::Path;

use bmo_face_core::geometry::Point;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc::UnboundedSender;

use crate::journal::say;

const SOCKET_MODE: u32 = 0o600;

/// A line read as a direction: `{"x": …, "y": …}`, each a number.
pub fn direction(line: &str) -> Result<Point, String> {
  let value: Value = serde_json::from_str(line).map_err(|fault| format!("not JSON: {fault}"))?;
  match (value["x"].as_f64(), value["y"].as_f64()) {
    (Some(x), Some(y)) => Ok([x, y]),
    _ => Err("no numbers x and y".to_owned()),
  }
}

/// Listens on the socket's path, a socket left there by an earlier run taken away first.
pub fn listen(path: &Path) -> io::Result<UnixListener> {
  if fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_socket()) {
    fs::remove_file(path)?;
  }
  if let Some(folder) = path.parent() {
    fs::create_dir_all(folder)?;
  }
  let listener = UnixListener::bind(path)?;
  fs::set_permissions(path, Permissions::from_mode(SOCKET_MODE))?;
  Ok(listener)
}

/// Takes every sender for as long as the displayer runs, telling it each direction heard.
pub async fn serve(listener: UnixListener, tell: UnboundedSender<Point>) {
  loop {
    match listener.accept().await {
      Ok((stream, _)) => {
        tokio::spawn(read(stream, tell.clone()));
      }
      Err(fault) => say(&format!("the gaze socket: {fault}")),
    }
  }
}

async fn read(stream: UnixStream, tell: UnboundedSender<Point>) {
  let mut lines = BufReader::new(stream).lines();
  while let Ok(Some(line)) = lines.next_line().await {
    if line.trim().is_empty() {
      continue;
    }
    match direction(&line) {
      Ok(point) => {
        if tell.send(point).is_err() {
          return;
        }
      }
      Err(fault) => say(&format!("a gaze line left: {fault}")),
    }
  }
}
