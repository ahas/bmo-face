//! BMO's face server (docs/face.md, 「The server」): the expression document, the editor's sessions,
//! the saves announced to every open display, the generated screen under `/bmo-face/`, a narrow
//! pass-through to BMO's listener routes, the console screen's manifest, bundle and guard, and the
//! `face/expression` tool entry.

pub mod accounts;
pub mod console;
pub mod faces;
pub mod handler;
pub mod login;
pub mod numbers;
pub mod pass;
pub mod store;
pub mod tool;
pub mod upgrade;
