// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! U20 checkout of a verified model revision into a directory of ordinary USD layers.

use crate::coordination::revision_descriptor;
use beamcraft_engine::scope::check_code;
use beamcraft_host::output::{Installed, OutputError, resolve_destination, stage_file};
use beamcraft_host::revision::{LayerRole, Revision, RevisionStore, Uuid7};
use beamcraft_model::error::ErrorCode;
use session::Refusal;
use session::pipeline::AUTHORED_LAYER;
use session::project::check_project_name;
use std::collections::BTreeSet;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

/// The exact revision and entry point written by a checkout.
pub struct Checkout {
    /// The revision exported from the store.
    pub revision: Uuid7,
    /// Absolute path to the coordination root in the output directory.
    pub root_layer: PathBuf,
    /// Number of layer files written.
    pub layer_count: u64,
}

/// Exports an exact retained revision, or the head captured when the command runs.
///
/// The destination must be a new directory with an existing parent. The output keeps the
/// manifest's relative layer paths, renaming only the root when a project code is available.
/// Every blob is verified. The entry point is written after its dependencies.
pub fn checkout_revision(
    store: &RevisionStore,
    revision: Option<&Uuid7>,
    out: &Path,
) -> Result<Checkout, Refusal> {
    let destination = resolve_destination(out).map_err(output_error)?;
    let out = destination.directory.join(destination.name);
    if out.to_str().is_none() {
        return Err(Refusal::invalid("the checkout path is not UTF-8"));
    }
    if out.starts_with(store.blobs().canonical_root()) {
        return Err(Refusal::invalid(
            "the checkout must be outside the source store",
        ));
    }
    let revision = match revision {
        Some(id) => {
            let sequence = store.sequence_of(id)?.ok_or_else(|| {
                Refusal::new(
                    ErrorCode::NotFound,
                    "the revision is not retained in the store",
                )
            })?;
            let revision = store.open_at(sequence)?;
            if revision.info().revision() != id {
                return Err(Refusal::new(
                    ErrorCode::BaseRevisionMismatch,
                    "the revision changed during checkout",
                ));
            }
            revision
        }
        None => store.open()?,
    };
    let manifest = revision.manifest().content();
    let root = manifest.root_layer();
    let output_root = output_root_name(&revision)?;
    portable_name(&output_root)?;
    for entry in manifest.entries() {
        portable_name(Path::new(entry.name()))?;
        if entry.name() != root
            && output_root
                .to_str()
                .is_some_and(|name| entry.name().eq_ignore_ascii_case(name))
        {
            return Err(Refusal::new(
                ErrorCode::DuplicateId,
                "the coordination root name collides with a layer",
            ));
        }
    }
    let layer_count = u64::try_from(manifest.entries().len())
        .map_err(|_| Refusal::new(ErrorCode::LimitExceeded, "too many checkout layers"))?;

    fs::create_dir(&out).map_err(directory_error)?;
    let mut directories = BTreeSet::new();
    let mut installed = Vec::new();
    let result = (|| {
        for entry in manifest
            .entries()
            .iter()
            .filter(|entry| entry.name() != root)
        {
            let path = Path::new(entry.name());
            output_parents(&out, path, &mut directories)?;
            installed.push(write_layer(&revision, entry.name(), &out.join(path))?);
        }
        output_parents(&out, &output_root, &mut directories)?;
        let root_layer = out.join(&output_root);
        write_layer(&revision, root, &root_layer)?;
        Ok(Checkout {
            revision: revision.info().revision().clone(),
            root_layer,
            layer_count,
        })
    })();
    if result.is_err() {
        for file in installed.into_iter().rev() {
            file.undo();
        }
        for directory in directories.into_iter().rev() {
            let _ = fs::remove_dir(directory);
        }
        let _ = fs::remove_dir(&out);
    }
    result
}

fn output_root_name(revision: &Revision) -> Result<PathBuf, Refusal> {
    let manifest = revision.manifest().content();
    let root = Path::new(manifest.root_layer());
    let descriptor = revision_descriptor(revision)?;
    let code = descriptor
        .as_ref()
        .map(|descriptor| descriptor.metadata.code.as_str())
        .filter(|code| !code.is_empty())
        .or_else(|| {
            // Sectioned projects already record the layout code in their authored layer names.
            manifest.entries().iter().find_map(|entry| {
                if entry.role() != LayerRole::Authored
                    || entry.name() == manifest.root_layer()
                    || entry.name() == AUTHORED_LAYER
                {
                    return None;
                }
                entry.name().split_once('_').map(|(code, _)| code)
            })
        });
    match code {
        Some(code) => {
            check_code(code)?;
            Ok(root.with_file_name(format!("{code}_Coord.usda")))
        }
        None => Ok(root.to_path_buf()),
    }
}

fn portable_name(path: &Path) -> Result<(), Refusal> {
    for segment in path.components() {
        let name = segment
            .as_os_str()
            .to_str()
            .ok_or_else(|| Refusal::invalid("a layer name is not UTF-8"))?;
        check_project_name(name)?;
    }
    Ok(())
}

fn output_parents(
    out: &Path,
    layer: &Path,
    directories: &mut BTreeSet<PathBuf>,
) -> Result<(), Refusal> {
    let parent = layer
        .parent()
        .ok_or_else(|| Refusal::invalid("a checkout layer has no parent"))?;
    let mut directory = out.to_path_buf();
    for component in parent.components() {
        directory.push(component);
        if !directories.contains(&directory) {
            fs::create_dir(&directory).map_err(directory_error)?;
            directories.insert(directory.clone());
        }
    }
    Ok(())
}

fn write_layer(revision: &Revision, name: &str, path: &Path) -> Result<Installed, Refusal> {
    let bytes = revision.layer_bytes(name)?;
    stage_file(path, false, |file| {
        file.write_all(&bytes).map_err(|_| OutputError {
            code: ErrorCode::BackendUnavailable,
            message: "a checkout layer could not be written",
        })
    })
    .and_then(|staged| staged.install())
    .map_err(output_error)
}

fn output_error(error: OutputError) -> Refusal {
    Refusal::new(error.code, error.message)
}

fn directory_error(error: std::io::Error) -> Refusal {
    match error.kind() {
        ErrorKind::AlreadyExists => Refusal::invalid("the checkout destination already exists"),
        ErrorKind::PermissionDenied => Refusal::new(
            ErrorCode::PermissionDenied,
            "the checkout directory is not writable",
        ),
        _ => Refusal::new(
            ErrorCode::BackendUnavailable,
            "the checkout directory could not be created",
        ),
    }
}
