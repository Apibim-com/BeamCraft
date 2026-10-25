// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Help owned by the reference command family.

pub(super) const COMMANDS: &[&str] = &[
    "reference-attach",
    "reference-move",
    "reference-correct-units",
    "reference-write",
    "reference-inspect",
];

pub(super) const HELP: &str = concat!(
    "Reference (R01, a foreign USD file attached read-only; its bytes are stored under their ",
    "SHA-256 and never rewritten): reference-attach --store ROOT --source FILE [--name TEXT] ",
    "[--translation X,Y,Z] [--rotation-z-deg DEG] [--new-id UUID] [--op-id UUID] ",
    "[--revision-id UUID] [--actor-id UUID] [--project-id UUID] [--workspace-id UUID] (FILE ",
    "is a local USDA or restricted USDC layer of at most 65536 bytes with a defaultPrim and ",
    "no other assets; a URL or network path is PERMISSION_DENIED before anything is read; the ",
    "placement belongs to the wrapper and is stored apart from the foreign content); ",
    "reference-move --store ROOT --id UUID [--translation X,Y,Z] [--rotation-z-deg DEG] [same ",
    "ids] (absolute placement of the wrapper only); reference-correct-units --store ROOT --id ",
    "UUID (--meters-per-unit F --up-axis Y|Z | --clear true) [same ids] (the explicit units ",
    "and up-axis correction of the wrapper; the source declares what it declares and is never ",
    "edited, and without a correction it is taken as stored); reference-write --store ROOT ",
    "--id UUID (--target source | --prim /DefaultPrim/Path [--attribute NAME]) (always ",
    "FOREIGN_READ_ONLY: the source layer and every foreign prim are read-only, including ",
    "overrides in owned layers); reference-inspect --store ROOT (verified head: wrappers, ",
    "pinned source, declared and explicit units, units status, foreign prims, external layers ",
    "with their digests)",
);
