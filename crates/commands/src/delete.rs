// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! P08 adapter: `DeleteElement` for a plate, a member, an A01 assembly, an A06 weld or a D2 level,
//! committed as the next revision.
//!
//! The delete removes the whole element, its authored prim and its result prims, and nothing else:
//! the other elements, the unknown content and the immutable definitions it pinned stay. Before any
//! layer is touched the layer engine's element index decides whether something still links to the
//! element (`beamcraft_layer_engine::authored::Author::check_delete`): an incoming relationship is
//! `DEPENDENCY_CONFLICT`, an index
//! that cannot prove the links complete is `DEPENDENCY_UNAVAILABLE`, and there is no implicit
//! cascade and no dangling link. The one cascade is explicit: `with_dependents` deletes a plate's
//! or member's welds and bolt groups with it, in the same revision and undo unit, after the user
//! confirmed the list [`delete_preview`] showed (owner decision 2026-11-14). The layer engine's write set is checked against the footprint the engine declares.
//!
//! Honesty limits: replay of an `op_id` is decided from the head model (the delete is a replay when
//! the element is already absent and the ledger proves the operation committed), not from a stored
//! canonical request. The element relationship authored today is the A01 `part-of` record, so an
//! assembly with parts cannot be deleted; deleting a part removes its own record with it. One
//! implicit workspace, in-process layer
//! engine, as in `plate`.

use crate::definition::material_lookup;
use crate::plate::{decode, evaluate_decoded, part_mesh_change, prior_op_with, replay_summary};
use crate::publication::{PublicationTail, prepare_and_publish};
use beamcraft_engine::footprint::Footprint;
use beamcraft_host::revision::{CommitHook, RevisionStore, Uuid7};
use beamcraft_layer_engine::Reader;
use beamcraft_layer_engine::authored::{Change, schema_identity};
use beamcraft_layer_engine::index::ElementId as LayerElementId;
use beamcraft_model::command::{CommandError, DeleteElement};
use beamcraft_model::error::ErrorCode;
use beamcraft_model::id::{ElementId, OpId, ProjectId, RevisionId, WorkspaceId};
use session::Refusal;
use session::pipeline::{
    Base, RevisionSummary, control, digest_of, host_uuid, load_base, open_base, summarize,
};
use steel::engine::transaction::PriorOp;
use steel::layer as steel_layer;

/// Inputs of one `delete_element` call beyond the command itself.
#[derive(Debug, Clone)]
pub struct DeleteRequest {
    /// The logical operation.
    pub op_id: OpId,
    /// Identity of the revision this call creates.
    pub revision_id: RevisionId,
    /// The acting user (trusted context).
    pub actor: Uuid7,
    /// Project of a store that has no revision yet; a delete in such a store is `NOT_FOUND`.
    pub project: ProjectId,
    /// Workspace of a store that has no revision yet.
    pub workspace: WorkspaceId,
    /// The command.
    pub command: DeleteElement,
}

/// What was deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeletedKind {
    /// A plate.
    Plate,
    /// A beam or a column.
    Member,
    /// An assembly (A01); refused while it still has parts.
    Assembly,
    /// A weld (A06), a `connection` element; deleting it removes its `connects` record.
    Weld,
    /// A bolt group (A05), a `connection` element; deleting it removes its `connects` record.
    Bolt,
    /// A level (D2); refused while a plate, beam or column is assigned to it.
    Level,
    /// Shared project grid (T01).
    Grid,
    /// A T02 construction object.
    Construction,
    /// A D06 annotation.
    Annotation,
}

/// Result of `delete_element`.
#[derive(Debug, Clone, PartialEq)]
pub struct Deleted {
    /// True when the same operation was already committed and nothing was written.
    pub replayed: bool,
    /// The kind of the deleted element (`None` for a replay: it is no longer in the head).
    pub kind: Option<DeletedKind>,
    /// The revision that holds the result.
    pub revision: RevisionSummary,
    /// The declared footprint; `None` for a replay.
    pub footprint: Option<Footprint>,
    /// Names of the layers whose bytes changed (empty for a replay).
    pub changed_layers: Vec<String>,
    /// The welds and bolt groups deleted with the part (`with_dependents`), in index order.
    pub dependents: Vec<(ElementId, DeletedKind)>,
}

/// What deleting a part would take with it: the read-only answer the confirmation shows.
#[derive(Debug, Clone, PartialEq)]
pub struct DeletePreview {
    /// The welds and bolt groups a `with_dependents` delete removes with the part.
    pub dependents: Vec<(ElementId, DeletedKind)>,
    /// Elements whose links no cascade removes (an assembly whose main part it is, a
    /// penetration, a grid binding, a foreign source): while any is listed the delete refuses.
    pub blockers: Vec<String>,
}

fn not_found() -> Refusal {
    Refusal::from(CommandError::new(
        ErrorCode::NotFound,
        "target_id",
        "the element is not in the head revision",
    ))
}

/// Deletes one element and publishes the next revision, or replays the operation.
///
/// Steps, each failing closed: open and verify the head; find the target; decide replay from the
/// host ledger; build the layer engine's element index and refuse a delete that would leave a
/// dangling link; author the removal with U02; check the layer engine's writes against the declared
/// footprint; commit through U06/U07 with the strict verifier.
pub fn delete_element(
    store: &RevisionStore,
    request: &DeleteRequest,
    hook: &mut dyn CommitHook,
) -> Result<Deleted, Refusal> {
    let command = &request.command;
    let control = control();
    let (head, base) = load_base(store, &request.project, &request.workspace)?;
    if head.is_none() {
        return Err(not_found());
    }
    let before: std::collections::BTreeMap<String, _> = base
        .assets
        .iter()
        .map(|a| (a.name.clone(), a.digest))
        .collect();
    let base_request = open_base(&base)?;
    let Base {
        project,
        workspace,
        generation: base_generation,
        expected,
        ..
    } = base;
    let (plate_ids, members) = {
        let mut reader = Reader::default();
        let opened = reader.open(base_request.clone(), &control)?;
        let decoded = decode(&reader, &opened, &control)?;
        let (_, members) = evaluate_decoded(&decoded)?;
        (
            decoded
                .plates
                .iter()
                .map(|(p, _)| p.id)
                .collect::<std::collections::BTreeSet<_>>(),
            members,
        )
    };
    let kind = if plate_ids.contains(&command.target_id) {
        Some(DeletedKind::Plate)
    } else if members.iter().any(|r| r.authored.id == command.target_id) {
        Some(DeletedKind::Member)
    } else if crate::assembly::is_assembly(&base_request, &command.target_id, &control)? {
        Some(DeletedKind::Assembly)
    } else if crate::weld::is_weld(&base_request, &command.target_id, &control)? {
        Some(DeletedKind::Weld)
    } else if crate::bolt::is_bolt(&base_request, &command.target_id, &control)? {
        Some(DeletedKind::Bolt)
    } else if crate::grid::is_grid(&base_request, &command.target_id, &control)? {
        Some(DeletedKind::Grid)
    } else if crate::construction::is_construction(&base_request, &command.target_id, &control)? {
        Some(DeletedKind::Construction)
    } else if crate::annotation::is_annotation(&base_request, &command.target_id, &control)? {
        Some(DeletedKind::Annotation)
    } else if crate::level::is_level(&base_request, &command.target_id, &control)? {
        Some(DeletedKind::Level)
    } else {
        None
    };

    let op_host = host_uuid(&request.op_id.to_string())?;
    let fingerprint = command.content_fingerprint()?;
    let (prior, recorded) = prior_op_with(store, &op_host, || {
        // The head holds the result of this delete exactly when the element is absent.
        if kind.is_none() {
            Ok(fingerprint)
        } else {
            digest_of(b"no accepted request matches the head state")
        }
    })?;
    match prior {
        PriorOp::Accepted { fingerprint: seen } if seen == fingerprint => {
            return Ok(Deleted {
                replayed: true,
                kind: None,
                revision: replay_summary(&head, recorded)?,
                footprint: None,
                changed_layers: Vec::new(),
                dependents: Vec::new(),
            });
        }
        PriorOp::Accepted { .. } => {
            return Err(CommandError::new(
                ErrorCode::OpIdReuse,
                "op_id",
                "operation id was accepted with different content",
            )
            .into());
        }
        PriorOp::Unseen => {}
    }
    let Some(kind) = kind else {
        // A pinned coordination grid is foreign bytes: the refusal precedes the local miss,
        // because no authority of this store can reach another project's content (PC-OWN-002).
        if crate::coordination::pinned_owns_element(store, &command.target_id)? {
            return Err(Refusal {
                rule_id: Some("PC-OWN-002"),
                code: ErrorCode::ForeignReadOnly,
                message: "the element belongs to a pinned coordination project and is read-only",
            });
        }
        return Err(not_found());
    };
    // Grids and levels are shared data (T01 §9): their delete answers to the shared grids/
    // levels scope, exactly like create and edit, not to the element scope.
    let shared = matches!(kind, DeletedKind::Grid | DeletedKind::Level);
    // T01.8 (§7.5): a grid with followers refuses deletion; a binding is a real element
    // relationship and this guard answers to every relationship, bindings included. A line
    // removal never reaches here and is unaffected by bindings.
    if matches!(kind, DeletedKind::Grid) {
        let mut reader = Reader::default();
        let opened = reader.open(base_request.clone(), &control)?;
        let index = reader.element_index(&opened.context, &control)?;
        let id = beamcraft_layer_engine::index::ElementId::parse(&command.target_id.to_string())?;
        if !index.delete_blockers(&id)?.is_empty() {
            return Err(CommandError::new(
                ErrorCode::DependencyConflict,
                "target_id",
                "GRID_IN_USE; the grid has followers, unbind them first",
            )
            .into());
        }
    }
    // A16.1: a main part cannot leave its assembly through a delete, exactly as a detach of it
    // is refused; another main part is named first.
    if crate::assembly::is_main_part(&base_request, &command.target_id, &control)? {
        return Err(CommandError::new(
            ErrorCode::DependencyConflict,
            "target_id",
            "the element is the main part of its assembly; set another main part first",
        )
        .into());
    }

    // The explicit cascade: a part's welds and bolt groups go with it. Anything else that links
    // to the part stays a blocker, and the set check below refuses it.
    let dependents =
        if command.with_dependents && matches!(kind, DeletedKind::Plate | DeletedKind::Member) {
            cascade_dependents(&base_request, &command.target_id, &control)?.dependents
        } else {
            Vec::new()
        };
    let mut footprint = Footprint::for_delete(command.target_id)?;
    let schema = schema_identity()?;
    let id_text = command.target_id.to_string();
    let mut changes = vec![Change::Delete {
        id: id_text.clone(),
    }];
    for (dependent, _) in &dependents {
        changes.push(Change::Delete {
            id: dependent.to_string(),
        });
        footprint = footprint.union(Footprint::for_delete(*dependent)?)?;
    }
    // The bolt groups whose generated holes go with them: the deleted group itself, or the
    // groups the cascade deletes. The part being deleted takes its own holes with it.
    let groups: std::collections::BTreeSet<ElementId> = dependents
        .iter()
        .filter(|(_, k)| *k == DeletedKind::Bolt)
        .map(|(id, _)| *id)
        .chain((kind == DeletedKind::Bolt).then_some(command.target_id))
        .collect();
    // A14.4: deleting a generating bolt group removes its hole records from every plate in the
    // same revision and undo unit (bolt amendment section 5); the parts themselves stay. An
    // A05 annotation group and a quiet A14.3 group touch no part.
    if !groups.is_empty() {
        let mut reader = Reader::default();
        let opened = reader.open(base_request.clone(), &control)?;
        let decoded = decode(&reader, &opened, &control)?;
        let (plates, _) = evaluate_decoded(&decoded)?;
        let materials = material_lookup(&decoded.definitions);
        let owned = |owner: Option<ElementId>| owner.is_some_and(|o| groups.contains(&o));
        for plate in &plates {
            if plate.authored.id == command.target_id
                || !plate.authored.hole_records.iter().any(|r| owned(r.owner))
            {
                continue;
            }
            let mut next = plate.authored.clone();
            next.hole_records.retain(|r| !owned(r.owner));
            let body = steel::engine::command::evaluate_plate(&next)?;
            let quantities =
                body.quantities_for(next.material.as_ref().map(&materials).transpose()?)?;
            let record = steel::engine::committed::PlateRecord {
                authored: next.clone(),
                body: body.clone(),
                quantities,
            };
            changes.push(steel_layer::hole_records(
                next.id.to_string(),
                next.hole_records.clone(),
            ));
            changes.push(part_mesh_change(&record)?);
            footprint = footprint.union(Footprint::for_update(
                next.id, false, false, true, false, None,
            )?)?;
        }
    }
    let (mut author, opened) =
        crate::scope::open_author(store, base_request, shared, schema, &control)?;
    let index = author.element_index(&opened.context, &control)?;
    if dependents.is_empty() {
        author.check_delete(&index, &LayerElementId::parse(&id_text)?, &control)?;
    } else {
        let mut set = vec![LayerElementId::parse(&id_text)?];
        for (dependent, _) in &dependents {
            set.push(LayerElementId::parse(&dependent.to_string())?);
        }
        author.check_delete_set(&index, &set, &control)?;
    }
    let (committed, changed_layers) = prepare_and_publish(
        &mut author,
        &opened,
        changes,
        &footprint,
        &before,
        PublicationTail {
            store,
            control: &control,
            schema,
            fingerprint,
            project: &project,
            workspace: &workspace,
            revision_id: &request.revision_id,
            op: op_host,
            op_id: request.op_id.to_string(),
            actor: request.actor.to_string(),
            base_generation,
            expected: &expected,
        },
        hook,
    )?;
    Ok(Deleted {
        replayed: false,
        kind: Some(kind),
        revision: summarize(&committed.info),
        footprint: Some(footprint),
        changed_layers,
        dependents,
    })
}

/// The welds and bolt groups that link to `target` and that an explicit cascade deletes with it,
/// and the identities of every other element that links to it (no cascade removes those).
fn cascade_dependents(
    request: &beamcraft_layer_engine::OpenRequest,
    target: &ElementId,
    control: &beamcraft_layer_engine::Control,
) -> Result<DeletePreview, Refusal> {
    let mut reader = Reader::default();
    let opened = reader.open(request.clone(), control)?;
    let index = reader.element_index(&opened.context, control)?;
    let id = LayerElementId::parse(&target.to_string())?;
    let mut cascade = Vec::new();
    let mut blockers = Vec::new();
    for link in index.delete_blockers(&id)? {
        let Some(owner) = link.owner else {
            blockers.push(link.source_path.clone());
            continue;
        };
        let text = owner.to_string();
        if !link.source_owned {
            blockers.push(text);
            continue;
        }
        let dependent = ElementId::parse(&text).map_err(|_| Refusal {
            rule_id: Some("DV-BASE-001"),
            code: ErrorCode::InvalidId,
            message: "a dependent has no valid element identity",
        })?;
        let kind = if crate::weld::is_weld(request, &dependent, control)? {
            DeletedKind::Weld
        } else if crate::bolt::is_bolt(request, &dependent, control)? {
            DeletedKind::Bolt
        } else {
            blockers.push(text);
            continue;
        };
        if !cascade.iter().any(|(d, _)| *d == dependent) {
            cascade.push((dependent, kind));
        }
    }
    blockers.sort();
    blockers.dedup();
    Ok(DeletePreview {
        dependents: cascade,
        blockers,
    })
}

/// Lists what deleting `target` with its dependents would remove and what would still refuse,
/// without writing anything: the confirmation's content. A part that is its assembly's main part
/// is a blocker, as the delete itself refuses it.
pub fn delete_preview(
    store: &RevisionStore,
    project: &ProjectId,
    workspace: &WorkspaceId,
    target: &ElementId,
) -> Result<DeletePreview, Refusal> {
    let control = control();
    let (head, base) = load_base(store, project, workspace)?;
    if head.is_none() {
        return Err(not_found());
    }
    let request = open_base(&base)?;
    let mut preview = cascade_dependents(&request, target, &control)?;
    if crate::assembly::is_main_part(&request, target, &control)? {
        preview
            .blockers
            .push("the assembly whose main part it is".to_owned());
    }
    Ok(preview)
}
