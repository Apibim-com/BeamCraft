// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Help owned by the workspace command family.

pub(super) const COMMANDS: &[&str] = &[
    "workspace-create",
    "workspace-status",
    "workspace-publish",
    "workspace-undo",
    "workspace-redo",
    "workspace-history",
];

pub(super) const HELP: &str = concat!(
    "Workspace (P06, a personal draft is a separate store directory; edit it with the usual ",
    "commands and --store DIR): workspace-create --store PUBLISHED --workspace-dir DIR ",
    "[--workspace-id UUID] [--user-id UUID] [--revision-id UUID] [--op-id UUID] (forks the ",
    "published head); workspace-status --store PUBLISHED --workspace-dir DIR (base, unsent ",
    "revisions, whether the published head moved); workspace-publish --store PUBLISHED ",
    "--workspace-dir DIR (the published revision id is the draft head's id, so a retry is the ",
    "same request; fast-forward only: BASE_REVISION_MISMATCH when the published head moved, ",
    "no merge)",
);

pub(super) const UNDO: &str = concat!(
    "Undo (P07, durable undo and redo of the draft; each is one new verified revision with ",
    "the restored layers): workspace-undo --workspace-dir DIR [--revision-id UUID] [--op-id ",
    "UUID]; workspace-redo --workspace-dir DIR [same]; workspace-history --workspace-dir DIR ",
    "(undo and redo units; an edit after an undo discards redo; INVALID_ARGUMENT when there ",
    "is nothing to undo or redo)",
);
