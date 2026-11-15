//! Carrying a stored document of an older form into this one (docs/face.md, 「Preparing the document」),
//! proved on the default of form 1, on the documents form 1's check accepted and refused, and on the
//! documents of form 0 form 1 was carried from.

use bmo_face_core::check::check;
use bmo_face_core::default;
use bmo_face_core::migrate::migrate;
use bmo_face_core::names::FALLBACK;
use serde_json::{Value, json};

fn fixture(name: &str) -> Value {
  let text =
    std::fs::read_to_string(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap();
  serde_json::from_str(&text).unwrap()
}

#[test]
fn the_default_of_form_one_is_carried_whole_its_mirrored_eyes_one_side() {
  let old = fixture("form-1-default.json");
  let carried = migrate(&old);
  let checked = check(&carried).expect("the carried default passes").to_json();
  assert_eq!(checked["talk"], default::document().to_json()["talk"]);
  assert_eq!(checked["colors"]["ink"], old["colors"]["ink"]);
  assert_eq!(checked["colors"]["tongueLight"], default::document().to_json()["colors"]["tongueLight"]);
  for (name, face) in old["faces"].as_object().unwrap() {
    let eyes = &checked["faces"][name]["eyes"];
    let count = face["eyes"].as_array().unwrap().len();
    // Every face of the old default draws its right eye as its left reflected, but the one whose eyes
    // both look to the left.
    if name == "thinking" {
      assert_eq!(eyes["left"].as_array().unwrap().len() + eyes["right"].as_array().unwrap().len(), count);
    } else {
      assert_eq!(eyes["right"], Value::Null, "{name}");
      assert_eq!(eyes["left"].as_array().unwrap().len() * 2, count, "{name}");
    }
    assert_eq!(checked["faces"][name]["mouth"], face["mouth"], "{name}");
  }
  assert_eq!(checked["faces"][FALLBACK]["eyes"]["reach"], json!([2.2, 1.6]));
}

#[test]
fn a_right_side_that_is_no_reflection_is_kept_as_drawn() {
  let mut old = fixture("form-1-default.json");
  old["faces"][FALLBACK]["eyes"][1]["nodes"][0]["anchor"][1] = json!(26);
  old["savedAt"] = json!(5);
  let checked = check(&migrate(&old)).unwrap().to_json();
  let eyes = &checked["faces"][FALLBACK]["eyes"];
  assert_eq!(eyes["left"], json!([old["faces"][FALLBACK]["eyes"][0]]));
  assert_eq!(eyes["right"], json!([old["faces"][FALLBACK]["eyes"][1]]));
}

#[test]
fn what_form_one_accepted_is_carried_and_accepted_and_what_it_refused_stays_refused() {
  let cases = fixture("form-1-checks.json");
  for case in cases.as_array().unwrap() {
    let name = case["name"].as_str().unwrap();
    let carried = check(&migrate(&case["document"]));
    match case["verdict"].get("ok") {
      Some(_) => assert!(carried.is_ok(), "{name}: {carried:?} {}", migrate(&case["document"])),
      // A document form 1 refused is either still refused or, where it lacked what the default gives —
      // a colour, its faces, the fallback face — or held a part of form 0 or mouth stages, which give
      // way to the talking mouth set, carried and given it.
      None => {
        let mended = [
          "a paint's colour missing".to_owned(),
          format!("no {FALLBACK} face"),
          "a part of the old ellipse shape".to_owned(),
          "not a document".to_owned(),
          "too few mouth stages".to_owned(),
          "too many mouth stages".to_owned(),
          "a mouth stage that is no list".to_owned(),
        ];
        assert!(carried.is_err() || mended.iter().any(|one| one == name), "{name}");
      }
    }
  }
}

#[test]
fn a_document_of_form_zero_is_carried_as_form_one_carried_it() {
  let old_default = fixture("form-1-default.json");
  for case in fixture("form-0-upgrades.json").as_array().unwrap() {
    let name = case["name"].as_str().unwrap();
    let mut stored = case["stored"].clone();
    for face in case.get("storedDefaultFaces").and_then(Value::as_array).into_iter().flatten() {
      let face = face.as_str().unwrap();
      stored["faces"][face] = old_default["faces"][face].clone();
    }
    let carried = check(&migrate(&stored));
    let Some(prepared) = case["prepared"].get("ok") else {
      assert!(carried.is_err(), "{name}");
      continue;
    };
    let mut form_one = prepared.clone();
    for face in case["prepared"]["defaultFaces"].as_array().into_iter().flatten() {
      let face = face.as_str().unwrap();
      form_one["faces"][face] = old_default["faces"][face].clone();
    }
    let expected = check(&migrate(&form_one)).unwrap().to_json();
    let carried = carried.unwrap_or_else(|refusal| panic!("{name}: {refusal:?}")).to_json();
    for (face, drawn) in stored["faces"].as_object().unwrap() {
      if drawn.is_object() {
        assert_eq!(carried["faces"][face], expected["faces"][face], "{name}: {face}");
      }
    }
    // The colours it held are kept; those it lacked come from the default of this form.
    let defaults = default::document().to_json()["colors"].clone();
    for (key, colour) in carried["colors"].as_object().unwrap() {
      let held = stored["colors"].get(key).and_then(serde_json::Value::as_str);
      assert_eq!(
        colour.as_str(),
        held.map(str::to_ascii_lowercase).as_deref().or(defaults[key].as_str()),
        "{name}: {key}"
      );
    }
  }
}

#[test]
fn a_document_of_this_form_is_only_given_what_it_lacks() {
  let mut document = default::document().to_json();
  document["faces"].as_object_mut().unwrap().shift_remove("idle");
  document["faces"][FALLBACK]["marks"] = json!([]);
  let carried = migrate(&document);
  assert_eq!(carried["faces"]["idle"], default::document().to_json()["faces"]["idle"]);
  assert_eq!(check(&carried).unwrap().to_json(), default::document().to_json());
  assert_eq!(migrate(&json!("not a document")), json!("not a document"));
}
