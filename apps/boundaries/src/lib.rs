// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Module-boundary checker for the BeamCraft workspace (task F08).
//!
//! The module table in `docs/contracts/f08-module-boundaries-v0.1.md` is the single source of truth
//! for which packages exist, which way their dependencies may point and which of them must build
//! for `wasm32-unknown-unknown`. This development tool reads that table and the workspace
//! manifests with the standard library only, reports every broken rule and prints the portable
//! crate list that CI passes to the wasm check.
//!
//! The command-line front end is the `beamcraft-boundaries` binary.

pub mod check;
pub mod manifest;
pub mod table;
pub mod workspace;

use std::fmt;
use std::fs;
use std::io::Read as _;
use std::path::Path;

pub use check::{Violation, check_ci, check_table, check_workspace};
pub use table::{ModuleTable, parse_document};

/// Repository path of the CI workflow.
pub const CI_PATH: &str = ".github/workflows/ci.yml";

/// Why a check could not run at all (as opposed to finding violations).
#[derive(Debug)]
pub enum Error {
    /// A file could not be read.
    Io(String, std::io::Error),
    /// A file is too large.
    TooLarge(String),
    /// The boundary document is malformed.
    Table(table::TableError),
    /// The workspace could not be loaded.
    Load(workspace::LoadError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(path, e) => write!(f, "cannot read `{path}`: {e}"),
            Self::TooLarge(path) => write!(f, "`{path}` is larger than the 1 MiB limit"),
            Self::Table(e) => write!(f, "{}: {e}", table::DOCUMENT_PATH),
            Self::Load(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

fn read_text(root: &Path, rel: &str) -> Result<String, Error> {
    let file = fs::File::open(root.join(rel)).map_err(|e| Error::Io(rel.to_owned(), e))?;
    let mut text = String::new();
    let limit = u64::try_from(manifest::MAX_MANIFEST_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    file.take(limit)
        .read_to_string(&mut text)
        .map_err(|e| Error::Io(rel.to_owned(), e))?;
    if text.len() > manifest::MAX_MANIFEST_BYTES {
        return Err(Error::TooLarge(rel.to_owned()));
    }
    Ok(text)
}

/// Reads the module table from the boundary document under `root`.
///
/// # Errors
///
/// [`Error`] if the document is unreadable or malformed.
pub fn load_table(root: &Path) -> Result<ModuleTable, Error> {
    parse_document(&read_text(root, table::DOCUMENT_PATH)?).map_err(Error::Table)
}

/// Runs every check on the workspace under `root` and returns the violations (empty when clean).
///
/// # Errors
///
/// [`Error`] if an input cannot be read.
pub fn check_root(root: &Path) -> Result<Vec<Violation>, Error> {
    let table = load_table(root)?;
    let workspace = workspace::load(root).map_err(Error::Load)?;
    let mut violations = check_workspace(&workspace, &table);
    violations.extend(check_ci(&read_text(root, CI_PATH)?));
    Ok(violations)
}

/// The `-p <package>` arguments for the wasm check, one per portable crate.
///
/// # Errors
///
/// [`Error`] if the table cannot be read.
pub fn portable_args(root: &Path) -> Result<String, Error> {
    let table = load_table(root)?;
    Ok(table
        .portable()
        .iter()
        .map(|name| format!("-p {name}"))
        .collect::<Vec<_>>()
        .join(" "))
}
