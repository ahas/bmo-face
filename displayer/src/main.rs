//! `bmo-face-displayer` (docs/face.md, 「The native displayer」): `run` shows BMO's face on the panel, or
//! writes its frames into a folder; `sign-in` signs the displayer in to BMO once; `shots` writes the
//! default faces, a blink and a talking sequence as pictures.

use std::io::{BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use bmo_face_displayer::config::Config;
use bmo_face_displayer::run::{Output, run};
use bmo_face_displayer::server::{FaceServer, Fault};
use bmo_face_displayer::shots;
use bmo_face_displayer::token::TokenFile;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bmo-face-displayer", about = "BMO's face on a machine's own panel.")]
struct Command {
  #[command(subcommand)]
  verb: Option<Verb>,
}

#[derive(Subcommand)]
enum Verb {
  /// Shows the face on the panel, for as long as the machine runs; the default.
  Run {
    /// Writes every drawing as a PNG into this folder instead of taking a panel.
    #[arg(long)]
    frames: Option<PathBuf>,
  },
  /// Signs in to BMO with a web account, the password read from standard input.
  SignIn {
    /// The BMO web account.
    #[arg(long)]
    name: String,
    /// The label the displayer stands by on BMO's account screen.
    #[arg(long)]
    device: String,
  },
  /// Writes the default faces, a blink and a talking sequence as PNGs into a folder.
  Shots { folder: PathBuf },
}

/// Turns the terminal's echo off while a password is typed, and back on after.
struct Unechoed(Option<libc::termios>);

impl Unechoed {
  fn new() -> Unechoed {
    let stdin = libc::STDIN_FILENO;
    let mut was = std::mem::MaybeUninit::<libc::termios>::uninit();
    // SAFETY: tcgetattr fills `was` for a terminal and touches nothing where it fails.
    if !std::io::stdin().is_terminal() || unsafe { libc::tcgetattr(stdin, was.as_mut_ptr()) } != 0 {
      return Unechoed(None);
    }
    // SAFETY: filled by the successful call above.
    let was = unsafe { was.assume_init() };
    let mut quiet = was;
    quiet.c_lflag &= !libc::ECHO;
    // SAFETY: a copy of the terminal's own settings, echo cleared.
    unsafe { libc::tcsetattr(stdin, libc::TCSANOW, &quiet) };
    Unechoed(Some(was))
  }
}

impl Drop for Unechoed {
  fn drop(&mut self) {
    if let Some(was) = &self.0 {
      // SAFETY: the settings the terminal had before.
      unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, was) };
    }
  }
}

fn password() -> Result<String, String> {
  let mut line = String::new();
  {
    if std::io::stdin().is_terminal() {
      eprint!("password: ");
      let _ = std::io::stderr().flush();
    }
    let _quiet = Unechoed::new();
    std::io::stdin()
      .lock()
      .read_line(&mut line)
      .map_err(|fault| format!("the password: {fault}"))?;
  }
  if std::io::stdin().is_terminal() {
    eprintln!();
  }
  let password = line.trim_end_matches(['\r', '\n']).to_owned();
  if password.is_empty() { Err("no password was given".to_owned()) } else { Ok(password) }
}

async fn sign_in(config: &Config, name: &str, device: &str) -> Result<(), String> {
  let password = password()?;
  let server = FaceServer::new(&config.face_url);
  let token = match server.sign_in(name, &password, device).await {
    Ok(token) => token,
    Err(Fault::Refused(_) | Fault::Status(hyper::StatusCode::BAD_REQUEST)) => {
      return Err("BMO did not take the account and password".to_owned());
    }
    Err(fault) => return Err(format!("signing in: {fault}")),
  };
  let file = TokenFile::new(&config.data);
  file.write(&token).map_err(|fault| format!("{}: {fault}", file.path().display()))?;
  eprintln!("signed in; the token is kept in {}", file.path().display());
  Ok(())
}

fn main() -> ExitCode {
  let command = Command::parse();
  let outcome = (|| -> Result<(), String> {
    if let Some(Verb::Shots { folder }) = &command.verb {
      let written = shots::write_all(folder)?;
      eprintln!("{written} pictures written into {}", folder.display());
      return Ok(());
    }
    let config = Config::from_env()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .map_err(|fault| format!("the runtime: {fault}"))?;
    runtime.block_on(async {
      match command.verb {
        Some(Verb::SignIn { name, device }) => sign_in(&config, &name, &device).await,
        Some(Verb::Run {
          frames: Some(folder),
        }) => {
          std::fs::create_dir_all(&folder).map_err(|fault| format!("{}: {fault}", folder.display()))?;
          run(config, Output::Frames(folder)).await
        }
        Some(Verb::Run { frames: None }) | None => run(config, Output::Panel).await,
        Some(Verb::Shots { .. }) => Ok(()),
      }
    })
  })();
  match outcome {
    Ok(()) => ExitCode::SUCCESS,
    Err(fault) => {
      eprintln!("bmo-face-displayer: {fault}");
      ExitCode::FAILURE
    }
  }
}
