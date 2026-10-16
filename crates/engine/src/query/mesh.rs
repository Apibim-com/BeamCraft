// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Mesh blobs as the snapshot stores them: exact BCMESH01 bytes identified by SHA-256.
//!
//! The engine does not trust a blob because the kernel produced it: every blob is re-inspected
//! (F06 section 6 header, caps, sizes, index ranges) and its digest recomputed before it is
//! admitted. Coordinates are not re-certified; the closed-shell checks stay with the kernel.

use crate::error::{ViewCode, ViewError};
use crate::limits::{MAX_BLOB_FACES, MAX_BLOB_TRIANGLES, MAX_BLOB_VERTICES, MAX_MESH_BLOB_BYTES};
use beamcraft_model::digest::{ContentDigest, Sha256};

/// One immutable mesh blob and its identity. The bytes are element-local (space 0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshEntry {
    digest: ContentDigest,
    bytes: Vec<u8>,
    vertex_count: u32,
    triangle_count: u32,
}

impl MeshEntry {
    /// Validates `bytes` structurally and checks that their SHA-256 equals `claimed`.
    pub fn from_blob(bytes: &[u8], claimed: &ContentDigest) -> Result<Self, ViewError> {
        let (vertex_count, triangle_count) = inspect_blob(bytes)?;
        let actual = Sha256::digest(bytes).map_err(|_| ViewError::new(ViewCode::BufferLimit))?;
        if actual != *claimed {
            return Err(ViewError::new(ViewCode::BlobInvalid));
        }
        Ok(Self {
            digest: actual,
            bytes: bytes.to_vec(),
            vertex_count,
            triangle_count,
        })
    }

    /// SHA-256 of the exact bytes.
    pub const fn digest(&self) -> ContentDigest {
        self.digest
    }

    /// The exact blob bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Vertex count from the validated header.
    pub const fn vertex_count(&self) -> u32 {
        self.vertex_count
    }

    /// Triangle count from the validated header.
    pub const fn triangle_count(&self) -> u32 {
        self.triangle_count
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    Some(u32::from_le_bytes(bytes.get(offset..end)?.try_into().ok()?))
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let end = offset.checked_add(2)?;
    Some(u16::from_le_bytes(bytes.get(offset..end)?.try_into().ok()?))
}

fn pad8(n: u64) -> Option<u64> {
    Some(n.checked_add(7)? & !7)
}

/// F06 section 6 structural check. Returns `(vertices, triangles)`.
///
/// Caps are compared before any size is computed; sizes use checked 64-bit arithmetic; the
/// supplied length must equal the computed length before any section is read.
pub(crate) fn inspect_blob(bytes: &[u8]) -> Result<(u32, u32), ViewError> {
    let bad = || ViewError::new(ViewCode::BlobInvalid);
    if bytes.len() > MAX_MESH_BLOB_BYTES {
        return Err(ViewError::new(ViewCode::BufferLimit));
    }
    if bytes.get(..8) != Some(b"BCMESH01".as_slice()) {
        return Err(bad());
    }
    let field = |offset: usize| read_u32(bytes, offset).ok_or_else(bad);
    let (version, v, t, k) = (field(8)?, field(12)?, field(16)?, field(20)?);
    let (space, reserved) = (field(24)?, field(28)?);
    if version != 1 || space != 0 || reserved != 0 || v == 0 || t == 0 || k == 0 {
        return Err(bad());
    }
    if v > MAX_BLOB_VERTICES || t > MAX_BLOB_TRIANGLES {
        return Err(ViewError::new(ViewCode::BufferLimit));
    }
    if k > MAX_BLOB_FACES {
        return Err(ViewError::new(ViewCode::TopologyLimit));
    }
    let positions = 24u64.checked_mul(u64::from(v)).ok_or_else(bad)?;
    let indices = pad8(12u64.checked_mul(u64::from(t)).ok_or_else(bad)?).ok_or_else(bad)?;
    let faces = pad8(2u64.checked_mul(u64::from(t)).ok_or_else(bad)?).ok_or_else(bad)?;
    let expected = 32u64
        .checked_add(positions)
        .and_then(|n| n.checked_add(indices))
        .and_then(|n| n.checked_add(faces))
        .ok_or_else(bad)?;
    if u64::try_from(bytes.len()).ok() != Some(expected) {
        return Err(bad());
    }
    let index_start = usize::try_from(32 + positions).map_err(|_| bad())?;
    let face_start = index_start
        .checked_add(usize::try_from(indices).map_err(|_| bad())?)
        .ok_or_else(bad)?;
    let index_count = usize::try_from(t)
        .map_err(|_| bad())?
        .checked_mul(3)
        .ok_or_else(bad)?;
    for i in 0..index_count {
        let offset = i
            .checked_mul(4)
            .and_then(|o| o.checked_add(index_start))
            .ok_or_else(bad)?;
        if read_u32(bytes, offset).ok_or_else(bad)? >= v {
            return Err(bad());
        }
    }
    for i in 0..usize::try_from(t).map_err(|_| bad())? {
        let offset = i
            .checked_mul(2)
            .and_then(|o| o.checked_add(face_start))
            .ok_or_else(bad)?;
        if u32::from(read_u16(bytes, offset).ok_or_else(bad)?) >= k {
            return Err(bad());
        }
    }
    Ok((v, t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use beamcraft_kernel::{RectangularPlate, rectangular_plate};

    fn plate_blob() -> Vec<u8> {
        rectangular_plate(RectangularPlate {
            width_mm: 100.0,
            length_mm: 80.0,
            thickness_mm: 10.0,
        })
        .unwrap()
        .mesh()
        .blob()
        .to_vec()
    }

    fn code(bytes: &[u8]) -> ViewCode {
        inspect_blob(bytes).unwrap_err().code()
    }

    fn put(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn accepts_the_kernel_plate_blob_and_verifies_its_digest() {
        let blob = plate_blob();
        assert_eq!(inspect_blob(&blob), Ok((8, 12)));
        let digest = Sha256::digest(&blob).unwrap();
        let entry = MeshEntry::from_blob(&blob, &digest).unwrap();
        assert_eq!(entry.digest(), digest);
        assert_eq!(entry.bytes(), blob.as_slice());
        assert_eq!((entry.vertex_count(), entry.triangle_count()), (8, 12));
        let mut wrong = *digest.as_bytes();
        wrong[0] ^= 1;
        assert_eq!(
            MeshEntry::from_blob(&blob, &ContentDigest::from_bytes(wrong))
                .unwrap_err()
                .code(),
            ViewCode::BlobInvalid
        );
    }

    #[test]
    fn rejects_structural_damage() {
        let good = plate_blob();
        assert_eq!(code(&[]), ViewCode::BlobInvalid);
        assert_eq!(code(&good[..391]), ViewCode::BlobInvalid);
        let mut longer = good.clone();
        longer.push(0);
        assert_eq!(code(&longer), ViewCode::BlobInvalid);
        let mut b = good.clone();
        b[0] = b'X';
        assert_eq!(code(&b), ViewCode::BlobInvalid);
        for (offset, value) in [(8, 2), (12, 0), (16, 0), (20, 0), (24, 1), (28, 1)] {
            let mut b = good.clone();
            put(&mut b, offset, value);
            assert_eq!(code(&b), ViewCode::BlobInvalid, "offset {offset}");
        }
        let mut b = good.clone();
        put(&mut b, 12, 9);
        assert_eq!(
            code(&b),
            ViewCode::BlobInvalid,
            "length disagrees with header"
        );
        let mut b = good.clone();
        let first_index = 32 + 24 * 8;
        put(&mut b, first_index, 8);
        assert_eq!(code(&b), ViewCode::BlobInvalid, "vertex index out of range");
        let mut b = good;
        let first_face = 32 + 24 * 8 + 144;
        b[first_face] = 6;
        assert_eq!(code(&b), ViewCode::BlobInvalid, "face index out of table");
    }

    #[test]
    fn caps_are_checked_before_sizes() {
        let mut b = plate_blob();
        put(&mut b, 12, MAX_BLOB_VERTICES + 1);
        assert_eq!(code(&b), ViewCode::BufferLimit);
        let mut b = plate_blob();
        put(&mut b, 16, MAX_BLOB_TRIANGLES + 1);
        assert_eq!(code(&b), ViewCode::BufferLimit);
        let mut b = plate_blob();
        put(&mut b, 20, MAX_BLOB_FACES + 1);
        assert_eq!(code(&b), ViewCode::TopologyLimit);
        assert_eq!(
            code(&vec![0u8; MAX_MESH_BLOB_BYTES + 1]),
            ViewCode::BufferLimit
        );
        // Exactly at the three caps the formula gives the F06 byte cap, and the size matches.
        let mut b = vec![0u8; MAX_MESH_BLOB_BYTES];
        b[..8].copy_from_slice(b"BCMESH01");
        put(&mut b, 8, 1);
        put(&mut b, 12, MAX_BLOB_VERTICES);
        put(&mut b, 16, MAX_BLOB_TRIANGLES);
        put(&mut b, 20, MAX_BLOB_FACES);
        assert_eq!(
            inspect_blob(&b),
            Ok((MAX_BLOB_VERTICES, MAX_BLOB_TRIANGLES))
        );
    }
}
