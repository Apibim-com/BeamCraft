// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Help owned by the commands inventory.

pub(super) const HELP: &str = concat!(
    "Commands: commands [--tier core|ui-assist] (the registry of every command with its ",
    "family and tier; core is the headless surface an agent drives, ui-assist mirrors interactive ",
    "UI affordances; needs no store; read-only)",
);
