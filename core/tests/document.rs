//! The expression document and its check (docs/face.md, 「The expression document」).

use bmo_face_core::check::check;
use bmo_face_core::document::{FORM, SCREEN};
use bmo_face_core::names::{FALLBACK, face_names};
use serde_json::{Value, json};

use crate::support::document;

fn default_json() -> Value {
  document().to_json()
}

fn refusal(value: &Value) -> Value {
  check(value).expect_err("the check refuses it").to_json()
}

#[test]
fn the_default_holds_every_face_and_passes_its_own_check_as_it_stands() {
  let default = default_json();
  assert_eq!(default["form"], json!(FORM));
  let names: Vec<&str> = default["faces"].as_object().unwrap().keys().map(String::as_str).collect();
  assert_eq!(names, face_names().collect::<Vec<_>>());
  let checked = check(&default).expect("the default passes");
  assert_eq!(checked.to_json(), default);
}

#[test]
fn every_anchor_and_handle_of_the_default_stands_on_the_screen() {
  let default = document();
  let parts = default
    .faces
    .iter()
    .flat_map(|(_, face)| bmo_face_core::look::Look::of(face).parts().cloned().collect::<Vec<_>>())
    .chain(default.talk.anchors.iter().flat_map(|anchor| anchor.parts.clone()));
  for part in parts.flat_map(|part| std::iter::once(part.clone()).chain(part.inside.clone())) {
    for node in &part.nodes {
      for [x, y] in [
        node.anchor,
        bmo_face_core::geometry::add(node.anchor, node.coming),
        bmo_face_core::geometry::add(node.anchor, node.leaving),
      ] {
        assert!((0.0..=SCREEN[0]).contains(&x) && (0.0..=SCREEN[1]).contains(&y), "{x} {y}");
      }
    }
  }
}

#[test]
fn a_refusal_names_its_code_where_it_was_met_and_the_limits_it_turns_on() {
  let mut wide = default_json();
  wide["faces"][FALLBACK]["mouth"][0]["line"]["width"] = json!(12);
  assert_eq!(
    refusal(&wide),
    json!({ "code": "line-width", "at": [{ "face": FALLBACK }, { "element": "mouth" }, { "part": 1 }], "least": 0.2, "most": 10 })
  );

  let mut outside = default_json();
  outside["faces"][FALLBACK]["eyes"]["left"][0]["nodes"][1]["anchor"] = json!([101, 20]);
  assert_eq!(
    refusal(&outside)["at"],
    json!([{ "face": FALLBACK }, { "element": "eyes" }, { "side": "left" }, { "part": 1 }, { "node": 2 }])
  );

  let mut unknown = default_json();
  unknown["faces"]["bored"] = unknown["faces"][FALLBACK].clone();
  assert_eq!(refusal(&unknown), json!({ "code": "unknown-face", "at": [], "name": "bored" }));
}

#[test]
fn the_check_refuses_a_document_whole_for_each_fault() {
  type Change = Box<dyn Fn(&mut Value)>;
  let cases: Vec<(&str, Change)> = vec![
    ("not-a-document", Box::new(|doc| doc["form"] = json!(1))),
    ("color", Box::new(|doc| doc["colors"]["tongueLight"] = json!("green"))),
    (
      "no-fallback",
      Box::new(|doc| {
        doc["faces"].as_object_mut().unwrap().shift_remove(FALLBACK);
      }),
    ),
    ("not-a-pair", Box::new(|doc| doc["faces"][FALLBACK]["brows"] = json!([]))),
    ("reach", Box::new(|doc| doc["faces"][FALLBACK]["eyes"]["reach"] = json!([7, 0]))),
    ("node-count", Box::new(|doc| doc["faces"][FALLBACK]["mouth"][0]["nodes"] = json!([]))),
    ("open-filled", Box::new(|doc| doc["faces"][FALLBACK]["mouth"][0]["fill"] = json!("ink"))),
    ("inside-open", Box::new(|doc| doc["faces"][FALLBACK]["mouth"][0]["inside"] = json!([]))),
    (
      "nested-inside",
      Box::new(|doc| {
        let mouth = doc["talk"]["anchors"][0]["parts"][0].clone();
        doc["talk"]["anchors"][0]["parts"][0]["inside"][0]["inside"] = json!([mouth]);
      }),
    ),
    (
      "eye-voice",
      Box::new(|doc| doc["faces"][FALLBACK]["eyes"]["voice"]["grow"] = json!(0.6)),
    ),
    ("corners", Box::new(|doc| doc["faces"][FALLBACK]["corners"] = json!(2))),
    ("anchors", Box::new(|doc| doc["talk"]["anchors"] = json!([]))),
    ("anchors", Box::new(|doc| doc["talk"] = json!([]))),
    ("anchor-value", Box::new(|doc| doc["talk"]["anchors"][1]["at"] = json!([1.2, 0]))),
    ("anchor-value", Box::new(|doc| doc["talk"]["anchors"][1]["teeth"] = json!("no"))),
    (
      "anchor-place",
      Box::new(|doc| doc["talk"]["anchors"][2]["at"] = doc["talk"]["anchors"][0]["at"].clone()),
    ),
    ("tuning", Box::new(|doc| doc["talk"]["tuning"]["attack"] = json!(900))),
    ("tuning", Box::new(|doc| doc["talk"]["tuning"]["midHz"] = json!([2_000, 1_000]))),
    ("tuning", Box::new(|doc| doc["talk"]["tuning"]["onTwos"] = json!(1))),
  ];
  for (code, change) in cases {
    let mut document = default_json();
    change(&mut document);
    assert_eq!(refusal(&document)["code"], json!(code), "{code}");
  }
}

#[test]
fn a_refusal_in_the_talking_set_names_the_anchor_or_the_tuning_value() {
  let mut crowded = default_json();
  crowded["talk"]["anchors"][3]["at"] = crowded["talk"]["anchors"][1]["at"].clone();
  assert_eq!(refusal(&crowded), json!({ "code": "anchor-place", "at": [{ "anchor": 4 }] }));
  // The same place on the other plane is no crowding.
  let mut apart = default_json();
  apart["talk"]["anchors"][16]["at"] = apart["talk"]["anchors"][1]["at"].clone();
  assert!(check(&apart).is_ok());

  let mut slow = default_json();
  slow["talk"]["tuning"]["rest"] = json!(-1);
  assert_eq!(
    refusal(&slow),
    json!({ "code": "tuning", "at": [{ "tuning": "rest" }], "least": 0, "most": 2000 })
  );

  let mut wide = default_json();
  wide["talk"]["anchors"][0]["parts"][0]["line"]["width"] = json!(12);
  assert_eq!(refusal(&wide)["at"], json!([{ "anchor": 1 }, { "part": 1 }]));
}

#[test]
fn the_check_keeps_only_what_it_knows() {
  let mut document = default_json();
  document["faces"][FALLBACK]["eyes"]["left"][0]["label"] = json!("eye");
  document["faces"][FALLBACK]["eyes"]["left"][0]["inside"] = json!([]);
  document["faces"][FALLBACK]["extra"] = json!(1);
  document["colors"]["ink"] = json!("#000A00");
  let checked = check(&document).unwrap().to_json();
  assert_eq!(
    checked["faces"][FALLBACK]["eyes"]["left"][0],
    default_json()["faces"][FALLBACK]["eyes"]["left"][0]
  );
  assert_eq!(checked["faces"][FALLBACK].get("extra"), None);
  assert_eq!(checked["colors"]["ink"], json!("#000a00"));
}

#[test]
fn a_refusal_of_the_rest_names_the_idle_value_and_holds_sleep_after_the_blank_face() {
  let mut long = default_json();
  long["idle"]["blank"] = json!(100_000);
  assert_eq!(
    refusal(&long),
    json!({ "code": "idle", "at": [{ "idle": "blank" }], "least": 0, "most": 86400 })
  );
  let mut early = default_json();
  early["idle"]["asleep"] = json!(300);
  assert_eq!(
    refusal(&early),
    json!({ "code": "idle", "at": [{ "idle": "asleep" }], "least": 600, "most": 86400 })
  );
}
