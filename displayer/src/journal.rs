//! What the displayer says (docs/face.md, 「What it says」): into the journal under its own identifier
//! where its standard error is the journal's stream, else on standard error. The journal's own protocol
//! is used rather than the stream because a logind session moves the process into a scope of its own,
//! which the stream's lines are then filed under; the identifier is what still names them.

use std::sync::OnceLock;
use std::time::Duration;

use log::{Level, Record};
use systemd_journal_logger::{JournalLog, connected_to_journal};

/// The name the displayer's lines stand under, in the journal and on standard error.
pub const IDENTIFIER: &str = "bmo-face-displayer";

static JOURNAL: OnceLock<JournalLog> = OnceLock::new();

/// Opens the journal where standard error is its stream; nothing otherwise.
pub fn open() {
  if !connected_to_journal() {
    return;
  }
  match JournalLog::new() {
    Ok(journal) => {
      let _ = JOURNAL.set(journal.with_syslog_identifier(IDENTIFIER.to_owned()));
    }
    Err(fault) => eprintln!("{IDENTIFIER}: the journal: {fault}; saying on standard error"),
  }
}

/// Says one line: into the journal where it is open, else, or where it refuses, on standard error.
pub fn say(line: &str) {
  let sent = JOURNAL.get().is_some_and(|journal| {
    journal
      .journal_send(&Record::builder().level(Level::Info).args(format_args!("{line}")).build())
      .is_ok()
  });
  if !sent {
    eprintln!("{IDENTIFIER}: {line}");
  }
}

/// A wait as a line says it, in seconds.
pub fn seconds(wait: Duration) -> String {
  format!("{} s", wait.as_secs_f64())
}
