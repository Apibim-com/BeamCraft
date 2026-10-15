// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Loads the workspace members, their manifests and their Rust sources from disk.
//!
//! Every read is bounded: manifests and sources are at most 1 MiB, at most 256 members and 512
//! source files per member are read and directories are walked at most eight levels deep.
//! Symbolic links are not followed.

use std::fmt;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use crate::manifest::{Dependency, MAX_MANIFEST_BYTES, ManifestError, parse_manifest};

/// Most workspace members read.
pub const MAX_MEMBERS: usize = 256;
/// Most source files read per member.
pub const MAX_SOURCE_FILES: usize = 512;
/// Deepest source directory level walked.
pub const MAX_DEPTH: usize = 8;

/// One Rust source file of a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// Path relative to the workspace root, with `/` separators.
    pub path: String,
    /// File text.
    pub text: String,
}

/// One workspace member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// Cargo package name.
    pub name: String,
    /// Directory relative to the workspace root, with `/` separators.
    pub path: String,
    /// `publish = false`.
    pub publish_false: bool,
    /// Declared dependencies of every table.
    pub dependencies: Vec<Dependency>,
    /// Rust sources under `src/`.
    pub sources: Vec<Source>,
}

/// All members of a workspace.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Workspace {
    /// Members in path order.
    pub packages: Vec<Package>,
}

/// Why the workspace was not loaded.
#[derive(Debug)]
pub enum LoadError {
    /// A file or directory could not be read.
    Io {
        /// Path relative to the root.
        path: String,
        /// The operating-system error.
        source: std::io::Error,
    },
    /// A file is larger than the limit.
    TooLarge(String),
    /// A manifest exceeded a parser limit.
    Manifest {
        /// Path relative to the root.
        path: String,
        /// The parser error.
        error: ManifestError,
    },
    /// A manifest has no `[package] name`.
    NoName(String),
    /// A members pattern is not `dir/name` or `dir/*`, or leaves the workspace.
    BadMember(String),
    /// More members or source files than the limits allow.
    TooMany(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "cannot read `{path}`: {source}"),
            Self::TooLarge(path) => write!(f, "`{path}` is larger than the 1 MiB limit"),
            Self::Manifest { path, error } => write!(f, "`{path}`: {error}"),
            Self::NoName(path) => write!(f, "`{path}` has no [package] name"),
            Self::BadMember(pattern) => write!(f, "unsupported workspace member `{pattern}`"),
            Self::TooMany(what) => write!(f, "too many {what}"),
        }
    }
}

impl std::error::Error for LoadError {}

fn read_bounded(root: &Path, rel: &str) -> Result<String, LoadError> {
    let io = |source| LoadError::Io {
        path: rel.to_owned(),
        source,
    };
    let file = fs::File::open(root.join(rel)).map_err(io)?;
    let mut text = String::new();
    let limit = u64::try_from(MAX_MANIFEST_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    file.take(limit).read_to_string(&mut text).map_err(io)?;
    if text.len() > MAX_MANIFEST_BYTES {
        return Err(LoadError::TooLarge(rel.to_owned()));
    }
    Ok(text)
}

fn valid_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment.contains(['\\', ':', '*', '?'])
}

/// Expands one `members` entry: a literal `a/b` or a one-level glob `a/*`.
fn expand(root: &Path, pattern: &str) -> Result<Vec<String>, LoadError> {
    let bad = || LoadError::BadMember(pattern.to_owned());
    if let Some(dir) = pattern.strip_suffix("/*") {
        if dir.is_empty() || !dir.split('/').all(valid_segment) {
            return Err(bad());
        }
        let io = |source| LoadError::Io {
            path: dir.to_owned(),
            source,
        };
        let mut found = Vec::new();
        for entry in fs::read_dir(root.join(dir)).map_err(io)? {
            let entry = entry.map_err(io)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if valid_segment(&name) && entry.path().join("Cargo.toml").is_file() {
                found.push(format!("{dir}/{name}"));
            }
        }
        found.sort();
        Ok(found)
    } else if pattern.split('/').all(valid_segment) {
        Ok(vec![pattern.to_owned()])
    } else {
        Err(bad())
    }
}

fn collect_sources(
    root: &Path,
    rel: &str,
    depth: usize,
    out: &mut Vec<Source>,
) -> Result<(), LoadError> {
    let io = |source| LoadError::Io {
        path: rel.to_owned(),
        source,
    };
    let mut entries: Vec<(String, bool, bool)> = Vec::new();
    for entry in fs::read_dir(root.join(rel)).map_err(io)? {
        let entry = entry.map_err(io)?;
        let kind = entry.file_type().map_err(io)?;
        entries.push((
            entry.file_name().to_string_lossy().into_owned(),
            kind.is_dir(),
            kind.is_file(),
        ));
    }
    entries.sort();
    for (name, is_dir, is_file) in entries {
        let child = format!("{rel}/{name}");
        if is_dir && depth < MAX_DEPTH {
            collect_sources(root, &child, depth.saturating_add(1), out)?;
        } else if is_file && name.ends_with(".rs") {
            if out.len() >= MAX_SOURCE_FILES {
                return Err(LoadError::TooMany("source files".to_owned()));
            }
            let text = read_bounded(root, &child)?;
            out.push(Source { path: child, text });
        }
    }
    Ok(())
}

/// Loads the workspace rooted at `root` (the directory with the workspace `Cargo.toml`).
///
/// # Errors
///
/// [`LoadError`] for unreadable, oversized or unsupported input.
pub fn load(root: &Path) -> Result<Workspace, LoadError> {
    let root_text = read_bounded(root, "Cargo.toml")?;
    let root_manifest = parse_manifest(&root_text).map_err(|error| LoadError::Manifest {
        path: "Cargo.toml".to_owned(),
        error,
    })?;
    let mut paths = Vec::new();
    for pattern in &root_manifest.members {
        paths.extend(expand(root, pattern)?);
    }
    if paths.len() > MAX_MEMBERS {
        return Err(LoadError::TooMany("workspace members".to_owned()));
    }
    paths.sort();
    paths.dedup();
    let mut packages = Vec::new();
    for path in paths {
        let manifest_path = format!("{path}/Cargo.toml");
        let text = read_bounded(root, &manifest_path)?;
        let manifest = parse_manifest(&text).map_err(|error| LoadError::Manifest {
            path: manifest_path.clone(),
            error,
        })?;
        let name = manifest.name.ok_or(LoadError::NoName(manifest_path))?;
        let mut sources = Vec::new();
        let src = format!("{path}/src");
        if PathBuf::from(root).join(&src).is_dir() {
            collect_sources(root, &src, 0, &mut sources)?;
        }
        packages.push(Package {
            name,
            path,
            publish_false: manifest.publish_false,
            dependencies: manifest.dependencies,
            sources,
        });
    }
    Ok(Workspace { packages })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn members_must_stay_inside_the_workspace() {
        let root = Path::new(".");
        for bad in [
            "../x", "/abs", "a/../b", "a\\b", "C:/x", "*", "a/*/b", "/*", "*/x", "",
        ] {
            assert!(
                matches!(expand(root, bad), Err(LoadError::BadMember(_))),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn a_literal_member_is_returned_as_is() {
        assert_eq!(
            expand(Path::new("."), "tools/fixtures").unwrap(),
            ["tools/fixtures"]
        );
    }
}
