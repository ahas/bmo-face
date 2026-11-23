//! The faces a document holds, by the names BMO sends (docs/face.md, 「The faces」): one per emotion a
//! briefing is spoken with, the names the briefing carries as they stand, and one per presence.

/// The emotions BMO speaks a briefing with; one face each. The first is the face every missing one
/// falls back to.
pub const EMOTIONS: [&str; 10] = [
  "neutral",
  "happy",
  "sad",
  "surprised",
  "angry",
  "sleepy",
  "laughing",
  "worried",
  "hurt",
  "straining",
];

/// The names a stored document may still hold an emotion's face under, each beside the name it stands
/// for (`former-emotions.json` beside this file); preparing the document moves such a face to its name (docs/face.md,
/// 「Preparing the document」).
static FORMER_EMOTIONS: std::sync::LazyLock<Vec<(String, String)>> = std::sync::LazyLock::new(|| {
  let held: serde_json::Map<String, serde_json::Value> =
    serde_json::from_str(include_str!("former-emotions.json")).expect("former-emotions.json is an object");
  held
    .into_iter()
    .map(|(former, present)| (former, present.as_str().unwrap_or_default().to_owned()))
    .collect()
});

/// The name a face is held under: an emotion's former name gives way to its name, any other stands.
pub fn present_name(name: &str) -> &str {
  FORMER_EMOTIONS
    .iter()
    .find(|(former, _)| former == name)
    .and_then(|(_, present)| EMOTIONS.into_iter().find(|emotion| emotion == present))
    .unwrap_or(name)
}

/// The former name an emotion's face may still be held under in a stored document, where it has one.
pub fn former_name(present: &str) -> Option<&'static str> {
  FORMER_EMOTIONS
    .iter()
    .find(|(_, now)| now == present)
    .map(|(former, _)| former.as_str())
}

/// What BMO is doing, as its presence names it; one face each.
pub const STATES: [&str; 3] = ["writing", "thinking", "idle"];

/// The face every missing face falls back to, which a document must therefore hold.
pub const FALLBACK: &str = EMOTIONS[0];

/// The face shown once BMO has been idle a long stretch.
pub const ASLEEP: &str = EMOTIONS[5];

/// The presence of a BMO at rest, which is also the blank face its rest gives way to.
pub const IDLE: &str = STATES[2];

/// Every face name, the emotions first.
pub fn face_names() -> impl Iterator<Item = &'static str> {
  EMOTIONS.into_iter().chain(STATES)
}

pub fn is_face_name(name: &str) -> bool {
  face_names().any(|known| known == name)
}

/// The presence states.
pub fn is_state(name: &str) -> bool {
  STATES.contains(&name)
}
