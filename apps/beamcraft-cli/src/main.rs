// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! `beamcraft`: command-line entry point.

fn main() {
    let arg = std::env::args().nth(1);
    match arg.as_deref() {
        Some("--version" | "-V") => println!("beamcraft {}", env!("CARGO_PKG_VERSION")),
        _ => println!("BeamCraft by Apibim. Run `beamcraft --version`."),
    }
}
