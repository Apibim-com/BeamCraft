// SPDX-License-Identifier: MPL-2.0
//! One-case process entry point for the offline supervisor.

use beamcraft_n04_truck_spike::{Case, evaluate};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let name = args.next().ok_or("case name required")?;
    let mode = args.next().ok_or("caught/direct required")?;
    let output = PathBuf::from(args.next().ok_or("output prefix required")?);
    let case = Case::named(&name).ok_or("unknown bounded case")?;
    let result = match mode.as_str() {
        // All state is created inside the closure and discarded after unwinding.
        // No truck Solid with interior mutability crosses this recovery boundary.
        "caught" => match std::panic::catch_unwind(|| evaluate(&case)) {
            Ok(result) => result,
            Err(_) => {
                println!("{name}\tPANIC\tcaught unwind");
                return Ok(());
            }
        },
        "direct" => evaluate(&case),
        _ => return Err("unknown mode".into()),
    };
    match result {
        Ok(value) => {
            std::fs::write(output.with_extension("mesh"), value.mesh_bytes)?;
            std::fs::write(output.with_extension("tri"), value.triangle_bytes)?;
            println!("{}", value.row);
        }
        Err(error) => println!("{name}\tFAIL\t{error}"),
    }
    Ok(())
}
