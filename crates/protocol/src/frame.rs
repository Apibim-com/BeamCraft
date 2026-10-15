// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The frame header, the binary attachment prefix and the bundle budget (F05 section 3).
//!
//! All multi-byte integers are unsigned and big-endian. Decoding is bounded: the fixed header is
//! validated against the limits before any payload-sized allocation, which is the caller's job and
//! must follow a successful decode. Nothing here scans for a magic or resynchronizes.

use core::fmt;

use crate::code::TransportCode;
use crate::limits::{
    BINARY_PREFIX_LEN, HEADER_LEN, Limits, MAGIC, MAX_ATTACHMENTS, MAXIMA, VERSION_MAJOR,
    VERSION_MINOR, VERSION_PATCH,
};

/// The kind byte of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    /// One UTF-8 JSON object (kind byte 1).
    Control,
    /// One opaque binary attachment (kind byte 2).
    Binary,
}

impl FrameKind {
    const fn byte(self) -> u8 {
        match self {
            Self::Control => 1,
            Self::Binary => 2,
        }
    }

    const fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            1 => Some(Self::Control),
            2 => Some(Self::Binary),
            _ => None,
        }
    }
}

/// Why a header, prefix or bundle charge was rejected. [`FrameError::code`] gives the F05 code.
///
/// Variants carry only small numbers taken from the input, never text, so they are safe to log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// Fewer bytes than the structure needs (`IPC_TRUNCATED`).
    Truncated,
    /// The first four bytes are not `BCWP` (`IPC_FRAME`); this is not a version probe.
    BadMagic,
    /// Recognized magic with another version (`IPC_VERSION`); carries the received version.
    Version {
        /// Received major version.
        major: u8,
        /// Received minor version.
        minor: u8,
        /// Received patch version.
        patch: u16,
    },
    /// Kind byte other than 1 or 2 (`IPC_FRAME`).
    BadKind(u8),
    /// A nonzero flags byte; compression and extensions are forbidden (`IPC_FRAME`).
    BadFlags(u8),
    /// A binary frame declares attachments (`IPC_FRAME`).
    BinaryAttachments(u16),
    /// A frame of the wrong kind where another was required (`IPC_FRAME`).
    WrongKind,
    /// A zero-length control payload (`IPC_FRAME`).
    EmptyPayload,
    /// A binary payload too short to hold the prefix and one byte (`IPC_FRAME`).
    PayloadTooSmall(u32),
    /// Nonzero reserved byte in a binary prefix (`IPC_FRAME`).
    Reserved(u8),
    /// More attachments than the negotiated or fixed limit (`IPC_LIMIT`).
    TooManyAttachments(u16),
    /// Payload length above its limit (`IPC_LIMIT`).
    PayloadTooLarge(u32),
    /// Attachment byte length above its limit (`IPC_LIMIT`).
    BufferTooLarge(u32),
    /// The bundle would exceed its byte budget (`IPC_LIMIT`).
    BundleTooLarge,
    /// Element type other than 1 (`IPC_BUFFER`).
    ElementType(u8),
    /// Attachment byte length of zero (`IPC_BUFFER`).
    EmptyBuffer,
    /// Element count differing from the byte length for type `u8` (`IPC_BUFFER`).
    ElementCount {
        /// Declared byte length.
        byte_length: u32,
        /// Declared element count.
        element_count: u32,
    },
    /// Descriptor index outside 0..8 (`IPC_BUFFER`).
    IndexOutOfRange(u16),
    /// Descriptor index differing from the expected position (`IPC_BUFFER`).
    IndexMismatch,
    /// Header payload length differing from 44 plus the prefix byte length (`IPC_BUFFER`).
    LengthMismatch,
}

impl FrameError {
    /// The F05 transport code for this rejection.
    pub const fn code(self) -> TransportCode {
        match self {
            Self::Truncated => TransportCode::Truncated,
            Self::Version { .. } => TransportCode::Version,
            Self::BadMagic
            | Self::BadKind(_)
            | Self::BadFlags(_)
            | Self::BinaryAttachments(_)
            | Self::WrongKind
            | Self::EmptyPayload
            | Self::PayloadTooSmall(_)
            | Self::Reserved(_) => TransportCode::Frame,
            Self::TooManyAttachments(_)
            | Self::PayloadTooLarge(_)
            | Self::BufferTooLarge(_)
            | Self::BundleTooLarge => TransportCode::Limit,
            Self::ElementType(_)
            | Self::EmptyBuffer
            | Self::ElementCount { .. }
            | Self::IndexOutOfRange(_)
            | Self::IndexMismatch
            | Self::LengthMismatch => TransportCode::Buffer,
        }
    }
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Version {
                major,
                minor,
                patch,
            } => {
                write!(
                    f,
                    "{}: received version {major}.{minor}.{patch}",
                    self.code()
                )
            }
            _ => write!(f, "{}: {self:?}", self.code()),
        }
    }
}

impl std::error::Error for FrameError {}

/// Reads `N` bytes at `at`, or `None` if the input is too short.
fn array<const N: usize>(bytes: &[u8], at: usize) -> Option<[u8; N]> {
    let end = at.checked_add(N)?;
    bytes.get(at..end)?.try_into().ok()
}

/// A limit never exceeds its contract maximum, even if a caller passes an unvalidated value.
fn capped(limit: u32, maximum: u32) -> u64 {
    u64::from(limit.min(maximum))
}

/// A validated 16-byte frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    kind: FrameKind,
    attachment_count: u16,
    payload_len: u32,
}

impl FrameHeader {
    /// Validates and builds a header.
    ///
    /// A control payload is 1..=`control_bytes` bytes with at most `attachments` attachments. A
    /// binary payload is `44 + 1` to `44 + buffer_bytes` bytes and declares no attachments.
    ///
    /// # Errors
    ///
    /// The [`FrameError`] naming the violated rule.
    pub fn new(
        kind: FrameKind,
        attachment_count: u16,
        payload_len: u32,
        limits: &Limits,
    ) -> Result<Self, FrameError> {
        match kind {
            FrameKind::Control => {
                let allowed =
                    capped(limits.attachments, MAXIMA.attachments).min(u64::from(MAX_ATTACHMENTS));
                if u64::from(attachment_count) > allowed {
                    return Err(FrameError::TooManyAttachments(attachment_count));
                }
                if payload_len == 0 {
                    return Err(FrameError::EmptyPayload);
                }
                if u64::from(payload_len) > capped(limits.control_bytes, MAXIMA.control_bytes) {
                    return Err(FrameError::PayloadTooLarge(payload_len));
                }
            }
            FrameKind::Binary => {
                if attachment_count != 0 {
                    return Err(FrameError::BinaryAttachments(attachment_count));
                }
                let prefix = BINARY_PREFIX_LEN as u64;
                let len = u64::from(payload_len);
                if len <= prefix {
                    return Err(FrameError::PayloadTooSmall(payload_len));
                }
                let most = prefix + capped(limits.buffer_bytes, MAXIMA.buffer_bytes);
                if len > most {
                    return Err(FrameError::PayloadTooLarge(payload_len));
                }
            }
        }
        Ok(Self {
            kind,
            attachment_count,
            payload_len,
        })
    }

    /// Decodes the first 16 bytes of `bytes`; later bytes are not examined.
    ///
    /// Checks run in this order: length, magic, exact version, kind, flags, then the rules of
    /// [`FrameHeader::new`]. The version is checked before the payload is considered, so an
    /// unsupported version never leads to payload parsing.
    ///
    /// # Errors
    ///
    /// The [`FrameError`] naming the violated rule; [`FrameError::code`] gives the F05 code.
    pub fn decode(bytes: &[u8], limits: &Limits) -> Result<Self, FrameError> {
        let head: [u8; HEADER_LEN] = array(bytes, 0).ok_or(FrameError::Truncated)?;
        let [
            m0,
            m1,
            m2,
            m3,
            major,
            minor,
            p0,
            p1,
            kind,
            flags,
            c0,
            c1,
            l0,
            l1,
            l2,
            l3,
        ] = head;
        if [m0, m1, m2, m3] != MAGIC {
            return Err(FrameError::BadMagic);
        }
        let patch = u16::from_be_bytes([p0, p1]);
        if (major, minor, patch) != (VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH) {
            return Err(FrameError::Version {
                major,
                minor,
                patch,
            });
        }
        let kind = FrameKind::from_byte(kind).ok_or(FrameError::BadKind(kind))?;
        if flags != 0 {
            return Err(FrameError::BadFlags(flags));
        }
        Self::new(
            kind,
            u16::from_be_bytes([c0, c1]),
            u32::from_be_bytes([l0, l1, l2, l3]),
            limits,
        )
    }

    /// Encodes the header. The result always decodes under the maximum limits or tighter ones
    /// that admit these values.
    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let [m0, m1, m2, m3] = MAGIC;
        let [p0, p1] = VERSION_PATCH.to_be_bytes();
        let [c0, c1] = self.attachment_count.to_be_bytes();
        let [l0, l1, l2, l3] = self.payload_len.to_be_bytes();
        [
            m0,
            m1,
            m2,
            m3,
            VERSION_MAJOR,
            VERSION_MINOR,
            p0,
            p1,
            self.kind.byte(),
            0,
            c0,
            c1,
            l0,
            l1,
            l2,
            l3,
        ]
    }

    /// The frame kind.
    pub const fn kind(&self) -> FrameKind {
        self.kind
    }

    /// The declared attachment count (zero for a binary frame).
    pub const fn attachment_count(&self) -> u16 {
        self.attachment_count
    }

    /// The payload length, excluding this header.
    pub const fn payload_len(&self) -> u32 {
        self.payload_len
    }
}

/// The validated 44-byte prefix of a binary payload.
///
/// The prefix carries the claimed SHA-256 of the bytes that follow. Computing and comparing that
/// digest is the receiver's streaming job and needs a SHA-256 implementation, which this crate
/// does not contain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryPrefix {
    index: u16,
    byte_length: u32,
    sha256: [u8; 32],
}

impl BinaryPrefix {
    /// Validates and builds a prefix for element type `u8`.
    ///
    /// # Errors
    ///
    /// [`FrameError::IndexOutOfRange`] for an index of 8 or more, [`FrameError::EmptyBuffer`]
    /// for a zero length and [`FrameError::BufferTooLarge`] above `buffer_bytes`.
    pub fn new(
        index: u16,
        byte_length: u32,
        sha256: [u8; 32],
        limits: &Limits,
    ) -> Result<Self, FrameError> {
        if index >= MAX_ATTACHMENTS {
            return Err(FrameError::IndexOutOfRange(index));
        }
        if byte_length == 0 {
            return Err(FrameError::EmptyBuffer);
        }
        if u64::from(byte_length) > capped(limits.buffer_bytes, MAXIMA.buffer_bytes) {
            return Err(FrameError::BufferTooLarge(byte_length));
        }
        Ok(Self {
            index,
            byte_length,
            sha256,
        })
    }

    /// Decodes the first 44 bytes of a binary payload; later bytes are not examined.
    ///
    /// # Errors
    ///
    /// [`FrameError::Truncated`] below 44 bytes, [`FrameError::ElementType`] for a type other
    /// than 1, [`FrameError::Reserved`] for a nonzero reserved byte, [`FrameError::ElementCount`]
    /// when the count differs from the length, then the rules of [`BinaryPrefix::new`].
    pub fn decode(bytes: &[u8], limits: &Limits) -> Result<Self, FrameError> {
        let index = array::<2>(bytes, 0).ok_or(FrameError::Truncated)?;
        let [element_type] = array::<1>(bytes, 2).ok_or(FrameError::Truncated)?;
        let [reserved] = array::<1>(bytes, 3).ok_or(FrameError::Truncated)?;
        let length = array::<4>(bytes, 4).ok_or(FrameError::Truncated)?;
        let count = array::<4>(bytes, 8).ok_or(FrameError::Truncated)?;
        let sha256 = array::<32>(bytes, 12).ok_or(FrameError::Truncated)?;
        if element_type != 1 {
            return Err(FrameError::ElementType(element_type));
        }
        if reserved != 0 {
            return Err(FrameError::Reserved(reserved));
        }
        let (byte_length, element_count) = (u32::from_be_bytes(length), u32::from_be_bytes(count));
        if byte_length != element_count {
            return Err(FrameError::ElementCount {
                byte_length,
                element_count,
            });
        }
        Self::new(u16::from_be_bytes(index), byte_length, sha256, limits)
    }

    /// Encodes the prefix.
    pub fn encode(&self) -> [u8; BINARY_PREFIX_LEN] {
        let mut out = [0u8; BINARY_PREFIX_LEN];
        let [i0, i1] = self.index.to_be_bytes();
        let len = self.byte_length.to_be_bytes();
        let head = [
            i0, i1, 1, 0, len[0], len[1], len[2], len[3], len[0], len[1], len[2], len[3],
        ];
        for (slot, byte) in out.iter_mut().zip(head.iter().chain(self.sha256.iter())) {
            *slot = *byte;
        }
        out
    }

    /// Checks this prefix against its frame header and the descriptor position it must occupy.
    ///
    /// # Errors
    ///
    /// [`FrameError::WrongKind`] for a control header, [`FrameError::IndexMismatch`] when the
    /// index is not `expected_index` and [`FrameError::LengthMismatch`] unless the header
    /// payload length equals 44 plus the prefix byte length.
    pub fn check_against(
        &self,
        header: &FrameHeader,
        expected_index: u16,
    ) -> Result<(), FrameError> {
        if header.kind() != FrameKind::Binary {
            return Err(FrameError::WrongKind);
        }
        if self.index != expected_index {
            return Err(FrameError::IndexMismatch);
        }
        let expected = (BINARY_PREFIX_LEN as u64).checked_add(u64::from(self.byte_length));
        if expected != Some(u64::from(header.payload_len())) {
            return Err(FrameError::LengthMismatch);
        }
        Ok(())
    }

    /// The descriptor index.
    pub const fn index(&self) -> u16 {
        self.index
    }

    /// The number of data bytes that follow the prefix.
    pub const fn byte_length(&self) -> u32 {
        self.byte_length
    }

    /// The claimed raw SHA-256 of the data bytes.
    pub const fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }
}

/// Running size of one bundle: every header, prefix and payload counts (F05 section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BundleBudget {
    used: u64,
    limit: u64,
}

impl BundleBudget {
    /// An empty budget of `bundle_bytes`, never more than the contract maximum.
    pub fn new(limits: &Limits) -> Self {
        Self {
            used: 0,
            limit: capped(limits.bundle_bytes, MAXIMA.bundle_bytes),
        }
    }

    /// Adds `16 + payload_len` for one frame. A rejected charge leaves the budget unchanged.
    ///
    /// # Errors
    ///
    /// [`FrameError::BundleTooLarge`] if the total would exceed the budget.
    pub fn charge(&mut self, header: &FrameHeader) -> Result<(), FrameError> {
        let total = (HEADER_LEN as u64)
            .checked_add(u64::from(header.payload_len()))
            .and_then(|frame| self.used.checked_add(frame))
            .filter(|total| *total <= self.limit)
            .ok_or(FrameError::BundleTooLarge)?;
        self.used = total;
        Ok(())
    }

    /// Bytes charged so far.
    pub const fn used(&self) -> u64 {
        self.used
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX: Limits = MAXIMA;

    fn header(kind: FrameKind, count: u16, len: u32) -> FrameHeader {
        FrameHeader::new(kind, count, len, &MAX).unwrap()
    }

    #[test]
    fn control_header_has_the_documented_bytes() {
        let bytes = header(FrameKind::Control, 3, 0x0001_0203).encode();
        assert_eq!(
            bytes,
            [
                0x42, 0x43, 0x57, 0x50, 0, 1, 0, 0, 1, 0, 0, 3, 0x00, 0x01, 0x02, 0x03
            ]
        );
        assert_eq!(
            FrameHeader::decode(&bytes, &MAX),
            Ok(header(FrameKind::Control, 3, 66_051))
        );
    }

    #[test]
    fn extra_bytes_after_the_header_are_not_examined() {
        let mut bytes = header(FrameKind::Control, 0, 10).encode().to_vec();
        bytes.extend_from_slice(&[0xFF; 100]);
        assert_eq!(
            FrameHeader::decode(&bytes, &MAX),
            Ok(header(FrameKind::Control, 0, 10))
        );
    }

    #[test]
    fn every_prefix_of_a_valid_header_is_truncated() {
        let bytes = header(FrameKind::Control, 0, 10).encode();
        for n in 0..HEADER_LEN {
            let err = FrameHeader::decode(&bytes[..n], &MAX).unwrap_err();
            assert_eq!(err, FrameError::Truncated, "{n}");
            assert_eq!(err.code(), TransportCode::Truncated);
        }
    }

    #[test]
    fn bad_magic_is_a_frame_error_not_a_version_probe() {
        for i in 0..4 {
            let mut bytes = header(FrameKind::Control, 0, 10).encode();
            bytes[i] ^= 0x20;
            let err = FrameHeader::decode(&bytes, &MAX).unwrap_err();
            assert_eq!(err, FrameError::BadMagic);
            assert_eq!(err.code(), TransportCode::Frame);
        }
    }

    #[test]
    fn exact_version_is_required_including_patch() {
        for (at, value) in [(4, 1u8), (5, 0), (5, 2), (6, 1), (7, 1)] {
            let mut bytes = header(FrameKind::Control, 0, 10).encode();
            bytes[at] = value;
            let err = FrameHeader::decode(&bytes, &MAX).unwrap_err();
            assert!(
                matches!(err, FrameError::Version { .. }),
                "{at}/{value}: {err:?}"
            );
            assert_eq!(err.code(), TransportCode::Version);
        }
        let mut bytes = header(FrameKind::Control, 0, 10).encode();
        bytes[4] = 9;
        bytes[5] = 8;
        bytes[7] = 7;
        assert_eq!(
            FrameHeader::decode(&bytes, &MAX),
            Err(FrameError::Version {
                major: 9,
                minor: 8,
                patch: 7
            })
        );
    }

    #[test]
    fn version_is_judged_before_the_payload_rules() {
        // A wrong version with an absurd length must report the version, not the length.
        let mut bytes = header(FrameKind::Control, 0, 10).encode();
        bytes[5] = 2;
        bytes[12..16].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(
            FrameHeader::decode(&bytes, &MAX).unwrap_err().code(),
            TransportCode::Version
        );
    }

    #[test]
    fn unknown_kind_and_flags_are_frame_errors() {
        for kind in [0u8, 3, 255] {
            let mut bytes = header(FrameKind::Control, 0, 10).encode();
            bytes[8] = kind;
            assert_eq!(
                FrameHeader::decode(&bytes, &MAX),
                Err(FrameError::BadKind(kind))
            );
        }
        for flags in [1u8, 0x80, 255] {
            let mut bytes = header(FrameKind::Control, 0, 10).encode();
            bytes[9] = flags;
            let err = FrameHeader::decode(&bytes, &MAX).unwrap_err();
            assert_eq!(err, FrameError::BadFlags(flags));
            assert_eq!(err.code(), TransportCode::Frame);
        }
    }

    #[test]
    fn control_payload_bounds_are_one_to_one_mib() {
        assert_eq!(
            FrameHeader::new(FrameKind::Control, 0, 0, &MAX),
            Err(FrameError::EmptyPayload)
        );
        assert!(FrameHeader::new(FrameKind::Control, 0, 1, &MAX).is_ok());
        assert!(FrameHeader::new(FrameKind::Control, 0, 1_048_576, &MAX).is_ok());
        let err = FrameHeader::new(FrameKind::Control, 0, 1_048_577, &MAX).unwrap_err();
        assert_eq!(err, FrameError::PayloadTooLarge(1_048_577));
        assert_eq!(err.code(), TransportCode::Limit);
        assert!(FrameHeader::new(FrameKind::Control, 0, u32::MAX, &MAX).is_err());
    }

    #[test]
    fn attachment_count_is_zero_to_eight_on_control_and_zero_on_binary() {
        assert!(FrameHeader::new(FrameKind::Control, 8, 10, &MAX).is_ok());
        let err = FrameHeader::new(FrameKind::Control, 9, 10, &MAX).unwrap_err();
        assert_eq!(err, FrameError::TooManyAttachments(9));
        assert_eq!(err.code(), TransportCode::Limit);
        assert!(FrameHeader::new(FrameKind::Control, u16::MAX, 10, &MAX).is_err());
        let err = FrameHeader::new(FrameKind::Binary, 1, 100, &MAX).unwrap_err();
        assert_eq!(err, FrameError::BinaryAttachments(1));
        assert_eq!(err.code(), TransportCode::Frame);
    }

    #[test]
    fn binary_payload_bounds_are_prefix_plus_one_to_sixteen_mib() {
        for len in [0u32, 1, 43, 44] {
            let err = FrameHeader::new(FrameKind::Binary, 0, len, &MAX).unwrap_err();
            assert_eq!(err, FrameError::PayloadTooSmall(len));
            assert_eq!(err.code(), TransportCode::Frame);
        }
        assert!(FrameHeader::new(FrameKind::Binary, 0, 45, &MAX).is_ok());
        assert!(FrameHeader::new(FrameKind::Binary, 0, 16_777_216 + 44, &MAX).is_ok());
        let err = FrameHeader::new(FrameKind::Binary, 0, 16_777_216 + 45, &MAX).unwrap_err();
        assert_eq!(err.code(), TransportCode::Limit);
    }

    #[test]
    fn negotiated_limits_tighten_but_unvalidated_limits_never_widen() {
        let tight = Limits {
            control_bytes: 100,
            attachments: 2,
            buffer_bytes: 10,
            ..MAX
        };
        assert!(FrameHeader::new(FrameKind::Control, 2, 100, &tight).is_ok());
        assert!(FrameHeader::new(FrameKind::Control, 3, 100, &tight).is_err());
        assert!(FrameHeader::new(FrameKind::Control, 2, 101, &tight).is_err());
        assert!(FrameHeader::new(FrameKind::Binary, 0, 54, &tight).is_ok());
        assert!(FrameHeader::new(FrameKind::Binary, 0, 55, &tight).is_err());

        let wide = Limits::from_values([u32::MAX; 14]);
        assert!(FrameHeader::new(FrameKind::Control, 0, 1_048_577, &wide).is_err());
        assert!(FrameHeader::new(FrameKind::Control, 9, 10, &wide).is_err());
        assert!(FrameHeader::new(FrameKind::Binary, 0, 16_777_216 + 45, &wide).is_err());
        let mut budget = BundleBudget::new(&wide);
        let big = header(FrameKind::Binary, 0, 16_777_216 + 44);
        assert!(budget.charge(&big).is_ok());
        assert!(budget.charge(&big).is_err());
    }

    #[test]
    fn decoding_never_panics_on_arbitrary_bytes() {
        let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
        for _ in 0..2000 {
            let mut bytes = [0u8; 20];
            for b in &mut bytes {
                seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                *b = (seed >> 56) as u8;
            }
            for n in 0..bytes.len() {
                let _ = FrameHeader::decode(&bytes[..n], &MAX);
                let _ = BinaryPrefix::decode(&bytes[..n], &MAX);
            }
        }
    }

    fn prefix(index: u16, len: u32) -> BinaryPrefix {
        BinaryPrefix::new(index, len, [7; 32], &MAX).unwrap()
    }

    #[test]
    fn binary_prefix_has_the_documented_layout_and_round_trips() {
        let bytes = prefix(2, 0x0001_0203).encode();
        assert_eq!(&bytes[..12], &[0, 2, 1, 0, 0, 1, 2, 3, 0, 1, 2, 3]);
        assert_eq!(&bytes[12..], &[7u8; 32][..]);
        assert_eq!(bytes.len(), 44);
        let decoded = BinaryPrefix::decode(
            &bytes,
            &Limits {
                buffer_bytes: u32::MAX,
                ..MAX
            },
        );
        assert_eq!(decoded, Ok(prefix(2, 0x0001_0203)));
        let p = decoded.unwrap();
        assert_eq!(
            (p.index(), p.byte_length(), p.sha256()),
            (2, 0x0001_0203, &[7u8; 32])
        );
    }

    #[test]
    fn every_short_prefix_is_truncated() {
        let bytes = prefix(0, 16).encode();
        for n in 0..BINARY_PREFIX_LEN {
            assert_eq!(
                BinaryPrefix::decode(&bytes[..n], &MAX),
                Err(FrameError::Truncated),
                "{n}"
            );
        }
        assert!(BinaryPrefix::decode(&bytes, &MAX).is_ok());
    }

    #[test]
    fn prefix_rejects_each_malformed_field() {
        let good = prefix(1, 16).encode();
        let mutate = |f: &dyn Fn(&mut [u8; 44])| {
            let mut b = good;
            f(&mut b);
            BinaryPrefix::decode(&b, &MAX)
        };
        assert_eq!(mutate(&|b| b[2] = 2), Err(FrameError::ElementType(2)));
        assert_eq!(mutate(&|b| b[2] = 0), Err(FrameError::ElementType(0)));
        assert_eq!(mutate(&|b| b[3] = 1), Err(FrameError::Reserved(1)));
        assert_eq!(
            mutate(&|b| b[11] = 17),
            Err(FrameError::ElementCount {
                byte_length: 16,
                element_count: 17
            })
        );
        assert_eq!(mutate(&|b| b[1] = 8), Err(FrameError::IndexOutOfRange(8)));
        assert_eq!(mutate(&|b| b[0] = 1), Err(FrameError::IndexOutOfRange(257)));
        // zero length (count also zero)
        assert_eq!(
            mutate(&|b| {
                b[4..12].fill(0);
            }),
            Err(FrameError::EmptyBuffer)
        );
        // 16 MiB + 1 on both fields
        let over = mutate(&|b| {
            b[4..8].copy_from_slice(&16_777_217u32.to_be_bytes());
            b[8..12].copy_from_slice(&16_777_217u32.to_be_bytes());
        });
        assert_eq!(over, Err(FrameError::BufferTooLarge(16_777_217)));
        // exactly 16 MiB is accepted
        assert!(BinaryPrefix::new(0, 16_777_216, [0; 32], &MAX).is_ok());
    }

    #[test]
    fn prefix_error_codes_follow_the_contract() {
        assert_eq!(FrameError::ElementType(2).code(), TransportCode::Buffer);
        assert_eq!(FrameError::Reserved(1).code(), TransportCode::Frame);
        assert_eq!(FrameError::BufferTooLarge(1).code(), TransportCode::Limit);
        assert_eq!(FrameError::LengthMismatch.code(), TransportCode::Buffer);
        assert_eq!(FrameError::IndexMismatch.code(), TransportCode::Buffer);
    }

    #[test]
    fn prefix_must_agree_with_header_and_position() {
        let p = prefix(1, 100);
        let ok = header(FrameKind::Binary, 0, 144);
        assert_eq!(p.check_against(&ok, 1), Ok(()));
        assert_eq!(p.check_against(&ok, 0), Err(FrameError::IndexMismatch));
        let short = header(FrameKind::Binary, 0, 143);
        assert_eq!(p.check_against(&short, 1), Err(FrameError::LengthMismatch));
        let long = header(FrameKind::Binary, 0, 145);
        assert_eq!(p.check_against(&long, 1), Err(FrameError::LengthMismatch));
        let control = header(FrameKind::Control, 0, 144);
        assert_eq!(p.check_against(&control, 1), Err(FrameError::WrongKind));
    }

    #[test]
    fn bundle_budget_allows_exactly_thirty_two_mib() {
        let mut budget = BundleBudget::new(&MAX);
        let big = header(FrameKind::Binary, 0, 16_777_216 + 44);
        assert_eq!(budget.charge(&big), Ok(()));
        assert_eq!(budget.used(), 16_777_216 + 44 + 16);
        // Remaining: 33_554_432 - 16_777_276 = 16_777_156 = 16 + 16_777_140.
        let fits = header(FrameKind::Binary, 0, 16_777_140);
        let over = header(FrameKind::Binary, 0, 16_777_141);
        let before = budget;
        assert_eq!(budget.charge(&over), Err(FrameError::BundleTooLarge));
        assert_eq!(
            budget, before,
            "a rejected charge must not change the budget"
        );
        assert_eq!(budget.charge(&fits), Ok(()));
        assert_eq!(budget.used(), 33_554_432);
        assert_eq!(
            budget.charge(&header(FrameKind::Control, 0, 1)),
            Err(FrameError::BundleTooLarge)
        );
        assert_eq!(FrameError::BundleTooLarge.code(), TransportCode::Limit);
    }

    #[test]
    fn display_does_not_echo_more_than_numbers() {
        let text = FrameError::Version {
            major: 1,
            minor: 2,
            patch: 3,
        }
        .to_string();
        assert_eq!(text, "IPC_VERSION: received version 1.2.3");
        assert!(FrameError::BadMagic.to_string().starts_with("IPC_FRAME"));
    }
}
