// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
// Contributor-original synthetic plates and driver, MPL-2.0. The OpenUSD Python package is used
// only as a local, optional test oracle: it is not vendored, not a dependency and nothing from it
// is committed.

//! Independent-reader oracle: plates created by the real `beamcraft create-plate` binary are read
//! by the OpenUSD reference implementation (`pxr`, through the script `pxr_oracle.py`) and
//! compared with the closed form and the CLI output.
//!
//! The test finds an interpreter with `pxr` in this order: the `BEAMCRAFT_PXR_PYTHON` environment
//! variable, `py -3.11`, then `%LOCALAPPDATA%\Programs\Python\Python311\python.exe`. If none can
//! `import pxr` it prints an explicit SKIP message and passes without running anything: CI has no
//! pxr. A skip is never evidence. Nothing is downloaded or installed.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use beamcraft_protocol::{
    json::{self, Json},
    message,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

fn num(json: &Json, path: &[&str]) -> f64 {
    path.iter()
        .fold(json, |j, k| j.get(k).unwrap())
        .float()
        .unwrap()
}

fn plate_args<'a>(
    store: &'a str,
    w: &'a str,
    l: &'a str,
    t: &'a str,
    extra: &[&'a str],
) -> Vec<&'a str> {
    let mut args = vec![
        "create-plate",
        "--store",
        store,
        "--width",
        w,
        "--length",
        l,
        "--thickness",
        t,
    ];
    args.extend_from_slice(extra);
    args
}

#[test]
fn pxr_reads_the_committed_revision_and_agrees_with_the_closed_form() {
    let Some((python, python_args)) = find_python() else {
        println!(
            "SKIP: no Python interpreter with pxr found (set BEAMCRAFT_PXR_PYTHON, or install \
             usd-core for `py -3.11`). The independent pxr gate was NOT run."
        );
        return;
    };
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("p01-pxr");
    fs::create_dir_all(&base).unwrap();
    let root = base.join(format!("run-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let store = root.join("store");
    let layers = root.join("layers");
    fs::create_dir_all(&store).unwrap();
    fs::create_dir_all(&layers).unwrap();
    let store_arg = store.to_str().unwrap();

    let a = run_binary(&plate_args(
        store_arg,
        "1000",
        "500",
        "10",
        &["--name", "Oracle A"],
    ));
    let b = run_binary(&plate_args(
        store_arg,
        "800",
        "300",
        "12.5",
        &["--translation", "2000,-150.5,25", "--name", "Oracle B"],
    ));
    let inspected = run_binary(&["plate-inspect", "--store", store_arg]);
    copy_layers(&store, &layers, &inspected);
    let expectation = |reply: &Json, name: &str, w: f64, l: f64, t: f64, tr: [f64; 3]| {
        format!(
            r#"{{"id":"{}","name":"{name}","width":{w:?},"length":{l:?},"thickness":{t:?},"translation":[{:?},{:?},{:?}],"cli_volume_mm3":{:?}}}"#,
            text(reply.get("plate").unwrap(), "element_id"),
            tr[0],
            tr[1],
            tr[2],
            num(reply, &["plate", "quantities", "volume", "mm3"]),
        )
    };
    let expect = format!(
        "[{},{}]",
        expectation(&a, "Oracle A", 1000.0, 500.0, 10.0, [0.0, 0.0, 0.0]),
        expectation(&b, "Oracle B", 800.0, 300.0, 12.5, [2000.0, -150.5, 25.0]),
    );
    let report = run_script(&python, &python_args, &layers, &expect);
    assert_eq!(report.get("plates").unwrap().array().unwrap().len(), 2);
    let _ = fs::remove_dir_all(&root);
}

/// Copies the layers of the head revision under their manifest names, where a plain USD reader
/// finds them next to the root layer.
fn copy_layers(store: &Path, layers: &Path, inspected: &Json) {
    for entry in inspected
        .get("revision")
        .unwrap()
        .get("manifest")
        .unwrap()
        .get("layers")
        .unwrap()
        .array()
        .unwrap()
    {
        let digest = text(entry, "digest");
        fs::copy(
            store.join("blobs").join(&digest[..2]).join(&digest),
            layers.join(text(entry, "name")),
        )
        .unwrap();
    }
}

/// Runs the pxr script over the copied layers and returns its report after checking that the
/// script passed and the OpenUSD compliance checker reported nothing.
fn run_script(python: &str, python_args: &[String], layers: &Path, expect: &str) -> Json {
    let expect_path = layers.join("expect.json");
    fs::write(&expect_path, expect).unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("pxr_oracle.py");
    let output = Command::new(python)
        .args(python_args)
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
    for key in ["errors", "failed_checks", "warnings"] {
        assert!(
            compliance.get(key).unwrap().array().unwrap().is_empty(),
            "compliance {key}: {compliance:?}"
        );
    }
    report
}

#[test]
fn pxr_reads_pinned_materials_and_the_mass_agrees_with_the_closed_form() {
    let Some((python, python_args)) = find_python() else {
        println!(
            "SKIP: no Python interpreter with pxr found (set BEAMCRAFT_PXR_PYTHON, or install \
             usd-core for `py -3.11`). The independent pxr gate was NOT run."
        );
        return;
    };
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("p02-pxr");
    fs::create_dir_all(&base).unwrap();
    let root = base.join(format!("run-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let store = root.join("store");
    let layers = root.join("layers");
    fs::create_dir_all(&store).unwrap();
    fs::create_dir_all(&layers).unwrap();
    let store_arg = store.to_str().unwrap();

    let digest_of = |reply: &Json| text(reply.get("definition").unwrap(), "digest");
    let steel = digest_of(&run_binary(&[
        "define-material",
        "--store",
        store_arg,
        "--density",
        "7850",
    ]));
    let bare = digest_of(&run_binary(&["define-material", "--store", store_arg]));
    // A profile is published too: pxr must read it as an ordinary immutable definition prim.
    run_binary(&[
        "define-profile",
        "--store",
        store_arg,
        "--width",
        "200",
        "--height",
        "300",
    ]);
    let pinned = run_binary(&plate_args(
        store_arg,
        "1000",
        "500",
        "10",
        &["--name", "Steel", "--material-digest", &steel],
    ));
    let unknown = run_binary(&plate_args(
        store_arg,
        "800",
        "300",
        "12.5",
        &["--name", "Unknown", "--material-digest", &bare],
    ));
    let plain = run_binary(&plate_args(
        store_arg,
        "600",
        "400",
        "8",
        &["--name", "Plain"],
    ));
    let inspected = run_binary(&["plate-inspect", "--store", store_arg]);
    copy_layers(&store, &layers, &inspected);
    let expectation = |reply: &Json,
                       name: &str,
                       w: f64,
                       l: f64,
                       t: f64,
                       material: Option<(&str, Option<f64>)>| {
        let quantities = |key: &[&str]| {
            let mut node = reply.get("plate").unwrap().get("quantities").unwrap();
            for k in key {
                node = node.get(k).unwrap();
            }
            node
        };
        let mass = match quantities(&["mass", "kg"]) {
            Json::Null => "null".to_owned(),
            value => format!("{:?}", value.float().unwrap()),
        };
        let material = material.map_or("null".to_owned(), |(digest, density)| {
            let density = density.map_or("null".to_owned(), |d| format!("{d:?}"));
            format!(r#"{{"digest":"{digest}","density":{density}}}"#)
        });
        format!(
            r#"{{"id":"{}","name":"{name}","width":{w:?},"length":{l:?},"thickness":{t:?},"translation":[0.0,0.0,0.0],"cli_volume_mm3":{:?},"cli_mass_kg":{mass},"material":{material}}}"#,
            text(reply.get("plate").unwrap(), "element_id"),
            num(reply, &["plate", "quantities", "volume", "mm3"]),
        )
    };
    let expect = format!(
        "[{},{},{}]",
        expectation(
            &pinned,
            "Steel",
            1000.0,
            500.0,
            10.0,
            Some((&steel, Some(7850.0)))
        ),
        expectation(&unknown, "Unknown", 800.0, 300.0, 12.5, Some((&bare, None))),
        expectation(&plain, "Plain", 600.0, 400.0, 8.0, None),
    );
    // The closed-form mass of the pinned plate: 0.005 m3 * 7850 kg/m3.
    assert_eq!(num(&pinned, &["plate", "quantities", "mass", "kg"]), 39.25);
    let report = run_script(&python, &python_args, &layers, &expect);
    assert_eq!(report.get("plates").unwrap().array().unwrap().len(), 3);
    let _ = fs::remove_dir_all(&root);
}
