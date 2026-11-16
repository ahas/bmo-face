//! `bmo-face-displayer` (docs/face.md, 「The native displayer」): `run` shows BMO's face on the panel, or
//! writes its frames into a folder; `shots` writes the default faces, a blink and a talking sequence as
//! pictures.

use std::path::PathBuf;
use std::process::ExitCode;

use bmo_face_displayer::config::Config;
use bmo_face_displayer::run::{Output, run};
use bmo_face_displayer::shots;
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
  /// Writes the default faces, a blink and a talking sequence as PNGs into a folder.
  Shots { folder: PathBuf },
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
