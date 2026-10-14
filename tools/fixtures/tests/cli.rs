// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! End-to-end tests of the `beamcraft-fixtures` binary: real generation into real directories,
//! byte-for-byte comparison with the committed set, tamper detection, and acceptance/rejection
//! of every committed fixture against the hand-written expectations.

// Test-only crate: helpers outside `#[test]` functions may panic on setup failures.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use beamcraft_fixtures::json::{self, Json, ParseLimits};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn committed() -> PathBuf {
    repo_root().join("tests").join("fixtures").join("f07")
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_beamcraft-fixtures"))
        .args(args)
        .output()
        .expect("run beamcraft-fixtures")
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// A fresh, empty scratch directory removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("beamcraft-fixtures-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn path(&self) -> &str {
        self.0.to_str().unwrap()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Every file under `dir`, as sorted (relative path, bytes).
fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path
                    .strip_prefix(dir)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((rel, fs::read(&path).unwrap()));
            }
        }
    }
    out.sort();
    out
}

const LIMITS: ParseLimits = ParseLimits {
    max_bytes: 1 << 20,
    max_depth: 16,
    max_values: 1 << 16,
    max_string_bytes: 1024,
    max_number_chars: 32,
    max_members: 64,
};

fn load(path: &Path) -> Json {
    json::parse(&fs::read(path).unwrap(), &LIMITS).unwrap()
}

#[test]
fn generation_is_deterministic_and_matches_the_committed_set() {
    let a = Scratch::new("gen-a");
    let b = Scratch::new("gen-b");
    for dir in [&a, &b] {
        let out = run(&["generate", "--out", dir.path()]);
        assert!(out.status.success(), "{}", stderr(&out));
        assert!(
            stdout(&out).starts_with("generated 34 files"),
            "{}",
            stdout(&out)
        );
    }
    let first = snapshot(&a.0);
    assert_eq!(first.len(), 34);
    assert_eq!(first, snapshot(&b.0));
    assert_eq!(
        first,
        snapshot(&committed()),
        "tests/fixtures/f07 is stale; regenerate it"
    );

    // Regenerating over an existing set is allowed and changes nothing.
    let again = run(&["generate", "--out", a.path()]);
    assert!(again.status.success(), "{}", stderr(&again));
    assert_eq!(first, snapshot(&a.0));
}

#[test]
fn verify_accepts_the_committed_set() {
    let out = run(&["verify", "--dir", committed().to_str().unwrap()]);
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).starts_with("verified 34 files and 33 manifest digests"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn verify_detects_tampering_missing_and_foreign_files() {
    let dir = Scratch::new("tamper");
    assert!(run(&["generate", "--out", dir.path()]).status.success());

    let target = dir.0.join("valid").join("plate-axis-aligned.json");
    let mut bytes = fs::read(&target).unwrap();
    let last = bytes.len() - 2;
    bytes[last] ^= 0x01;
    fs::write(&target, &bytes).unwrap();
    fs::remove_file(dir.0.join("invalid").join("utf8.json")).unwrap();
    fs::write(dir.0.join("valid").join("notes.txt"), b"x").unwrap();

    let out = run(&["verify", "--dir", dir.path()]);
    assert_eq!(out.status.code(), Some(1));
    let text = stdout(&out);
    assert!(
        text.contains("valid/plate-axis-aligned.json: content differs"),
        "{text}"
    );
    assert!(
        text.contains("valid/plate-axis-aligned.json: sha256"),
        "{text}"
    );
    assert!(text.contains("missing invalid/utf8.json"), "{text}");
    assert!(text.contains("unexpected entry valid/notes.txt"), "{text}");
}

#[test]
fn verify_detects_a_manifest_digest_that_does_not_match_the_bytes() {
    let dir = Scratch::new("digest");
    assert!(run(&["generate", "--out", dir.path()]).status.success());
    let manifest = dir.0.join("manifest.json");
    let text = fs::read_to_string(&manifest).unwrap();
    let set = beamcraft_fixtures::generate().unwrap();
    let digest = beamcraft_fixtures::sha256::hex_digest(&set.cases[0].bytes);
    fs::write(&manifest, text.replace(&digest, &"0".repeat(64))).unwrap();

    let out = run(&["verify", "--dir", dir.path()]);
    assert_eq!(out.status.code(), Some(1));
    let text = stdout(&out);
    assert!(text.contains("manifest.json: content differs"), "{text}");
    assert!(
        text.contains(&format!(
            "{}: sha256 {digest} does not match manifest 000",
            set.cases[0].path
        )),
        "{text}"
    );
}

#[test]
fn generate_refuses_a_directory_with_foreign_entries() {
    let dir = Scratch::new("foreign");
    fs::write(dir.0.join("keep-me.txt"), b"user data").unwrap();
    let out = run(&["generate", "--out", dir.path()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("keep-me.txt"), "{}", stderr(&out));
    // Nothing was written and the foreign file is untouched.
    assert!(!dir.0.join("manifest.json").exists());
    assert_eq!(fs::read(dir.0.join("keep-me.txt")).unwrap(), b"user data");
}

#[test]
fn every_committed_fixture_has_its_independently_expected_outcome() {
    let manifest = load(&committed().join("manifest.json"));
    let expectations = load(
        &repo_root()
            .join("tests")
            .join("fixtures")
            .join("expectations")
            .join("f07.json"),
    );
    let entries = manifest.get("fixtures").unwrap().as_array().unwrap();
    let cases = expectations.get("cases").unwrap().as_array().unwrap();
    assert_eq!(entries.len(), cases.len());

    for entry in entries {
        let id = entry.get("id").unwrap().as_str().unwrap();
        let case = cases
            .iter()
            .find(|c| c.get("id").unwrap().as_str() == Some(id))
            .unwrap_or_else(|| panic!("no expectation for {id}"));
        assert_eq!(entry.get("support"), case.get("support"), "{id}");

        let path = committed().join(entry.get("path").unwrap().as_str().unwrap());
        let out = run(&["check", path.to_str().unwrap()]);
        let text = stdout(&out);
        match case.get("outcome").unwrap().as_str().unwrap() {
            "accept" => {
                assert!(out.status.success(), "{id}: {text}");
                assert!(text.starts_with(&format!("accepted {id} ")), "{id}: {text}");
            }
            "reject" => {
                let code = case.get("error").unwrap().as_str().unwrap();
                assert_eq!(out.status.code(), Some(3), "{id}: {text}");
                assert!(
                    text.starts_with(&format!("rejected {code}: ")),
                    "{id}: {text}"
                );
            }
            other => panic!("{id}: unknown outcome {other}"),
        }
    }
}

#[test]
fn check_bounds_its_input_and_reports_io_and_usage_errors() {
    let dir = Scratch::new("check");
    let big = dir.0.join("big.json");
    fs::write(
        &big,
        vec![b' '; beamcraft_fixtures::fixture::MAX_INPUT_BYTES + 1],
    )
    .unwrap();
    let out = run(&["check", big.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(3));
    assert!(
        stdout(&out).starts_with("rejected limit_exceeded"),
        "{}",
        stdout(&out)
    );

    let out = run(&["check", dir.0.join("absent.json").to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));

    for args in [
        &[][..],
        &["generate"][..],
        &["frobnicate"][..],
        &["verify", "--out", "x"][..],
    ] {
        let out = run(args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(stderr(&out).contains("usage:"));
    }

    let out = run(&["verify", "--dir", dir.0.join("absent").to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn list_and_version_work() {
    let out = run(&["list"]);
    assert!(out.status.success());
    let text = stdout(&out);
    assert_eq!(text.lines().count(), 33);
    assert!(text.contains("f07.copy.two-plates\tsupported\tvalid/copy-two-plates.json"));
    assert!(text.contains("f07.plate.far-from-origin\tunverified\t"));

    let out = run(&["--version"]);
    assert!(stdout(&out).starts_with("beamcraft-fixtures "));
}
