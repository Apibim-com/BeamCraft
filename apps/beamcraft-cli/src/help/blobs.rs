// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Help owned by the blobs command family.

pub(super) const COMMANDS: &[&str] = &["blob-put", "blob-verify"];

pub(super) const HELP: &str = concat!(
    "Blobs: blob-put --store ROOT --file FILE (stores the bytes under their SHA-256); ",
    "blob-verify --store ROOT --digest SHA256",
);
