//! Where the server listens (docs/face.md, 「The server」): `bmo-face address` prints the loopback address
//! under the base path at the port `BMO_FACE_PORT` names, 3217 where it names none, which the deployment's
//! health asks the server at once the unit's environment file is read.

use std::process::Command;

fn address(port: Option<&str>) -> String {
  let mut command = Command::new(env!("CARGO_BIN_EXE_bmo-face"));
  command.arg("address").env_remove("BMO_FACE_PORT");
  if let Some(port) = port {
    command.env("BMO_FACE_PORT", port);
  }
  let ran = command.output().expect("the binary runs");
  assert!(ran.status.success(), "{}", String::from_utf8_lossy(&ran.stderr));
  String::from_utf8(ran.stdout).expect("the address is text")
}

#[test]
fn the_address_is_the_port_the_environment_names_else_3217() {
  assert_eq!(address(Some("4321")), "http://127.0.0.1:4321/bmo-face\n");
  assert_eq!(address(None), "http://127.0.0.1:3217/bmo-face\n");
}
