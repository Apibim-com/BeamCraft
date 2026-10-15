//! Headless entry point; hostile cases wait for the supervisor to install memory limits.
#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unreachable,
    clippy::indexing_slicing
)]
use std::io::Read;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mode = args.next().ok_or("mode required")?;
    let mut gate = [0_u8; 1];
    std::io::stdin().read_exact(&mut gate)?;
    beamcraft_n02_ifc_lite_spike::run(&mode)
}
