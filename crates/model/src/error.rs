// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The stable error codes of the F04 model and command contract (section 8).
//!
//! Only the code vocabulary lives here. Messages, diagnostics and the validation order belong to
//! the command engine. Other contracts (F05 transport, F06 geometry) map their own codes onto
//! these.

use core::fmt;

/// A stable F04 error code. The wire form is the upper-case snake name from [`ErrorCode::as_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// Invalid encoding/JSON, duplicate keys or a structurally invalid envelope.
    MalformedCommand,
    /// Contract version outside the accepted set.
    UnsupportedVersion,
    /// Operation outside the accepted contract.
    UnsupportedCommand,
    /// Property outside the accepted contract.
    UnsupportedProperty,
    /// Invalid full identity.
    InvalidId,
    /// Creation ID that is already owned.
    DuplicateId,
    /// Missing target or definition.
    NotFound,
    /// Bad field, type or range.
    InvalidArgument,
    /// Non-finite f64.
    NonFiniteNumber,
    /// Bounded-input cap violation.
    LimitExceeded,
    /// Untrusted or mismatched caller.
    PermissionDenied,
    /// Attempted foreign edit or override.
    ForeignReadOnly,
    /// Failed base-revision precondition.
    BaseRevisionMismatch,
    /// Failed workspace-generation precondition.
    StaleWorkspace,
    /// Changed request under an accepted `op_id`.
    OpIdReuse,
    /// Invalid link.
    DanglingRelationship,
    /// Forbidden delete or change because of a dependency.
    DependencyConflict,
    /// Incomplete validation inputs.
    DependencyUnavailable,
    /// Shape or transform outside the supported scope.
    UnsupportedGeometry,
    /// Revision or blob bytes inconsistent with their identity.
    IntegrityError,
    /// Cancelled before the commit point.
    Cancelled,
    /// Uncertain committed outcome.
    OutcomeUnknown,
    /// Required backend unavailable.
    BackendUnavailable,
}

impl ErrorCode {
    /// Every code, in the order of the F04 section 8 table.
    pub const ALL: [Self; 23] = [
        Self::MalformedCommand,
        Self::UnsupportedVersion,
        Self::UnsupportedCommand,
        Self::UnsupportedProperty,
        Self::InvalidId,
        Self::DuplicateId,
        Self::NotFound,
        Self::InvalidArgument,
        Self::NonFiniteNumber,
        Self::LimitExceeded,
        Self::PermissionDenied,
        Self::ForeignReadOnly,
        Self::BaseRevisionMismatch,
        Self::StaleWorkspace,
        Self::OpIdReuse,
        Self::DanglingRelationship,
        Self::DependencyConflict,
        Self::DependencyUnavailable,
        Self::UnsupportedGeometry,
        Self::IntegrityError,
        Self::Cancelled,
        Self::OutcomeUnknown,
        Self::BackendUnavailable,
    ];

    /// The stable wire name, for example `"STALE_WORKSPACE"`. At most 32 ASCII bytes, which is
    /// the F05 `reason_code` bound.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MalformedCommand => "MALFORMED_COMMAND",
            Self::UnsupportedVersion => "UNSUPPORTED_VERSION",
            Self::UnsupportedCommand => "UNSUPPORTED_COMMAND",
            Self::UnsupportedProperty => "UNSUPPORTED_PROPERTY",
            Self::InvalidId => "INVALID_ID",
            Self::DuplicateId => "DUPLICATE_ID",
            Self::NotFound => "NOT_FOUND",
            Self::InvalidArgument => "INVALID_ARGUMENT",
            Self::NonFiniteNumber => "NON_FINITE_NUMBER",
            Self::LimitExceeded => "LIMIT_EXCEEDED",
            Self::PermissionDenied => "PERMISSION_DENIED",
            Self::ForeignReadOnly => "FOREIGN_READ_ONLY",
            Self::BaseRevisionMismatch => "BASE_REVISION_MISMATCH",
            Self::StaleWorkspace => "STALE_WORKSPACE",
            Self::OpIdReuse => "OP_ID_REUSE",
            Self::DanglingRelationship => "DANGLING_RELATIONSHIP",
            Self::DependencyConflict => "DEPENDENCY_CONFLICT",
            Self::DependencyUnavailable => "DEPENDENCY_UNAVAILABLE",
            Self::UnsupportedGeometry => "UNSUPPORTED_GEOMETRY",
            Self::IntegrityError => "INTEGRITY_ERROR",
            Self::Cancelled => "CANCELLED",
            Self::OutcomeUnknown => "OUTCOME_UNKNOWN",
            Self::BackendUnavailable => "BACKEND_UNAVAILABLE",
        }
    }

    /// Parses an exact wire name. Unknown names, other cases and padded text give `None`.
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|code| code.as_str() == name)
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for code in ErrorCode::ALL {
            assert_eq!(ErrorCode::parse(code.as_str()), Some(code));
            assert_eq!(code.to_string(), code.as_str());
        }
    }

    #[test]
    fn names_are_unique_and_fit_the_reason_code_bound() {
        for (i, a) in ErrorCode::ALL.iter().enumerate() {
            assert!(a.as_str().len() <= 32 && a.as_str().is_ascii());
            for b in ErrorCode::ALL.iter().skip(i + 1) {
                assert_ne!(a.as_str(), b.as_str());
            }
        }
    }

    #[test]
    fn rejects_unknown_cased_and_padded_names() {
        for bad in [
            "",
            "stale_workspace",
            "Stale_Workspace",
            " STALE_WORKSPACE",
            "STALE_WORKSPACE ",
            "STALE_WORKSPACE\0",
            "IPC_FRAME",
            "NOPE",
        ] {
            assert_eq!(ErrorCode::parse(bad), None, "{bad:?}");
        }
    }
}
