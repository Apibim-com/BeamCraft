// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
// Contributor-original synthetic source and driver, MPL-2.0. The OpenUSD Python package is used
// only as a local, optional test oracle: it is not vendored, not a dependency and nothing from it
// is committed.

//! Independent-reader oracle for R01: a revision with a reference wrapper, written by the real
//! `beamcraft` binary, is read by the OpenUSD reference implementation (`pxr`, through
//! `reference_oracle.py`). pxr composes the foreign model below the owned wrapper, places its
//! points exactly where the engine's `source_point_to_world` says (a three-way agreement: pxr, the
//! engine function and a closed form), sees no owned opinion below the arc holder, and finds the
//! foreign layer byte-identical to the file that was attached.
//!
//! The interpreter is found as in `pxr_oracle.rs` (`BEAMCRAFT_PXR_PYTHON`, `py -3.11`, then the
//! per-user Python 3.11). If none can `import pxr` the test prints an explicit SKIP and passes
//! without running anything: CI has no pxr, and a skip is never evidence.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use beamcraft_engine::reference::ReferenceRecord;
use beamcraft_model::command::Placement;
use beamcraft_model::digest::Sha256;
use beamcraft_model::id::ElementId;
use beamcraft_model::reference::{SourceFormat, SourcePin, UnitsInterpretation, UpAxis};
use beamcraft_protocol::{
    json::{self, Json},
    message,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SITE: &str = r#"#usda 1.0
(
    defaultPrim = "Site"
    metersPerUnit = 1.0
    upAxis = "Y"
)
def Xform "Site"
{
    def Xform "Block"
    {
        custom string note = "declared by the source"
        def Mesh "Body"
        {
            point3f[] points = [(0, 0, 0), (2, 0, 0), (2, 1, 0), (0, 1, 0), (0, 0, 3), (2, 0, 3), (2, 1, 3), (0, 1, 3)]
            int[] faceVertexCounts = [4, 4, 4, 4, 4, 4]
            int[] faceVertexIndices = [0, 3, 2, 1, 4, 5, 6, 7, 0, 1, 5, 4, 1, 2, 6, 5, 2, 3, 7, 6, 3, 0, 4, 7]
        }
    }
}
"#;

fn probe(program: &str, args: &[&str]) -> Option<(String, Vec<String>)> {
    let mut probe_args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
    probe_args.extend(["-c".into(), "import pxr".into()]);
    let ok = Command::new(program)
        .args(&probe_args)
        .output()
        .is_ok_and(|o| o.status.success());
    ok.then(|| {
        (
            program.to_owned(),
            args.iter().map(|a| (*a).to_owned()).collect(),
        )
    })
}

fn find_python() -> Option<(String, Vec<String>)> {
    if let Ok(path) = std::env::var("BEAMCRAFT_PXR_PYTHON") {
        return probe(&path, &[]);
    }
    probe("py", &["-3.11"]).or_else(|| {
        let local = std::env::var("LOCALAPPDATA").ok()?;
        let path: PathBuf = [&local, "Programs", "Python", "Python311", "python.exe"]
            .iter()
            .collect();
        probe(path.to_str()?, &[])
    })
}

fn run_binary(args: &[&str]) -> Json {
    let output = Command::new(env!("CARGO_BIN_EXE_beamcraft"))
        .args(args)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    json::decode(&output.stdout, &message::read_limits()).unwrap()
}

fn text(json: &Json, key: &str) -> String {
    json.get(key).unwrap().string().unwrap().to_owned()
}

#[test]
fn pxr_composes_the_reference_and_places_the_foreign_points_as_the_engine_says() {
    let Some((python, python_args)) = find_python() else {
        println!(
            "SKIP: no Python interpreter with pxr found (set BEAMCRAFT_PXR_PYTHON, or install \
             usd-core for `py -3.11`). The independent pxr gate was NOT run."
        );
        return;
    };
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("r01-pxr");
    fs::create_dir_all(&base).unwrap();
    let root = base.join(format!("run-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let store = root.join("store");
    let layers = root.join("layers");
    fs::create_dir_all(&store).unwrap();
    fs::create_dir_all(layers.join("external")).unwrap();
    let source = root.join("site.usda");
    fs::write(&source, SITE).unwrap();
    let store_arg = store.to_str().unwrap();

    let attached = run_binary(&[
        "reference-attach",
        "--store",
        store_arg,
        "--source",
        source.to_str().unwrap(),
        "--name",
        "Oracle site",
    ]);
    let id = text(attached.get("reference").unwrap(), "id");
    // The source declares metres and Y up. The owner states exactly that, then places the wrapper
    // by a quarter turn about Z and a translation.
    run_binary(&[
        "reference-correct-units",
        "--store",
        store_arg,
        "--id",
        &id,
        "--meters-per-unit",
        "1",
        "--up-axis",
        "Y",
    ]);
    run_binary(&[
        "reference-move",
        "--store",
        store_arg,
        "--id",
        &id,
        "--translation",
        "10,20,30",
        "--rotation-z-deg",
        "90",
    ]);
    let inspected = run_binary(&["reference-inspect", "--store", store_arg]);
    let manifest = inspected
        .get("revision")
        .unwrap()
        .get("manifest")
        .unwrap()
        .get("layers")
        .unwrap()
        .array()
        .unwrap();
    for entry in manifest {
        let digest = text(entry, "digest");
        fs::copy(
            store.join("blobs").join(&digest[..2]).join(&digest),
            layers.join(text(entry, "name")),
        )
        .unwrap();
    }
    // The foreign layer pxr reads is byte-identical to the attached file.
    let digest = Sha256::digest(SITE.as_bytes()).unwrap().to_string();
    assert_eq!(
        fs::read(layers.join(format!("external/{digest}.usda"))).unwrap(),
        SITE.as_bytes()
    );

    // Expected world points: the engine's own mapping of the eight corners.
    let half = std::f64::consts::FRAC_1_SQRT_2;
    let record = ReferenceRecord {
        id: ElementId::parse(&id).unwrap(),
        name: "Oracle site".into(),
        placement: Placement {
            translation_mm: [10.0, 20.0, 30.0],
            orientation_xyzw: [0.0, 0.0, half, half],
        },
        source: SourcePin {
            digest: Sha256::digest(SITE.as_bytes()).unwrap(),
            bytes: SITE.len() as u64,
            format: SourceFormat::Usda,
            default_prim: "Site".into(),
            version: 1,
            label: "site.usda".into(),
            declared_meters_per_unit: Some(1.0),
            declared_up_axis: Some(UpAxis::Y),
        },
        units: Some(UnitsInterpretation {
            meters_per_unit: 1.0,
            up_axis: UpAxis::Y,
        }),
    };
    let corners = [
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [2.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 3.0],
        [2.0, 0.0, 3.0],
        [2.0, 1.0, 3.0],
        [0.0, 1.0, 3.0],
    ];
    let world: Vec<[f64; 3]> = corners
        .iter()
        .map(|p| record.source_point_to_world(*p).unwrap())
        .collect();
    // Closed form for the far corner (2, 1, 3) m: scale to mm (2000, 1000, 3000), Y up to Z up
    // (2000, -3000, 1000), a quarter turn about Z (3000, 2000, 1000), then the translation.
    let far = world[6];
    for (got, want) in far.iter().zip([3010.0, 2020.0, 1030.0]) {
        assert!((got - want).abs() < 1e-9, "{far:?}");
    }
    let min = (0..3).map(|i| world.iter().map(|p| p[i]).fold(f64::INFINITY, f64::min));
    let max = (0..3).map(|i| world.iter().map(|p| p[i]).fold(f64::NEG_INFINITY, f64::max));
    let list = |values: &mut dyn Iterator<Item = f64>| {
        format!(
            "[{}]",
            values
                .map(|v| format!("{v:?}"))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    let expect = format!(
        r#"{{"id":"{id}","name":"Oracle site","digest":"{digest}","world_min":{},"world_max":{},"world_points":[{}]}}"#,
        list(&mut min.clone()),
        list(&mut max.clone()),
        world
            .iter()
            .map(|p| list(&mut p.iter().copied()))
            .collect::<Vec<_>>()
            .join(","),
    );
    let expect_path = layers.join("expect.json");
    fs::write(&expect_path, expect).unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("reference_oracle.py");
    let output = Command::new(&python)
        .args(&python_args)
        .arg(&script)
        .arg(layers.join("root.usda"))
        .arg(&expect_path)
        .output()
        .unwrap();
    let report = String::from_utf8_lossy(&output.stdout).into_owned();
    println!("PXR REPORT: {report}");
    assert!(
        output.status.success(),
        "pxr rejected or disagreed: {}\n{report}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = json::decode(report.trim().as_bytes(), &message::read_limits()).unwrap();
    let compliance = report.get("compliance").unwrap();
    println!("COMPLIANCE: {compliance:?}");
    for key in ["errors", "failed_checks"] {
        assert!(
            compliance.get(key).unwrap().array().unwrap().is_empty(),
            "compliance {key}: {compliance:?}"
        );
    }
    let _ = fs::remove_dir_all(&root);
}
