// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Thin separate-process Rust USD read worker. Never a user-facing service.

#[path = "../usd_worker.rs"]
mod usd_worker;

fn main() {
    if usd_worker::run().is_err() {
        std::process::exit(3);
    }
}
