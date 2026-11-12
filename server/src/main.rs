//! The face server the unit `bmo-face` runs (docs/face.md, 「The server」), configured by its
//! environment: `BMO_FACE_DATA`, `BMO_FACE_PORT`, `BMO_FACE_STATIC`, `BMO_FACE_CONSOLE`,
//! `BMO_FACE_TOKEN`, BMO's archive, `BMO_CONSOLE_URL` and `BMO_FACE_DRAIN_MS`, and drained when asked
//! to stop.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bmo_face::accounts::{Accounts, archive_place};
use bmo_face::console::Console;
use bmo_face::handler::{Face, PREFIX, router};
use bmo_face::login::EditorLogin;
use bmo_face::stop::{DRAIN, drain};
use bmo_face::store::{FaceStore, Preparation};
use tokio::net::TcpListener;

fn named(variable: &str) -> Option<String> {
  std::env::var(variable).ok().filter(|value| !value.is_empty())
}

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
  // `tool` is the tool host's one call; every other start is the server.
  if std::env::args().nth(1).as_deref() == Some("tool") {
    return bmo_face::tool::run();
  }

  tokio::runtime::Runtime::new().map_or_else(
    |why| {
      eprintln!("bmo-face: the runtime could not be started: {why}");
      ExitCode::FAILURE
    },
    |runtime| runtime.block_on(serve()),
  )
}

async fn serve() -> ExitCode {
  let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| "/".into()));
  let root = named("BMO_FACE_DATA")
    .map(PathBuf::from)
    .unwrap_or_else(|| home.join(".local/share/bmo-face"));
  let port: u16 = named("BMO_FACE_PORT").and_then(|port| port.parse().ok()).unwrap_or(3217);
  let static_dir = named("BMO_FACE_STATIC").map(PathBuf::from).unwrap_or_else(|| own_output("public"));
  let console = Console::new(
    named("BMO_FACE_CONSOLE").map(PathBuf::from).unwrap_or_else(|| own_output("console")),
    named("BMO_FACE_TOKEN"),
  );
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
