// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! U14: who is using a store right now. One advisory lock on `maintenance.lock` in the store root
//! separates the users of a store from history compaction:
//!
//! * a committer, a recovery and a coordinator hold a **shared** lease while they work (the
//!   coordinator for as long as it serves);
//! * compaction needs the **exclusive** lease, so it cannot start while any of them is active and
//!   they cannot start while it runs.
//!
//! The operating system releases the lock when the holder exits or crashes, so a killed process
//! never leaves a lease behind and there is nothing to clean up. Advisory means a process that
//! ignores the protocol (a raw file copy, an older build) is not stopped. The lock is a local
//! filesystem feature: it is not a protection on a network share.

use std::fmt;
use std::fs::{File, TryLockError};

use beamcraft_model::ErrorCode;

use crate::storage::{BlobStore, StorageError};

/// Why a lease was not granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseError {
    /// Another holder has the store: a coordinator, a commit, a recovery or a compaction.
    Busy,
    /// The lock file is unsafe or could not be used.
    Storage(StorageError),
}

impl LeaseError {
    /// Maps the failure onto the F04 section 8 vocabulary: a busy store is unavailable for now.
    pub const fn code(self) -> ErrorCode {
        match self {
            Self::Busy => ErrorCode::BackendUnavailable,
            Self::Storage(error) => error.code(),
        }
    }
}

impl fmt::Display for LeaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Busy => "the store is in use by another holder",
            Self::Storage(_) => "the store lock file is unusable",
        })
    }
}

impl std::error::Error for LeaseError {}

impl From<StorageError> for LeaseError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

/// A granted lease; dropping it (or the end of the process) releases it.
#[derive(Debug)]
pub struct StoreLease {
    _file: File,
}

fn lock_error(error: TryLockError) -> LeaseError {
    match error {
        TryLockError::WouldBlock => LeaseError::Busy,
        TryLockError::Error(e) => LeaseError::Storage(StorageError::Io(e.kind())),
    }
}

impl StoreLease {
    /// A shared lease: any number of users at once, refused while compaction holds the store.
    /// Never waits.
    pub fn shared(store: &BlobStore) -> Result<Self, LeaseError> {
        let file = store
            .open_lock_file(true)?
            .ok_or(LeaseError::Storage(StorageError::NotFound))?;
        file.try_lock_shared().map_err(lock_error)?;
        Ok(Self { _file: file })
    }

    /// The exclusive lease compaction needs: refused while any other lease exists. Never waits.
    pub fn exclusive(store: &BlobStore) -> Result<Self, LeaseError> {
        let file = store
            .open_lock_file(true)?
            .ok_or(LeaseError::Storage(StorageError::NotFound))?;
        file.try_lock().map_err(lock_error)?;
        Ok(Self { _file: file })
    }

    /// Whether the store is free of every other lease right now (an exclusive lease would be
    /// granted). Changes nothing: a store whose lock file does not exist has had no holder.
    pub fn is_free(store: &BlobStore) -> Result<bool, LeaseError> {
        let Some(file) = store.open_lock_file(false)? else {
            return Ok(true);
        };
        match file.try_lock() {
            Ok(()) => Ok(true),
            Err(TryLockError::WouldBlock) => Ok(false),
            Err(TryLockError::Error(e)) => Err(LeaseError::Storage(StorageError::Io(e.kind()))),
        }
    }
}
