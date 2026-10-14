// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! End-to-end test: the binary reports its version.

use std::process::Command;

#[test]
fn prints_version() {
    let out = Command::new(env!("CARGO_BIN_EXE_beamcraft"))
        .arg("--version")
        .output()
        .expect("run beamcraft");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("beamcraft "), "got: {stdout}");
}
