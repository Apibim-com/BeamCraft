// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! U06 headless revision commands over a protected store directory. They call
//! `beamcraft_host::revision` and parse no USD: layer content checks need the layer engine,
//! which runs in its own process and is not wired to these commands (see the README).

use beamcraft_host::{
    revision::{
        CommitOutcome, ExpectedBase, LayerEntry, LayerRole, RevisionDraft, RevisionError,
        RevisionInfo, RevisionStore, Uuid7,
    },
    storage::BlobStore,
    worker::HostError,
};
use beamcraft_model::{ErrorCode, digest::ContentDigest};
use beamcraft_protocol::json::Json;
use std::collections::BTreeMap;

/// The shell argument cap (4096 bytes) already bounds the list; this is a clearer limit.
const MAX_CLI_LAYERS: usize = 16;

fn invalid() -> HostError {
    HostError::logical(ErrorCode::InvalidArgument)
}

fn failure(error: RevisionError) -> HostError {
    HostError::logical(error.code())
}

/// Canonical unsigned decimal, as the logical `WorkspaceGeneration` form requires.
fn generation(text: &str) -> Option<u64> {
    let canonical = !text.is_empty()
        && text.bytes().all(|b| b.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    if canonical { text.parse().ok() } else { None }
}

fn summary(outcome: &str, info: &RevisionInfo) -> Json {
    let manifest = info.manifest();
    let content = manifest.content();
    let layers = content
        .entries()
        .iter()
        .map(|e| {
            Json::object([
                ("role", Json::text(e.role().as_str())),
                ("name", Json::text(e.name())),
                ("digest", Json::text(e.digest().to_string())),
                ("size", Json::text(e.size().to_string())),
            ])
        })
        .collect();
    Json::object([
        ("outcome", Json::text(outcome)),
        ("revision_id", Json::text(info.revision().as_str())),
        ("sequence", Json::text(info.sequence().to_string())),
        (
            "manifest_digest",
            Json::text(info.manifest_digest().to_string()),
        ),
        ("project_id", Json::text(content.project().as_str())),
        ("workspace_id", Json::text(content.workspace().as_str())),
        ("op_id", Json::text(content.op().as_str())),
        (
            "workspace_generation",
            Json::text(content.workspace_generation().to_string()),
        ),
        ("schema", Json::text(content.schema().to_string())),
        ("root_layer", Json::text(content.root_layer())),
        (
            "base",
            manifest.base().map_or(Json::Null, |b| {
                Json::object([
                    ("revision_id", Json::text(b.revision.as_str())),
                    ("manifest_digest", Json::text(b.manifest.to_string())),
                ])
            }),
        ),
        (
            "index",
            manifest.index().map_or(Json::Null, |i| {
                Json::object([
                    ("structure_digest", Json::text(i.structure.to_string())),
                    ("prim_count", Json::text(i.prims.to_string())),
                ])
            }),
        ),
        ("layers", Json::Array(layers)),
    ])
}

/// `revision-commit`: all layer blobs must already be in the store (`blob-put`). Each
/// `--layers` item is `ROLE:NAME:SHA256`; the size is taken from the verified stored bytes.
pub(crate) fn commit(blobs: &BlobStore, options: &BTreeMap<&str, &str>) -> Result<Json, HostError> {
    let get = |key: &str| options.get(key).copied().ok_or_else(invalid);
    let uuid = |key: &str| Uuid7::parse(get(key)?).map_err(failure);
    let digest = |key: &str| get(key)?.parse::<ContentDigest>().map_err(|_| invalid());
    let mut entries = Vec::new();
    for item in get("--layers")?.split(',') {
        if entries.len() >= MAX_CLI_LAYERS {
            return Err(HostError::logical(ErrorCode::LimitExceeded));
        }
        let mut parts = item.split(':');
        let (Some(role), Some(name), Some(layer), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(invalid());
        };
        let layer = layer.parse::<ContentDigest>().map_err(|_| invalid())?;
        let size = blobs
            .read(&layer)
            .map_err(|e| HostError::logical(e.code()))?
            .len();
        entries.push(
            LayerEntry::new(
                name,
                LayerRole::parse(role).ok_or_else(invalid)?,
                layer,
                u64::try_from(size).map_err(|_| invalid())?,
            )
            .map_err(failure)?,
        );
    }
    let expected = match get("--base")? {
        "none" => ExpectedBase::Initial,
        id => ExpectedBase::Revision(Uuid7::parse(id).map_err(failure)?),
    };
    let draft = RevisionDraft {
        project: uuid("--project-id")?,
        workspace: uuid("--workspace-id")?,
        revision: uuid("--revision-id")?,
        op: uuid("--op-id")?,
        workspace_generation: generation(get("--generation")?).ok_or_else(invalid)?,
        schema: digest("--schema")?,
        root_layer: get("--root-layer")?.to_owned(),
        entries,
    };
    let committed = RevisionStore::new(blobs.clone())
        .commit(&draft, &expected)
        .map_err(failure)?;
    Ok(summary(
        match committed.outcome {
            CommitOutcome::Created => "revision_committed",
            CommitOutcome::Replayed => "revision_replayed",
        },
        &committed.info,
    ))
}

/// `revision-open`: opens the current root and verifies the manifest and every layer blob.
pub(crate) fn open(blobs: &BlobStore) -> Result<Json, HostError> {
    let revision = RevisionStore::new(blobs.clone()).open().map_err(failure)?;
    Ok(summary("revision_verified", revision.info()))
}
