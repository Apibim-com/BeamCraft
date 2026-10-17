// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! U06 headless revision commands over a protected store directory. They call
//! `beamcraft_host::revision` and parse no USD: layer content checks need the layer engine,
//! which runs in its own process and is not wired to these commands (see the README).

use beamcraft_host::{
    ledger::{Outcome, OutcomeSource, RecoveryPolicy},
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

/// Most unreferenced digests `revision-recover` prints; the total is always exact.
const MAX_CLI_UNREFERENCED: usize = 256;

fn uuids(ids: &[Uuid7]) -> Json {
    Json::Array(ids.iter().map(|id| Json::text(id.as_str())).collect())
}

/// `revision-recover`: closes every unresolved intent from the root, removes abandoned
/// temporary files, reports unreferenced blobs (never deletes one) and verifies the root.
/// `--min-age-secs 0` asserts that no commit is running on the store; a larger value leaves
/// younger intents and files alone. Exit 0 with `recovery_incomplete` when something could not
/// be decided; the report says what.
pub(crate) fn recover(
    blobs: &BlobStore,
    options: &BTreeMap<&str, &str>,
) -> Result<Json, HostError> {
    let min_age = options
        .get("--min-age-secs")
        .copied()
        .and_then(generation)
        .ok_or_else(invalid)?;
    let store = RevisionStore::new(blobs.clone());
    let report = store
        .recover(RecoveryPolicy::live(min_age))
        .map_err(failure)?;
    let (revision, revision_error) = match store.open() {
        Ok(revision) => (summary("revision_verified", revision.info()), Json::Null),
        Err(RevisionError::NoRevision) => (Json::Null, Json::Null),
        Err(error) => (Json::Null, Json::text(error.code().as_str())),
    };
    let complete = report.unprovable.is_empty()
        && report.disagreements.is_empty()
        && report.in_flight.is_empty()
        && report.root_error.is_none()
        && revision_error == Json::Null;
    let unreferenced = report.unreferenced.as_ref().map_or(Json::Null, |u| {
        Json::object([
            ("total", Json::text(u.total.to_string())),
            (
                "digests",
                Json::Array(
                    u.digests
                        .iter()
                        .take(MAX_CLI_UNREFERENCED)
                        .map(|d| Json::text(d.to_string()))
                        .collect(),
                ),
            ),
        ])
    });
    Ok(Json::object([
        (
            "outcome",
            Json::text(if complete {
                "recovery_complete"
            } else {
                "recovery_incomplete"
            }),
        ),
        ("min_age_secs", Json::text(min_age.to_string())),
        ("committed", uuids(&report.committed)),
        ("aborted", uuids(&report.aborted)),
        ("in_flight", uuids(&report.in_flight)),
        (
            "unprovable",
            Json::Array(
                report
                    .unprovable
                    .iter()
                    .map(|(op, error)| {
                        Json::object([
                            ("op_id", Json::text(op.as_str())),
                            ("code", Json::text(error.code().as_str())),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("disagreements", uuids(&report.disagreements)),
        (
            "root_error",
            report
                .root_error
                .map_or(Json::Null, |e| Json::text(e.code().as_str())),
        ),
        (
            "removed_temp_files",
            Json::text(report.removed_temp_files.to_string()),
        ),
        ("unreferenced", unreferenced),
        (
            "ledger",
            Json::object([
                ("bytes", Json::text(report.ledger.bytes.to_string())),
                ("records", Json::text(report.ledger.records.to_string())),
                (
                    "damaged_lines",
                    Json::text(report.ledger.damaged_lines.to_string()),
                ),
                ("torn_tail", Json::Bool(report.ledger.torn_tail)),
            ]),
        ),
        ("revision", revision),
        ("revision_error", revision_error),
    ]))
}

/// `revision-outcome`: the host-ledger QueryOutcome for one op id. Read-only. A recorded
/// result is exit 0; `OUTCOME_UNKNOWN` is the standard unknown result (exit 4); a ledger that
/// contradicts the root is `INTEGRITY_ERROR`.
pub(crate) fn outcome(
    blobs: &BlobStore,
    options: &BTreeMap<&str, &str>,
) -> Result<Json, HostError> {
    let op = Uuid7::parse(options.get("--op-id").copied().ok_or_else(invalid)?).map_err(failure)?;
    let answer = RevisionStore::new(blobs.clone())
        .query_outcome(&op)
        .map_err(failure)?;
    Ok(match answer {
        Outcome::Committed {
            revision,
            sequence,
            manifest,
            source,
        } => Json::object([
            ("outcome", Json::text("outcome_committed")),
            ("op_id", Json::text(op.as_str())),
            ("revision_id", Json::text(revision.as_str())),
            ("sequence", Json::text(sequence.to_string())),
            ("manifest_digest", Json::text(manifest.to_string())),
            (
                "source",
                Json::text(match source {
                    OutcomeSource::Ledger => "ledger",
                    OutcomeSource::Root => "root",
                }),
            ),
        ]),
        Outcome::Aborted { reason } => Json::object([
            ("outcome", Json::text("outcome_aborted")),
            ("op_id", Json::text(op.as_str())),
            ("reason", Json::text(reason.as_str())),
        ]),
        Outcome::NotFound => Json::object([
            ("outcome", Json::text("outcome_not_recorded")),
            ("op_id", Json::text(op.as_str())),
        ]),
        Outcome::Unknown => return Err(HostError::logical(ErrorCode::OutcomeUnknown)),
    })
}
