//! The `face/expression` entry of the tool host (docs/face.md, 「The expression entry」): the face a
//! briefing shows, by the emotion it is spoken with, answered on the call protocol's first version.
//! The emotions and the default stand in the entry's schema alone, which the host holds a call to
//! before it runs and which the deployment places beside the executable.

use std::io::{Read, Write};

use serde_json::{Value, json};

/// The protocol version this side speaks.
pub const PROTOCOL: u64 = 1;

/// The entry this executable answers.
pub const ENTRY: &str = "face/expression";

/// The schema's own default emotion, the face shown where none is named, read beside the executable.
pub fn default_emotion(dir: &std::path::Path) -> Result<String, String> {
  let text = std::fs::read_to_string(dir.join("expression.json"))
    .map_err(|why| format!("expression.json: {why}"))?;
  let schema: Value = serde_json::from_str(&text).map_err(|why| format!("expression.json: {why}"))?;
  let named = schema
    .pointer("/properties/emotion/default")
    .and_then(Value::as_str)
    .ok_or("the schema beside the executable names no default emotion")?;

  Ok(named.to_owned())
}

/// What the entry answers for one call, or why it refuses it.
pub fn expression(arguments: &Value, dir: &std::path::Path) -> Result<Value, String> {
  match arguments.get("emotion") {
    None | Some(Value::Null) => default_emotion(dir).map(|emotion| json!({ "expression": emotion })),
    Some(Value::String(emotion)) => Ok(json!({ "expression": emotion })),
    Some(_) => Err("the emotion is no word".to_owned()),
  }
}

/// A call's result saying a structured object: the object, and the same object as its text.
fn structured(object: Value) -> Value {
  json!({
    "content": [{ "type": "text", "text": object.to_string() }],
    "isError": false,
    "structuredContent": object,
  })
}

/// A refused call, which the model reads as any other answer.
fn refusal(said: &str) -> Value {
  json!({
    "content": [{ "type": "text", "text": format!("ERROR: {said}") }],
    "isError": true,
  })
}

/// The answer to one handed request.
pub fn answer(handed: &Value, dir: &std::path::Path) -> Value {
  if handed.get("kind").and_then(Value::as_str) != Some("call") {
    return refusal("this executable answers calls alone");
  }

  let entry = handed.get("entry").and_then(Value::as_str).unwrap_or_default();

  if entry != ENTRY {
    return refusal(&format!("this executable holds no entry {entry}"));
  }

  let arguments = handed.get("arguments").cloned().unwrap_or(Value::Null);

  match expression(&arguments, dir) {
    Ok(object) => structured(object),
    Err(said) => refusal(&said),
  }
}

/// One call read off standard input and answered on standard output; a request that does not read is
/// said on standard error and ends unclean, which the host takes as a failed call.
pub fn run() -> std::process::ExitCode {
  let mut text = String::new();

  if let Err(why) = std::io::stdin().read_to_string(&mut text) {
    eprintln!("the call could not be read: {why}");
    return std::process::ExitCode::FAILURE;
  }

  let handed: Value = match serde_json::from_str(&text) {
    Ok(handed) => handed,
    Err(why) => {
      eprintln!("the call does not read: {why}");
      return std::process::ExitCode::FAILURE;
    }
  };

  let dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
  let said = answer(&handed, &dir);

  match std::io::stdout().lock().write_all(said.to_string().as_bytes()) {
    Ok(()) => std::process::ExitCode::SUCCESS,
    Err(why) => {
      eprintln!("the answer could not be written: {why}");
      std::process::ExitCode::FAILURE
    }
  }
}
