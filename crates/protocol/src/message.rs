// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Closed experimental U04 read profile over F05 envelopes. No mutation schema is admitted.

use crate::{Limits, TransportCode as T, json::Json, limits::MAXIMA};
use beamcraft_model::digest::{ContentDigest, Sha256};

/// Exact backend archive admitted by U01.
pub const ARCHIVE: &str = "54f38d869c64c96492e1a482fb621eda620d120d91dbe93072112ee315a8923c";
/// Closed transport profile; distinct from a U02 authored schema.
pub const PROFILE: &str = "u04-rust-read-v1";
/// Actually implemented read-transport features. Full F05 feature admission remains open.
pub const FEATURES: [&str; 3] = ["private_pipes", "sha256_buffers", "cancel"];

/// Profile commitment, not a claim to the future U02 schema.
pub fn schema_digest() -> ContentDigest {
    // This fixed short input cannot exceed SHA-256's u64 bit counter.
    let mut h = Sha256::new();
    let _ = h.update(PROFILE.as_bytes());
    h.finalize()
}
/// Checks a complete digest/token spelling.
pub fn digest(value: &Json) -> Result<ContentDigest, T> {
    value.string()?.parse().map_err(|_| T::Message)
}
/// Canonical UUIDv7 validation without a shortened ID representation.
pub fn uuid(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else if i == 14 {
                b == b'7'
            } else if i == 19 {
                matches!(b, b'8' | b'9' | b'a' | b'b')
            } else {
                b.is_ascii_digit() || matches!(b, b'a'..=b'f')
            }
        })
}
/// Canonical generation string, lossless across JSON implementations.
pub fn generation(value: &Json) -> Result<u64, T> {
    let s = value.string()?;
    if s.is_empty() || (s.len() > 1 && s.starts_with('0')) || !s.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(T::Message);
    }
    s.parse().map_err(|_| T::Message)
}
/// A canonical virtual asset identifier; never an operating-system path.
pub fn asset_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 1024
        && s.split('/').count() <= 16
        && s.split('/').all(|p| {
            !p.is_empty()
                && p != "."
                && p != ".."
                && p.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        })
}
/// A bounded prim/property path from the static profile.
pub fn prim_path(s: &str) -> bool {
    s.starts_with('/')
        && s.len() <= 1024
        && s.split('/').count() <= 18
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'_' | b':' | b'-' | b'.'))
        && s.split('/')
            .skip(1)
            .all(|part| !part.is_empty() && part != "." && part != "..")
}
/// Verified pinned workspace tuple; digest revision follows U01 pending the F04 alignment erratum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    /// Full project UUIDv7.
    pub project_id: String,
    /// Full workspace UUIDv7.
    pub workspace_id: String,
    /// Complete U01 revision commitment.
    pub base_revision: ContentDigest,
    /// Pinned unpublished workspace generation.
    pub workspace_generation: u64,
}
impl Identity {
    /// Requires valid project/workspace identities.
    pub fn validate(&self) -> Result<(), T> {
        if uuid(&self.project_id) && uuid(&self.workspace_id) {
            Ok(())
        } else {
            Err(T::Message)
        }
    }
    /// Tuple encoded without a stage token.
    pub fn json(&self) -> Json {
        Json::object([
            ("project_id", Json::text(&self.project_id)),
            ("workspace_id", Json::text(&self.workspace_id)),
            ("base_revision", Json::text(self.base_revision.to_string())),
            (
                "workspace_generation",
                Json::text(self.workspace_generation.to_string()),
            ),
        ])
    }
    /// Decodes the four identity fields within a separately closed object.
    pub fn parse(j: &Json) -> Result<Self, T> {
        let identity = Self {
            project_id: j.get("project_id")?.string()?.into(),
            workspace_id: j.get("workspace_id")?.string()?.into(),
            base_revision: digest(j.get("base_revision")?)?,
            workspace_generation: generation(j.get("workspace_generation")?)?,
        };
        identity.validate()?;
        Ok(identity)
    }
}
/// Opaque wire incarnation bound to a complete workspace tuple.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    /// Pinned identity.
    pub identity: Identity,
    /// Non-authorizing incarnation token.
    pub stage_token: ContentDigest,
}
impl Context {
    /// Encodes the exact F05 context fields.
    pub fn json(&self) -> Json {
        let mut j = self.identity.json();
        if let Json::Object(o) = &mut j {
            o.insert(
                "stage_token".into(),
                Json::text(self.stage_token.to_string()),
            );
        }
        j
    }
    /// Decodes a closed context.
    pub fn parse(j: &Json) -> Result<Self, T> {
        j.closed(&[
            "project_id",
            "workspace_id",
            "base_revision",
            "workspace_generation",
            "stage_token",
        ])?;
        Ok(Self {
            identity: Identity::parse(j)?,
            stage_token: digest(j.get("stage_token")?)?,
        })
    }
}
/// F05 limit object.
pub fn limits_json(limits: Limits) -> Json {
    Json::Object(
        Limits::KEYS
            .into_iter()
            .zip(limits.values())
            .map(|(k, v)| (k.into(), Json::uint(u64::from(v))))
            .collect(),
    )
}
/// Validates the closed limit object before use.
pub fn parse_limits(j: &Json) -> Result<Limits, T> {
    j.closed(&Limits::KEYS)?;
    let mut v = [0u32; 14];
    for (slot, key) in v.iter_mut().zip(Limits::KEYS) {
        *slot = u32::try_from(j.get(key)?.unsigned()?).map_err(|_| T::CapabilityMismatch)?;
    }
    let l = Limits::from_values(v);
    l.validate()?;
    Ok(l)
}
/// Limits of this read profile: blobs use U05 files, so no wire attachment is admitted.
pub fn read_limits() -> Limits {
    Limits {
        attachments: 0,
        ..MAXIMA
    }
}
/// Builds a common control envelope.
pub fn envelope(
    kind: &str,
    nonce: &str,
    fields: impl IntoIterator<Item = (&'static str, Json)>,
) -> Json {
    let mut o = std::collections::BTreeMap::from([
        ("type".into(), Json::text(kind)),
        ("session_nonce".into(), Json::text(nonce)),
        ("buffers".into(), Json::Array(vec![])),
    ]);
    for (k, v) in fields {
        o.insert(k.into(), v);
    }
    Json::Object(o)
}
/// Checks type, nonce and empty buffers in an otherwise separately closed envelope.
pub fn common(j: &Json, kind: &str, nonce: &str) -> Result<(), T> {
    if j.get("type")?.string()? != kind {
        return Err(T::Message);
    }
    if j.get("session_nonce")?.string()? != nonce {
        return Err(T::Session);
    }
    if !j.get("buffers")?.array()?.is_empty() {
        return Err(T::Buffer);
    }
    Ok(())
}
/// Validates the closed body/context of an implemented transport method before dispatch.
/// No engineering mutation, raw USD setter or plugin method is registered.
pub fn read_request(method: &str, context: &Json, body: &Json) -> Result<(), T> {
    match method {
        "OpenStage" => {
            if *context != Json::Null {
                return Err(T::Message);
            }
            body.closed(&[
                "project_id",
                "workspace_id",
                "base_revision",
                "workspace_generation",
                "manifest_digest",
            ])?;
            Identity::parse(body)?;
            digest(body.get("manifest_digest")?)?;
        }
        "ReadSnapshot" => {
            Context::parse(context)?;
            body.closed(&["cursor"])?;
            if *body.get("cursor")? != Json::Null {
                digest(body.get("cursor")?)?;
            }
        }
        "CloseStage" => {
            Context::parse(context)?;
            body.closed(&[])?;
        }
        "QueryOutcome" => {
            if *context != Json::Null {
                Context::parse(context)?;
            }
            body.closed(&["project_id", "workspace_id", "op_id"])?;
            for key in ["project_id", "workspace_id", "op_id"] {
                if !uuid(body.get(key)?.string()?) {
                    return Err(T::Message);
                }
            }
        }
        _ => return Err(T::Message),
    }
    Ok(())
}
/// Validates the bounded closed typed static-value representation again at the host boundary.
pub fn validate_value(j: &Json, depth: usize, count: &mut usize) -> Result<(), T> {
    *count = count.checked_add(1).ok_or(T::Limit)?;
    if depth > 16 || *count > 4096 {
        return Err(T::Limit);
    }
    j.closed(&["kind", "value"])?;
    let v = j.get("value")?;
    match j.get("kind")?.string()? {
        "block" if *v == Json::Null => (),
        "bool" => {
            v.boolean()?;
        }
        "int" => {
            if let Json::String(s) = v {
                let n = s.parse::<i64>().map_err(|_| T::Message)?;
                if n.to_string() != *s {
                    return Err(T::Message);
                }
            } else {
                return Err(T::Message);
            }
        }
        "uint" => {
            generation(v)?;
        }
        "number" => {
            let number = v.float()?;
            if !number.is_finite() || number.abs() > 1_000_000_000.0 {
                return Err(T::Message);
            }
        }
        "text" => {
            if v.string()?.len() > 1024 {
                return Err(T::Limit);
            }
        }
        "array" => {
            let a = v.array()?;
            if a.len() > 1024 {
                return Err(T::Limit);
            }
            for e in a {
                validate_value(e, depth + 1, count)?;
            }
        }
        "dictionary" => {
            if let Json::Object(o) = v {
                if o.len() > 1024 {
                    return Err(T::Limit);
                }
                for (key, v) in o {
                    if key.len() > 1024 {
                        return Err(T::Limit);
                    }
                    validate_value(v, depth + 1, count)?;
                }
            } else {
                return Err(T::Message);
            }
        }
        _ => return Err(T::Message),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identities_and_values_are_rechecked() {
        assert!(!uuid("0199e7a0-0000-4000-8000-000000000001"));
        assert!(!asset_name("../outside"));
        assert!(!asset_name("https://site"));
        assert!(generation(&Json::text(u64::MAX.to_string())).is_ok());
        assert!(generation(&Json::text("01")).is_err());
        assert!(
            validate_value(
                &Json::object([("kind", Json::text("number")), ("value", Json::text("1"))]),
                0,
                &mut 0
            )
            .is_err()
        );
    }
}
