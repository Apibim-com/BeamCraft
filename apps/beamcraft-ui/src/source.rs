// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Loading the accepted snapshot: the headless CLI exports it in a separate process
//! (`beamcraft view-export --store ROOT --out -`) and the engine rebuilds and re-validates it.
//! The viewport never opens USD itself and never trusts the bytes it receives.

use crate::scene::Scene;
use crate::tools::{Definitions, ToolError};
use crate::{Error, command};
use beamcraft_engine::query::bundle::max_bundle_bytes;
use beamcraft_engine::{Limits, Representation, ViewSnapshot};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The `beamcraft` CLI next to the running executable.
pub fn default_cli() -> Result<PathBuf, Error> {
    let exe = std::env::current_exe().map_err(Error::Io)?;
    let dir = exe
        .parent()
        .ok_or(Error::Input("executable has no directory"))?;
    Ok(dir.join(format!("beamcraft{}", std::env::consts::EXE_SUFFIX)))
}

/// The pinned view snapshot of the published head of `store`. The CLI runs under the same
/// deadline ([`command::DEADLINE`]) as the create commands, so a hung CLI ends in a typed error.
pub fn load(cli: &Path, store: &Path) -> Result<ViewSnapshot, Error> {
    load_within(cli, store, command::DEADLINE)
}

/// [`load`] with an explicit deadline.
pub fn load_within(cli: &Path, store: &Path, deadline: Duration) -> Result<ViewSnapshot, Error> {
    let limits = Limits::default();
    let cap = max_bundle_bytes(&limits);
    let args = [
        "view-export".to_owned(),
        "--store".into(),
        store.to_string_lossy().into_owned(),
        "--out".into(),
        "-".into(),
    ];
    let output = command::run_capped(cli, &args, deadline, cap).map_err(export_error)?;
    if output.code != Some(0) {
        // A rejection is one JSON object; report its stable code only.
        let text = String::from_utf8_lossy(&output.stdout);
        let code = text
            .split("\"code\":\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .filter(|c| c.len() <= 32 && c.bytes().all(|b| b.is_ascii_uppercase() || b == b'_'))
            .unwrap_or("UNKNOWN");
        return Err(Error::Cli(format!("view-export rejected: {code}")));
    }
    Ok(ViewSnapshot::from_bundle(&output.stdout, &limits)?)
}

/// A runner failure (no CLI, deadline, oversize output) as the viewport's typed error.
fn export_error(e: ToolError) -> Error {
    match e.code.as_str() {
        "OUTCOME_UNKNOWN" => {
            Error::Cli("view-export did not finish in time and was stopped".into())
        }
        "CLI_FAILED" if e.message.contains("size limit") => {
            Error::Cli("view bundle exceeds the snapshot limits".into())
        }
        code => Error::Cli(format!("view-export failed: {code}")),
    }
}

/// True when the failure is only that the store has no published revision yet (`NOT_FOUND`):
/// a normal state of a new store, not an error to show in red.
pub fn is_no_revision(error: &Error) -> bool {
    matches!(error, Error::Cli(m) if m.ends_with("view-export rejected: NOT_FOUND"))
}
/// Everything one window refresh reads, gathered without a window or a GPU so it can run on a
/// worker thread: the accepted snapshot with its scene, and the store's definitions. The two
/// results are independent; one failing never hides the other.
pub struct Reloaded {
    /// The accepted snapshot and the scene built from it.
    pub scene: Result<(ViewSnapshot, Scene), Error>,
    /// The profile and material definitions of the same store.
    pub definitions: Result<Definitions, ToolError>,
}

/// Runs the CLI queries of one refresh (blocking; call it off the UI thread). The two queries
/// are independent CLI processes, so they run concurrently and the refresh waits for the slower
/// one instead of paying both full inspections in sequence (issue 26, finding 2).
pub fn reload(cli: &Path, store: &Path) -> Reloaded {
    let (scene, definitions) = std::thread::scope(|scope| {
        let scene = scope.spawn(|| {
            load(cli, store).and_then(|snapshot| {
                let scene = Scene::from_snapshot(&snapshot, Representation::Full)?;
                Ok((snapshot, scene))
            })
        });
        let definitions = scope.spawn(|| command::definitions(cli, store));
        // A panicked worker is a bug, not model data: report it as its typed failure.
        let scene = scene
            .join()
            .unwrap_or_else(|_| Err(Error::Cli("the view-export thread failed".into())));
        let definitions = definitions.join().unwrap_or_else(|_| {
            Err(ToolError::local(
                "CLI_FAILED",
                "the definitions thread failed",
            ))
        });
        (scene, definitions)
    });
    Reloaded { scene, definitions }
}
