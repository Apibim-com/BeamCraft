// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Bounds of the view contract. A caller may lower a limit but never raise it above the maximum,
//! mirroring how F06 section 10 treats negotiated limits.

/// Elements in one snapshot. This is the F06 section 9 scan bound.
pub const MAX_ELEMENTS: usize = 65_536;
/// One mesh blob (F06 section 6 hard caps at 4,096 vertices and 8,192 triangles).
pub const MAX_MESH_BLOB_BYTES: usize = 213_024;
/// Vertices per mesh blob (F06 section 6).
pub const MAX_BLOB_VERTICES: u32 = 4_096;
/// Triangles per mesh blob (F06 section 6).
pub const MAX_BLOB_TRIANGLES: u32 = 8_192;
/// Face-table entries per mesh blob (F06 section 6).
pub const MAX_BLOB_FACES: u32 = 1_024;
/// Distinct mesh bytes retained by one snapshot (64 MiB).
pub const MAX_TOTAL_MESH_BYTES: usize = 64 * 1024 * 1024;
/// Triangles drawn by one projection; keeps every triangle index inside `u32`.
pub const MAX_PROJECTED_TRIANGLES: u32 = 4_194_304;
/// Elements named by one selection.
pub const MAX_SELECTION: usize = 65_536;
/// Entries of one `ListElements` page.
pub const MAX_PAGE_SIZE: usize = 1_024;
/// Snapshots retained for incremental deltas.
pub const MAX_RETAINED_SNAPSHOTS: usize = 8;

/// Effective limits of one snapshot, each at most its maximum above.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    max_elements: usize,
    max_total_mesh_bytes: usize,
    max_projected_triangles: u32,
    max_selection: usize,
}

impl Limits {
    /// The contract maxima.
    pub const MAXIMA: Self = Self {
        max_elements: MAX_ELEMENTS,
        max_total_mesh_bytes: MAX_TOTAL_MESH_BYTES,
        max_projected_triangles: MAX_PROJECTED_TRIANGLES,
        max_selection: MAX_SELECTION,
    };

    /// Lowers the maxima; a value above its maximum is clamped to it.
    pub const fn new(
        max_elements: usize,
        max_total_mesh_bytes: usize,
        max_projected_triangles: u32,
        max_selection: usize,
    ) -> Self {
        Self {
            max_elements: min_usize(max_elements, MAX_ELEMENTS),
            max_total_mesh_bytes: min_usize(max_total_mesh_bytes, MAX_TOTAL_MESH_BYTES),
            max_projected_triangles: if max_projected_triangles > MAX_PROJECTED_TRIANGLES {
                MAX_PROJECTED_TRIANGLES
            } else {
                max_projected_triangles
            },
            max_selection: min_usize(max_selection, MAX_SELECTION),
        }
    }

    /// Element cap.
    pub const fn max_elements(self) -> usize {
        self.max_elements
    }
    /// Cap on the distinct mesh bytes of a snapshot.
    pub const fn max_total_mesh_bytes(self) -> usize {
        self.max_total_mesh_bytes
    }
    /// Cap on the triangles of one projection.
    pub const fn max_projected_triangles(self) -> u32 {
        self.max_projected_triangles
    }
    /// Selection cap.
    pub const fn max_selection(self) -> usize {
        self.max_selection
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::MAXIMA
    }
}

const fn min_usize(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_clamp_to_the_maxima_and_may_be_lowered() {
        let high = Limits::new(usize::MAX, usize::MAX, u32::MAX, usize::MAX);
        assert_eq!(high, Limits::MAXIMA);
        assert_eq!(Limits::default(), Limits::MAXIMA);
        let low = Limits::new(2, 1_000, 24, 3);
        assert_eq!(low.max_elements(), 2);
        assert_eq!(low.max_total_mesh_bytes(), 1_000);
        assert_eq!(low.max_projected_triangles(), 24);
        assert_eq!(low.max_selection(), 3);
    }
}
