// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The boundary rules applied to the real workspace on disk, plus end-to-end runs of the binary.

use std::path::{Path, PathBuf};
use std::process::Command;

use beamcraft_boundaries::{check_root, load_table, portable_args};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

#[test]
fn the_real_workspace_obeys_the_module_table() {
    let violations = check_root(&root()).unwrap();
    let text: Vec<String> = violations.iter().map(ToString::to_string).collect();
    assert!(text.is_empty(), "boundary violations:\n{}", text.join("\n"));
}

#[test]
fn the_portable_list_is_derived_from_the_table() {
    let args = portable_args(&root()).unwrap();
    for name in ["beamcraft-geom", "beamcraft-model", "beamcraft-protocol"] {
        assert!(args.contains(&format!("-p {name}")), "{args}");
    }
    for name in [
        "beamcraft-host",
        "beamcraft-cli",
        "beamcraft-fixtures",
        "beamcraft-boundaries",
    ] {
        assert!(!args.contains(name), "{args}");
    }
    let table = load_table(&root()).unwrap();
    assert!(
        table.third_party.is_empty(),
        "domain crates have no third-party crate today"
    );
}

#[test]
fn the_binary_reports_success_and_the_portable_list() {
    let exe = env!("CARGO_BIN_EXE_beamcraft-boundaries");
    let check = Command::new(exe)
        .args(["check", "--root"])
        .arg(root())
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&check.stdout).trim(),
        "module boundaries: ok"
    );

    let portable = Command::new(exe)
        .args(["portable", "--root"])
        .arg(root())
        .output()
        .unwrap();
    assert!(portable.status.success());
    let text = String::from_utf8_lossy(&portable.stdout);
    assert!(
        text.starts_with("-p beamcraft-geom -p beamcraft-model"),
        "{text}"
    );
}

#[test]
fn the_binary_rejects_bad_usage_and_a_foreign_root() {
    let exe = env!("CARGO_BIN_EXE_beamcraft-boundaries");
    let usage = Command::new(exe).arg("bogus").output().unwrap();
    assert_eq!(usage.status.code(), Some(2));
    let empty = std::env::temp_dir();
    let missing = Command::new(exe)
        .args(["check", "--root"])
        .arg(&empty)
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
}
