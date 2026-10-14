// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Reader and validator for the `beamcraft.fixture/1` test-description format.
//!
//! The format describes original synthetic test bodies: rectangular plates (prisms) placed by a
//! rigid motion, and straight members with a rectangular profile. It is a test-input format, not
//! the BeamCraft model contract. See `docs/fixtures/README.md` for the full definition.
//!
//! Validation reports the first violation in a fixed order and never computes expected
//! quantities; expected results live in a separate, hand-derived expectations file.

use crate::json::{self, Json, ParseError, ParseLimits};

/// Format identifier of version 1 fixture descriptions.
pub const FORMAT: &str = "beamcraft.fixture/1";
/// Maximum size of one fixture description, in bytes.
pub const MAX_INPUT_BYTES: usize = 64 * 1024;
/// Maximum JSON nesting depth of a fixture description.
pub const MAX_DEPTH: usize = 16;
/// Maximum number of objects in one fixture.
pub const MAX_OBJECTS: usize = 16;
/// Maximum absolute value of a coordinate or direction component, in millimetres (10 km).
pub const MAX_ABS_COORDINATE_MM: f64 = 1.0e7;
/// Maximum plate size or profile dimension, in millimetres (100 m).
pub const MAX_DIMENSION_MM: f64 = 1.0e5;
/// Smallest accepted plate size, profile dimension or member axis length, in millimetres.
pub const MIN_DIMENSION_MM: f64 = 1.0e-3;
/// Maximum absolute rotation angle, in degrees.
pub const MAX_ABS_ANGLE_DEG: f64 = 360.0;
/// Maximum density, in kg/m³.
pub const MAX_DENSITY_KG_M3: f64 = 1.0e6;
/// Maximum length of any string, in bytes.
pub const MAX_TEXT_BYTES: usize = 256;
/// Maximum length of an identifier or object key, in bytes.
pub const MAX_IDENTIFIER_BYTES: usize = 64;

/// Below this length, a rotation axis or member `up` vector counts as zero.
const MIN_DIRECTION_LENGTH: f64 = 1.0e-9;
/// Largest sine of the angle between `up` and a member axis still treated as parallel.
const PARALLEL_SINE: f64 = 1.0e-9;

/// Parser limits for fixture descriptions.
pub const PARSE_LIMITS: ParseLimits = ParseLimits {
    max_bytes: MAX_INPUT_BYTES,
    max_depth: MAX_DEPTH,
    max_values: 4096,
    max_string_bytes: MAX_TEXT_BYTES,
    max_number_chars: 32,
    max_members: 32,
};

/// Stable rejection codes. Their string forms are part of the fixture contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Not valid JSON: syntax, invalid UTF-8, trailing data or duplicate member names.
    MalformedJson,
    /// `NaN`/`Infinity` tokens or a number that overflows `f64`.
    NonFiniteNumber,
    /// A size, depth, count or magnitude limit was exceeded.
    LimitExceeded,
    /// The `format` member is not `beamcraft.fixture/1`.
    UnsupportedFormat,
    /// A unit other than mm, deg or kg/m3.
    UnsupportedUnit,
    /// A profile shape other than `rect`.
    UnsupportedProfile,
    /// A missing, unknown or wrongly typed field, or an empty object list.
    InvalidField,
    /// Two objects share the same key.
    DuplicateObjectKey,
    /// A size or profile dimension is zero or negative.
    NonPositiveDimension,
    /// A density is zero or negative.
    NonPositiveDensity,
    /// Positive but degenerate geometry: tiny dimensions, zero-length axes or parallel vectors.
    DegenerateGeometry,
}

impl ErrorCode {
    /// Every code, in documentation order.
    pub const ALL: [ErrorCode; 11] = [
        ErrorCode::MalformedJson,
        ErrorCode::NonFiniteNumber,
        ErrorCode::LimitExceeded,
        ErrorCode::UnsupportedFormat,
        ErrorCode::UnsupportedUnit,
        ErrorCode::UnsupportedProfile,
        ErrorCode::InvalidField,
        ErrorCode::DuplicateObjectKey,
        ErrorCode::NonPositiveDimension,
        ErrorCode::NonPositiveDensity,
        ErrorCode::DegenerateGeometry,
    ];

    /// The stable string form, e.g. `"non_positive_dimension"`.
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::MalformedJson => "malformed_json",
            ErrorCode::NonFiniteNumber => "non_finite_number",
            ErrorCode::LimitExceeded => "limit_exceeded",
            ErrorCode::UnsupportedFormat => "unsupported_format",
            ErrorCode::UnsupportedUnit => "unsupported_unit",
            ErrorCode::UnsupportedProfile => "unsupported_profile",
            ErrorCode::InvalidField => "invalid_field",
            ErrorCode::DuplicateObjectKey => "duplicate_object_key",
            ErrorCode::NonPositiveDimension => "non_positive_dimension",
            ErrorCode::NonPositiveDensity => "non_positive_density",
            ErrorCode::DegenerateGeometry => "degenerate_geometry",
        }
    }

    /// Parses the string form produced by [`ErrorCode::as_str`].
    pub fn from_name(name: &str) -> Option<ErrorCode> {
        ErrorCode::ALL.into_iter().find(|c| c.as_str() == name)
    }
}

/// Why a fixture description was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejection {
    /// Stable code.
    pub code: ErrorCode,
    /// Human-readable detail, including the JSON path where possible.
    pub detail: String,
}

impl std::fmt::Display for Rejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.detail)
    }
}

/// Summary of an accepted fixture description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accepted {
    /// The fixture ID.
    pub id: String,
    /// Number of plates.
    pub plates: usize,
    /// Number of members.
    pub members: usize,
}

fn reject(code: ErrorCode, detail: impl Into<String>) -> Rejection {
    Rejection {
        code,
        detail: detail.into(),
    }
}

fn invalid(detail: impl Into<String>) -> Rejection {
    reject(ErrorCode::InvalidField, detail)
}

/// Validates one fixture description.
///
/// Checks run in this order and the first violation is reported: JSON syntax and parser limits;
/// root type; `format`; unknown top-level fields; `id`; `description`; `units`; object count;
/// then every object in document order. Within an object each field is fully checked (presence,
/// type, limit, sign, minimum, degeneracy) before the next one, in the order documented in
/// `docs/fixtures/README.md`; the object key and its uniqueness come last.
pub fn check(bytes: &[u8]) -> Result<Accepted, Rejection> {
    let root = json::parse(bytes, &PARSE_LIMITS).map_err(|e| match e {
        ParseError::Malformed { .. } => reject(ErrorCode::MalformedJson, e.to_string()),
        ParseError::NonFinite { .. } => reject(ErrorCode::NonFiniteNumber, e.to_string()),
        ParseError::Limit { .. } => reject(ErrorCode::LimitExceeded, e.to_string()),
    })?;

    let members = root
        .as_object()
        .ok_or_else(|| invalid("the document root must be an object"))?;
    let format = required(&root, "format", "$")?
        .as_str()
        .ok_or_else(|| invalid("$.format must be a string"))?;
    if format != FORMAT {
        return Err(reject(
            ErrorCode::UnsupportedFormat,
            format!("$.format is {format:?}; only {FORMAT:?} is supported"),
        ));
    }
    only_known(
        members,
        "$",
        &["format", "id", "description", "units", "objects"],
    )?;
    let id = identifier(required(&root, "id", "$")?, "$.id")?;
    if let Some(description) = root.get("description") {
        description
            .as_str()
            .ok_or_else(|| invalid("$.description must be a string"))?;
    }
    units(required(&root, "units", "$")?)?;

    let objects = required(&root, "objects", "$")?
        .as_array()
        .ok_or_else(|| invalid("$.objects must be an array"))?;
    if objects.is_empty() {
        return Err(invalid("$.objects must contain at least one object"));
    }
    if objects.len() > MAX_OBJECTS {
        return Err(reject(
            ErrorCode::LimitExceeded,
            format!(
                "$.objects has {} entries; the maximum is {MAX_OBJECTS}",
                objects.len()
            ),
        ));
    }

    let mut keys: Vec<&str> = Vec::with_capacity(objects.len());
    let (mut plates, mut members_count) = (0usize, 0usize);
    for (index, object) in objects.iter().enumerate() {
        let path = format!("$.objects[{index}]");
        let kind = required(object, "kind", &path)?
            .as_str()
            .ok_or_else(|| invalid(format!("{path}.kind must be a string")))?;
        match kind {
            "plate" => {
                plate(object, &path)?;
                plates = plates.saturating_add(1);
            }
            "member" => {
                member(object, &path)?;
                members_count = members_count.saturating_add(1);
            }
            other => {
                return Err(invalid(format!(
                    "{path}.kind is {other:?}; expected \"plate\" or \"member\""
                )));
            }
        }
        let key = identifier(required(object, "key", &path)?, &format!("{path}.key"))?;
        if keys.contains(&key) {
            return Err(reject(
                ErrorCode::DuplicateObjectKey,
                format!("{path}.key {key:?} is already used"),
            ));
        }
        keys.push(key);
    }

    Ok(Accepted {
        id: id.to_owned(),
        plates,
        members: members_count,
    })
}

fn required<'a>(value: &'a Json, name: &str, path: &str) -> Result<&'a Json, Rejection> {
    if value.as_object().is_none() {
        return Err(invalid(format!("{path} must be an object")));
    }
    value
        .get(name)
        .ok_or_else(|| invalid(format!("{path}.{name} is required")))
}

fn only_known(members: &[(String, Json)], path: &str, allowed: &[&str]) -> Result<(), Rejection> {
    match members.iter().find(|(n, _)| !allowed.contains(&n.as_str())) {
        Some((name, _)) => Err(invalid(format!("{path}.{name} is not a known field"))),
        None => Ok(()),
    }
}

fn object_fields<'a>(
    value: &'a Json,
    path: &str,
    allowed: &[&str],
) -> Result<&'a [(String, Json)], Rejection> {
    let members = value
        .as_object()
        .ok_or_else(|| invalid(format!("{path} must be an object")))?;
    only_known(members, path, allowed)?;
    Ok(members)
}

fn identifier<'a>(value: &'a Json, path: &str) -> Result<&'a str, Rejection> {
    let text = value
        .as_str()
        .ok_or_else(|| invalid(format!("{path} must be a string")))?;
    let valid_chars = text
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-');
    if text.is_empty() || text.len() > MAX_IDENTIFIER_BYTES || !valid_chars {
        return Err(invalid(format!(
            "{path} must be 1-{MAX_IDENTIFIER_BYTES} characters of [a-z0-9.-]"
        )));
    }
    Ok(text)
}

fn units(value: &Json) -> Result<(), Rejection> {
    object_fields(value, "$.units", &["length", "angle", "density"])?;
    for (name, wanted) in [("length", "mm"), ("angle", "deg"), ("density", "kg/m3")] {
        let path = format!("$.units.{name}");
        let unit = required(value, name, "$.units")?
            .as_str()
            .ok_or_else(|| invalid(format!("{path} must be a string")))?;
        if unit != wanted {
            return Err(reject(
                ErrorCode::UnsupportedUnit,
                format!("{path} is {unit:?}; only {wanted:?} is supported"),
            ));
        }
    }
    Ok(())
}

fn number(value: &Json, path: &str) -> Result<f64, Rejection> {
    value
        .as_f64()
        .ok_or_else(|| invalid(format!("{path} must be a number")))
}

fn vec3(value: &Json, path: &str) -> Result<[f64; 3], Rejection> {
    let items = value
        .as_array()
        .ok_or_else(|| invalid(format!("{path} must be an array of three numbers")))?;
    let [x, y, z] = items else {
        return Err(invalid(format!("{path} must be an array of three numbers")));
    };
    let out = [
        number(x, &format!("{path}[0]"))?,
        number(y, &format!("{path}[1]"))?,
        number(z, &format!("{path}[2]"))?,
    ];
    for (i, c) in out.iter().enumerate() {
        if c.abs() > MAX_ABS_COORDINATE_MM {
            return Err(reject(
                ErrorCode::LimitExceeded,
                format!("{path}[{i}] = {c} exceeds ±{MAX_ABS_COORDINATE_MM} mm"),
            ));
        }
    }
    Ok(out)
}

fn dimension(value: &Json, path: &str) -> Result<f64, Rejection> {
    let d = number(value, path)?;
    if d.abs() > MAX_DIMENSION_MM {
        Err(reject(
            ErrorCode::LimitExceeded,
            format!("{path} = {d} exceeds {MAX_DIMENSION_MM} mm"),
        ))
    } else if d <= 0.0 {
        Err(reject(
            ErrorCode::NonPositiveDimension,
            format!("{path} = {d} must be greater than zero"),
        ))
    } else if d < MIN_DIMENSION_MM {
        Err(reject(
            ErrorCode::DegenerateGeometry,
            format!("{path} = {d} is below the {MIN_DIMENSION_MM} mm minimum"),
        ))
    } else {
        Ok(d)
    }
}

fn density(object: &Json, path: &str) -> Result<(), Rejection> {
    let Some(value) = object.get("density") else {
        // Absent density means unknown density; it is not an error.
        return Ok(());
    };
    let path = format!("{path}.density");
    let d = number(value, &path)?;
    if d.abs() > MAX_DENSITY_KG_M3 {
        Err(reject(
            ErrorCode::LimitExceeded,
            format!("{path} = {d} exceeds {MAX_DENSITY_KG_M3} kg/m3"),
        ))
    } else if d <= 0.0 {
        Err(reject(
            ErrorCode::NonPositiveDensity,
            format!("{path} = {d} must be greater than zero"),
        ))
    } else {
        Ok(())
    }
}

fn length(v: [f64; 3]) -> f64 {
    let [x, y, z] = v;
    (x * x + y * y + z * z).sqrt()
}

fn plate(object: &Json, path: &str) -> Result<(), Rejection> {
    object_fields(
        object,
        path,
        &["key", "kind", "size", "placement", "density"],
    )?;
    let size_path = format!("{path}.size");
    let size = required(object, "size", path)?;
    let items = size
        .as_array()
        .ok_or_else(|| invalid(format!("{size_path} must be an array of three numbers")))?;
    if items.len() != 3 {
        return Err(invalid(format!(
            "{size_path} must be an array of three numbers"
        )));
    }
    for (i, item) in items.iter().enumerate() {
        dimension(item, &format!("{size_path}[{i}]"))?;
    }

    let placement_path = format!("{path}.placement");
    let placement = required(object, "placement", path)?;
    object_fields(placement, &placement_path, &["origin", "axis", "angle"])?;
    vec3(
        required(placement, "origin", &placement_path)?,
        &format!("{placement_path}.origin"),
    )?;
    let axis = vec3(
        required(placement, "axis", &placement_path)?,
        &format!("{placement_path}.axis"),
    )?;
    let angle_path = format!("{placement_path}.angle");
    let angle = number(required(placement, "angle", &placement_path)?, &angle_path)?;
    if angle.abs() > MAX_ABS_ANGLE_DEG {
        return Err(reject(
            ErrorCode::LimitExceeded,
            format!("{angle_path} = {angle} exceeds ±{MAX_ABS_ANGLE_DEG} deg"),
        ));
    }
    if length(axis) < MIN_DIRECTION_LENGTH {
        return Err(reject(
            ErrorCode::DegenerateGeometry,
            format!("{placement_path}.axis has zero length"),
        ));
    }
    density(object, path)
}

fn member(object: &Json, path: &str) -> Result<(), Rejection> {
    object_fields(
        object,
        path,
        &["key", "kind", "start", "end", "up", "profile", "density"],
    )?;
    let start = vec3(required(object, "start", path)?, &format!("{path}.start"))?;
    let end = vec3(required(object, "end", path)?, &format!("{path}.end"))?;
    let up = vec3(required(object, "up", path)?, &format!("{path}.up"))?;

    let profile_path = format!("{path}.profile");
    let profile = required(object, "profile", path)?;
    let shape = required(profile, "shape", &profile_path)?
        .as_str()
        .ok_or_else(|| invalid(format!("{profile_path}.shape must be a string")))?;
    if shape != "rect" {
        return Err(reject(
            ErrorCode::UnsupportedProfile,
            format!("{profile_path}.shape is {shape:?}; only \"rect\" is supported"),
        ));
    }
    object_fields(profile, &profile_path, &["shape", "width", "height"])?;
    dimension(
        required(profile, "width", &profile_path)?,
        &format!("{profile_path}.width"),
    )?;
    dimension(
        required(profile, "height", &profile_path)?,
        &format!("{profile_path}.height"),
    )?;

    let [sx, sy, sz] = start;
    let [ex, ey, ez] = end;
    let axis = [ex - sx, ey - sy, ez - sz];
    let axis_length = length(axis);
    if axis_length < MIN_DIMENSION_MM {
        return Err(reject(
            ErrorCode::DegenerateGeometry,
            format!(
                "{path} axis length {axis_length} mm is below the {MIN_DIMENSION_MM} mm minimum"
            ),
        ));
    }
    let up_length = length(up);
    if up_length < MIN_DIRECTION_LENGTH {
        return Err(reject(
            ErrorCode::DegenerateGeometry,
            format!("{path}.up has zero length"),
        ));
    }
    let [ax, ay, az] = axis;
    let [ux, uy, uz] = up;
    let cross = [uy * az - uz * ay, uz * ax - ux * az, ux * ay - uy * ax];
    if length(cross) <= PARALLEL_SINE * up_length * axis_length {
        return Err(reject(
            ErrorCode::DegenerateGeometry,
            format!("{path}.up is parallel to the member axis"),
        ));
    }
    density(object, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLATE: &str = r#"{"format": "beamcraft.fixture/1", "id": "t.plate",
        "units": {"length": "mm", "angle": "deg", "density": "kg/m3"},
        "objects": [{"key": "p", "kind": "plate", "size": [SIZE],
            "placement": {"origin": [0, 0, 0], "axis": [AXIS], "angle": 0}}]}"#;

    const MEMBER: &str = r#"{"format": "beamcraft.fixture/1", "id": "t.member",
        "units": {"length": "mm", "angle": "deg", "density": "kg/m3"},
        "objects": [{"key": "m", "kind": "member", "start": [0, 0, 0], "end": [END],
            "up": [UP], "profile": {"shape": "rect", "width": 10, "height": 20}}]}"#;

    fn plate(size: &str, axis: &str) -> Result<Accepted, Rejection> {
        check(PLATE.replace("SIZE", size).replace("AXIS", axis).as_bytes())
    }

    fn member(end: &str, up: &str) -> Result<Accepted, Rejection> {
        check(MEMBER.replace("END", end).replace("UP", up).as_bytes())
    }

    fn code(r: Result<Accepted, Rejection>) -> ErrorCode {
        r.unwrap_err().code
    }

    #[test]
    fn accepts_minimal_bodies() {
        let a = plate("1, 2, 3", "0, 0, 1").unwrap();
        assert_eq!((a.id.as_str(), a.plates, a.members), ("t.plate", 1, 0));
        assert_eq!(member("100, 0, 0", "0, 0, 1").unwrap().members, 1);
        // The member up vector need not be unit length or exactly perpendicular.
        assert!(member("100, 0, 0", "1, 0, 1").is_ok());
    }

    #[test]
    fn classifies_dimension_violations_in_order() {
        assert_eq!(
            code(plate("1, 2, 0", "0, 0, 1")),
            ErrorCode::NonPositiveDimension
        );
        assert_eq!(
            code(plate("1, -2, 3", "0, 0, 1")),
            ErrorCode::NonPositiveDimension
        );
        assert_eq!(
            code(plate("1, 2, 0.0005", "0, 0, 1")),
            ErrorCode::DegenerateGeometry
        );
        assert_eq!(
            code(plate("1, 2, -200000", "0, 0, 1")),
            ErrorCode::LimitExceeded
        );
        assert!(plate("0.001, 100000, 3", "0, 0, 1").is_ok());
        assert_eq!(code(plate("1, 2", "0, 0, 1")), ErrorCode::InvalidField);
        assert_eq!(
            code(plate("1, 2, 3", "0, 0, 0")),
            ErrorCode::DegenerateGeometry
        );
        assert_eq!(
            code(plate("1, 2, 3", "0, 0, 1e-12")),
            ErrorCode::DegenerateGeometry
        );
    }

    #[test]
    fn detects_degenerate_members() {
        assert_eq!(
            code(member("0, 0, 0", "0, 0, 1")),
            ErrorCode::DegenerateGeometry
        );
        assert_eq!(
            code(member("0.0005, 0, 0", "0, 0, 1")),
            ErrorCode::DegenerateGeometry
        );
        assert_eq!(
            code(member("100, 0, 0", "-3, 0, 0")),
            ErrorCode::DegenerateGeometry
        );
        assert_eq!(
            code(member("100, 0, 0", "0, 0, 0")),
            ErrorCode::DegenerateGeometry
        );
        assert_eq!(
            code(member("20000000, 0, 0", "0, 0, 1")),
            ErrorCode::LimitExceeded
        );
    }

    #[test]
    fn maps_parser_failures() {
        assert_eq!(code(check(b"{")), ErrorCode::MalformedJson);
        assert_eq!(code(check(b"[NaN]")), ErrorCode::NonFiniteNumber);
        assert_eq!(
            code(check(&vec![b' '; MAX_INPUT_BYTES + 1])),
            ErrorCode::LimitExceeded
        );
        assert_eq!(code(check(b"[]")), ErrorCode::InvalidField);
    }

    #[test]
    fn reports_the_first_violation_in_documented_field_order() {
        let base = r#"{"format": "beamcraft.fixture/1", "id": "t.order",
            "units": {"length": "mm", "angle": "deg", "density": "kg/m3"}, "objects": [OBJ]}"#;
        let first = |object: &str| code(check(base.replace("OBJ", object).as_bytes()));
        // Size elements are checked one at a time: sign of [0] before the limit of [1].
        assert_eq!(
            first(
                r#"{"key": "p", "kind": "plate", "size": [0, 250000, 15],
                "placement": {"origin": [0, 0, 0], "axis": [0, 0, 1], "angle": 0}}"#
            ),
            ErrorCode::NonPositiveDimension
        );
        // Size before placement presence and limits.
        assert_eq!(
            first(r#"{"key": "p", "kind": "plate", "size": [640, 410, 0]}"#),
            ErrorCode::NonPositiveDimension
        );
        assert_eq!(
            first(
                r#"{"key": "p", "kind": "plate", "size": [640, 410, 0],
                "placement": {"origin": [2e7, 0, 0], "axis": [0, 0, 1], "angle": 0}}"#
            ),
            ErrorCode::NonPositiveDimension
        );
        // Zero rotation axis before density.
        assert_eq!(
            first(
                r#"{"key": "p", "kind": "plate", "size": [1, 1, 1], "density": "x",
                "placement": {"origin": [0, 0, 0], "axis": [0, 0, 0], "angle": 0}}"#
            ),
            ErrorCode::DegenerateGeometry
        );
        // Member: start limit before profile presence.
        assert_eq!(
            first(
                r#"{"key": "m", "kind": "member", "start": [2e7, 0, 0],
                "end": [1, 0, 0], "up": [0, 0, 1]}"#
            ),
            ErrorCode::LimitExceeded
        );
        // Unknown fields before any value check; key format after the body.
        assert_eq!(
            first(r#"{"key": "p", "kind": "plate", "size": [0, 0, 0], "extra": 1}"#),
            ErrorCode::InvalidField
        );
        assert_eq!(
            first(r#"{"key": "P!", "kind": "plate", "size": [0, 1, 1]}"#),
            ErrorCode::NonPositiveDimension
        );
    }

    #[test]
    fn error_codes_round_trip() {
        for c in ErrorCode::ALL {
            assert_eq!(ErrorCode::from_name(c.as_str()), Some(c));
        }
        assert_eq!(ErrorCode::from_name("nope"), None);
    }
}
