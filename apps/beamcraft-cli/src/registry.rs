// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The single inventory of `beamcraft` commands.
//!
//! Every command the binary dispatches is listed here once, with its family, tier and the
//! adapter that runs it. The binary routes a command word through [`route`], and the help
//! inventory and the `commands` command are rendered from this table, so a word that is not
//! listed here is not dispatched and a listed word cannot be left without a route. Add a command
//! here when it is added to its adapter.

use beamcraft_json::Json;
use std::ffi::OsStr;

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

/// The adapter in the binary that runs a command.
///
/// A route is a dispatch detail, not a family: `resolve-workspace` is documented with the
/// workspace commands but runs in its own adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// The store-level blob and revision commands.
    Generic,
    /// `commands`, which lists this registry.
    Commands,
    /// `export-view`, which writes a framed or raw byte stream.
    ExportView,
    /// The IFC export commands, whose output is rendered by the command itself.
    Export,
    /// The view snapshot commands.
    View,
    /// The history inspection and compaction commands.
    History,
    /// The reference model commands.
    Reference,
    /// The assembly commands.
    Assembly,
    /// `rebase-workspace`.
    Rebase,
    /// `resolve-workspace`.
    Resolve,
    /// The changeset commands.
    Changeset,
    /// The coordinator commands.
    Coordinator,
    /// The drawing commands.
    Drawing,
    /// The work plane and construction point commands.
    WorkPlane,
    /// The picking and snapping queries.
    Pick,
    /// The weld commands.
    Weld,
    /// The bolt commands.
    Bolt,
    /// `export-bom`.
    Bom,
    /// `schedule-export`.
    Schedule,
    /// The library commands.
    Library,
    /// `fastener-geometry`.
    Fastener,
    /// The project and start-screen commands.
    Project,
    /// The grid commands.
    Grid,
    /// The level commands.
    Level,
    /// The attribute commands.
    Attribute,
    /// The numbering commands.
    Numbering,
    /// The modelling, definition, edit and workspace commands.
    Plate,
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
    /// The adapter that runs the command.
    pub route: Route,
}

const fn core(route: Route, family: &'static str, name: &'static str) -> Command {
    Command {
        name,
        family,
        tier: Tier::Core,
        route,
    }
}

const fn assist(route: Route, family: &'static str, name: &'static str) -> Command {
    Command {
        name,
        family,
        tier: Tier::UiAssist,
        route,
    }
}

/// Every command, in the order the help inventory lists them.
pub const COMMANDS: &[Command] = &[
    core(Route::Generic, "blobs", "put-blob"),
    core(Route::Generic, "blobs", "verify-blob"),
    core(Route::Generic, "revision", "commit-revision"),
    core(Route::Generic, "revision", "open-revision"),
    core(Route::Generic, "revision", "recover-revision"),
    core(Route::Generic, "revision", "inspect-revision-outcome"),
    core(Route::Plate, "plate", "create-plate"),
    core(Route::Plate, "plate", "add-hole"),
    core(Route::Plate, "plate", "update-hole"),
    core(Route::Plate, "plate", "remove-hole"),
    core(Route::Plate, "plate", "inspect-elements"),
    core(Route::Plate, "definitions", "define-material"),
    core(Route::Plate, "definitions", "define-profile"),
    core(Route::Plate, "definitions", "define-fastener"),
    core(Route::Plate, "definitions", "definitions"),
    core(Route::Library, "library", "list-library"),
    core(Route::Library, "library", "show-library"),
    core(Route::Library, "library", "add-library"),
    core(Route::Plate, "member", "create-member"),
    core(Route::Plate, "member", "create-polybeam"),
    core(Route::Plate, "member", "create-curved-beam"),
    core(Route::WorkPlane, "work_plane", "work-plane"),
    core(Route::WorkPlane, "work_plane", "resolve-construction-point"),
    core(Route::Plate, "edit", "update-element"),
    core(Route::Plate, "edit", "move-plate-vertex"),
    core(Route::Plate, "edit", "move-plate-edge"),
    core(Route::Plate, "edit", "insert-plate-vertex"),
    core(Route::Plate, "edit", "delete-plate-vertex"),
    core(Route::Plate, "edit", "set-plate-corner"),
    core(Route::Plate, "edit", "cut-by-plane"),
    core(Route::Plate, "edit", "update-cut"),
    core(Route::Plate, "edit", "remove-cut"),
    core(Route::Plate, "edit", "cut-by-line"),
    core(Route::Plate, "edit", "cut-by-polygon"),
    core(Route::Plate, "edit", "cut-by-part"),
    core(Route::Plate, "edit", "fit-end-to-part"),
    core(Route::Plate, "edit", "chamfer-part"),
    core(Route::Plate, "edit", "split-part"),
    core(Route::Plate, "edit", "combine-parts"),
    core(Route::Plate, "delete", "delete-element"),
    core(Route::Plate, "copy", "copy-element"),
    core(Route::Plate, "copy", "copy-instance"),
    core(Route::Plate, "workspace", "create-workspace"),
    core(Route::Plate, "workspace", "inspect-workspace"),
    core(Route::Plate, "workspace", "publish-workspace"),
    core(Route::Resolve, "workspace", "resolve-workspace"),
    core(Route::Plate, "workspace", "undo-workspace"),
    core(Route::Plate, "workspace", "redo-workspace"),
    core(Route::Plate, "workspace", "inspect-workspace-history"),
    core(Route::Rebase, "rebase", "rebase-workspace"),
    core(Route::Reference, "reference", "attach-reference"),
    core(Route::Reference, "reference", "move-reference"),
    core(Route::Reference, "reference", "correct-reference-units"),
    core(Route::Reference, "reference", "write-reference"),
    core(Route::Reference, "reference", "inspect-reference"),
    core(Route::Reference, "reference", "measure-reference"),
    core(Route::Assembly, "assembly", "create-assembly"),
    core(Route::Assembly, "assembly", "attach-to-assembly"),
    core(Route::Assembly, "assembly", "detach-from-assembly"),
    core(Route::Assembly, "assembly", "set-main-part"),
    core(Route::Assembly, "assembly", "explode-assembly"),
    core(Route::Assembly, "assembly", "assemblies"),
    core(Route::Weld, "weld", "create-weld"),
    core(Route::Weld, "weld", "welds"),
    core(Route::Bolt, "bolt", "create-bolt"),
    core(Route::Bolt, "bolt", "bolts"),
    core(Route::Fastener, "bolt", "fastener-geometry"),
    core(Route::Level, "level", "create-level"),
    core(Route::Level, "level", "levels"),
    core(Route::Attribute, "attribute", "attribute-define"),
    core(Route::Attribute, "attribute", "attribute-undefine"),
    core(Route::Attribute, "attribute", "attribute-set"),
    core(Route::Attribute, "attribute", "attribute-clear"),
    core(Route::Attribute, "attribute", "attribute-get"),
    core(Route::Attribute, "attribute", "attributes"),
    core(Route::Grid, "grid", "create-grid"),
    core(Route::Grid, "grid", "update-grid"),
    core(Route::Grid, "grid", "list-grids"),
    core(Route::Grid, "grid", "resolve-grid-point"),
    core(Route::Grid, "grid", "resolve-grid-plane"),
    core(Route::Grid, "grid", "draw-grid-line"),
    core(Route::Grid, "grid", "reshape-grid-line"),
    core(Route::Grid, "grid", "remove-grid-line"),
    core(Route::Grid, "grid", "relabel-grid-line"),
    core(Route::Export, "export", "export-ifc"),
    core(Route::Export, "export", "ifc-guid"),
    core(Route::Numbering, "numbering", "preview-numbering"),
    core(Route::Numbering, "numbering", "approve-numbering"),
    core(Route::Numbering, "numbering", "inspect-numbering"),
    core(Route::Changeset, "changeset", "create-changeset"),
    core(Route::Changeset, "changeset", "inspect-changeset"),
    core(Route::Changeset, "changeset", "apply-changeset"),
    core(Route::Coordinator, "coordinator", "serve-coordinator"),
    core(Route::Coordinator, "coordinator", "publish-changeset"),
    core(Route::Coordinator, "coordinator", "coordinator-rebase"),
    core(
        Route::Coordinator,
        "coordinator",
        "inspect-coordinator-head",
    ),
    core(Route::History, "history", "inspect-history"),
    core(Route::History, "history", "compact-history"),
    core(Route::View, "view", "snapshot-view"),
    core(Route::View, "view", "inspect-view-changes"),
    assist(Route::Pick, "pick", "pick"),
    core(Route::Pick, "pick", "lookup-feature"),
    core(Route::Pick, "pick", "list-feature"),
    assist(Route::Pick, "pick", "snap"),
    core(Route::Pick, "pick", "element-edges"),
    core(Route::ExportView, "view_export", "export-view"),
    core(Route::Bom, "bom", "export-bom"),
    core(Route::Schedule, "schedules", "schedule-export"),
    core(Route::Drawing, "drawing", "preview-drawing"),
    core(Route::Drawing, "drawing", "create-drawing"),
    core(Route::Drawing, "drawing", "refresh-drawing"),
    core(Route::Drawing, "drawing", "edit-drafting"),
    core(Route::Drawing, "drawing", "publish-drawing"),
    core(Route::Drawing, "drawing", "inspect-drawing"),
    core(Route::Drawing, "drawing", "inspect-drawing-versions"),
    core(Route::Drawing, "drawing", "export-drawing"),
    assist(Route::Project, "start", "show-settings"),
    assist(Route::Project, "start", "set-settings"),
    assist(Route::Project, "start", "list-environment"),
    core(Route::Project, "start", "create-project"),
    assist(Route::Project, "start", "suggest-project-name"),
    core(Route::Project, "start", "list-project"),
    assist(Route::Project, "start", "list-recent"),
    assist(Route::Project, "start", "add-recent"),
    core(Route::Commands, "commands", "commands"),
];

/// The adapter that runs `word`, or `None` when it is not a registered command.
///
/// The binary falls back to the store-level path for an unregistered word, which refuses it.
pub fn route(word: &OsStr) -> Option<Route> {
    let word = word.to_str()?;
    COMMANDS.iter().find(|c| c.name == word).map(|c| c.route)
}

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
    use std::ffi::OsStr;

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
    fn every_registered_word_has_its_route_and_others_have_none() {
        for c in COMMANDS {
            assert_eq!(route(OsStr::new(c.name)), Some(c.route), "{}", c.name);
        }
        assert_eq!(route(OsStr::new("no-such-command")), None);
        assert_eq!(route(OsStr::new("")), None);
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
