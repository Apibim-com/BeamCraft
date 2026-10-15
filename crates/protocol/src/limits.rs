// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The fixed sizes and the negotiable limits of F05 sections 3, 4 and 10.
//!
//! Negotiation can only lower a limit. [`MAXIMA`] are the draft values of the contract; no code
//! path here widens them.

use crate::code::TransportCode;

/// Length of the common frame header, bytes.
pub const HEADER_LEN: usize = 16;
/// Length of the binary attachment prefix, bytes.
pub const BINARY_PREFIX_LEN: usize = 44;
/// Frame magic, ASCII `BCWP`.
pub const MAGIC: [u8; 4] = *b"BCWP";
/// Wire protocol major version.
pub const VERSION_MAJOR: u8 = 0;
/// Wire protocol minor version.
pub const VERSION_MINOR: u8 = 1;
/// Wire protocol patch version.
pub const VERSION_PATCH: u16 = 0;
/// Largest number of attachments per bundle; indices run 0..=7.
pub const MAX_ATTACHMENTS: u16 = 8;
/// Largest request, ping and event identifier. Identifiers never wrap.
pub const MAX_ID: u16 = 65_535;

/// Position of `attachments` in [`Limits::KEYS`]: the only field that may be zero.
const ATTACHMENTS_AT: usize = 5;

/// The 14 negotiable limit fields of F05 section 4, in the order of its table.
///
/// `host_limits`, `worker_limits` and `effective_limits` are closed objects with exactly these
/// integer keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Control payload bytes.
    pub control_bytes: u32,
    /// Bytes of one binary attachment.
    pub buffer_bytes: u32,
    /// Bytes of one bundle, including headers and prefixes.
    pub bundle_bytes: u32,
    /// Retained wire bytes per endpoint.
    pub endpoint_wire_bytes: u32,
    /// Parser and canonicalizer scratch bytes.
    pub scratch_bytes: u32,
    /// Attachments per bundle; the only field that may be zero.
    pub attachments: u32,
    /// JSON nesting depth.
    pub json_depth: u32,
    /// JSON tokens.
    pub json_tokens: u32,
    /// Decoded string bytes.
    pub string_bytes: u32,
    /// Cursor bytes.
    pub cursor_bytes: u32,
    /// Safe message bytes.
    pub message_bytes: u32,
    /// Items per page or batch.
    pub batch_items: u32,
    /// Pages per delivery.
    pub delivery_pages: u32,
    /// Request timeout, milliseconds.
    pub request_timeout_ms: u32,
}

/// The draft maxima of F05 section 4. Values above these are incompatible.
pub const MAXIMA: Limits = Limits {
    control_bytes: 1_048_576,
    buffer_bytes: 16_777_216,
    bundle_bytes: 33_554_432,
    endpoint_wire_bytes: 67_108_864,
    scratch_bytes: 8_388_608,
    attachments: 8,
    json_depth: 16,
    json_tokens: 65_536,
    string_bytes: 4_096,
    cursor_bytes: 256,
    message_bytes: 1_024,
    batch_items: 4_096,
    delivery_pages: 1_024,
    request_timeout_ms: 120_000,
};

impl Limits {
    /// The contract key names, in table order.
    pub const KEYS: [&'static str; 14] = [
        "control_bytes",
        "buffer_bytes",
        "bundle_bytes",
        "endpoint_wire_bytes",
        "scratch_bytes",
        "attachments",
        "json_depth",
        "json_tokens",
        "string_bytes",
        "cursor_bytes",
        "message_bytes",
        "batch_items",
        "delivery_pages",
        "request_timeout_ms",
    ];

    /// The values in [`Limits::KEYS`] order.
    pub const fn values(&self) -> [u32; 14] {
        [
            self.control_bytes,
            self.buffer_bytes,
            self.bundle_bytes,
            self.endpoint_wire_bytes,
            self.scratch_bytes,
            self.attachments,
            self.json_depth,
            self.json_tokens,
            self.string_bytes,
            self.cursor_bytes,
            self.message_bytes,
            self.batch_items,
            self.delivery_pages,
            self.request_timeout_ms,
        ]
    }

    /// Builds limits from values in [`Limits::KEYS`] order.
    pub const fn from_values(values: [u32; 14]) -> Self {
        let [
            control_bytes,
            buffer_bytes,
            bundle_bytes,
            endpoint_wire_bytes,
            scratch_bytes,
            attachments,
            json_depth,
            json_tokens,
            string_bytes,
            cursor_bytes,
            message_bytes,
            batch_items,
            delivery_pages,
            request_timeout_ms,
        ] = values;
        Self {
            control_bytes,
            buffer_bytes,
            bundle_bytes,
            endpoint_wire_bytes,
            scratch_bytes,
            attachments,
            json_depth,
            json_tokens,
            string_bytes,
            cursor_bytes,
            message_bytes,
            batch_items,
            delivery_pages,
            request_timeout_ms,
        }
    }

    /// Checks that every value is positive (`attachments` may be zero) and at most its maximum.
    ///
    /// # Errors
    ///
    /// [`TransportCode::CapabilityMismatch`] for a value outside its range.
    pub fn validate(&self) -> Result<(), TransportCode> {
        let max = MAXIMA.values();
        let ok = self
            .values()
            .iter()
            .zip(max.iter())
            .enumerate()
            .all(|(i, (v, m))| v <= m && (*v > 0 || i == ATTACHMENTS_AT));
        if ok {
            Ok(())
        } else {
            Err(TransportCode::CapabilityMismatch)
        }
    }

    /// The effective limits: the per-field minimum of host and worker. Both inputs must be valid.
    ///
    /// # Errors
    ///
    /// [`TransportCode::CapabilityMismatch`] if either side is outside its allowed range.
    pub fn effective(host: &Self, worker: &Self) -> Result<Self, TransportCode> {
        host.validate()?;
        worker.validate()?;
        let (h, w) = (host.values(), worker.values());
        let mut out = [0u32; 14];
        for (slot, (a, b)) in out.iter_mut().zip(h.iter().zip(w.iter())) {
            *slot = (*a).min(*b);
        }
        Ok(Self::from_values(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maxima_are_valid_and_match_the_contract_table() {
        assert_eq!(MAXIMA.validate(), Ok(()));
        assert_eq!(
            MAXIMA.values(),
            [
                1_048_576, 16_777_216, 33_554_432, 67_108_864, 8_388_608, 8, 16, 65_536, 4_096,
                256, 1_024, 4_096, 1_024, 120_000
            ]
        );
        assert_eq!(Limits::KEYS.len(), MAXIMA.values().len());
        assert_eq!(Limits::KEYS[ATTACHMENTS_AT], "attachments");
    }

    #[test]
    fn any_field_one_above_its_maximum_is_incompatible() {
        for i in 0..14 {
            let mut v = MAXIMA.values();
            v[i] += 1;
            assert_eq!(
                Limits::from_values(v).validate(),
                Err(TransportCode::CapabilityMismatch),
                "field {}",
                Limits::KEYS[i]
            );
        }
    }

    #[test]
    fn zero_is_valid_only_for_attachments() {
        for i in 0..14 {
            let mut v = MAXIMA.values();
            v[i] = 0;
            let result = Limits::from_values(v).validate();
            if i == ATTACHMENTS_AT {
                assert_eq!(result, Ok(()));
            } else {
                assert_eq!(result, Err(TransportCode::CapabilityMismatch), "field {i}");
            }
        }
    }

    #[test]
    fn effective_is_the_per_field_minimum() {
        let host = Limits {
            control_bytes: 1000,
            attachments: 2,
            ..MAXIMA
        };
        let worker = Limits {
            control_bytes: 5000,
            attachments: 0,
            json_depth: 8,
            ..MAXIMA
        };
        assert_eq!(
            Limits::effective(&host, &worker),
            Ok(Limits {
                control_bytes: 1000,
                attachments: 0,
                json_depth: 8,
                ..MAXIMA
            })
        );
    }

    #[test]
    fn effective_rejects_an_out_of_range_side() {
        let wide = Limits {
            string_bytes: 4097,
            ..MAXIMA
        };
        assert_eq!(
            Limits::effective(&wide, &MAXIMA),
            Err(TransportCode::CapabilityMismatch)
        );
        assert_eq!(
            Limits::effective(&MAXIMA, &wide),
            Err(TransportCode::CapabilityMismatch)
        );
    }
}
