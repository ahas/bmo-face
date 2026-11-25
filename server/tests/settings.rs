//! The server's settings and the deployment's three commands (docs/face.md, 「The server」): the environment
//! over the install-written `env/face.env` under the data root, read by `bmo-face address`, `bmo-face health`
//! and `bmo-face console-reach` as the server reads it.

use std::path::Path;
use std::process::{Command, Output};

use axum::Router;
use axum::routing::get;
use serde_json::Value;

/// The binary run as `command` over the data root `data`, with `environment` set and no other setting of
/// the face's in its environment.
fn run(command: &str, data: &Path, environment: &[(&str, &str)]) -> Output {
  let mut run = Command::new(env!("CARGO_BIN_EXE_bmo-face"));
  run.arg(command).env("BMO_DATA_DIR", data);
  for variable in ["BMO_FACE_PORT", "BMO_FACE_TOKEN"] {
    run.env_remove(variable);
  }
  for (variable, value) in environment {
    run.env(variable, value);
  }
  run.output().expect("the binary runs")
}

fn said(output: &Output) -> String {
  format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr))
}

/// A data root whose `env/face.env` holds `lines`.
fn data_root(lines: &str) -> tempfile::TempDir {
  let data = tempfile::tempdir().expect("a scratch data root");
  std::fs::create_dir(data.path().join("env")).unwrap();
  std::fs::write(data.path().join("env/face.env"), lines).unwrap();
  data
}

#[test]
fn the_address_is_the_port_the_settings_file_names_under_the_environment() {
  let data = data_root("BMO_FACE_PORT=4321\nBMO_FACE_TOKEN=a-console-credential\n");

  let from_file = run("address", data.path(), &[]);
  assert_eq!(said(&from_file), "http://127.0.0.1:4321/bmo-face\n");
  let over_it = run("address", data.path(), &[("BMO_FACE_PORT", "5000")]);
  assert_eq!(said(&over_it), "http://127.0.0.1:5000/bmo-face\n");
  let masked = run("address", data.path(), &[("BMO_FACE_PORT", "")]);
  assert_eq!(said(&masked), "http://127.0.0.1:3217/bmo-face\n");

  let empty = tempfile::tempdir().unwrap();
  let no_file = run("address", empty.path(), &[]);
  assert_eq!(said(&no_file), "http://127.0.0.1:3217/bmo-face\n");
}

#[test]
fn a_settings_file_that_does_not_read_stops_the_binary() {
  let data = data_root("BMO_FACE_PORT='4321\n");
  let ran = run("address", data.path(), &[]);

  assert_eq!(ran.status.code(), Some(1));
  assert!(said(&ran).starts_with("bmo-face: the settings could not be read"), "{}", said(&ran));
}

#[test]
fn the_console_reach_is_the_settings_address_and_credential_with_a_fresh_stand_in() {
  let data = data_root("BMO_FACE_PORT=4321\nBMO_FACE_TOKEN=a-console-credential\n");
  let reach = |output: Output| -> Value {
    assert!(output.status.success(), "{}", said(&output));
    serde_json::from_slice(&output.stdout).expect("the reach is JSON")
  };

  let first = reach(run("console-reach", data.path(), &[]));
  let second = reach(run("console-reach", data.path(), &[]));
  assert_eq!(first["address"], "http://127.0.0.1:4321/bmo-face");
  assert_eq!(first["consoleToken"], "a-console-credential");
  let token = first["token"].as_str().expect("a stand-in token");
  assert_eq!(token.len(), 48);
  assert_ne!(first["token"], second["token"]);
  assert_ne!(token, "a-console-credential");

  let without = data_root("BMO_FACE_PORT=4321\n");
  let refused = run("console-reach", without.path(), &[]);
  assert_eq!(refused.status.code(), Some(1));
  assert!(said(&refused).contains("BMO_FACE_TOKEN"), "{}", said(&refused));
}

#[tokio::test]
async fn the_health_asks_the_server_at_the_port_its_settings_name() {
  let screen = Router::new()
    .route("/bmo-face/api/faces", get(|| async { "{}" }))
    .route("/bmo-face/", get(|| async { "<!DOCTYPE html><HTML><body></body></HTML>" }));
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let serving = listener.local_addr().unwrap().port();
  tokio::spawn(async move { axum::serve(listener, screen).await.unwrap() });
  let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let nobody = closed.local_addr().unwrap().port();
  drop(closed);

  let at_the_server = data_root(&format!("BMO_FACE_PORT={serving}\n"));
  let ready = tokio::task::spawn_blocking(move || run("health", at_the_server.path(), &[]))
    .await
    .unwrap();
  assert_eq!(ready.status.code(), Some(0), "{}", said(&ready));

  let elsewhere = data_root(&format!("BMO_FACE_PORT={nobody}\n"));
  let unreached = tokio::task::spawn_blocking(move || run("health", elsewhere.path(), &[])).await.unwrap();
  assert_eq!(unreached.status.code(), Some(1));
  assert!(said(&unreached).starts_with("bmo-face: not ready"), "{}", said(&unreached));
}
