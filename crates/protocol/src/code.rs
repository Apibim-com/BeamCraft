// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Transport error codes (F05 section 9).

use beamcraft_model::ErrorCode;
use core::fmt;

/// An `IPC_` transport code. These are distinct from the F04 stable codes; [`TransportCode::f04`]
/// gives the F04 code used when a command outcome is reportable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportCode {
    /// Unsupported header or negotiated version.
    Version,
    /// Bad magic, kind, flags, reserved field or structural framing.
    Frame,
    /// Byte, count, depth or queue budget exceeded.
    Limit,
    /// End of input inside a frame or bundle.
    Truncated,
    /// Undeclared buffer, or length, type, index, count or hash disagreement.
    Buffer,
    /// Invalid encoding, JSON grammar or duplicate decoded keys.
    Json,
    /// Unknown type, method or closed control-schema field or value.
    Message,
    /// Reused or decreasing correlation ID, or a repeated cancel.
    DuplicateId,
    /// Uncorrelated response, cancel or pong.
    UnknownId,
    /// More than one outstanding normal request.
    Busy,
    /// Missing feature, unexpected runtime or schema, or limits outside their range.
    CapabilityMismatch,
    /// Wrong nonce or invalid event sequence.
    Session,
    /// Wrong stage incarnation token.
    StaleStage,
    /// The model tried to request scripts or plugin loading.
    ModelExecution,
    /// Assembly or request deadline, or liveness expiry.
    Deadline,
    /// Process exit, broken pipe or unresponsive backend.
    BackendLost,
}

impl TransportCode {
    /// Every code, in the order of the F05 section 9 table.
    pub const ALL: [Self; 16] = [
        Self::Version,
        Self::Frame,
        Self::Limit,
        Self::Truncated,
        Self::Buffer,
        Self::Json,
        Self::Message,
        Self::DuplicateId,
        Self::UnknownId,
        Self::Busy,
        Self::CapabilityMismatch,
        Self::Session,
        Self::StaleStage,
        Self::ModelExecution,
        Self::Deadline,
        Self::BackendLost,
    ];

    /// The stable wire name, for example `"IPC_FRAME"`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Version => "IPC_VERSION",
            Self::Frame => "IPC_FRAME",
            Self::Limit => "IPC_LIMIT",
            Self::Truncated => "IPC_TRUNCATED",
            Self::Buffer => "IPC_BUFFER",
            Self::Json => "IPC_JSON",
            Self::Message => "IPC_MESSAGE",
            Self::DuplicateId => "IPC_DUPLICATE_ID",
            Self::UnknownId => "IPC_UNKNOWN_ID",
            Self::Busy => "IPC_BUSY",
            Self::CapabilityMismatch => "IPC_CAPABILITY_MISMATCH",
            Self::Session => "IPC_SESSION",
            Self::StaleStage => "IPC_STALE_STAGE",
            Self::ModelExecution => "IPC_MODEL_EXECUTION",
            Self::Deadline => "IPC_DEADLINE",
            Self::BackendLost => "IPC_BACKEND_LOST",
        }
    }

    /// Parses an exact wire name. Unknown names, other cases and padded text give `None`.
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|code| code.as_str() == name)
    }

    /// The F04 code reported when a command outcome is reportable (F05 section 9 table).
    ///
    /// F05 section 8's uncertainty override still applies to a previously dispatched mutation:
    /// a transport diagnostic never proves rollback, so the host may need `OUTCOME_UNKNOWN`.
    pub const fn f04(self) -> ErrorCode {
        match self {
            Self::Version | Self::CapabilityMismatch => ErrorCode::UnsupportedVersion,
            Self::Frame | Self::Json | Self::DuplicateId | Self::UnknownId => {
                ErrorCode::MalformedCommand
            }
            Self::Limit | Self::Busy => ErrorCode::LimitExceeded,
            Self::Truncated | Self::Deadline | Self::BackendLost => ErrorCode::BackendUnavailable,
            Self::Buffer => ErrorCode::IntegrityError,
            Self::Message => ErrorCode::UnsupportedCommand,
            Self::Session | Self::StaleStage => ErrorCode::StaleWorkspace,
            Self::ModelExecution => ErrorCode::PermissionDenied,
        }
    }
}

impl fmt::Display for TransportCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip_and_are_unique() {
        for (i, a) in TransportCode::ALL.iter().enumerate() {
            assert_eq!(TransportCode::parse(a.as_str()), Some(*a));
            assert!(a.as_str().starts_with("IPC_") && a.as_str().is_ascii());
            for b in TransportCode::ALL.iter().skip(i + 1) {
                assert_ne!(a.as_str(), b.as_str());
            }
        }
    }

    #[test]
    fn f04_mapping_matches_the_contract_table() {
        let table = [
            ("IPC_VERSION", "UNSUPPORTED_VERSION"),
            ("IPC_FRAME", "MALFORMED_COMMAND"),
            ("IPC_LIMIT", "LIMIT_EXCEEDED"),
            ("IPC_TRUNCATED", "BACKEND_UNAVAILABLE"),
            ("IPC_BUFFER", "INTEGRITY_ERROR"),
            ("IPC_JSON", "MALFORMED_COMMAND"),
            ("IPC_MESSAGE", "UNSUPPORTED_COMMAND"),
            ("IPC_DUPLICATE_ID", "MALFORMED_COMMAND"),
            ("IPC_UNKNOWN_ID", "MALFORMED_COMMAND"),
            ("IPC_BUSY", "LIMIT_EXCEEDED"),
            ("IPC_CAPABILITY_MISMATCH", "UNSUPPORTED_VERSION"),
            ("IPC_SESSION", "STALE_WORKSPACE"),
            ("IPC_STALE_STAGE", "STALE_WORKSPACE"),
            ("IPC_MODEL_EXECUTION", "PERMISSION_DENIED"),
            ("IPC_DEADLINE", "BACKEND_UNAVAILABLE"),
            ("IPC_BACKEND_LOST", "BACKEND_UNAVAILABLE"),
        ];
        assert_eq!(table.len(), TransportCode::ALL.len());
        for (ipc, f04) in table {
            let code = TransportCode::parse(ipc);
            assert_eq!(code.map(|c| c.f04().as_str()), Some(f04), "{ipc}");
        }
    }

    #[test]
    fn rejects_unknown_cased_and_f04_names() {
        for bad in [
            "",
            "ipc_frame",
            "IPC_FRAME ",
            "IPC_",
            "STALE_WORKSPACE",
            "IPC_OTHER",
        ] {
            assert_eq!(TransportCode::parse(bad), None, "{bad:?}");
        }
    }
}
