// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! C01: the durable binding of a changeset id to one payload digest.
//!
//! A receiver binds a changeset id to the digest of its canonical bytes **before** it publishes
//! anything for it, in one append-only file, `changesets/claims.log`, of checksummed text lines.
//! The first valid record of an id is the binding; it is never replaced. A later receipt of the
//! same id therefore proves either the same payload (a retry: replay or resume) or a different
//! one (`OP_ID_REUSE`), across processes and restarts, whether or not the first receipt was
//! eventually published.
//!
//! Concurrency follows the U07 ledger: one small append per record (`O_APPEND`, synced) and no
//! lock file. Two receivers racing on one id may both append; both then re-read the file and the
//! earlier line decides for both. A torn or corrupt line is ignored, so a crash mid-append leaves
//! at most an ignored fragment and the id unbound.

use crate::revision::{RevisionError, RevisionStore, Uuid7};
use crate::storage::StorageError;
use beamcraft_model::digest::{ContentDigest, Sha256};
use std::collections::BTreeMap;

const DIR: &str = "changesets";
const FILE: &str = "claims.log";
const VERSION: &str = "beamcraft-changeset-claim/1";
/// Hard size cap of the claims file (about 90 000 claims).
pub const MAX_CLAIMS_BYTES: u64 = 16 * 1024 * 1024;

/// What a claim found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Claim {
    /// This call bound the id to the digest.
    New,
    /// The id was already bound to this digest (a retry).
    Same,
}

fn checksum(body: &str) -> Result<ContentDigest, RevisionError> {
    Sha256::digest(body.as_bytes()).map_err(|_| RevisionError::LimitExceeded)
}

fn record(id: &Uuid7, digest: &ContentDigest) -> Result<String, RevisionError> {
    let body = format!("{VERSION} {id} {digest}");
    Ok(format!("{body} {}\n", checksum(&body)?))
}

/// The first valid binding of every id; invalid lines are skipped.
fn parse(bytes: &[u8]) -> BTreeMap<String, ContentDigest> {
    let mut bindings = BTreeMap::new();
    // A torn multi-byte sequence can only be the last fragment; keep the valid prefix.
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(e) => bytes
            .get(..e.valid_up_to())
            .and_then(|valid| std::str::from_utf8(valid).ok())
            .unwrap_or_default(),
    };
    for line in text.lines() {
        let mut parts = line.split(' ');
        let (Some(version), Some(id), Some(digest), Some(sum), None) = (
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
        ) else {
            continue;
        };
        let body = format!("{version} {id} {digest}");
        let proven = checksum(&body).is_ok_and(|c| c.to_string() == sum);
        if version != VERSION || !proven || Uuid7::parse(id).is_err() {
            continue;
        }
        let Ok(digest) = digest.parse::<ContentDigest>() else {
            continue;
        };
        bindings.entry(id.to_owned()).or_insert(digest);
    }
    bindings
}

impl RevisionStore {
    fn claims(&self) -> Result<BTreeMap<String, ContentDigest>, RevisionError> {
        let directory = match self.blobs().namespace_dir(DIR, false) {
            Ok(path) => path,
            Err(StorageError::NotFound) => return Ok(BTreeMap::new()),
            Err(error) => return Err(RevisionError::Storage(error)),
        };
        let cap = usize::try_from(MAX_CLAIMS_BYTES).map_err(|_| RevisionError::LimitExceeded)?;
        match self
            .blobs()
            .read_namespace_exact(&directory, FILE, cap)
            .map_err(RevisionError::Storage)?
        {
            Some(bytes) => Ok(parse(&bytes)),
            None => Ok(BTreeMap::new()),
        }
    }

    /// The digest a changeset id is bound to, if any.
    pub fn changeset_claim(&self, id: &Uuid7) -> Result<Option<ContentDigest>, RevisionError> {
        Ok(self.claims()?.get(id.as_str()).copied())
    }

    /// Binds a changeset id to a payload digest, durably, before anything is published for it.
    /// An id bound to another digest is [`RevisionError::OpIdReuse`] and nothing is written.
    pub fn claim_changeset(
        &self,
        id: &Uuid7,
        digest: &ContentDigest,
    ) -> Result<Claim, RevisionError> {
        match self.changeset_claim(id)? {
            Some(seen) if seen == *digest => return Ok(Claim::Same),
            Some(_) => return Err(RevisionError::OpIdReuse),
            None => {}
        }
        let directory = self
            .blobs()
            .namespace_dir(DIR, true)
            .map_err(RevisionError::Storage)?;
        self.blobs()
            .append_namespace_file(
                &directory,
                FILE,
                record(id, digest)?.as_bytes(),
                MAX_CLAIMS_BYTES,
            )
            .map_err(RevisionError::Storage)?;
        // A concurrent receiver may have appended first: the earliest line decides.
        match self.changeset_claim(id)? {
            Some(seen) if seen == *digest => Ok(Claim::New),
            Some(_) => Err(RevisionError::OpIdReuse),
            None => Err(RevisionError::Ledger(
                crate::ledger::LedgerError::Inconsistent,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> Uuid7 {
        Uuid7::parse(&format!("019a7bf0-0000-7000-8000-0000000000{n:02x}")).unwrap()
    }

    #[test]
    fn the_first_valid_line_binds_and_torn_or_forged_lines_are_ignored() {
        let a = Sha256::digest(b"a").unwrap();
        let b = Sha256::digest(b"b").unwrap();
        let mut text = record(&id(1), &a).unwrap();
        text.push_str(&record(&id(1), &b).unwrap());
        // Forged checksum, wrong version, torn fragment.
        text.push_str(&format!("{VERSION} {} {b} {}\n", id(2), "0".repeat(64)));
        text.push_str(&record(&id(3), &a).unwrap().replace(VERSION, "other/1"));
        let torn = record(&id(4), &a).unwrap();
        text.push_str(&torn[..torn.len() - 10]);
        let bindings = parse(text.as_bytes());
        assert_eq!(bindings.get(id(1).as_str()), Some(&a));
        assert_eq!(bindings.len(), 1);
    }
}
