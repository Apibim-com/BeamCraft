// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! E03 headless command `bom-export`: the verified head revision's bill of materials as CSV
//! files. It calls the engine's pure tables through `beamcraft_cli::bom` and prints one JSON
//! object with the same error shape and exit codes as the other commands.

use beamcraft_cli::bom::rows;
use beamcraft_cli::plate::PlateError;
use beamcraft_cli::plate::inspect;
use beamcraft_engine::bom::{elements_csv, summary_csv, tables};
use beamcraft_engine::limits::MAX_ELEMENTS;
use beamcraft_host::{revision::RevisionStore, storage::BlobStore};
use beamcraft_model::error::ErrorCode;
use beamcraft_protocol::json::Json;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::Write;

type Run<T> = Result<T, PlateError>;

fn invalid(message: &'static str) -> PlateError {
    PlateError {
        code: ErrorCode::InvalidArgument,
        message,
    }
}

/// Whether `command` is `bom-export`.
pub fn handles(command: &OsString) -> bool {
    command == "bom-export"
}

/// Runs `bom-export` and returns the JSON result and the process exit code.
pub fn execute(args: &[OsString]) -> (Json, u8) {
    match run(args) {
        Ok(json) => (json, 0),
        Err(e) => (
            Json::object([
                (
                    "outcome",
                    Json::text(if e.code == ErrorCode::OutcomeUnknown {
                        "outcome_unknown"
                    } else {
                        "rejected"
                    }),
                ),
                ("code", Json::text(e.code.as_str())),
                ("message", Json::text(e.message)),
                ("transport_code", Json::Null),
                ("observed_context", Json::Null),
                ("diagnostics", Json::Array(vec![])),
            ]),
            match e.code {
                ErrorCode::OutcomeUnknown => 4,
                ErrorCode::BackendUnavailable => 3,
                _ => 2,
            },
        ),
    }
}

/// Creates a new file (never overwrites) and writes the bytes, synced.
fn write_new(path: &str, bytes: &str) -> Run<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| invalid("the output file exists or cannot be created"))?;
    file.write_all(bytes.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|_| PlateError {
            code: ErrorCode::BackendUnavailable,
            message: "the report could not be written",
        })
}

fn run(args: &[OsString]) -> Run<Json> {
    let args: Vec<&str> = args
        .iter()
        .skip(1)
        .map(|a| {
            a.to_str()
                .filter(|s| s.len() <= 4096)
                .ok_or_else(|| invalid("argument is not bounded UTF-8"))
        })
        .collect::<Run<_>>()?;
    if !args.len().is_multiple_of(2) || args.len() > 6 {
        return Err(invalid("options are KEY VALUE pairs"));
    }
    let mut options = BTreeMap::new();
    for pair in args.as_chunks::<2>().0 {
        let [key, value] = pair;
        if !matches!(*key, "--store" | "--out" | "--summary")
            || options.insert(*key, *value).is_some()
        {
            return Err(invalid("unknown or repeated option"));
        }
    }
    let store = options
        .get("--store")
        .ok_or_else(|| invalid("--store is required"))?;
    let out = options
        .get("--out")
        .ok_or_else(|| invalid("--out is required"))?;
    let blobs = BlobStore::new(store).map_err(|e| PlateError {
        code: e.code(),
        message: "store root is not usable",
    })?;
    let inspection = inspect(&RevisionStore::new(blobs))?;
    let bom = tables(&rows(&inspection)?, MAX_ELEMENTS)?;
    // Both files are rendered before the first one is created, so a rendering failure writes
    // nothing.
    let elements = elements_csv(&bom);
    let summary = options
        .get("--summary")
        .map(|path| (*path, summary_csv(&bom)));
    write_new(out, &elements)?;
    if let Some((path, bytes)) = &summary {
        write_new(path, bytes)?;
    }
    Ok(Json::object([
        ("outcome", Json::text("bom_exported")),
        (
            "revision_id",
            Json::text(inspection.revision.revision_id.clone()),
        ),
        ("elements", Json::uint(bom.elements.len() as u64)),
        ("groups", Json::uint(bom.groups.len() as u64)),
        ("elements_file", Json::text((*out).to_owned())),
        (
            "summary_file",
            options
                .get("--summary")
                .map_or(Json::Null, |p| Json::text((*p).to_owned())),
        ),
    ]))
}
