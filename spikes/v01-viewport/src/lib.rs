// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
//! Native-only, isolated V01 measurement harness; no engineering geometry in the renderer.
#![cfg(not(target_arch = "wasm32"))]
pub mod camera;
pub mod render;
pub mod scene;

/// Bounded spike failure, including adapter absence rather than a fabricated result.
#[derive(Debug)]
pub enum Error {
    /// Caller exceeded a spike limit or supplied non-finite camera input.
    Input(&'static str),
    /// Real kernel evaluation failed.
    Kernel(beamcraft_kernel::diagnostic::Failure),
    /// No native adapter exists.
    NoAdapter,
    /// Graphics device, validation, mapping or completion failure.
    Graphics(String),
    /// A bounded CPU allocation could not be reserved.
    Allocation,
    /// Filesystem operation failed.
    Io(std::io::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

/// JSON string escaping without adding a serialization dependency.
pub fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
