// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! One foreign, read-only, self-contained USD layer: a library or an environment file.
//!
//! The bytes come from outside the project, so they are opened under fixed, tight budgets as a
//! single owned-by-nobody layer in millimetres and Z up. Nothing is written and nothing is
//! composed: a layer that needs sublayers or references is refused.

use crate::Refusal;
use beamcraft_layer_engine::{
    Asset, BACKEND, Budgets, Cancellation, Control, Identity, OpenRequest, Opened, Profile, Reader,
};
use beamcraft_model::digest::{ContentDigest, Sha256};
use beamcraft_model::error::ErrorCode;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

/// Largest library or environment layer, in bytes.
pub const MAX_LAYER_BYTES: usize = 8 * 1024 * 1024;
/// Most entries a library may hold.
pub const MAX_LIBRARY_ENTRIES: usize = 4096;

const LAYER: &str = "library.usda";
const LAYER_BINARY: &str = "library.usdc";
/// A fixed read-only identity for the stage of a foreign layer: it is not a project or a workspace.
const READ_PROJECT: &str = "01900000-0000-7000-8000-0000000000a7";
const READ_WORKSPACE: &str = "01900000-0000-7000-8000-0000000000a8";

/// The stage ceilings of a foreign layer: tighter than a model stage, one layer, a few thousand
/// prims.
const BUDGETS: Budgets = Budgets {
    layer_bytes: MAX_LAYER_BYTES,
    total_bytes: MAX_LAYER_BYTES,
    layers: 1,
    depth: 16,
    prims: 2 * MAX_LIBRARY_ENTRIES + 16,
    values: 262_144,
    array_items: 8192,
    text_bytes: 1024,
    decoded_bytes: 64 * 1024 * 1024,
    snapshot_bytes: 64 * 1024 * 1024,
    page_items: 64,
};

/// One layer opened read-only under the foreign-layer budgets.
pub struct ForeignLayer {
    /// The reader that holds the opened stage.
    pub reader: Reader,
    /// The opened stage: metadata and the root of the layer.
    pub opened: Opened,
    /// The cancellation and deadline the layer was opened under.
    pub control: Control,
    /// SHA-256 of the layer bytes.
    pub digest: ContentDigest,
    /// Size of the layer in bytes.
    pub bytes: u64,
}

fn digest_of(bytes: &[u8]) -> Result<ContentDigest, Refusal> {
    Sha256::digest(bytes).map_err(|_| Refusal::new(ErrorCode::LimitExceeded, "layer too large"))
}

fn control() -> Control {
    Control {
        cancellation: Cancellation::default(),
        deadline: Instant::now() + Duration::from_secs(60),
    }
}

fn request(name: &str, digest: ContentDigest, bytes: Vec<u8>) -> Result<OpenRequest, Refusal> {
    let asset = Asset {
        name: name.into(),
        digest,
        bytes,
    };
    // Foreign and read-only: the host owns nothing here, so no layer is owned.
    let mut request = OpenRequest {
        identity: Identity {
            project: READ_PROJECT.into(),
            workspace: READ_WORKSPACE.into(),
            revision: digest,
            generation: 0,
        },
        root: name.into(),
        assets: vec![asset],
        owned_layers: BTreeSet::new(),
        manifest: digest_of(b"placeholder")?,
        backend: BACKEND,
        profile: Profile::StaticLocal,
        budgets: BUDGETS,
    };
    request.manifest = request.manifest_digest()?;
    Ok(request)
}

/// Opens the bytes of a library or environment as one foreign, read-only, self-contained layer in
/// millimetres and Z up. `not_self_contained` is the `INVALID_ARGUMENT` text for a layer that the
/// engine cannot open on its own (sublayers, references, other units).
///
/// # Errors
/// A layer over the budgets, or one the engine refuses, is a typed [`Refusal`].
pub fn open(bytes: Vec<u8>, not_self_contained: &'static str) -> Result<ForeignLayer, Refusal> {
    let digest = digest_of(&bytes)?;
    let name = if bytes.starts_with(b"PXR-USDC") {
        LAYER_BINARY
    } else {
        LAYER
    };
    let size = bytes.len() as u64;
    let request = request(name, digest, bytes)?;
    let control = control();
    let mut reader = Reader::default();
    let opened = reader.open(request, &control).map_err(|diagnostic| {
        let error = Refusal::from(diagnostic);
        match error.code {
            ErrorCode::IntegrityError | ErrorCode::DependencyUnavailable => {
                Refusal::invalid(not_self_contained)
            }
            _ => error,
        }
    })?;
    Ok(ForeignLayer {
        reader,
        opened,
        control,
        digest,
        bytes: size,
    })
}
