//! Bounded child-process smoke test using only Rust std and the stdlib supervisor.
use beamcraft_n01_openusd_spike::{Result, failure};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn hostile_child_smoke_finishes_under_external_deadline() -> Result<()> {
    let mut child = Command::new("python")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "harness.py",
            "hostile",
            "--smoke",
            "--binary",
            env!("CARGO_BIN_EXE_beamcraft-n01-openusd-spike"),
        ])
        .stdin(Stdio::null())
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(50);
    loop {
        if let Some(status) = child.try_wait()? {
            assert!(status.success(), "supervisor failed: {status}");
            break;
        }
        if Instant::now() >= deadline {
            child.kill()?;
            child.wait()?;
            return Err(failure(
                "hostile supervisor exceeded 50-second suite deadline",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}
