// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! `beamcraft-boundaries`: check the module boundaries and list the portable crates.
//!
//! Exit codes: 0 success, 1 rule violations or unreadable input, 2 usage error.

use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "\
usage:
  beamcraft-boundaries check [--root DIR]     check the workspace against the module table
  beamcraft-boundaries portable [--root DIR]  print `-p <crate>` for every wasm-portable crate
  beamcraft-boundaries --version";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).take(4).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["check"] => check(Path::new(".")),
        ["check", "--root", dir] => check(Path::new(dir)),
        ["portable"] => portable(Path::new(".")),
        ["portable", "--root", dir] => portable(Path::new(dir)),
        ["--version" | "-V"] => {
            println!("beamcraft-boundaries {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn check(root: &Path) -> ExitCode {
    match beamcraft_boundaries::check_root(root) {
        Ok(violations) if violations.is_empty() => {
            println!("module boundaries: ok");
            ExitCode::SUCCESS
        }
        Ok(violations) => {
            for violation in &violations {
                eprintln!("violation: {violation}");
            }
            eprintln!("module boundaries: {} violation(s)", violations.len());
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn portable(root: &Path) -> ExitCode {
    match beamcraft_boundaries::portable_args(root) {
        Ok(args) if !args.is_empty() => {
            println!("{args}");
            ExitCode::SUCCESS
        }
        Ok(_) => {
            eprintln!("error: the module table lists no portable crate");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
