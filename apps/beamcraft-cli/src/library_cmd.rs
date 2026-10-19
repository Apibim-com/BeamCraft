// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! A07.2 headless commands: `library-list` and `library-show`. They read one library file named
//! on the command line (no hidden global, no environment lookup yet), never a store, and print
//! one JSON object with the error shape and exit codes of the plate commands.

use crate::plate_cmd::{Run, definition_json, fit_page, invalid, page_of, report};
use beamcraft_cli::library::{LibraryEntry, LibraryInfo, RejectedEntry, load};
use beamcraft_protocol::json::Json;
use std::collections::BTreeMap;
use std::ffi::OsString;

/// Whether `command` is one of the library commands.
pub fn handles(command: &OsString) -> bool {
    command == "library-list" || command == "library-show"
}

/// Runs one library command and returns the JSON result and the process exit code.
pub fn execute(args: &[OsString]) -> (Json, u8) {
    report(run(args))
}

fn info_json(info: &LibraryInfo) -> Json {
    let optional = |value: &Option<String>| value.as_ref().map_or(Json::Null, Json::text);
    Json::object([
        ("id", Json::text(info.id.clone())),
        ("title", optional(&info.title)),
        ("version", optional(&info.version)),
        ("source", optional(&info.source)),
        ("licence", optional(&info.licence)),
        ("digest", Json::text(info.digest.to_string())),
        ("bytes", Json::uint(info.bytes)),
    ])
}

fn entry_json(entry: &LibraryEntry) -> Run<Json> {
    let digest = entry.record.resource()?.digest;
    let Json::Object(mut object) = definition_json(&digest, &entry.record)? else {
        return Err(invalid("entry result is not an object"));
    };
    // The digest is what `define-profile --library` would pin; the library prim is not a
    // definition path.
    object.remove("prim_path");
    object.insert("library_prim".into(), Json::text(entry.prim_path.clone()));
    object.insert("designation".into(), Json::text(entry.designation.clone()));
    Ok(Json::Object(object))
}

fn rejected_json<'a>(rejected: impl Iterator<Item = &'a RejectedEntry>) -> Json {
    Json::Array(
        rejected
            .map(|r| {
                Json::object([
                    ("prim_path", Json::text(r.prim_path.clone())),
                    (
                        "designation",
                        r.designation.as_ref().map_or(Json::Null, Json::text),
                    ),
                    ("code", Json::text(r.code.as_str())),
                    ("message", Json::text(r.message)),
                ])
            })
            .collect(),
    )
}

fn run(args: &[OsString]) -> Run<Json> {
    let args: Vec<&str> = args
        .iter()
        .map(|a| {
            a.to_str()
                .filter(|s| s.len() <= 4096)
                .ok_or_else(|| invalid("argument is not bounded UTF-8"))
        })
        .collect::<Run<_>>()?;
    let (command, tail) = args
        .split_first()
        .ok_or_else(|| invalid("missing command"))?;
    let allowed: &[&str] = match *command {
        "library-list" => &["--library", "--family", "--filter", "--offset", "--limit"],
        "library-show" => &["--library", "--designation"],
        _ => return Err(invalid("unknown command")),
    };
    if !tail.len().is_multiple_of(2) || tail.len() > 10 {
        return Err(invalid("options are KEY VALUE pairs"));
    }
    let mut options = BTreeMap::new();
    for pair in tail.as_chunks::<2>().0 {
        let [key, value] = pair;
        if !allowed.contains(key) || options.insert(*key, *value).is_some() {
            return Err(invalid("unknown or repeated option"));
        }
    }
    let library = load(
        options
            .get("--library")
            .ok_or_else(|| invalid("--library is required"))?,
    )?;
    if *command == "library-show" {
        let designation = options
            .get("--designation")
            .ok_or_else(|| invalid("--designation is required"))?;
        return Ok(Json::object([
            ("outcome", Json::text("library_entry")),
            ("library", info_json(&library.info)),
            ("entry", entry_json(library.entry(designation)?)?),
        ]));
    }
    let family = options.get("--family");
    let filter = options.get("--filter").map(|f| f.to_ascii_lowercase());
    let (offset, limit) = page_of(&options)?;
    let matching: Vec<&LibraryEntry> = library
        .entries
        .iter()
        .filter(|e| family.is_none_or(|f| e.record.kind() == *f))
        .filter(|e| {
            filter
                .as_ref()
                .is_none_or(|f| e.designation.to_ascii_lowercase().contains(f.as_str()))
        })
        .collect();
    // One position space: the matching entries, then the refused ones.
    let (entry_total, rejected_total) = (matching.len(), library.rejected.len());
    let total = entry_total.saturating_add(rejected_total);
    fit_page(total.saturating_sub(offset), limit, |count| {
        let entries_here = entry_total.saturating_sub(offset).min(count);
        let entries = matching
            .iter()
            .skip(offset)
            .take(entries_here)
            .map(|e| entry_json(e))
            .collect::<Run<Vec<_>>>()?;
        let rejected = rejected_json(
            library
                .rejected
                .iter()
                .skip(offset.saturating_sub(entry_total))
                .take(count - entries_here),
        );
        let next = offset.saturating_add(count);
        Ok(Json::object([
            ("outcome", Json::text("library")),
            ("library", info_json(&library.info)),
            ("entry_count", Json::uint(entry_total as u64)),
            ("rejected_count", Json::uint(rejected_total as u64)),
            ("offset", Json::uint(offset as u64)),
            ("page_size", Json::uint(count as u64)),
            (
                "next_offset",
                if next < total {
                    Json::uint(next as u64)
                } else {
                    Json::Null
                },
            ),
            ("entries", Json::Array(entries)),
            ("rejected", rejected),
        ]))
    })
}
