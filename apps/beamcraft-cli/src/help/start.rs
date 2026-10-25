// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Help owned by the start command family.

pub(super) const COMMANDS: &[&str] = &[
    "settings-show",
    "settings-set",
    "environment-list",
    "project-create",
    "project-list",
    "recent-list",
    "recent-add",
];

pub(super) const HELP: &str = concat!(
    "Start (settings of this user, environments and projects; --config-dir DIR replaces the ",
    "user configuration directory, BEAMCRAFT_CONFIG_DIR or the platform default): ",
    "settings-show; settings-set [--name TEXT --email TEXT] [--environment ID|none] ",
    "[--skip-launcher true|false] [--models-dir DIR|none] (the first profile gets an author ",
    "UUIDv7 that later changes keep); environment-list (environments/<name>/environment.usda ",
    "next to the program, then in the configuration directory; the first of an id wins, later ",
    "ones are shadowed, unreadable ones are listed with their refusal); project-create ",
    "--parent DIR --name NAME [--op-id UUID] [--revision-id UUID] [--actor-id UUID] ",
    "[--project-id UUID] [--workspace-id UUID] (makes the folder DIR/NAME, refused when it ",
    "exists, and commits an empty first revision; the author is --actor-id, else the settings ",
    "profile; the project is remembered as recent); project-list --dir DIR (the store folders ",
    "directly inside DIR by name, read-only; created_ms is the ProjectId timestamp, ",
    "modified_ms the head revision id timestamp); recent-list; recent-add --store DIR",
);
