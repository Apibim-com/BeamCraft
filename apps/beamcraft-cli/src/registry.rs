// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The single inventory of `beamcraft` commands.
//!
//! Every command the binary dispatches is listed here once, with its family and tier. The help
//! inventory and the `commands` command are rendered from this table; routing still lives in the
//! per-family adapters, and a test runs every entry through the real binary to prove it is
//! routed. Add a command here when it is added to its adapter.

use beamcraft_json::Json;

/// Who a command is meant for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// Part of the headless modelling, query and exchange surface an agent drives.
    Core,
    /// Mirrors an interactive-UI affordance (cursor snapping, picking, start screen and
    /// appearance settings). Kept for tests and debugging; an agent addresses geometry directly.
    UiAssist,
}

impl Tier {
    /// The stable name used by `commands --tier`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::UiAssist => "ui-assist",
        }
    }
}

/// One dispatched command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Command {
    /// The command word.
    pub name: &'static str,
    /// The command family; it matches the file under `src/help/` that documents the command.
    pub family: &'static str,
    /// Who the command is meant for.
    pub tier: Tier,
}

const fn core(family: &'static str, name: &'static str) -> Command {
    Command {
        name,
        family,
        tier: Tier::Core,
    }
}

const fn assist(family: &'static str, name: &'static str) -> Command {
    Command {
        name,
        family,
        tier: Tier::UiAssist,
    }
}

/// Every command, in the order the help inventory lists them.
pub const COMMANDS: &[Command] = &[
    core("blobs", "put-blob"),
    core("blobs", "verify-blob"),
    core("revision", "commit-revision"),
    core("revision", "open-revision"),
    core("revision", "recover-revision"),
    core("revision", "inspect-revision-outcome"),
    core("plate", "create-plate"),
    core("plate", "add-hole"),
    core("plate", "update-hole"),
    core("plate", "remove-hole"),
    core("plate", "inspect-elements"),
    core("definitions", "define-material"),
    core("definitions", "define-profile"),
    core("definitions", "define-fastener"),
    core("definitions", "definitions"),
    core("library", "list-library"),
    core("library", "show-library"),
    core("library", "add-library"),
    core("member", "create-member"),
    core("member", "create-polybeam"),
    core("member", "create-curved-beam"),
    core("work_plane", "work-plane"),
    core("work_plane", "resolve-construction-point"),
    core("edit", "update-element"),
    core("edit", "cut-by-plane"),
    core("edit", "update-cut"),
    core("edit", "remove-cut"),
    core("edit", "cut-by-line"),
    core("edit", "cut-by-polygon"),
    core("edit", "cut-by-part"),
    core("edit", "fit-end-to-part"),
    core("edit", "chamfer-part"),
    core("edit", "split-part"),
    core("edit", "combine-parts"),
    core("delete", "delete-element"),
    core("copy", "copy-element"),
    core("copy", "copy-instance"),
    core("workspace", "create-workspace"),
    core("workspace", "inspect-workspace"),
    core("workspace", "publish-workspace"),
    core("workspace", "resolve-workspace"),
    core("workspace", "undo-workspace"),
    core("workspace", "redo-workspace"),
    core("workspace", "inspect-workspace-history"),
    core("rebase", "rebase-workspace"),
    core("reference", "attach-reference"),
    core("reference", "move-reference"),
    core("reference", "correct-reference-units"),
    core("reference", "write-reference"),
    core("reference", "inspect-reference"),
    core("reference", "measure-reference"),
    core("assembly", "create-assembly"),
    core("assembly", "attach-to-assembly"),
    core("assembly", "detach-from-assembly"),
    core("assembly", "set-main-part"),
    core("assembly", "explode-assembly"),
    core("assembly", "assemblies"),
    core("weld", "create-weld"),
    core("weld", "welds"),
    core("bolt", "create-bolt"),
    core("bolt", "bolts"),
    core("level", "create-level"),
    core("level", "levels"),
    core("attribute", "attribute-define"),
    core("attribute", "attribute-undefine"),
    core("attribute", "attribute-set"),
    core("attribute", "attribute-clear"),
    core("attribute", "attribute-get"),
    core("attribute", "attributes"),
    core("grid", "create-grid"),
    core("grid", "update-grid"),
    core("grid", "list-grids"),
    core("grid", "resolve-grid-point"),
    core("grid", "resolve-grid-plane"),
    core("grid", "draw-grid-line"),
    core("grid", "reshape-grid-line"),
    core("grid", "remove-grid-line"),
    core("grid", "relabel-grid-line"),
    core("export", "export-ifc"),
    core("export", "ifc-guid"),
    core("numbering", "preview-numbering"),
    core("numbering", "approve-numbering"),
    core("numbering", "inspect-numbering"),
    core("changeset", "create-changeset"),
    core("changeset", "inspect-changeset"),
    core("changeset", "apply-changeset"),
    core("coordinator", "serve-coordinator"),
    core("coordinator", "publish-changeset"),
    core("coordinator", "coordinator-rebase"),
    core("coordinator", "inspect-coordinator-head"),
    core("history", "inspect-history"),
    core("history", "compact-history"),
    core("view", "snapshot-view"),
    core("view", "inspect-view-changes"),
    assist("pick", "pick"),
    core("pick", "lookup-feature"),
    core("pick", "list-feature"),
    assist("pick", "snap"),
    core("pick", "element-edges"),
    core("view_export", "export-view"),
    core("bom", "export-bom"),
    core("schedules", "schedule-export"),
    core("drawing", "preview-drawing"),
    core("drawing", "create-drawing"),
    core("drawing", "refresh-drawing"),
    core("drawing", "edit-drafting"),
    core("drawing", "publish-drawing"),
    core("drawing", "inspect-drawing"),
    core("drawing", "inspect-drawing-versions"),
    core("drawing", "export-drawing"),
    assist("start", "show-settings"),
    assist("start", "set-settings"),
    assist("start", "list-environment"),
    core("start", "create-project"),
    assist("start", "suggest-project-name"),
    core("start", "list-project"),
    assist("start", "list-recent"),
    assist("start", "add-recent"),
    core("commands", "commands"),
];

/// The `commands` result: the registry, optionally narrowed to one tier.
///
/// `tier` is the value of `--tier` (`core` or `ui-assist`); anything else is `None`.
pub fn listing(tier: Option<&str>) -> Option<Json> {
    let wanted = match tier {
        None => None,
        Some(name) => Some(
            [Tier::Core, Tier::UiAssist]
                .into_iter()
                .find(|t| t.as_str() == name)?,
        ),
    };
    let commands = COMMANDS
        .iter()
        .filter(|c| wanted.is_none_or(|t| c.tier == t))
        .map(|c| {
            Json::object([
                ("name", Json::text(c.name)),
                ("family", Json::text(c.family)),
                ("tier", Json::text(c.tier.as_str())),
            ])
        })
        .collect();
    Some(Json::object([
        ("outcome", Json::text("commands_listed")),
        ("commands", Json::Array(commands)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn names_are_unique_and_command_words() {
        let mut seen = BTreeSet::new();
        for c in COMMANDS {
            assert!(seen.insert(c.name), "duplicate command: {}", c.name);
            assert!(
                !c.name.is_empty() && c.name.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'),
                "not a command word: {}",
                c.name
            );
        }
    }

    #[test]
    fn listing_filters_by_tier_and_refuses_unknown_tiers() {
        let all = listing(None);
        assert!(all.is_some());
        assert!(listing(Some("core")).is_some());
        assert!(listing(Some("ui-assist")).is_some());
        assert!(listing(Some("human")).is_none());
    }
}
