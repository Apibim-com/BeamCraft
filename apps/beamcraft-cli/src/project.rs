// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Creating projects, and putting an environment into one.
//!
//! [`create_project`] makes the folder inside an existing parent and commits its first revision:
//! an empty model whose root layer records the ProjectId (D1). Reading project folders, suggesting
//! names and listing environments is platform state and lives in the `session` crate, which the
//! window uses as well.

use crate::library::load_environment;
use crate::publication::commit_verified;
use beamcraft_host::revision::{
    CommitHook, CommitOutcome, LayerEntry, LayerRole, RevisionDraft, RevisionStore,
};
use beamcraft_host::storage::BlobStore;
use beamcraft_layer_engine::authored::schema_identity;
use beamcraft_model::error::ErrorCode;
use beamcraft_model::id::{OpId, ProjectId, RevisionId, Uuid7, WorkspaceId};
use session::Refusal;
use session::environment::ENVIRONMENT_FILE;
use session::pipeline::{DERIVED_LAYER, ROOT_LAYER, claim_project, host_uuid, load_base};
use session::project::{ProjectInfo, check_project_name, numbered_name, project_info};
use std::fs;
use std::path::{Path, PathBuf};

/// Folder of a project that holds the copy of its environment, made by `create-project`
/// `--environment`: the project carries the libraries it was created with, so it opens the same
/// on another machine and the next session needs no setting.
pub const PROJECT_ENVIRONMENT_DIR: &str = "environment";

const fn refuse(code: ErrorCode, message: &'static str) -> Refusal {
    Refusal::new(code, message)
}

const fn invalid(message: &'static str) -> Refusal {
    refuse(ErrorCode::InvalidArgument, message)
}

/// Identities of one project creation.
#[derive(Debug, Clone)]
pub struct ProjectContext {
    /// The logical operation of the first revision.
    pub op_id: OpId,
    /// Identity of the first revision.
    pub revision_id: RevisionId,
    /// The author (trusted context).
    pub actor: Uuid7,
    /// The new project's identity.
    pub project: ProjectId,
    /// The workspace that authors the first revision.
    pub workspace: WorkspaceId,
}

/// Creates the folder `parent/name` and commits the first revision of a new, empty project.
///
/// The parent must exist; a folder of that name must not. When the commit fails, the folder this
/// call created is removed again, so a failed creation leaves nothing behind.
pub fn create_project(
    parent: &Path,
    name: &str,
    context: &ProjectContext,
    hook: &mut dyn CommitHook,
) -> Result<ProjectInfo, Refusal> {
    create_named_project(parent, name, false, context, hook)
}

/// Create with the first free name: `name`, `name 1`, `name 2`, up to 10,000 candidates.
/// Directory reservation is atomic; an existing file, directory or link is never overwritten.
pub fn create_project_unique(
    parent: &Path,
    name: &str,
    context: &ProjectContext,
    hook: &mut dyn CommitHook,
) -> Result<ProjectInfo, Refusal> {
    create_named_project(parent, name, true, context, hook)
}

fn create_named_project(
    parent: &Path,
    name: &str,
    unique: bool,
    context: &ProjectContext,
    hook: &mut dyn CommitHook,
) -> Result<ProjectInfo, Refusal> {
    check_project_name(name)?;
    let parent =
        std::path::absolute(parent).map_err(|_| invalid("the path cannot be made absolute"))?;
    if !parent.is_dir() {
        return Err(refuse(
            ErrorCode::NotFound,
            "the parent folder does not exist",
        ));
    }
    let mut reserved = None;
    for index in 0..if unique { 10_000 } else { 1 } {
        let candidate = numbered_name(name, index);
        check_project_name(&candidate)?;
        let dir = parent.join(&candidate);
        match fs::create_dir(&dir) {
            Ok(()) => {
                reserved = Some(dir);
                break;
            }
            Err(error) if unique && error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => {
                return Err(match e.kind() {
                    std::io::ErrorKind::AlreadyExists => refuse(
                        ErrorCode::DuplicateId,
                        "a file or folder of that name already exists",
                    ),
                    std::io::ErrorKind::PermissionDenied => {
                        refuse(ErrorCode::PermissionDenied, "the folder cannot be created")
                    }
                    _ => invalid("the folder cannot be created"),
                });
            }
        }
    }
    let dir = reserved.ok_or_else(|| {
        refuse(
            ErrorCode::LimitExceeded,
            "no free project name within the candidate limit",
        )
    })?;
    match first_revision(&dir, context, hook) {
        Ok(()) => Ok(project_info(&dir)?),
        Err(error) => {
            // Only what this call made is removed: the folder did not exist before it.
            let _ = fs::remove_dir_all(&dir);
            Err(error)
        }
    }
}

/// Commits the initial layer set of a store without a revision: the root that records the
/// ProjectId, and empty authored and derived layers. An empty model is not an edit, so it is not
/// authored through the layer engine; it is the same base every first command starts from, and
/// it is verified at commit like every revision (U03 index, structural validation).
fn first_revision(
    dir: &Path,
    context: &ProjectContext,
    hook: &mut dyn CommitHook,
) -> Result<(), Refusal> {
    let blobs = BlobStore::new(dir).map_err(|e| refuse(e.code(), "store root is not usable"))?;
    let store = RevisionStore::new(blobs);
    let _claim = claim_project(context.project);
    let (head, base) = load_base(&store, &context.project, &context.workspace)?;
    if head.is_some() {
        return Err(refuse(
            ErrorCode::DuplicateId,
            "the folder already holds a project",
        ));
    }
    let mut entries = Vec::with_capacity(base.assets.len());
    for asset in &base.assets {
        store
            .blobs()
            .publish(&asset.bytes)
            .map_err(|e| refuse(e.code(), "layer blob could not be stored"))?;
        let role = if asset.name == DERIVED_LAYER {
            LayerRole::Derived
        } else {
            LayerRole::Authored
        };
        entries.push(LayerEntry::from_bytes(&asset.name, role, &asset.bytes)?);
    }
    let draft = RevisionDraft {
        project: host_uuid(&base.project)?,
        workspace: host_uuid(&base.workspace)?,
        revision: host_uuid(&context.revision_id.to_string())?,
        op: host_uuid(&context.op_id.to_string())?,
        workspace_generation: base
            .generation
            .checked_add(1)
            .ok_or(refuse(ErrorCode::LimitExceeded, "generation exhausted"))?,
        schema: schema_identity()?.digest,
        root_layer: ROOT_LAYER.into(),
        entries,
    };
    let committed = commit_verified(&store, &draft, &base.expected, hook)?;
    if committed.outcome == CommitOutcome::Replayed {
        return Err(refuse(
            ErrorCode::IntegrityError,
            "a new project was reported as replayed",
        ));
    }
    Ok(())
}

/// The environment file of the project at `dir`, when the project was created with one.
pub fn project_environment_file(dir: &Path) -> Option<PathBuf> {
    let file = dir.join(PROJECT_ENVIRONMENT_DIR).join(ENVIRONMENT_FILE);
    file.is_file().then_some(file)
}

/// Copies the environment `file` and the libraries it lists into the project at `dir`
/// (`environment/`). The environment is read and admitted first, so a refused one writes nothing.
/// Commands of the project then resolve profile names against this copy.
pub fn install_environment(dir: &Path, file: &Path) -> Result<(), Refusal> {
    let path = file
        .to_str()
        .ok_or_else(|| invalid("the environment path is not UTF-8"))?;
    let environment = load_environment(path)?;
    let source = file.parent().unwrap_or_else(|| Path::new("."));
    let target = dir.join(PROJECT_ENVIRONMENT_DIR);
    let unusable = || {
        refuse(
            ErrorCode::BackendUnavailable,
            "the project environment could not be copied",
        )
    };
    fs::create_dir(&target).map_err(|_| unusable())?;
    fs::copy(file, target.join(ENVIRONMENT_FILE)).map_err(|_| unusable())?;
    for reference in &environment.info.libraries {
        let to = target.join(reference);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|_| unusable())?;
        }
        fs::copy(source.join(reference), &to).map_err(|_| unusable())?;
    }
    Ok(())
}
