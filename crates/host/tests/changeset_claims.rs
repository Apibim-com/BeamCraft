// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
// Contributor-original synthetic test, MPL-2.0.

//! C01 claims: a changeset id is bound to the first payload digest, durably and once, also when
//! receivers race.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use beamcraft_host::changeset::Claim;
use beamcraft_host::revision::{RevisionError, RevisionStore, Uuid7};
use beamcraft_host::storage::BlobStore;
use beamcraft_model::digest::Sha256;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Barrier};

fn store(name: &str) -> RevisionStore {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("c01-claims")
        .join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    RevisionStore::new(BlobStore::new(&root).unwrap())
}

#[test]
fn an_id_binds_to_its_first_payload_across_reopen() {
    let s = store("reopen");
    let id = Uuid7::parse("019a7bf0-0000-7000-8000-00000000d001").unwrap();
    let a = Sha256::digest(b"payload a").unwrap();
    let b = Sha256::digest(b"payload b").unwrap();
    assert_eq!(s.changeset_claim(&id).unwrap(), None);
    assert_eq!(s.claim_changeset(&id, &a).unwrap(), Claim::New);
    assert_eq!(s.claim_changeset(&id, &a).unwrap(), Claim::Same);
    assert_eq!(s.claim_changeset(&id, &b), Err(RevisionError::OpIdReuse));
    let reopened = RevisionStore::new(s.blobs().clone());
    assert_eq!(reopened.changeset_claim(&id).unwrap(), Some(a));
}

#[test]
fn racing_receivers_agree_on_one_binding() {
    let s = store("race");
    let id = Uuid7::parse("019a7bf0-0000-7000-8000-00000000d002").unwrap();
    let barrier = Arc::new(Barrier::new(8));
    let handles: Vec<_> = (0..8u8)
        .map(|n| {
            let s = RevisionStore::new(s.blobs().clone());
            let id = id.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let digest = Sha256::digest(&[n]).unwrap();
                barrier.wait();
                (digest, s.claim_changeset(&id, &digest))
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let winners: Vec<_> = results
        .iter()
        .filter(|(_, r)| *r == Ok(Claim::New))
        .collect();
    assert_eq!(winners.len(), 1);
    assert!(
        results
            .iter()
            .filter(|(_, r)| *r != Ok(Claim::New))
            .all(|(_, r)| *r == Err(RevisionError::OpIdReuse))
    );
    assert_eq!(s.changeset_claim(&id).unwrap(), Some(winners[0].0));
}
