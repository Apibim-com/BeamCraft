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
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The `beamcraft` CLI next to the running executable.
pub fn default_cli() -> Result<PathBuf, Error> {
    let exe = std::env::current_exe().map_err(Error::Io)?;
    let dir = exe
        .parent()
        .ok_or(Error::Input("executable has no directory"))?;
    Ok(dir.join(format!("beamcraft{}", std::env::consts::EXE_SUFFIX)))
}

/// The pinned view snapshot of the published head of `store`.
pub fn load(cli: &Path, store: &Path) -> Result<ViewSnapshot, Error> {
    let limits = Limits::default();
    let cap = max_bundle_bytes(&limits);
    let mut child = Command::new(cli)
        .arg("view-export")
        .arg("--store")
        .arg(store)
        .args(["--out", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(Error::Io)?;
    let mut bytes = Vec::new();
    let stdout = child
        .stdout
        .take()
        .ok_or(Error::Cli("no output pipe".into()))?;
    let read = stdout
        .take(u64::try_from(cap).unwrap_or(u64::MAX).saturating_add(1))
        .read_to_end(&mut bytes);
    let status = child.wait().map_err(Error::Io)?;
    read.map_err(Error::Io)?;
    if bytes.len() > cap {
        return Err(Error::Cli("view bundle exceeds the snapshot limits".into()));
    }
    if !status.success() {
        // A rejection is one JSON object; report its stable code only.
        let text = String::from_utf8_lossy(&bytes);
        let code = text
            .split("\"code\":\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .filter(|c| c.len() <= 32 && c.bytes().all(|b| b.is_ascii_uppercase() || b == b'_'))
            .unwrap_or("UNKNOWN");
        return Err(Error::Cli(format!("view-export rejected: {code}")));
    }
    Ok(ViewSnapshot::from_bundle(&bytes, &limits)?)
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

/// Runs the CLI queries of one refresh (blocking; call it off the UI thread).
pub fn reload(cli: &Path, store: &Path) -> Reloaded {
    let scene = load(cli, store).and_then(|snapshot| {
        let scene = Scene::from_snapshot(&snapshot, Representation::Full)?;
        Ok((snapshot, scene))
    });
    Reloaded {
        scene,
        definitions: command::definitions(cli, store),
    }
}
