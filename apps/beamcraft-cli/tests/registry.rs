// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The command registry is the inventory of what the real binary dispatches.

#![allow(clippy::expect_used)]

use beamcraft_cli::registry::{COMMANDS, Tier};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Runs `beamcraft ARGS`, killing it after a deadline so a command that waits for input or
/// serves cannot hang the suite. Returns the exit code and standard output.
fn run(args: &[&str]) -> (Option<i32>, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_beamcraft"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn beamcraft");
    let mut stdout = child.stdout.take().expect("stdout");
    let reader = std::thread::spawn(move || {
        let mut text = String::new();
        stdout.read_to_string(&mut text).ok();
        text
    });
    let deadline = Instant::now() + Duration::from_secs(20);
    let code = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status.code();
        }
        assert!(Instant::now() < deadline, "{args:?} did not finish");
        std::thread::sleep(Duration::from_millis(10));
    };
    (code, reader.join().expect("reader"))
}

/// A scratch store directory: with `--store` the generic blob and revision path reaches its command
/// check, so an undispatched word answers `UNSUPPORTED_COMMAND` instead of an argument refusal.
fn scratch_store() -> String {
    let dir = std::env::temp_dir().join(format!("beamcraft-registry-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch store");
    dir.to_str().expect("utf-8 path").to_owned()
}

#[test]
fn an_unregistered_word_is_unsupported() {
    let store = scratch_store();
    let (code, out) = run(&["no-such-command", "--store", &store]);
    assert_eq!(code, Some(2));
    assert!(out.contains("UNSUPPORTED_COMMAND"), "{out}");
}

#[test]
fn every_registered_command_is_routed_by_the_binary() {
    let store = scratch_store();
    for command in COMMANDS {
        let (code, out) = run(&[command.name, "--store", &store]);
        assert!(
            !out.contains("UNSUPPORTED_COMMAND"),
            "{} is registered but not dispatched: {out}",
            command.name
        );
        assert!(code.is_some(), "{}: no exit code", command.name);
    }
}

fn names(out: &str) -> Vec<String> {
    out.split("\"name\":\"")
        .skip(1)
        .map(|rest| rest.split('"').next().expect("name").to_owned())
        .collect()
}

#[test]
fn commands_lists_the_registry_and_filters_by_tier() {
    let (code, out) = run(&["commands"]);
    assert_eq!(code, Some(0));
    assert!(out.contains("\"outcome\":\"commands_listed\""), "{out}");
    let all = names(&out);
    assert_eq!(all.len(), COMMANDS.len());
    assert!(all.iter().any(|n| n == "create-member"));

    let (_, core) = run(&["commands", "--tier", "core"]);
    let (_, assist) = run(&["commands", "--tier", "ui-assist"]);
    let (core, assist) = (names(&core), names(&assist));
    assert_eq!(core.len() + assist.len(), all.len());
    assert!(assist.iter().any(|n| n == "snap"));
    assert!(!core.iter().any(|n| n == "snap"));
    let expected = COMMANDS.iter().filter(|c| c.tier == Tier::UiAssist).count();
    assert_eq!(assist.len(), expected);
}

#[test]
fn commands_refuses_unknown_tiers_and_stray_arguments() {
    for args in [
        &["commands", "--tier", "human"][..],
        &["commands", "--tier"],
        &["commands", "extra"],
    ] {
        let (code, out) = run(args);
        assert_eq!(code, Some(2), "{args:?}: {out}");
        assert!(out.contains("INVALID_ARGUMENT"), "{args:?}: {out}");
    }
}

#[test]
fn the_help_inventory_is_the_registry_in_order() {
    let (_, help) = run(&["--help"]);
    let line = help.lines().next().expect("inventory");
    let listed: Vec<_> = line
        .strip_prefix("BeamCraft: ")
        .expect("title")
        .split(" | ")
        .collect();
    let registered: Vec<_> = COMMANDS.iter().map(|c| c.name).collect();
    assert_eq!(listed, registered);
}
