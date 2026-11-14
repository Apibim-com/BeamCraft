// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Help owned by the delete command family.

pub(super) const HELP: &str = concat!(
    "Delete (P08): delete-element --store ROOT --id UUID [--with-dependents true|false] ",
    "[--dry-run true|false] [--op-id UUID] [--revision-id UUID] [--actor-id UUID] ",
    "[--project-id UUID] [--workspace-id UUID] (removes the whole element; DEPENDENCY_CONFLICT ",
    "while another element links to it, never an implicit cascade; --with-dependents true ",
    "deletes a plate's or member's welds and bolt groups with it in the same revision and undo ",
    "unit, the holes of those groups leaving the other parts, while any other link (the ",
    "assembly whose main part it is, a penetration, a grid binding) still refuses; --dry-run ",
    "true writes nothing and lists the dependents that would go and the blockers that would ",
    "refuse; the definitions it pinned stay)",
);
