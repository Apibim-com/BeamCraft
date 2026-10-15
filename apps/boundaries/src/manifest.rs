// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! A small, bounded reader for the parts of `Cargo.toml` that the boundary rules need.
//!
//! It understands `[package]` (`name`, `publish`), `[workspace]` (`members`) and every dependency
//! table form: `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, their
//! `[target.<cfg>.*]` variants and the `[dependencies.<name>]` sub-table form. It is not a general
//! TOML parser; anything else is ignored.

use std::fmt;

/// Largest manifest accepted, bytes.
pub const MAX_MANIFEST_BYTES: usize = 1 << 20;
/// Most lines read from one manifest.
pub const MAX_LINES: usize = 10_000;

/// One dependency edge declared by a manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    /// The package name (after any `package = "..."` rename).
    pub name: String,
    /// The table it was declared in, for example `dependencies` or `target.'cfg(unix)'.dev-dependencies`.
    pub section: String,
}

/// What the boundary rules need from one manifest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Manifest {
    /// `[package] name`, if the manifest has a package.
    pub name: Option<String>,
    /// `[package] publish = false`.
    pub publish_false: bool,
    /// `[workspace] members` patterns.
    pub members: Vec<String>,
    /// Declared dependencies of every table, in file order.
    pub dependencies: Vec<Dependency>,
}

/// Why a manifest was not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestError {
    /// The text is larger than [`MAX_MANIFEST_BYTES`].
    TooLarge,
    /// The text has more than [`MAX_LINES`] lines.
    TooManyLines,
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => f.write_str("manifest is larger than the 1 MiB limit"),
            Self::TooManyLines => f.write_str("manifest has more than 10000 lines"),
        }
    }
}

impl std::error::Error for ManifestError {}

/// Removes a trailing `#` comment that is not inside a string.
fn strip_comment(line: &str) -> &str {
    let mut quote: Option<char> = None;
    for (i, c) in line.char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), _) if q == c => quote = None,
            (None, '#') => return line.get(..i).unwrap_or(line),
            _ => {}
        }
    }
    line
}

/// Every double- or single-quoted string in `text`.
fn quoted_strings(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Option<(char, String)> = None;
    for c in text.chars() {
        match current.take() {
            None if c == '"' || c == '\'' => current = Some((c, String::new())),
            None => {}
            Some((q, done)) if q == c => out.push(done),
            Some((q, mut s)) => {
                s.push(c);
                current = Some((q, s));
            }
        }
    }
    out
}

/// The value of a single quoted string, such as `"x"`.
fn string_value(value: &str) -> Option<String> {
    let strings = quoted_strings(value);
    match strings.as_slice() {
        [one] if value.trim().len() == one.len().saturating_add(2) => Some(one.clone()),
        _ => None,
    }
}

/// `package = "real"` inside an inline table value, if present.
fn inline_package(value: &str) -> Option<String> {
    let at = value.find("package")?;
    let rest = value.get(at.checked_add("package".len())?..)?;
    let rest = rest.trim_start().strip_prefix('=')?;
    quoted_strings(rest).into_iter().next()
}

/// Splits a table header into dot-separated tokens, keeping quoted tokens whole.
fn header_tokens(header: &str) -> Vec<String> {
    let mut tokens = vec![String::new()];
    let mut quote: Option<char> = None;
    for c in header.chars() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), _) if q == c => quote = None,
            (None, '.') => tokens.push(String::new()),
            (_, _) => {
                if let Some(last) = tokens.last_mut() {
                    last.push(c);
                }
            }
        }
    }
    tokens.into_iter().map(|t| t.trim().to_owned()).collect()
}

fn is_dependency_table(token: &str) -> bool {
    matches!(
        token,
        "dependencies" | "dev-dependencies" | "build-dependencies"
    )
}

/// How the current section relates to dependencies.
enum Section {
    Other,
    Package,
    Workspace,
    /// A table of `name = value` lines.
    Deps(String),
    /// A `[dependencies.name]` sub-table for one dependency.
    Dep(usize),
}

/// Reads the facts the boundary rules need from `text`.
///
/// # Errors
///
/// [`ManifestError`] if the text exceeds the size or line limits.
pub fn parse_manifest(text: &str) -> Result<Manifest, ManifestError> {
    if text.len() > MAX_MANIFEST_BYTES {
        return Err(ManifestError::TooLarge);
    }
    if text.lines().take(MAX_LINES.saturating_add(1)).count() > MAX_LINES {
        return Err(ManifestError::TooManyLines);
    }
    let mut manifest = Manifest::default();
    let mut section = Section::Other;
    let mut lines = text.lines();
    while let Some(raw) = lines.next() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(header) = line.strip_prefix('[') {
            let header = header
                .trim_start_matches('[')
                .trim_end()
                .trim_end_matches(']');
            section = classify(header, &mut manifest);
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match &section {
            Section::Package => match key {
                "name" => manifest.name = string_value(value),
                "publish" => manifest.publish_false = value == "false",
                _ => {}
            },
            Section::Workspace if key == "members" => {
                let mut array = value.to_owned();
                while !array.contains(']') {
                    let Some(next) = lines.next() else { break };
                    array.push(' ');
                    array.push_str(strip_comment(next));
                }
                manifest.members = quoted_strings(&array);
            }
            Section::Deps(name) => {
                let declared = key.split('.').next().unwrap_or(key).trim();
                let declared = declared.trim_matches(|c| c == '"' || c == '\'');
                let real = inline_package(value).unwrap_or_else(|| declared.to_owned());
                push_unique(&mut manifest, real, name);
            }
            Section::Dep(at) if key == "package" => {
                if let (Some(real), Some(dep)) =
                    (string_value(value), manifest.dependencies.get_mut(*at))
                {
                    dep.name = real;
                }
            }
            _ => {}
        }
    }
    Ok(manifest)
}

fn push_unique(manifest: &mut Manifest, name: String, section: &str) {
    let exists = manifest
        .dependencies
        .iter()
        .any(|d| d.name == name && d.section == section);
    if !exists {
        manifest.dependencies.push(Dependency {
            name,
            section: section.to_owned(),
        });
    }
}

fn classify(header: &str, manifest: &mut Manifest) -> Section {
    let tokens = header_tokens(header);
    let strs: Vec<&str> = tokens.iter().map(String::as_str).collect();
    match strs.as_slice() {
        ["package"] => Section::Package,
        ["workspace"] => Section::Workspace,
        ["workspace", ..] => Section::Other,
        [.., last] if is_dependency_table(last) => Section::Deps(header.to_owned()),
        [.., table, name] if is_dependency_table(table) => {
            let section = tokens
                .iter()
                .take(tokens.len().saturating_sub(1))
                .cloned()
                .collect::<Vec<_>>()
                .join(".");
            manifest.dependencies.push(Dependency {
                name: (*name).to_owned(),
                section,
            });
            Section::Dep(manifest.dependencies.len().saturating_sub(1))
        }
        _ => Section::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deps(m: &Manifest) -> Vec<(&str, &str)> {
        m.dependencies
            .iter()
            .map(|d| (d.name.as_str(), d.section.as_str()))
            .collect()
    }

    #[test]
    fn reads_package_and_inherited_dependencies() {
        let m = parse_manifest(
            "[package]\nname = \"beamcraft-model\" # c\npublish = false\n\n\
             [dependencies]\nbeamcraft-geom.workspace = true\nserde = { version = \"1\" }\n\n\
             [lints]\nworkspace = true\n",
        )
        .unwrap();
        assert_eq!(m.name.as_deref(), Some("beamcraft-model"));
        assert!(m.publish_false);
        assert_eq!(
            deps(&m),
            [
                ("beamcraft-geom", "dependencies"),
                ("serde", "dependencies")
            ]
        );
    }

    #[test]
    fn reads_every_dependency_table_form() {
        let m = parse_manifest(
            "[dev-dependencies]\na = \"1\"\n[build-dependencies]\nb = \"1\"\n\
             [target.'cfg(unix)'.dependencies]\nc = \"1\"\n\
             [target.x86_64-pc-windows-msvc.dev-dependencies]\nd = \"1\"\n\
             [dependencies.e]\nversion = \"1\"\n\
             [dependencies.f]\npackage = \"real-f\"\n",
        )
        .unwrap();
        let names: Vec<&str> = m.dependencies.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["a", "b", "c", "d", "e", "real-f"]);
        assert_eq!(
            m.dependencies.get(2).map(|d| d.section.as_str()),
            Some("target.'cfg(unix)'.dependencies")
        );
    }

    #[test]
    fn renamed_dependency_uses_the_real_package_name() {
        let m = parse_manifest(
            "[dependencies]\nalias = { package = \"beamcraft-cli\", path = \"x\" }\n",
        )
        .unwrap();
        assert_eq!(deps(&m), [("beamcraft-cli", "dependencies")]);
    }

    #[test]
    fn workspace_tables_are_not_dependencies() {
        let m = parse_manifest(
            "[workspace]\nresolver = \"3\"\nmembers = [\n  \"crates/*\", # c\n  \"tools/fixtures\",\n]\n\
             [workspace.dependencies]\nbeamcraft-geom = { path = \"crates/geom\" }\n\
             [workspace.package]\nname = \"not-a-package\"\n",
        )
        .unwrap();
        assert_eq!(m.members, ["crates/*", "tools/fixtures"]);
        assert!(m.dependencies.is_empty());
        assert_eq!(m.name, None);
    }

    #[test]
    fn comments_and_hashes_inside_strings() {
        assert_eq!(strip_comment("a = \"x#y\" # z"), "a = \"x#y\" ");
        assert_eq!(strip_comment("# only"), "");
    }

    #[test]
    fn repeated_keys_of_one_dependency_count_once() {
        let m = parse_manifest("[dependencies]\nx.workspace = true\nx.optional = true\n").unwrap();
        assert_eq!(m.dependencies.len(), 1);
    }

    #[test]
    fn rejects_oversized_input() {
        let big = "#".repeat(MAX_MANIFEST_BYTES + 1);
        assert_eq!(parse_manifest(&big), Err(ManifestError::TooLarge));
        let many = "\n".repeat(MAX_LINES + 1);
        assert_eq!(parse_manifest(&many), Err(ManifestError::TooManyLines));
        assert!(parse_manifest(&"\n".repeat(MAX_LINES)).is_ok());
    }

    #[test]
    fn garbage_does_not_panic() {
        for text in [
            "[",
            "[[",
            "]",
            "=",
            "a = [",
            "[dependencies",
            "[workspace]\nmembers = [",
            "\"",
            "'''",
        ] {
            let _ = parse_manifest(text);
        }
    }
}
