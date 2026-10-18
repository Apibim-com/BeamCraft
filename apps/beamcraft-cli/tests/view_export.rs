// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
// Contributor-original synthetic plates, MPL-2.0.

//! V03 `view-export`: the verified head's view snapshot as one engine bundle. The bundle rebuilds
//! the identical snapshot (same token as `plate-inspect`), shares one mesh between copies, never
//! overwrites a file and is refused for a store without a revision.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use beamcraft_engine::{Limits, ViewSnapshot};
use beamcraft_protocol::{
    json::{self, Json},
    message,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("v03-export")
        .join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("store")).unwrap();
    root
}

fn run(args: &[&str]) -> (i32, Vec<u8>) {
    let output = Command::new(env!("CARGO_BIN_EXE_beamcraft"))
        .args(args)
        .output()
        .unwrap();
    (output.status.code().unwrap(), output.stdout)
}

fn json_of(bytes: &[u8]) -> Json {
    json::decode(bytes, &message::read_limits()).unwrap()
}

fn text(json: &Json, key: &str) -> String {
    json.get(key).unwrap().string().unwrap().to_owned()
}

#[test]
fn the_bundle_is_the_engine_snapshot_of_the_head() {
    let root = root("head");
    let store = root.join("store");
    let store = store.to_str().unwrap();
    let plate = "019a7bf0-0000-7000-8000-000000000010";
    let created = run(&[
        "create-plate",
        "--store",
        store,
        "--width",
        "800",
        "--length",
        "400",
        "--thickness",
        "12",
        "--new-id",
        plate,
    ]);
    assert_eq!(created.0, 0);
    for offset in ["1000,0,0", "2000,0,0"] {
        let copied = run(&[
            "copy-element",
            "--store",
            store,
            "--id",
            plate,
            "--offset",
            offset,
        ]);
        assert_eq!(copied.0, 0);
    }
    let (code, bundle) = run(&["view-export", "--store", store, "--out", "-"]);
    assert_eq!(code, 0);
    let snapshot = ViewSnapshot::from_bundle(&bundle, &Limits::default()).unwrap();
    assert_eq!(snapshot.len(), 3);
    // Copies share one mesh blob: the instancing input of the viewport.
    assert_eq!(snapshot.meshes().len(), 1);
    let (code, inspected) = run(&["plate-inspect", "--store", store]);
    assert_eq!(code, 0);
    let inspected = json_of(&inspected);
    let token = text(inspected.get("snapshot").unwrap(), "token");
    assert_eq!(snapshot.token().as_str(), token);

    let file = root.join("view.bin");
    let file = file.to_str().unwrap();
    let (code, summary) = run(&["view-export", "--store", store, "--out", file]);
    assert_eq!(code, 0);
    let summary = json_of(&summary);
    assert_eq!(text(&summary, "outcome"), "view_exported");
    assert_eq!(text(&summary, "token"), token);
    assert_eq!(fs::read(file).unwrap(), bundle);
    // An existing file is never overwritten.
    let (code, refused) = run(&["view-export", "--store", store, "--out", file]);
    assert_eq!(code, 2);
    assert_eq!(text(&json_of(&refused), "code"), "INVALID_ARGUMENT");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_store_without_a_revision_and_bad_options_are_refused() {
    let root = root("empty");
    let store = root.join("store");
    let store = store.to_str().unwrap();
    let (code, body) = run(&["view-export", "--store", store, "--out", "-"]);
    assert_eq!(code, 2);
    assert_eq!(text(&json_of(&body), "code"), "NOT_FOUND");
    for args in [
        vec!["view-export", "--store", store],
        vec![
            "view-export",
            "--store",
            store,
            "--out",
            "-",
            "--extra",
            "x",
        ],
        vec!["view-export", "--out", "-"],
    ] {
        let (code, body) = run(&args);
        assert_eq!(code, 2, "{args:?}");
        assert_eq!(text(&json_of(&body), "code"), "INVALID_ARGUMENT");
    }
    let _ = fs::remove_dir_all(&root);
}
