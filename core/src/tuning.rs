//! How the sound moves the face (docs/face.md, 「The mouth while speaking」 and 「The talking set」): the
//! tunable values of the mapping from what is heard to the mouth and the eyes, which the document's
//! talking set holds and the editor adjusts. Each value is declared once here, with its key, its bounds
//! and its default, and the check, the document's writing and the editor's fields all read the
//! declaration.

use serde_json::{Map, Value, json};

use crate::hearing::BAND_EDGES;
use crate::json::number;

/// The tunable values.
#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
  /// The quietest level, in dBFS, that opens the mouth.
  pub floor: f64,
  /// How far below the sound's own loud level, in dB, the mouth starts to open.
  pub range: f64,
  /// How wide the mouth opens at the sound's loud level, as a share of the widest opening.
  pub open: f64,
  /// How much of the opening the first formant decides: an open vowel opens wider than a close one.
  pub jaw: f64,
  /// How strongly a rounded vowel rounds the mouth.
  pub round: f64,
  /// How strongly a hiss shows the teeth.
  pub teeth: f64,
  /// How strongly an open voiced sound shows the tongue.
  pub tongue: f64,
  /// How far, in screen units, the corners of a talking mouth move at a face's full lift.
  pub corners: f64,
  /// How fast the mouth opens and closes with the level, in milliseconds.
  pub attack: f64,
  pub release: f64,
  /// How fast the mouth's roundness, teeth and tongue follow, in milliseconds.
  pub glide: f64,
  /// How slowly, in milliseconds, the mouth goes back to its resting mouth in a pause.
  pub rest: f64,
  /// How strongly the eyes follow the level, widen on a sharp start, squint through a long loud
  /// stretch and shine with the hiss, each as a share of what the eyes allow.
  pub eye_pulse: f64,
  pub eye_onset: f64,
  pub eye_squint: f64,
  pub eye_shine: f64,
  /// How far a timed text's sound bends the mouth toward round or wide.
  pub cues: f64,
  /// Where the first formant is sought, where the second is, and where a hiss stands, in hertz.
  pub low_hz: [f64; 2],
  pub mid_hz: [f64; 2],
  pub high_hz: [f64; 2],
  /// The first formant at which the jaw is closed and at which it is fully open.
  pub jaw_hz: [f64; 2],
  /// The band above a rounded vowel's two formants, whose energy against the energy under it rounds the
  /// lips.
  pub round_hz: [f64; 2],
  /// The mouth and the eyes drawn on twos, as the animation draws them, rather than every frame.
  pub on_twos: bool,
}

/// One tunable number: its key in the document, its bounds and where it stands in a tuning.
pub struct Number {
  pub key: &'static str,
  pub least: f64,
  pub most: f64,
  pub field: fn(&mut Tuning) -> &mut f64,
}

/// One tunable span of frequencies, its two ends within the bands the spectrum is read in.
pub struct Span {
  pub key: &'static str,
  pub field: fn(&mut Tuning) -> &mut [f64; 2],
}

/// The key of the switch between every frame and on twos.
pub const ON_TWOS: &str = "onTwos";

pub const NUMBERS: [Number; 17] = [
  Number {
    key: "floor",
    least: -90.0,
    most: -20.0,
    field: |tuning| &mut tuning.floor,
  },
  Number {
    key: "range",
    least: 6.0,
    most: 60.0,
    field: |tuning| &mut tuning.range,
  },
  Number {
    key: "open",
    least: 0.0,
    most: 2.0,
    field: |tuning| &mut tuning.open,
  },
  Number {
    key: "jaw",
    least: 0.0,
    most: 1.0,
    field: |tuning| &mut tuning.jaw,
  },
  Number {
    key: "round",
    least: 0.0,
    most: 2.0,
    field: |tuning| &mut tuning.round,
  },
  Number {
    key: "teeth",
    least: 0.0,
    most: 3.0,
    field: |tuning| &mut tuning.teeth,
  },
  Number {
    key: "tongue",
    least: 0.0,
    most: 2.0,
    field: |tuning| &mut tuning.tongue,
  },
  Number {
    key: "corners",
    least: 0.0,
    most: 4.0,
    field: |tuning| &mut tuning.corners,
  },
  Number {
    key: "attack",
    least: 0.0,
    most: 500.0,
    field: |tuning| &mut tuning.attack,
  },
  Number {
    key: "release",
    least: 0.0,
    most: 1_000.0,
    field: |tuning| &mut tuning.release,
  },
  Number {
    key: "glide",
    least: 0.0,
    most: 1_000.0,
    field: |tuning| &mut tuning.glide,
  },
  Number {
    key: "rest",
    least: 0.0,
    most: 2_000.0,
    field: |tuning| &mut tuning.rest,
  },
  Number {
    key: "eyePulse",
    least: 0.0,
    most: 1.0,
    field: |tuning| &mut tuning.eye_pulse,
  },
  Number {
    key: "eyeOnset",
    least: 0.0,
    most: 1.0,
    field: |tuning| &mut tuning.eye_onset,
  },
  Number {
    key: "eyeSquint",
    least: 0.0,
    most: 1.0,
    field: |tuning| &mut tuning.eye_squint,
  },
  Number {
    key: "eyeShine",
    least: 0.0,
    most: 1.0,
    field: |tuning| &mut tuning.eye_shine,
  },
  Number {
    key: "cues",
    least: 0.0,
    most: 1.0,
    field: |tuning| &mut tuning.cues,
  },
];

pub const SPANS: [Span; 5] = [
  Span {
    key: "lowHz",
    field: |tuning| &mut tuning.low_hz,
  },
  Span {
    key: "midHz",
    field: |tuning| &mut tuning.mid_hz,
  },
  Span {
    key: "highHz",
    field: |tuning| &mut tuning.high_hz,
  },
  Span {
    key: "jawHz",
    field: |tuning| &mut tuning.jaw_hz,
  },
  Span {
    key: "roundHz",
    field: |tuning| &mut tuning.round_hz,
  },
];

/// The lowest and highest frequency a span may name: the spectrum's own ends.
pub const SPAN_BOUNDS: (f64, f64) = (BAND_EDGES[0], BAND_EDGES[BAND_EDGES.len() - 1]);

impl Default for Tuning {
  fn default() -> Tuning {
    Tuning {
      floor: -55.0,
      range: 30.0,
      open: 1.0,
      jaw: 0.35,
      round: 1.0,
      teeth: 1.0,
      tongue: 1.0,
      corners: 2.0,
      attack: 25.0,
      release: 60.0,
      glide: 60.0,
      rest: 300.0,
      eye_pulse: 0.4,
      eye_onset: 0.6,
      eye_squint: 0.7,
      eye_shine: 1.0,
      cues: 0.5,
      low_hz: [250.0, 1_000.0],
      mid_hz: [900.0, 2_800.0],
      high_hz: [3_500.0, 10_240.0],
      jaw_hz: [350.0, 750.0],
      round_hz: [1_650.0, 3_000.0],
      on_twos: false,
    }
  }
}

/// A tuning value that is not what its declaration allows: its key, and the bounds it was held to.
#[derive(Clone, Debug, PartialEq)]
pub struct Wrong {
  pub key: &'static str,
  pub least: f64,
  pub most: f64,
}

impl Tuning {
  pub fn to_json(&self) -> Value {
    let mut tuning = self.clone();
    let mut written = Map::new();
    for one in &NUMBERS {
      written.insert(one.key.into(), number(*(one.field)(&mut tuning)));
    }
    for span in &SPANS {
      let [from, to] = *(span.field)(&mut tuning);
      written.insert(span.key.into(), json!([number(from), number(to)]));
    }
    written.insert(ON_TWOS.into(), Value::Bool(self.on_twos));
    Value::Object(written)
  }

  /// The tuning a document holds, every value within its bounds, or the first that is not.
  pub fn from_json(value: &Value) -> Result<Tuning, Wrong> {
    let mut tuning = Tuning::default();
    for one in &NUMBERS {
      let read = value
        .get(one.key)
        .and_then(Value::as_f64)
        .filter(|read| (one.least..=one.most).contains(read));
      *(one.field)(&mut tuning) = read.ok_or(Wrong {
        key: one.key,
        least: one.least,
        most: one.most,
      })?;
    }
    let (least, most) = SPAN_BOUNDS;
    for span in &SPANS {
      let read = match value.get(span.key).and_then(Value::as_array).map(Vec::as_slice) {
        Some([from, to]) => from.as_f64().zip(to.as_f64()),
        _ => None,
      };
      *(span.field)(&mut tuning) = read
        .filter(|(from, to)| least <= *from && from < to && *to <= most)
        .map(|(from, to)| [from, to])
        .ok_or(Wrong {
          key: span.key,
          least,
          most,
        })?;
    }
    tuning.on_twos = value.get(ON_TWOS).and_then(Value::as_bool).ok_or(Wrong {
      key: ON_TWOS,
      least: 0.0,
      most: 1.0,
    })?;
    Ok(tuning)
  }

  /// The tuning's declaration as a caller reads it: every number with its bounds and default, every
  /// span with the frequencies it may name, and the switch.
  pub fn declaration() -> Value {
    let mut defaults = Tuning::default();
    let numbers: Vec<Value> = NUMBERS
      .iter()
      .map(|one| {
        json!({ "key": one.key, "least": number(one.least), "most": number(one.most), "default": number(*(one.field)(&mut defaults)) })
      })
      .collect();
    let spans: Vec<Value> = SPANS
      .iter()
      .map(|span| {
        let [from, to] = *(span.field)(&mut defaults);
        json!({ "key": span.key, "least": number(SPAN_BOUNDS.0), "most": number(SPAN_BOUNDS.1), "default": [number(from), number(to)] })
      })
      .collect();
    json!({ "numbers": numbers, "spans": spans, "switch": ON_TWOS })
  }
}
