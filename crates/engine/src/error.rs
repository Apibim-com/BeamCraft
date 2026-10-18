// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Closed error vocabulary of the view snapshot and query contract.
//!
//! Codes that F06 section 3 already names keep their `GEO_` wire token; the codes that exist only
//! for the view layer use the `VIEW_` prefix. Every code maps onto an existing F04 code, the way
//! the kernel maps its own codes, so no model-layer code is added.

use beamcraft_model::error::ErrorCode;

/// A stable view-contract code. The set is closed: a new code needs a contract amendment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewCode {
    /// Token unknown, expired or older than the retained history (`GEO_STALE_TOKEN`).
    StaleToken,
    /// Vertex, triangle, mesh-byte or snapshot-triangle cap exceeded (`GEO_BUFFER_LIMIT`).
    BufferLimit,
    /// Face-table cap of a mesh blob exceeded (`GEO_TOPOLOGY_LIMIT`).
    TopologyLimit,
    /// A mesh blob disagrees with its header or digest (`GEO_BLOB_INVALID`).
    BlobInvalid,
    /// A derived coordinate was not finite (`GEO_NUMERIC_UNCERTAIN`).
    NumericUncertain,
    /// More elements than the element cap (`VIEW_ELEMENT_LIMIT`).
    ElementLimit,
    /// More selected items than the selection cap (`VIEW_SELECTION_LIMIT`).
    SelectionLimit,
    /// Malformed ElementId (`VIEW_ID_INVALID`).
    IdInvalid,
    /// Two scene elements share an ElementId (`VIEW_DUPLICATE_ELEMENT`).
    DuplicateElement,
    /// A well-formed ElementId absent from the snapshot (`VIEW_UNKNOWN_ELEMENT`).
    UnknownElement,
    /// A malformed token, setting, page size, index or option (`VIEW_ARGUMENT_INVALID`).
    ArgumentInvalid,
    /// NaN or infinity in a query number (`GEO_NON_FINITE`).
    NonFinite,
    /// A bad ray or point query input (`GEO_PICK_INPUT_INVALID`).
    PickInputInvalid,
    /// Hit count, radius or scanned elements above their caps (`GEO_PICK_LIMIT`).
    PickLimit,
    /// A malformed FeatureId (`GEO_FEATURE_ID_INVALID`).
    FeatureIdInvalid,
    /// A well-formed FeatureId naming no feature of the snapshot (`GEO_FEATURE_NOT_FOUND`).
    FeatureNotFound,
    /// A triangle, vertex or buffer index used as an address (`GEO_TRIANGLE_ADDRESS_FORBIDDEN`).
    TriangleAddressForbidden,
}

impl ViewCode {
    /// Every code, in declaration order.
    pub const ALL: [Self; 17] = [
        Self::StaleToken,
        Self::BufferLimit,
        Self::TopologyLimit,
        Self::BlobInvalid,
        Self::NumericUncertain,
        Self::ElementLimit,
        Self::SelectionLimit,
        Self::IdInvalid,
        Self::DuplicateElement,
        Self::UnknownElement,
        Self::ArgumentInvalid,
        Self::NonFinite,
        Self::PickInputInvalid,
        Self::PickLimit,
        Self::FeatureIdInvalid,
        Self::FeatureNotFound,
        Self::TriangleAddressForbidden,
    ];

    /// Stable wire token, at most 32 ASCII bytes.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StaleToken => "GEO_STALE_TOKEN",
            Self::BufferLimit => "GEO_BUFFER_LIMIT",
            Self::TopologyLimit => "GEO_TOPOLOGY_LIMIT",
            Self::BlobInvalid => "GEO_BLOB_INVALID",
            Self::NumericUncertain => "GEO_NUMERIC_UNCERTAIN",
            Self::ElementLimit => "VIEW_ELEMENT_LIMIT",
            Self::SelectionLimit => "VIEW_SELECTION_LIMIT",
            Self::IdInvalid => "VIEW_ID_INVALID",
            Self::DuplicateElement => "VIEW_DUPLICATE_ELEMENT",
            Self::UnknownElement => "VIEW_UNKNOWN_ELEMENT",
            Self::ArgumentInvalid => "VIEW_ARGUMENT_INVALID",
            Self::NonFinite => "GEO_NON_FINITE",
            Self::PickInputInvalid => "GEO_PICK_INPUT_INVALID",
            Self::PickLimit => "GEO_PICK_LIMIT",
            Self::FeatureIdInvalid => "GEO_FEATURE_ID_INVALID",
            Self::FeatureNotFound => "GEO_FEATURE_NOT_FOUND",
            Self::TriangleAddressForbidden => "GEO_TRIANGLE_ADDRESS_FORBIDDEN",
        }
    }

    /// The existing F04 code this code maps onto (F06 section 3 for the `GEO_` tokens).
    pub const fn f04_code(self) -> ErrorCode {
        match self {
            Self::StaleToken => ErrorCode::StaleWorkspace,
            Self::BufferLimit
            | Self::TopologyLimit
            | Self::ElementLimit
            | Self::SelectionLimit
            | Self::PickLimit => ErrorCode::LimitExceeded,
            Self::BlobInvalid => ErrorCode::IntegrityError,
            Self::NumericUncertain => ErrorCode::UnsupportedGeometry,
            Self::IdInvalid => ErrorCode::InvalidId,
            Self::DuplicateElement => ErrorCode::DuplicateId,
            Self::UnknownElement | Self::FeatureNotFound => ErrorCode::NotFound,
            Self::ArgumentInvalid | Self::PickInputInvalid | Self::FeatureIdInvalid => {
                ErrorCode::InvalidArgument
            }
            Self::NonFinite => ErrorCode::NonFiniteNumber,
            Self::TriangleAddressForbidden => ErrorCode::UnsupportedProperty,
        }
    }
}

/// A failed view query. It carries no partial data, no input text and no path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewError {
    code: ViewCode,
}

impl ViewError {
    /// Wraps a code.
    pub const fn new(code: ViewCode) -> Self {
        Self { code }
    }

    /// The view code.
    pub const fn code(self) -> ViewCode {
        self.code
    }

    /// The F04 code of [`Self::code`].
    pub const fn f04_code(self) -> ErrorCode {
        self.code.f04_code()
    }
}

impl core::fmt::Display for ViewError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.code.as_str())
    }
}

impl core::error::Error for ViewError {}

impl From<ViewCode> for ViewError {
    fn from(code: ViewCode) -> Self {
        Self::new(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_unique_ascii_and_fit_the_reason_bound() {
        for (i, a) in ViewCode::ALL.iter().enumerate() {
            assert!(a.as_str().is_ascii() && a.as_str().len() <= 32);
            for b in ViewCode::ALL.iter().skip(i + 1) {
                assert_ne!(a.as_str(), b.as_str());
            }
        }
    }

    #[test]
    fn mapping_follows_f06_section_3() {
        assert_eq!(ViewCode::StaleToken.f04_code(), ErrorCode::StaleWorkspace);
        assert_eq!(ViewCode::BufferLimit.f04_code(), ErrorCode::LimitExceeded);
        assert_eq!(ViewCode::TopologyLimit.f04_code(), ErrorCode::LimitExceeded);
        assert_eq!(ViewCode::BlobInvalid.f04_code(), ErrorCode::IntegrityError);
        assert_eq!(
            ViewCode::NumericUncertain.f04_code(),
            ErrorCode::UnsupportedGeometry
        );
        assert_eq!(ViewCode::ElementLimit.f04_code(), ErrorCode::LimitExceeded);
        assert_eq!(
            ViewCode::SelectionLimit.f04_code(),
            ErrorCode::LimitExceeded
        );
        assert_eq!(ViewCode::IdInvalid.f04_code(), ErrorCode::InvalidId);
        assert_eq!(
            ViewCode::DuplicateElement.f04_code(),
            ErrorCode::DuplicateId
        );
        assert_eq!(ViewCode::UnknownElement.f04_code(), ErrorCode::NotFound);
        assert_eq!(
            ViewCode::ArgumentInvalid.f04_code(),
            ErrorCode::InvalidArgument
        );
        assert_eq!(ViewCode::NonFinite.f04_code(), ErrorCode::NonFiniteNumber);
        for code in [ViewCode::PickInputInvalid, ViewCode::FeatureIdInvalid] {
            assert_eq!(code.f04_code(), ErrorCode::InvalidArgument);
        }
        assert_eq!(ViewCode::PickLimit.f04_code(), ErrorCode::LimitExceeded);
        assert_eq!(ViewCode::FeatureNotFound.f04_code(), ErrorCode::NotFound);
        assert_eq!(
            ViewCode::TriangleAddressForbidden.f04_code(),
            ErrorCode::UnsupportedProperty
        );
        let error = ViewError::from(ViewCode::StaleToken);
        assert_eq!(error.f04_code(), ErrorCode::StaleWorkspace);
        assert_eq!(error.to_string(), "GEO_STALE_TOKEN");
    }
}
