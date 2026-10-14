// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The catalog of original F07 synthetic fixtures.
//!
//! Every dimension, coordinate, angle and density below was chosen for this catalog by BeamCraft
//! contributors. None comes from a vendor catalogue, a national material table, another
//! product's samples or another task's observation notes. Densities are round synthetic test
//! values and do not describe any real material.
//!
//! The catalog only produces input bytes. It deliberately computes no volumes, lengths, masses
//! or bounds: those are hand-derived in `docs/fixtures/expectations.md` and stored separately.

use crate::fixture::{ErrorCode, FORMAT, MAX_OBJECTS};
use crate::json::{Json, to_pretty_string};

/// How a fixture relates to the `beamcraft.fixture/1` support scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Support {
    /// In scope; analytical expectations are given and must hold within the stated tolerances.
    Supported,
    /// Accepted by the format with analytical expectations, but numerical behaviour of an
    /// implementation at this scale is not yet verified by any task.
    Unverified,
    /// Outside the version 1 scope; must be rejected with a diagnostic, never approximated.
    Unsupported,
    /// Malformed, out-of-limit or degenerate input that must be rejected.
    Invalid,
}

impl Support {
    /// The stable string form used in the manifest and expectations.
    pub fn as_str(self) -> &'static str {
        match self {
            Support::Supported => "supported",
            Support::Unverified => "unverified",
            Support::Unsupported => "unsupported",
            Support::Invalid => "invalid",
        }
    }
}

/// The outcome the generator intends for a fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expected {
    /// The description is valid and must be accepted.
    Accept,
    /// The description must be rejected with this code.
    Reject(ErrorCode),
}

/// One generated fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    /// Stable fixture ID, `f07.<group>.<name>`.
    pub id: &'static str,
    /// Output path relative to the fixture directory, with `/` separators.
    pub path: &'static str,
    /// Support status.
    pub support: Support,
    /// Intended outcome.
    pub expected: Expected,
    /// One-line description.
    pub summary: &'static str,
    /// The exact file content.
    pub bytes: Vec<u8>,
}

fn n(v: f64) -> Json {
    Json::Number(v)
}

fn s(v: &str) -> Json {
    Json::String(v.to_owned())
}

fn v3(v: [f64; 3]) -> Json {
    Json::Array(v.iter().map(|c| n(*c)).collect())
}

fn obj(members: Vec<(&str, Json)>) -> Json {
    Json::Object(
        members
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
    )
}

fn units() -> Json {
    obj(vec![
        ("length", s("mm")),
        ("angle", s("deg")),
        ("density", s("kg/m3")),
    ])
}

fn document(id: &str, description: &str, objects: Vec<Json>) -> Json {
    obj(vec![
        ("format", s(FORMAT)),
        ("id", s(id)),
        ("description", s(description)),
        ("units", units()),
        ("objects", Json::Array(objects)),
    ])
}

/// A rectangular plate: local box [0, size.x] × [0, size.y] × [0, size.z], rotated about
/// `axis` through the local origin by `angle` degrees, then translated to `origin`.
struct Plate {
    key: &'static str,
    size: [f64; 3],
    origin: [f64; 3],
    axis: [f64; 3],
    angle: f64,
    density: Option<f64>,
}

impl Plate {
    fn json(&self) -> Json {
        let mut m = vec![
            ("key", s(self.key)),
            ("kind", s("plate")),
            ("size", v3(self.size)),
            (
                "placement",
                obj(vec![
                    ("origin", v3(self.origin)),
                    ("axis", v3(self.axis)),
                    ("angle", n(self.angle)),
                ]),
            ),
        ];
        if let Some(d) = self.density {
            m.push(("density", n(d)));
        }
        obj(m)
    }
}

/// A straight member with a rectangular profile centred on the start-end axis.
struct Member {
    key: &'static str,
    start: [f64; 3],
    end: [f64; 3],
    up: [f64; 3],
    width: f64,
    height: f64,
    density: Option<f64>,
}

impl Member {
    fn json(&self) -> Json {
        let mut m = vec![
            ("key", s(self.key)),
            ("kind", s("member")),
            ("start", v3(self.start)),
            ("end", v3(self.end)),
            ("up", v3(self.up)),
            (
                "profile",
                obj(vec![
                    ("shape", s("rect")),
                    ("width", n(self.width)),
                    ("height", n(self.height)),
                ]),
            ),
        ];
        if let Some(d) = self.density {
            m.push(("density", n(d)));
        }
        obj(m)
    }
}

/// Synthetic test density, kg/m³. A round number chosen for exact arithmetic, not a material.
const TEST_DENSITY: f64 = 6000.0;

const Z: [f64; 3] = [0.0, 0.0, 1.0];

fn plate_a(key: &'static str, origin: [f64; 3]) -> Plate {
    Plate {
        key,
        size: [640.0, 410.0, 15.0],
        origin,
        axis: Z,
        angle: 0.0,
        density: Some(TEST_DENSITY),
    }
}

fn member_a() -> Member {
    Member {
        key: "member-1",
        start: [0.0, 0.0, 0.0],
        end: [4200.0, 0.0, 0.0],
        up: Z,
        width: 120.0,
        height: 260.0,
        density: Some(TEST_DENSITY),
    }
}

fn render(doc: &Json) -> Vec<u8> {
    to_pretty_string(doc).into_bytes()
}

fn text(doc: &Json) -> String {
    to_pretty_string(doc)
}

/// Replaces exactly one occurrence of `from`, so a changed template cannot silently produce a
/// different negative case.
fn replace_once(text: &str, from: &str, to: &str) -> Result<String, String> {
    if text.matches(from).count() != 1 {
        return Err(format!("template marker {from:?} must occur exactly once"));
    }
    Ok(text.replacen(from, to, 1))
}

fn single_plate(id: &str, description: &str, plate: &Plate) -> Json {
    document(id, description, vec![plate.json()])
}

fn single_member(id: &str, description: &str, member: &Member) -> Json {
    document(id, description, vec![member.json()])
}

/// Builds every case in its stable catalog order.
///
/// Fails only if an internal text template no longer matches, which the tests rule out.
pub fn cases() -> Result<Vec<Case>, String> {
    use ErrorCode as E;
    use Expected::{Accept, Reject};
    use Support::{Invalid, Supported, Unsupported, Unverified};

    let mut out = Vec::new();
    let mut push = |id, path, support, expected, summary, bytes| {
        out.push(Case {
            id,
            path,
            support,
            expected,
            summary,
            bytes,
        });
    };

    // ---- Valid, supported bodies -------------------------------------------------------------

    let id = "f07.plate.axis-aligned";
    push(
        id,
        "valid/plate-axis-aligned.json",
        Supported,
        Accept,
        "640 x 410 x 15 mm plate at the origin, no rotation",
        render(&single_plate(
            id,
            "Rectangular plate with one corner at the origin and edges along the axes.",
            &plate_a("plate-1", [0.0, 0.0, 0.0]),
        )),
    );

    let id = "f07.plate.translated";
    push(
        id,
        "valid/plate-translated.json",
        Supported,
        Accept,
        "the axis-aligned plate translated by (1250, -340, 2200) mm",
        render(&single_plate(
            id,
            "The same plate as f07.plate.axis-aligned after a pure translation.",
            &plate_a("plate-1", [1250.0, -340.0, 2200.0]),
        )),
    );

    let id = "f07.plate.rotated-z-quarter";
    push(
        id,
        "valid/plate-rotated-z-quarter.json",
        Supported,
        Accept,
        "750 x 300 x 10 mm plate, +90 deg about a non-unit Z axis, origin (500, 0, 0)",
        render(&single_plate(
            id,
            "Quarter turn about +Z; the axis is given as (0, 0, 5) and must be normalised.",
            &Plate {
                key: "plate-1",
                size: [750.0, 300.0, 10.0],
                origin: [500.0, 0.0, 0.0],
                axis: [0.0, 0.0, 5.0],
                angle: 90.0,
                density: Some(TEST_DENSITY),
            },
        )),
    );

    let id = "f07.plate.rotated-z-30";
    push(
        id,
        "valid/plate-rotated-z-30.json",
        Supported,
        Accept,
        "900 x 260 x 22 mm plate, +30 deg about Z, density omitted (unknown)",
        render(&single_plate(
            id,
            "Oblique rotation with closed-form bounds; mass is unknown because density is absent.",
            &Plate {
                key: "plate-1",
                size: [900.0, 260.0, 22.0],
                origin: [0.0, 0.0, 0.0],
                axis: Z,
                angle: 30.0,
                density: None,
            },
        )),
    );

    let id = "f07.plate.rotated-x-quarter-translated";
    push(
        id,
        "valid/plate-rotated-x-quarter-translated.json",
        Supported,
        Accept,
        "480 x 350 x 12 mm plate, +90 deg about X, then translated to (-800, 2500, 300)",
        render(&single_plate(
            id,
            "Rigid motion: quarter turn about +X followed by a translation.",
            &Plate {
                key: "plate-1",
                size: [480.0, 350.0, 12.0],
                origin: [-800.0, 2500.0, 300.0],
                axis: [1.0, 0.0, 0.0],
                angle: 90.0,
                density: Some(TEST_DENSITY),
            },
        )),
    );

    let id = "f07.member.along-x";
    push(
        id,
        "valid/member-along-x.json",
        Supported,
        Accept,
        "4200 mm member along +X, rectangular 120 x 260 mm profile",
        render(&single_member(
            id,
            "Straight member on the X axis; profile height follows +Z.",
            &member_a(),
        )),
    );

    let id = "f07.member.vertical";
    push(
        id,
        "valid/member-vertical.json",
        Supported,
        Accept,
        "3150 mm vertical member at (300, 500), 140 x 220 mm profile, up = +Y",
        render(&single_member(
            id,
            "Vertical member; up must not be parallel to the axis, so +Y orients the profile.",
            &Member {
                key: "member-1",
                start: [300.0, 500.0, 0.0],
                end: [300.0, 500.0, 3150.0],
                up: [0.0, 1.0, 0.0],
                width: 140.0,
                height: 220.0,
                density: Some(TEST_DENSITY),
            },
        )),
    );

    let id = "f07.member.diagonal-5-12-13";
    push(
        id,
        "valid/member-diagonal-5-12-13.json",
        Supported,
        Accept,
        "3900 mm member along (5, 12, 0)/13 in plane z = 450, 100 x 300 mm profile",
        render(&single_member(
            id,
            "Oblique member whose direction cosines are exact fractions of 13.",
            &Member {
                key: "member-1",
                start: [1000.0, 200.0, 450.0],
                end: [2500.0, 3800.0, 450.0],
                up: Z,
                width: 100.0,
                height: 300.0,
                density: Some(TEST_DENSITY),
            },
        )),
    );

    let id = "f07.copy.two-plates";
    let original = Plate {
        key: "plate-original",
        size: [520.0, 330.0, 18.0],
        origin: [0.0, 0.0, 0.0],
        axis: Z,
        angle: 0.0,
        density: Some(TEST_DENSITY),
    };
    let copy = Plate {
        key: "plate-copy",
        origin: [700.0, 0.0, 0.0],
        ..original
    };
    push(
        id,
        "valid/copy-two-plates.json",
        Supported,
        Accept,
        "two complete, independent 520 x 330 x 18 mm plates 180 mm apart along X",
        render(&document(
            id,
            "Independent-copy input: both objects carry full parameters; neither references the other.",
            vec![original.json(), copy.json()],
        )),
    );

    // ---- Valid, unverified -------------------------------------------------------------------

    let id = "f07.plate.far-from-origin";
    push(
        id,
        "valid/plate-far-from-origin.json",
        Unverified,
        Accept,
        "the axis-aligned plate near the coordinate limit, origin (9990000, -9990000, 9990000)",
        render(&single_plate(
            id,
            "Large-coordinate case; analytical values are exact, implementation tolerance unverified.",
            &plate_a("plate-1", [9_990_000.0, -9_990_000.0, 9_990_000.0]),
        )),
    );

    // ---- Unsupported -------------------------------------------------------------------------

    let id = "f07.unsupported.profile-circle";
    let circle = replace_once(
        &text(&single_member(
            id,
            "Circular profiles are outside version 1.",
            &member_a(),
        )),
        "\"shape\": \"rect\",\n        \"width\": 120,\n        \"height\": 260",
        "\"shape\": \"circle\",\n        \"diameter\": 168",
    )?;
    push(
        id,
        "invalid/unsupported-profile-circle.json",
        Unsupported,
        Reject(E::UnsupportedProfile),
        "member with a circular profile",
        circle.into_bytes(),
    );

    let id = "f07.unsupported.length-unit-inch";
    let inch = replace_once(
        &text(&single_plate(
            id,
            "Only millimetres are supported at the fixture boundary.",
            &plate_a("plate-1", [0.0, 0.0, 0.0]),
        )),
        "\"length\": \"mm\"",
        "\"length\": \"in\"",
    )?;
    push(
        id,
        "invalid/unsupported-length-unit-inch.json",
        Unsupported,
        Reject(E::UnsupportedUnit),
        "plate whose length unit is inches",
        inch.into_bytes(),
    );

    // ---- Invalid: dimensions and degenerate geometry -----------------------------------------

    let id = "f07.invalid.plate-zero-thickness";
    push(
        id,
        "invalid/plate-zero-thickness.json",
        Invalid,
        Reject(E::NonPositiveDimension),
        "plate with zero thickness",
        render(&single_plate(
            id,
            "Zero is not a valid plate dimension.",
            &Plate {
                size: [640.0, 410.0, 0.0],
                ..plate_a("plate-1", [0.0, 0.0, 0.0])
            },
        )),
    );

    let id = "f07.invalid.member-negative-width";
    push(
        id,
        "invalid/member-negative-width.json",
        Invalid,
        Reject(E::NonPositiveDimension),
        "member with a negative profile width",
        render(&single_member(
            id,
            "Profile dimensions must be positive.",
            &Member {
                width: -120.0,
                ..member_a()
            },
        )),
    );

    let id = "f07.invalid.plate-below-minimum-thickness";
    push(
        id,
        "invalid/plate-below-minimum-thickness.json",
        Invalid,
        Reject(E::DegenerateGeometry),
        "plate 0.0004 mm thick, below the 0.001 mm minimum",
        render(&single_plate(
            id,
            "A positive but degenerate dimension.",
            &Plate {
                size: [640.0, 410.0, 0.0004],
                ..plate_a("plate-1", [0.0, 0.0, 0.0])
            },
        )),
    );

    let id = "f07.invalid.member-zero-length";
    push(
        id,
        "invalid/member-zero-length.json",
        Invalid,
        Reject(E::DegenerateGeometry),
        "member whose start and end coincide",
        render(&single_member(
            id,
            "A member axis must have positive length.",
            &Member {
                end: [0.0, 0.0, 0.0],
                ..member_a()
            },
        )),
    );

    let id = "f07.invalid.member-up-parallel";
    push(
        id,
        "invalid/member-up-parallel.json",
        Invalid,
        Reject(E::DegenerateGeometry),
        "member whose up vector is anti-parallel to its axis",
        render(&single_member(
            id,
            "The profile orientation is undefined when up is parallel to the axis.",
            &Member {
                up: [-2.0, 0.0, 0.0],
                ..member_a()
            },
        )),
    );

    let id = "f07.invalid.rotation-zero-axis";
    push(
        id,
        "invalid/rotation-zero-axis.json",
        Invalid,
        Reject(E::DegenerateGeometry),
        "plate rotated about a zero-length axis",
        render(&single_plate(
            id,
            "A rotation axis must have non-zero length.",
            &Plate {
                axis: [0.0, 0.0, 0.0],
                angle: 45.0,
                ..plate_a("plate-1", [0.0, 0.0, 0.0])
            },
        )),
    );

    let id = "f07.invalid.negative-density";
    push(
        id,
        "invalid/negative-density.json",
        Invalid,
        Reject(E::NonPositiveDensity),
        "plate with a negative density",
        render(&single_plate(
            id,
            "Density, when present, must be positive.",
            &Plate {
                density: Some(-6000.0),
                ..plate_a("plate-1", [0.0, 0.0, 0.0])
            },
        )),
    );

    // ---- Invalid: numbers ---------------------------------------------------------------------

    let plate_text = |id: &str, description: &str| {
        text(&single_plate(
            id,
            description,
            &plate_a("plate-1", [0.0, 0.0, 0.0]),
        ))
    };

    let id = "f07.invalid.number-overflow";
    let overflow = replace_once(
        &plate_text(id, "The literal 1e400 overflows a 64-bit float."),
        "\"density\": 6000",
        "\"density\": 1e400",
    )?;
    push(
        id,
        "invalid/number-overflow.json",
        Invalid,
        Reject(E::NonFiniteNumber),
        "density literal 1e400, which overflows f64",
        overflow.into_bytes(),
    );

    let id = "f07.invalid.number-nan";
    let nan = replace_once(
        &plate_text(id, "NaN is not a JSON number."),
        "[640, 410, 15]",
        "[640, NaN, 15]",
    )?;
    push(
        id,
        "invalid/number-nan.json",
        Invalid,
        Reject(E::NonFiniteNumber),
        "NaN token in the plate size",
        nan.into_bytes(),
    );

    // ---- Invalid: malformed data -------------------------------------------------------------

    let id = "f07.invalid.json-truncated";
    let full = plate_text(id, "The document ends in the middle of an object.");
    let cut = full
        .find("\"placement\"")
        .ok_or("truncation marker missing")?;
    let truncated = full.get(..cut).ok_or("truncation point out of range")?;
    push(
        id,
        "invalid/json-truncated.json",
        Invalid,
        Reject(E::MalformedJson),
        "document cut off before the placement member",
        truncated.as_bytes().to_vec(),
    );

    let id = "f07.invalid.json-duplicate-member";
    let duplicate = replace_once(
        &plate_text(id, "JSON member names must be unique."),
        "\"kind\": \"plate\",",
        "\"kind\": \"plate\",\n      \"kind\": \"plate\",",
    )?;
    push(
        id,
        "invalid/json-duplicate-member.json",
        Invalid,
        Reject(E::MalformedJson),
        "the member name \"kind\" appears twice in one object",
        duplicate.into_bytes(),
    );

    let id = "f07.invalid.utf8";
    let utf8 = plate_text(id, "The byte pair C3 28 is not valid UTF-8: [@@].");
    let marker = "@@";
    let at = utf8.find(marker).ok_or("UTF-8 marker missing")?;
    let mut utf8_bytes = utf8.as_bytes().to_vec();
    utf8_bytes.splice(at..at.saturating_add(marker.len()), [0xC3, 0x28]);
    push(
        id,
        "invalid/utf8.json",
        Invalid,
        Reject(E::MalformedJson),
        "invalid UTF-8 byte sequence inside the description string",
        utf8_bytes,
    );

    let id = "f07.invalid.wrong-type";
    let wrong_type = replace_once(
        &plate_text(id, "The size must be an array of numbers."),
        "[640, 410, 15]",
        "\"640 x 410 x 15\"",
    )?;
    push(
        id,
        "invalid/wrong-type.json",
        Invalid,
        Reject(E::InvalidField),
        "plate size given as a string",
        wrong_type.into_bytes(),
    );

    let id = "f07.invalid.unknown-field";
    let unknown = replace_once(
        &plate_text(id, "Unknown fields are rejected rather than ignored."),
        "\"kind\": \"plate\",",
        "\"kind\": \"plate\",\n      \"colour\": \"blue\",",
    )?;
    push(
        id,
        "invalid/unknown-field.json",
        Invalid,
        Reject(E::InvalidField),
        "plate with an unknown field",
        unknown.into_bytes(),
    );

    let id = "f07.invalid.unknown-format";
    let format = replace_once(
        &plate_text(id, "A future format version this reader does not know."),
        FORMAT,
        "beamcraft.fixture/99",
    )?;
    push(
        id,
        "invalid/unknown-format.json",
        Invalid,
        Reject(E::UnsupportedFormat),
        "format beamcraft.fixture/99",
        format.into_bytes(),
    );

    let id = "f07.invalid.empty-objects";
    push(
        id,
        "invalid/empty-objects.json",
        Invalid,
        Reject(E::InvalidField),
        "fixture without objects",
        render(&document(
            id,
            "A fixture must describe at least one object.",
            vec![],
        )),
    );

    let id = "f07.invalid.duplicate-object-key";
    let twin = plate_a("plate-1", [700.0, 0.0, 0.0]);
    push(
        id,
        "invalid/duplicate-object-key.json",
        Invalid,
        Reject(E::DuplicateObjectKey),
        "two plates that share the key plate-1",
        render(&document(
            id,
            "Object keys must be unique within a fixture.",
            vec![plate_a("plate-1", [0.0, 0.0, 0.0]).json(), twin.json()],
        )),
    );

    // ---- Invalid: exceeded limits ------------------------------------------------------------

    let id = "f07.invalid.limit-objects";
    const KEYS: [&str; MAX_OBJECTS + 1] = [
        "plate-01", "plate-02", "plate-03", "plate-04", "plate-05", "plate-06", "plate-07",
        "plate-08", "plate-09", "plate-10", "plate-11", "plate-12", "plate-13", "plate-14",
        "plate-15", "plate-16", "plate-17",
    ];
    let mut many = Vec::with_capacity(KEYS.len());
    let mut x = 0.0;
    for key in KEYS {
        many.push(plate_a(key, [x, 0.0, 0.0]).json());
        x += 700.0;
    }
    push(
        id,
        "invalid/limit-objects.json",
        Invalid,
        Reject(E::LimitExceeded),
        "17 valid plates, one more than the 16-object limit",
        render(&document(
            id,
            "Each object is valid; only the object count exceeds the limit.",
            many,
        )),
    );

    let id = "f07.invalid.limit-depth";
    let mut nested = Json::Array(Vec::new());
    for _ in 0..24 {
        nested = Json::Array(vec![nested]);
    }
    let mut deep = document(id, "Arrays nested 27 levels deep; the limit is 16.", vec![]);
    if let Json::Object(members) = &mut deep {
        for (name, value) in members.iter_mut() {
            if name == "objects" {
                *value = Json::Array(vec![nested.clone()]);
            }
        }
    }
    push(
        id,
        "invalid/limit-depth.json",
        Invalid,
        Reject(E::LimitExceeded),
        "nesting deeper than the 16-level parser limit",
        render(&deep),
    );

    let id = "f07.invalid.limit-coordinate";
    push(
        id,
        "invalid/limit-coordinate.json",
        Invalid,
        Reject(E::LimitExceeded),
        "plate origin x = 20000000 mm, beyond the 10 km coordinate limit",
        render(&single_plate(
            id,
            "Coordinates are limited to +/-10 000 000 mm.",
            &plate_a("plate-1", [20_000_000.0, 0.0, 0.0]),
        )),
    );

    let id = "f07.invalid.limit-dimension";
    push(
        id,
        "invalid/limit-dimension.json",
        Invalid,
        Reject(E::LimitExceeded),
        "plate 250000 mm long, beyond the 100 m dimension limit",
        render(&single_plate(
            id,
            "Plate and profile dimensions are limited to 100 000 mm.",
            &Plate {
                size: [250_000.0, 410.0, 15.0],
                ..plate_a("plate-1", [0.0, 0.0, 0.0])
            },
        )),
    );

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::check;

    #[test]
    fn ids_and_paths_are_unique_and_safe() {
        let cases = cases().unwrap();
        for (i, a) in cases.iter().enumerate() {
            for b in &cases[i + 1..] {
                assert_ne!(a.id, b.id);
                assert_ne!(a.path, b.path);
            }
            assert!(a.id.starts_with("f07."), "{}", a.id);
            assert!(
                a.path
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"-./".contains(&c)),
                "{}",
                a.path
            );
            assert!(!a.path.contains("..") && !a.path.starts_with('/'));
            assert!(a.path.starts_with("valid/") || a.path.starts_with("invalid/"));
            assert_eq!(a.path.starts_with("valid/"), a.expected == Expected::Accept);
        }
    }

    #[test]
    fn every_case_has_its_intended_outcome() {
        for case in cases().unwrap() {
            match (check(&case.bytes), case.expected) {
                (Ok(accepted), Expected::Accept) => assert_eq!(accepted.id, case.id),
                (Err(r), Expected::Reject(code)) => assert_eq!(r.code, code, "{}: {r}", case.id),
                (result, expected) => panic!("{}: {result:?} vs {expected:?}", case.id),
            }
            let accepts = matches!(case.support, Support::Supported | Support::Unverified);
            assert_eq!(accepts, case.expected == Expected::Accept, "{}", case.id);
        }
    }

    #[test]
    fn catalog_is_deterministic() {
        assert_eq!(cases().unwrap(), cases().unwrap());
    }
}
