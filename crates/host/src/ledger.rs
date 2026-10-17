// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! U07: the durable outcome ledger and crash recovery of the revision commit (F05 sections 7
//! and 8, F04 section 8).
//!
//! The ledger is one append-only file, `ledger/outcomes.log`, of checksummed text lines (see
//! [`Record`]). [`RevisionStore::commit_with`] writes an [`Intent`] and syncs it **before** it
//! touches the root, switches the root, then writes a `Committed` record. The head log of
//! [`crate::revision`] stays the only authority about what the root is; the ledger records what
//! was attempted and what was proven, so a caller that lost its connection, or a host that
//! crashed, can ask what happened to an operation id.
//!
//! # Commit order and crash points
//!
//! ```text
//! validate, replay, base      -- nothing written
//! intent appended + synced    -- ledger only; the root is unchanged
//! blobs verified, manifest blob stored, record staged and synced
//! root switch (hard link)     -- the one atomic step
//! committed appended + synced
//! ```
//!
//! A crash at any point leaves the root at the complete previous revision or the complete new
//! one. The ledger then shows nothing, an intent without outcome, or the outcome.
//! [`RevisionStore::recover`] closes every unresolved intent from the root: if the head log
//! holds the intent's revision at its sequence it appends `Committed`, otherwise `Aborted`. It
//! never trusts the ledger over the root and never the other way round.
//!
//! # QueryOutcome
//!
//! [`RevisionStore::query_outcome`] answers from the ledger, cross-checked against the root:
//! [`Outcome::Committed`], [`Outcome::Aborted`], [`Outcome::NotFound`] or [`Outcome::Unknown`]
//! (`OUTCOME_UNKNOWN`). It never fabricates acceptance and never guesses "not applied": an
//! intent without outcome that the root cannot prove is `Unknown`, and an operation absent from
//! a ledger that began after revisions already existed is `Unknown` too, because those
//! revisions were never journaled. `NotFound` is returned only when the ledger covers the whole
//! history of the store and holds no record of the operation: since no switch can happen
//! without a synced intent, the operation never reached the root.
//!
//! # Recovery, idempotence and concurrency
//!
//! Recovery only appends records that restate what the root proves, so running it twice
//! changes nothing and a second run reports nothing to do. There is no lock file: a crash
//! cannot leave anything that blocks later writers. Concurrent recoveries may both append the
//! same closing record; readers treat a repeated identical record as one, and a conflicting
//! `Committed` record is [`LedgerError::Inconsistent`]. Recovery cannot tell a crashed attempt
//! from one that is still running, so [`RecoveryPolicy`] leaves an intent younger than
//! `min_age_secs` alone (reported as in flight, answered `Unknown`) and removes only temporary
//! files older than that. `quiescent()` (age zero) asserts that no commit is running on the
//! store; with a live committer it can abort an attempt that is about to switch. The
//! committer re-checks, just before the switch, that recovery did not close its intent, which
//! narrows that window to the time between the re-check and the link but does not remove it.
//! A wrong clock can only make recovery act early or late, never accept something unproven.
//!
//! # What is promised
//!
//! * The intent is on stable storage (file `sync_all`) before the root switch; the `Committed`
//!   or `Aborted` record is synced when written.
//! * A torn, truncated, garbage or flipped record is ignored and counted
//!   ([`LedgerDiagnostics`]), never acted on; the next append starts on a fresh line.
//! * After a crash, [`RevisionStore::open`] shows a complete revision, and recovery makes the
//!   ledger agree with the root.
//!
//! # What is not promised
//!
//! * **Directory-entry crash durability.** Files are synced, directories are not (portable safe
//!   `std` cannot). After power loss the ledger file or the hard link of the root record may be
//!   missing although a commit was reported. The root is still a complete earlier revision; a
//!   ledger record that outlives its root record is reported as a disagreement and answered
//!   with an integrity error instead of being trusted.
//! * Protection from a hostile writer to the store directory: the per-record SHA-256 detects
//!   accidents, it does not authenticate the writer.
//! * Ledger rotation or compaction: the file is capped ([`MAX_LEDGER_BYTES`]); at the cap
//!   commits fail with `LIMIT_EXCEEDED` before they write anything.
//! * Garbage collection: recovery removes only abandoned temporary files and *reports*
//!   unreferenced blobs. It deletes no blob, because drafts, personal workspaces and retained
//!   revisions (not yet implemented) may still need them.

mod record;
mod recover;
mod state;

use std::fmt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use beamcraft_model::{ErrorCode, digest::ContentDigest};

use crate::revision::{RevisionStore, Uuid7};
use crate::storage::{BlobStore, StorageError};

pub use record::{AbortReason, CommittedRecord, Intent, MAX_RECORD_BYTES, Record};
pub use recover::{RecoveryPolicy, RecoveryReport, Unreferenced};
pub use state::{LedgerDiagnostics, OpStatus, Scan};

/// Largest ledger file the reader and writer accept.
pub const MAX_LEDGER_BYTES: u64 = 16_777_216;
/// Most lines (valid or not) one ledger read processes.
pub const MAX_LEDGER_RECORDS: usize = 131_072;
/// Most intents without resolution one ledger may hold.
pub const MAX_UNRESOLVED_INTENTS: usize = 4096;

const DIR: &str = "ledger";
const FILE: &str = "outcomes.log";

/// Typed failure of a ledger read or write. Variants carry no paths or raw OS text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerError {
    /// The file, line, record or open-intent cap was exceeded.
    LimitExceeded,
    /// Two accepted records for one operation disagree: the ledger contradicts itself.
    Inconsistent,
    /// The ledger records an accepted commit that the root does not contain.
    RootDisagrees,
    /// The ledger directory or file is unsafe, or storage failed.
    Storage(StorageError),
}

impl LedgerError {
    /// Maps the failure onto the F04 section 8 vocabulary.
    pub const fn code(self) -> ErrorCode {
        match self {
            Self::LimitExceeded => ErrorCode::LimitExceeded,
            Self::Inconsistent | Self::RootDisagrees => ErrorCode::IntegrityError,
            Self::Storage(error) => error.code(),
        }
    }
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::LimitExceeded => "ledger cap exceeded",
            Self::Inconsistent => "ledger records contradict each other",
            Self::RootDisagrees => "ledger records a commit that the root does not contain",
            Self::Storage(_) => "ledger storage failed",
        })
    }
}

impl std::error::Error for LedgerError {}

impl From<StorageError> for LedgerError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::LimitExceeded => Self::LimitExceeded,
            other => Self::Storage(other),
        }
    }
}

/// How a recorded outcome was proven.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeSource {
    /// A `Committed` record, confirmed against the root.
    Ledger,
    /// The root itself holds the operation's revision (no `Committed` record yet).
    Root,
}

/// The host-ledger answer to QueryOutcome for one operation id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The operation is accepted: its revision is in the root.
    Committed {
        /// Revision the operation created.
        revision: Uuid7,
        /// Head-log sequence of that revision.
        sequence: u64,
        /// Digest of the manifest blob.
        manifest: ContentDigest,
        /// What proves it.
        source: OutcomeSource,
    },
    /// Every attempt was closed as not accepted; the operation had no effect.
    Aborted {
        /// Why the last attempt was closed.
        reason: AbortReason,
    },
    /// The ledger covers the whole history and holds no record: the operation never reached
    /// the root.
    NotFound,
    /// `OUTCOME_UNKNOWN`: acceptance can be neither proven nor excluded. Run recovery, or retry
    /// with the same op id and identical content.
    Unknown,
}

/// Internal handle on the ledger file of one store.
pub(crate) struct Ledger<'a> {
    blobs: &'a BlobStore,
}

pub(crate) fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl<'a> Ledger<'a> {
    pub(crate) const fn new(blobs: &'a BlobStore) -> Self {
        Self { blobs }
    }

    fn dir(&self, create: bool) -> Result<Option<PathBuf>, LedgerError> {
        match self.blobs.namespace_dir(DIR, create) {
            Ok(path) => Ok(Some(path)),
            Err(StorageError::NotFound) if !create => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    /// Reads and parses the whole ledger; an absent ledger is empty.
    pub(crate) fn scan(&self) -> Result<Scan, LedgerError> {
        let Some(directory) = self.dir(false)? else {
            return Ok(Scan::default());
        };
        let cap = usize::try_from(MAX_LEDGER_BYTES).map_err(|_| LedgerError::LimitExceeded)?;
        match self.blobs.read_namespace_exact(&directory, FILE, cap)? {
            Some(bytes) => Scan::parse(&bytes),
            None => Ok(Scan::default()),
        }
    }

    /// Appends records in one synced write. `genesis_head` adds the genesis record first, for a
    /// ledger that has none.
    pub(crate) fn append(
        &self,
        records: &[Record],
        genesis_head: Option<u64>,
    ) -> Result<(), LedgerError> {
        let directory = self.dir(true)?.ok_or(StorageError::NotFound)?;
        let mut bytes = Vec::new();
        if let Some(head) = genesis_head {
            bytes.extend(Record::Genesis { head }.encode().0);
        }
        for record in records {
            bytes.extend(record.encode().0);
        }
        self.blobs
            .append_namespace_file(&directory, FILE, &bytes, MAX_LEDGER_BYTES)?;
        Ok(())
    }
}

impl RevisionStore {
    /// The ledger of this store, for the commit and recovery paths.
    pub(crate) fn ledger(&self) -> Ledger<'_> {
        Ledger::new(&self.blobs)
    }
}
