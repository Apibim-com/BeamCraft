// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
//! Real CLI equivalence and no-write checks for the native grid workbench forms.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
use beamcraft_json::{Json, Limits};
use beamcraft_ui::grid_tools::{GridAction, GridTools};
use std::{path::PathBuf, process::Command};
fn cli() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("BEAMCRAFT_TEST_CLI") {
        return Some(p.into());
    }
    if let Some(p) = std::env::var_os("NEXTEST_BIN_EXE_beamcraft") {
        return Some(p.into());
    }
    let p = std::env::current_exe()
        .ok()?
        .parent()?
        .parent()?
        .join(if cfg!(windows) {
            "beamcraft.exe"
        } else {
            "beamcraft"
        });
    p.is_file().then_some(p)
}
fn run(cli: &std::path::Path, args: &[String]) -> Json {
    let o = Command::new(cli).args(args).output().unwrap();
    let j = beamcraft_json::decode(&o.stdout, &Limits::DEFAULT).unwrap();
    assert!(o.status.success(), "{args:?}: {j:?}");
    j
}
fn query(cli: &std::path::Path, store: &std::path::Path, command: &str) -> Json {
    run(
        cli,
        &[
            command.into(),
            "--store".into(),
            store.to_str().unwrap().into(),
        ],
    )
}
#[test]
fn form_commands_match_headless_grid_geometry_and_cancel_visibility_publish_nothing() {
    let Some(cli) = cli() else {
        eprintln!("SKIP real CLI grid form test: set BEAMCRAFT_TEST_CLI");
        return;
    };
    let root = std::env::temp_dir().join(format!(
        "beamcraft-grid-ui-{}",
        format_args!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )
    ));
    std::fs::create_dir(&root).unwrap();
    let a = root.join("ui");
    let b = root.join("cli");
    std::fs::create_dir(&a).unwrap();
    std::fs::create_dir(&b).unwrap();
    let mut tool = GridTools::default();
    tool.form.name = "UI grid".into();
    tool.form.u_expression = "0 2*6000".into();
    tool.form.v_expression = "0 7500".into();
    tool.form.origin = "1000,2000".into();
    tool.form.rotation = "90".into();
    tool.staged = Some(tool.form.prepare(&a).unwrap());
    run(&cli, tool.staged.as_ref().unwrap());
    run(
        &cli,
        &[
            "create-grid",
            "--store",
            b.to_str().unwrap(),
            "--name",
            "UI grid",
            "--u-expression",
            "0 2*6000",
            "--v-expression",
            "0 7500",
            "--origin",
            "1000,2000",
            "--rotation",
            "90",
        ]
        .map(str::to_owned),
    );
    let listing = query(&cli, &a, "list-grids");
    tool.apply_listing(&listing).unwrap();
    let mut direct = GridTools::default();
    direct
        .apply_listing(&query(&cli, &b, "list-grids"))
        .unwrap();
    let gu = &tool.grids[0];
    let gc = &direct.grids[0];
    assert_eq!(gu.name, gc.name);
    assert_eq!(gu.translation_mm, gc.translation_mm);
    assert_eq!(gu.orientation_zw, gc.orientation_zw);
    let lines = |g| {
        beamcraft_engine::grid_query::segments(g)
            .unwrap()
            .into_iter()
            .map(|l| (l.label, l.start, l.end))
            .collect::<Vec<_>>()
    };
    assert_eq!(lines(gu), lines(gc));
    let grid = gu.clone();
    tool.form.select(&grid);
    tool.form.action = GridAction::Update;
    tool.form.u_expression = "0 6500 5500".into();
    run(&cli, &tool.form.prepare(&a).unwrap());
    tool.apply_listing(&query(&cli, &a, "list-grids")).unwrap();
    assert_eq!(
        grid.axis_ids().collect::<Vec<_>>(),
        tool.grids[0].axis_ids().collect::<Vec<_>>()
    );
    let head = query(&cli, &a, "open-revision");
    tool.form.name = "Pending rename".into();
    tool.staged = Some(tool.form.prepare(&a).unwrap());
    tool.drawing = true;
    tool.click([1., 2., 0.]);
    tool.cancel();
    tool.hidden.insert(grid.id);
    tool.labels = false;
    tool.snap = false;
    assert_eq!(head, query(&cli, &a, "open-revision"));
    assert!(tool.staged.is_none());
    assert!(tool.first.is_none());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn grid_visibility_and_snap_switch_control_only_the_pinned_read_query() {
    let Some(cli) = cli() else {
        return;
    };
    let root = std::env::temp_dir().join(format!(
        "beamcraft-grid-ui-snap-{}",
        format_args!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )
    ));
    std::fs::create_dir(&root).unwrap();
    let mut tool = GridTools::default();
    run(&cli, &tool.form.prepare(&root).unwrap());
    tool.apply_listing(&query(&cli, &root, "list-grids"))
        .unwrap();
    let pin = beamcraft_engine::ViewPin::new(tool.manifest.as_ref().unwrap().parse().unwrap(), 1);
    let snapshot =
        beamcraft_engine::ViewSnapshot::build(pin, Vec::new(), &Default::default()).unwrap();
    let p = [3., 2., 3000.];
    assert_ne!(tool.snap_point(&snapshot, p).unwrap(), p);
    tool.hidden.insert(tool.grids[0].id);
    assert_eq!(tool.snap_point(&snapshot, p).unwrap(), p);
    tool.hidden.clear();
    tool.snap = false;
    assert_eq!(tool.snap_point(&snapshot, p).unwrap(), p);
    std::fs::remove_dir_all(root).unwrap();
}
