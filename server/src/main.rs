//! The face server the unit `bmo-face` runs (docs/face.md, 「The server」), configured by its
//! environment over the install-written `env/face.env`: `BMO_FACE_DATA`, `BMO_FACE_PORT`,
//! `BMO_FACE_STATIC`, `BMO_FACE_CONSOLE`, `BMO_FACE_TOKEN`, BMO's archive, `BMO_CONSOLE_URL` and
//! `BMO_FACE_DRAIN_MS`, and drained when asked to stop. Started as `bmo-face address`, `health` or
//! `console-reach`, it answers the deployment's question from the same settings and serves nothing.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bmo_face::accounts::{Accounts, archive_place};
use bmo_face::console::Console;
use bmo_face::handler::{Face, PREFIX, router};
use bmo_face::health;
use bmo_face::login::EditorLogin;
use bmo_face::settings::Settings;
use bmo_face::stop::{DRAIN, drain};
use bmo_face::store::{FaceStore, Preparation};
use tokio::net::TcpListener;

fn now() -> u64 {
  SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_millis() as u64)
}

// A folder the screen's build writes beside the binary's project: `<project>/bin/bmo-face` serves
// `<project>/client/.output/public` and `<project>/client/.output/console`.
fn own_output(folder: &str) -> PathBuf {
  let output = PathBuf::from("client/.output").join(folder);
  std::env::current_exe()
    .ok()
    .and_then(|exe| Some(exe.parent()?.parent()?.join(&output)))
    .unwrap_or(output)
}

fn main() -> ExitCode {
  // `tool` is the tool host's one call; every other start reads the settings.
  if std::env::args().nth(1).as_deref() == Some("tool") {
    return bmo_face::tool::run();
  }
  let settings = match Settings::load() {
    Ok(settings) => settings,
    Err(why) => {
      eprintln!("bmo-face: the settings could not be read: {why}");
      return ExitCode::FAILURE;
    }
  };
  let command = std::env::args().nth(1);

  // The deployment's three questions, each answered from the settings the server reads.
  match command.as_deref() {
    Some("address") => {
      println!("{}", health::address(&settings));
      return ExitCode::SUCCESS;
    }
    Some("console-reach") => {
      return match health::console_reach(&settings) {
        Ok(reach) => {
          println!("{reach}");
          ExitCode::SUCCESS
        }
        Err(why) => {
          eprintln!("bmo-face: {why}");
          ExitCode::FAILURE
        }
      };
    }
    _ => {}
  }

  tokio::runtime::Runtime::new().map_or_else(
    |why| {
      eprintln!("bmo-face: the runtime could not be started: {why}");
      ExitCode::FAILURE
    },
    |runtime| match command.as_deref() {
      Some("health") => runtime.block_on(ready(&settings)),
      _ => runtime.block_on(serve(settings)),
    },
  )
}

async fn ready(settings: &Settings) -> ExitCode {
  let address = health::address(settings);
  match health::ready(&address).await {
    Ok(()) => {
      println!("bmo-face: ready at {address}/");
      ExitCode::SUCCESS
    }
    Err(why) => {
      eprintln!("bmo-face: not ready: {why}");
      ExitCode::FAILURE
    }
  }
}

async fn serve(settings: Settings) -> ExitCode {
  let named = |variable: &str| settings.named(variable);
  let home = settings.home();
  let root = named("BMO_FACE_DATA")
    .map(PathBuf::from)
    .unwrap_or_else(|| home.join(".local/share/bmo-face"));
  let port = settings.port();
  let static_dir = named("BMO_FACE_STATIC").map(PathBuf::from).unwrap_or_else(|| own_output("public"));
  let console_dir = named("BMO_FACE_CONSOLE").map(PathBuf::from).unwrap_or_else(|| own_output("console"));
  let console = match Console::new(&console_dir, named("BMO_FACE_TOKEN")) {
    Ok(console) => console,
    Err(fault) => {
      eprintln!("bmo-face: the console face could not be built from {}: {fault}", console_dir.display());
      return ExitCode::FAILURE;
    }
  };
  let console_url = named("BMO_CONSOLE_URL").unwrap_or_else(|| "http://127.0.0.1:3210/bmo".to_owned());
  let keep = Duration::from_millis(
    named("BMO_FACE_KEEP_MS")
      .and_then(|ms| ms.parse().ok())
      .filter(|ms| *ms > 0)
      .unwrap_or(5_000),
  );
  let deadline = named("BMO_FACE_DRAIN_MS")
    .and_then(|ms| ms.parse().ok())
    .map_or(DRAIN, Duration::from_millis);

  if let Err(fault) = std::fs::create_dir_all(&root) {
    eprintln!("bmo-face: {} could not be made: {fault}", root.display());
    return ExitCode::FAILURE;
  }

  // The document is made right once, here, before the server answers anything; after this a read
  // only reads.
  let store = FaceStore::new(root.clone(), now);
  match store.prepare() {
    Ok(Preparation::Kept) => {}
    Ok(Preparation::Broken { reason, moved_to }) => eprintln!(
      "bmo-face: the stored expression document was refused ({reason}); it stands at {} and the default was written in its place",
      moved_to.display()
    ),
    Ok(Preparation::Created) => println!("bmo-face: expression document missing; the default was written"),
    Ok(Preparation::Default) => {
      println!(
        "bmo-face: expression document never saved from the editor; the default drawn now was written"
      )
    }
    Ok(Preparation::Upgraded) => println!(
      "bmo-face: expression document of the point-and-ellipse shape, or without a face the default draws; it was brought up to this shape and written with its version kept"
    ),
    Err(fault) => {
      eprintln!("bmo-face: the expression document could not be prepared: {fault}");
      return ExitCode::FAILURE;
    }
  }

  let place = archive_place(named, &home);
  let accounts = match Accounts::at(&place) {
    Ok(accounts) => accounts,
    Err(fault) => {
      eprintln!("bmo-face: BMO's accounts at {place} could not be opened: {fault}");
      return ExitCode::FAILURE;
    }
  };
  let login = match EditorLogin::open(&root, accounts, now).await {
    Ok(login) => login,
    Err(fault) => {
      eprintln!("bmo-face: the session database could not be opened: {fault}");
      return ExitCode::FAILURE;
    }
  };
  let face = Arc::new(Face::new(store, login, static_dir, console_url.clone(), keep, console));

  let listener = match TcpListener::bind(("127.0.0.1", port)).await {
    Ok(listener) => listener,
    Err(fault) => {
      eprintln!("bmo-face: 127.0.0.1:{port} could not be listened on: {fault}");
      return ExitCode::FAILURE;
    }
  };
  // The stopping signals are heard before the port is named, so a stop sent from then on drains.
  let stop = face.stop.clone();
  tokio::spawn(stop.clone().on_signal());
  // A port of 0 is one the system chooses, named here as the server starts answering.
  let port = listener.local_addr().map_or(port, |at| at.port());
  println!("BMO face: http://127.0.0.1:{port}{PREFIX}/ (BMO at {console_url})");
  let app = router(face).into_make_service_with_connect_info::<SocketAddr>();
  match drain(listener, app, stop, deadline).await {
    Ok(()) => ExitCode::SUCCESS,
    Err(fault) => {
      eprintln!("bmo-face: {fault}");
      ExitCode::FAILURE
    }
  }
}
