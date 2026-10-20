//! The tool entry (docs/face.md, 「The expression entry」): the manifest's one entry, its schema holding
//! the emotion to the briefing's set, and the placed executable answering the face of the emotion named
//! and its schema's default where none is.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

fn root() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_owned()
}

fn manifest() -> toml::Value {
  let text = std::fs::read_to_string(root().join("tool.toml")).unwrap();

  text.parse().unwrap()
}

fn schema() -> Value {
  let text = std::fs::read_to_string(root().join("expression.json")).unwrap();

  serde_json::from_str(&text).unwrap()
}

// The call the host hands the executable, answered off standard output.
fn call(arguments: Value) -> Value {
  let handed = json!({
    "protocol": 1,
    "kind": "call",
    "entry": "face/expression",
    "arguments": arguments,
    "session": { "line": "chat", "capabilities": [] },
  });
  let mut ran = Command::new(env!("CARGO_BIN_EXE_bmo-face"))
    .arg("tool")
    .current_dir(root())
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();

  ran.stdin.take().unwrap().write_all(handed.to_string().as_bytes()).unwrap();

  let done = ran.wait_with_output().unwrap();

  assert!(done.status.success(), "{}", String::from_utf8_lossy(&done.stderr));
  serde_json::from_slice(&done.stdout).unwrap()
}

#[test]
fn the_entry_is_the_systems_alone_and_its_schema_holds_the_emotion_to_the_briefings_set() {
  let manifest = manifest();
  let entry = &manifest["entry"][0];
  let schema = schema();
  let emotions: Vec<&str> = schema["properties"]["emotion"]["enum"]
    .as_array()
    .unwrap()
    .iter()
    .map(|one| one.as_str().unwrap())
    .collect();

  assert_eq!(entry["namespace"].as_str(), Some("face"));
  assert_eq!(entry["name"].as_str(), Some("expression"));
  assert_eq!(entry["system"].as_bool(), Some(true));
  assert!(entry.get("capabilities").is_none());
  assert_eq!(
    emotions,
    ["기본", "기쁨", "슬픔", "놀람", "화남", "졸림", "폭소", "걱정", "아픔", "안간힘"]
  );
  assert!(emotions.contains(&schema["properties"]["emotion"]["default"].as_str().unwrap()));
}

#[test]
fn answers_the_face_of_the_emotion_named_and_the_schemas_default_where_none_is() {
  let default = schema()["properties"]["emotion"]["default"].as_str().unwrap().to_owned();

  assert_eq!(call(json!({ "emotion": "기쁨" }))["structuredContent"], json!({ "expression": "기쁨" }));
  assert_eq!(call(json!({}))["structuredContent"], json!({ "expression": default }));
  assert_eq!(call(json!({ "emotion": 3 }))["isError"], json!(true));
}

#[test]
fn refuses_an_entry_it_does_not_hold() {
  let handed =
    json!({ "protocol": 1, "kind": "call", "entry": "face/other", "arguments": {}, "session": {} });
  let mut ran = Command::new(env!("CARGO_BIN_EXE_bmo-face"))
    .arg("tool")
    .current_dir(root())
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()
    .unwrap();

  ran.stdin.take().unwrap().write_all(handed.to_string().as_bytes()).unwrap();

  let done = ran.wait_with_output().unwrap();
  let answered: Value = serde_json::from_slice(&done.stdout).unwrap();

  assert_eq!(answered["isError"], json!(true));
}
