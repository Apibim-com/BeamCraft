// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! U14: the compaction floor of the head log.
//!
//! Compaction removes the oldest head-log records and keeps the newest ones under their original
//! sequence numbers. The lowest record that remains is the **floor**; its manifest still names a
//! base revision that no longer exists. A small marker file, `revisions/floor`, states the floor:
//! its sequence, revision id and manifest digest. With a marker, the log is the contiguous run
//! `floor..=head`, the record at the floor must equal the marker, and the floor's missing base is
//! accepted because the marker anchors it. Without a marker the floor is 1 and every rule of U06
//! applies unchanged. Records below the floor that are still present (an interrupted compaction)
//! are leftovers: they are ignored, never read, and removed by the next compaction.
//!
//! The marker is installed with one atomic rename; that step is the commit point of a compaction.

use std::path::Path;

use super::RevisionError;
use super::read::RevisionStore;
use super::root::{self, MAGIC, RootRecord};
use crate::storage::StorageError;

/// Name of the marker file in the head-log directory.
pub(super) const FILE: &str = "floor";
const MARKER_MAGIC: &str = "beamcraft-history-floor 1";
/// Temporary-name prefix of a staged marker; the `.tmp-` start makes readers ignore it.
const STAGE_PREFIX: &str = ".tmp-floor-";
/// Largest marker file the reader accepts.
const MAX_MARKER_BYTES: usize = 512;

/// The verified extent of a head log: lowest and highest record sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Span {
    /// Lowest readable sequence (1 when the log was never compacted).
    pub(crate) floor: u64,
    /// The head sequence.
    pub(crate) head: u64,
}

fn encode_marker(record: &RootRecord) -> Vec<u8> {
    let encoded = record.encode();
    let rest = encoded.strip_prefix(MAGIC.as_bytes()).unwrap_or_default();
    let mut bytes = MARKER_MAGIC.as_bytes().to_vec();
    bytes.extend_from_slice(rest);
    bytes
}

fn parse_marker(bytes: &[u8]) -> Result<RootRecord, RevisionError> {
    let rest = bytes
        .strip_prefix(MARKER_MAGIC.as_bytes())
        .ok_or(RevisionError::TornRoot)?;
    let mut record_bytes = MAGIC.as_bytes().to_vec();
    record_bytes.extend_from_slice(rest);
    let record = RootRecord::parse(&record_bytes)?;
    // A floor of 1 is "never compacted" and is never written as a marker.
    if record.sequence() < 2 {
        return Err(RevisionError::TornRoot);
    }
    Ok(record)
}

impl RevisionStore {
    /// The floor marker of a head-log directory, if the log was compacted.
    pub(crate) fn floor_marker(
        &self,
        directory: &Path,
    ) -> Result<Option<RootRecord>, RevisionError> {
        match self
            .blobs
            .read_namespace_exact(directory, FILE, MAX_MARKER_BYTES)
        {
            Ok(Some(bytes)) => parse_marker(&bytes).map(Some),
            Ok(None) => Ok(None),
            Err(StorageError::LimitExceeded) => Err(RevisionError::TornRoot),
            Err(error) => Err(RevisionError::Storage(error)),
        }
    }

    /// The lowest readable sequence of the log: the marker's, or 1.
    pub(crate) fn floor_of(&self, directory: &Path) -> Result<u64, RevisionError> {
        Ok(self
            .floor_marker(directory)?
            .map_or(1, |marker| marker.sequence()))
    }

    /// Verifies the shape of the head log and returns its extent: `None` for an empty log,
    /// otherwise the floor and head with every sequence in between present exactly once.
    /// Records below the floor are ignored; a record missing inside the run, a marker without
    /// the record it names, or an unknown file name is [`RevisionError::TornRoot`]. Does not open
    /// the records.
    pub(crate) fn span(&self, directory: &Path) -> Result<Option<Span>, RevisionError> {
        let listing = root::list(directory)?;
        let marker = if listing.marker {
            self.floor_marker(directory)?
        } else {
            None
        };
        let floor = marker.as_ref().map_or(1, RootRecord::sequence);
        let run: Vec<u64> = listing
            .sequences
            .into_iter()
            .filter(|sequence| *sequence >= floor)
            .collect();
        let Some(head) = run.last().copied() else {
            return if marker.is_some() {
                Err(RevisionError::TornRoot)
            } else {
                Ok(None)
            };
        };
        let expected = head
            .checked_sub(floor)
            .and_then(|n| n.checked_add(1))
            .and_then(|n| usize::try_from(n).ok());
        if expected != Some(run.len()) {
            return Err(RevisionError::TornRoot);
        }
        // The marker and the record at the floor are one statement; they must be the same.
        if let Some(marker) = &marker
            && self.load_record(directory, floor)? != *marker
        {
            return Err(RevisionError::RootMismatch);
        }
        Ok(Some(Span { floor, head }))
    }

    /// The lowest sequence [`Self::info_at`] and [`Self::open_at`] can read: 1 until the store is
    /// compacted by [`crate::compaction`], then the compaction floor. `Ok(None)` when nothing is
    /// published.
    pub fn first_sequence(&self) -> Result<Option<u64>, RevisionError> {
        match self.head_dir()? {
            Some(directory) => Ok(self.span(&directory)?.map(|span| span.floor)),
            None => Ok(None),
        }
    }

    /// Writes the marker for `record` to a temporary file of the head-log directory, synced, and
    /// returns its name. Nothing is visible to readers yet.
    pub(crate) fn stage_floor(
        &self,
        directory: &Path,
        record: &RootRecord,
    ) -> Result<String, RevisionError> {
        Ok(self
            .blobs
            .stage_namespace_file(directory, STAGE_PREFIX, &encode_marker(record))?)
    }

    /// Installs a staged marker with one atomic rename. This is the commit point of a
    /// compaction: before it the log is the old one, after it the new one.
    pub(crate) fn install_floor(
        &self,
        directory: &Path,
        staged: &str,
    ) -> Result<(), RevisionError> {
        Ok(self.blobs.install_over(directory, staged, FILE)?)
    }

    /// Sequences of the records below the floor that are still on disk (leftovers of a compaction
    /// that was interrupted after its commit point), ascending.
    pub(crate) fn records_below(
        &self,
        directory: &Path,
        floor: u64,
    ) -> Result<Vec<u64>, RevisionError> {
        Ok(root::list(directory)?
            .sequences
            .into_iter()
            .filter(|sequence| *sequence < floor)
            .collect())
    }

    /// Removes one head-log record file by sequence. Only compaction calls this, for a sequence
    /// below the floor.
    pub(crate) fn remove_record(
        &self,
        directory: &Path,
        sequence: u64,
    ) -> Result<(), RevisionError> {
        self.blobs
            .remove_namespace_file(directory, &root::record_name(sequence))?;
        Ok(())
    }
}
