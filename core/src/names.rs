//! The faces a document holds, by the names BMO sends (docs/face.md, 「The faces」): one per emotion a
//! briefing is spoken with, the names the briefing carries as they stand, and one per presence.

/// The emotions BMO speaks a briefing with; one face each. The first is the face every missing one
/// falls back to.
pub const EMOTIONS: [&str; 10] =
  ["기본", "기쁨", "슬픔", "놀람", "화남", "졸림", "폭소", "걱정", "아픔", "안간힘"];

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
