// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
// Contributor-original synthetic model and frames, MPL-2.0.

//! Real CLI view lifecycle and immutable model regression coverage.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod support;
use beamcraft_json::Json;
use support::{StoreFixture, text};

fn ok(f: &StoreFixture, command: &str, args: &[&str]) -> Json {
    let (code, reply) = f.run(command, args);
    assert_eq!(code, 0, "{command}: {reply:?}");
    reply
}
fn fixture() -> StoreFixture {
    let f = StoreFixture::new("model-views");
    ok(
        &f,
        "create-plate",
        &["--width", "1000", "--length", "500", "--thickness", "10"],
    );
    f
}
fn views(reply: &Json) -> &[Json] {
    reply.get("views").unwrap().array().unwrap()
}
#[test]
fn all_construction_modes_reopen_without_changing_the_model_or_snapshot() {
    let f = fixture();
    let before = ok(&f, "open-revision", &[]);
    let snapshot = ok(&f, "inspect-elements", &[]);
    for (name, args) in [
        ("Plan", vec!["--parallel", "xy", "--offset", "3000"]),
        ("Section", vec!["--two-points", "10,20,30,1010,20,80"]),
        (
            "Inclined",
            vec!["--three-points", "0,0,0,1000,0,100,0,1000,200"],
        ),
        ("Work", vec!["--plane", "0,100,0,1,0,0,0,0,1"]),
    ] {
        let mut all = vec!["--name", name];
        all.extend(args);
        ok(&f, "create-model-view", &all);
    }
    let list = ok(&f, "model-views", &[]);
    assert_eq!(views(&list).len(), 4);
    let id = text(&views(&list)[0], "id");
    ok(
        &f,
        "update-model-view",
        &[
            "--id",
            &id,
            "--name",
            "Level 3",
            "--projection",
            "perspective",
            "--depth",
            "150,250",
            "--folder",
            "Plans/Upper",
            "--favorite",
            "true",
            "--hidden-kinds",
            "beam,column",
            "--work-area",
            "0,0,0,1000,500,4000",
        ],
    );
    let after = ok(&f, "model-views", &[]);
    assert_eq!(text(&views(&after)[0], "name"), "Level 3");
    assert_eq!(text(&views(&after)[0], "folder"), "Plans/Upper");
    assert_eq!(text(&views(&after)[0], "hidden_kinds"), "beam,column");
    assert_eq!(ok(&f, "open-revision", &[]), before);
    assert_eq!(ok(&f, "inspect-elements", &[]), snapshot);
    ok(&f, "delete-model-view", &["--id", &id]);
    assert_eq!(views(&ok(&f, "model-views", &[])).len(), 3);
}
#[test]
fn invalid_changes_preserve_the_collection_and_temporary_views_are_not_saved() {
    let f = fixture();
    ok(&f, "create-model-view", &["--name", "Plan"]);
    let path = f.root.join("store/views.usda");
    let before = std::fs::read(&path).unwrap();
    for args in [
        vec!["--name", "Plan"],
        vec!["--name", "Bad", "--span", "NaN"],
        vec!["--name", "Reserved", "--reserved-views", "255"],
        vec!["--name", "Bad", "--three-points", "0,0,0,1,0,0,2,0,0"],
        vec!["--name", "Bad", "--depth", "-1,2"],
        vec!["--name", "Bad", "--work-area", "2,0,0,1,1,1"],
    ] {
        assert_eq!(f.run("create-model-view", &args).0, 2);
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    let reply = ok(
        &f,
        "create-model-view",
        &["--name", "Surface preview", "--temporary", "true"],
    );
    assert_eq!(views(&reply).len(), 2);
    assert_eq!(views(&ok(&f, "model-views", &[])).len(), 1);
    std::fs::write(&path, b"broken collection").unwrap();
    assert_eq!(f.run("create-model-view", &["--name", "New"]).0, 2);
    assert_eq!(std::fs::read(&path).unwrap(), b"broken collection");
}
#[test]
fn grid_level_surface_and_four_part_views_use_real_model_geometry() {
    let f = fixture();
    ok(
        &f,
        "create-grid",
        &[
            "--name",
            "Axes",
            "--u-expression",
            "0 4000",
            "--v-expression",
            "0 5000",
            "--u-labels",
            "A B",
            "--v-labels",
            "1 2",
        ],
    );
    ok(
        &f,
        "create-model-view",
        &["--name", "Grid", "--grid", "Axes", "--axes", "all"],
    );
    assert_eq!(views(&ok(&f, "model-views", &[])).len(), 4);
    let inspection = ok(&f, "inspect-elements", &[]);
    let id = text(
        &inspection.get("plates").unwrap().array().unwrap()[0],
        "element_id",
    );
    ok(
        &f,
        "create-model-view",
        &["--name", "Part", "--element", &id, "--side", "four"],
    );
    assert_eq!(views(&ok(&f, "model-views", &[])).len(), 8);
    ok(
        &f,
        "create-model-view",
        &[
            "--name",
            "Surface",
            "--ray-origin",
            "500,250,1000",
            "--ray-direction",
            "0,0,-1",
        ],
    );
    assert_eq!(views(&ok(&f, "model-views", &[])).len(), 9);
    ok(
        &f,
        "create-level",
        &["--name", "Upper", "--elevation", "3000"],
    );
    let result = ok(
        &f,
        "create-model-view",
        &["--name", "Upper plan", "--level", "Upper"],
    );
    assert!(text(views(&result).last().unwrap(), "plane").starts_with("0.0,0.0,3000.0,"));
    let assembly = "019a7bf0-0000-7000-8000-000000000aaa";
    ok(
        &f,
        "create-assembly",
        &["--name", "Frame", "--new-id", assembly],
    );
    ok(
        &f,
        "attach-to-assembly",
        &["--id", &id, "--assembly-id", assembly],
    );
    let result = ok(
        &f,
        "create-model-view",
        &["--name", "Frame", "--group", assembly, "--side", "four"],
    );
    assert_eq!(views(&result).len(), 14);
}

#[test]
fn folders_and_global_appearance_persist_and_refuse_destructive_folder_removal() {
    let f = fixture();
    let list = ok(&f, "create-view-folder", &["--folder", "Plans/Upper"]);
    assert!(
        list.get("folders")
            .unwrap()
            .array()
            .unwrap()
            .iter()
            .any(|v| v.string().unwrap() == "Plans/Upper")
    );
    ok(
        &f,
        "create-model-view",
        &["--name", "Upper", "--folder", "Plans/Upper"],
    );
    assert_eq!(f.run("delete-view-folder", &["--folder", "Plans"]).0, 2);
    let before = ok(&f, "open-revision", &[]);
    let reply = ok(
        &f,
        "update-model-view",
        &[
            "--all",
            "true",
            "--color",
            "0.2,0.5,0.8",
            "--opacity",
            "0.4",
        ],
    );
    assert_eq!(
        views(&reply)[0].get("opacity").unwrap().float().unwrap(),
        0.4
    );
    assert_eq!(ok(&f, "open-revision", &[]), before);
    let id = text(&views(&reply)[0], "id");
    ok(&f, "update-model-view", &["--id", &id, "--folder", ""]);
    ok(&f, "delete-view-folder", &["--folder", "Plans"]);
    assert!(
        ok(&f, "model-views", &[])
            .get("folders")
            .unwrap()
            .array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn independent_grid_families_reuse_properties_and_generate_unique_names_atomically() {
    let f = fixture();
    ok(
        &f,
        "create-grid",
        &[
            "--name",
            "Axes",
            "--u-expression",
            "-5000 5000 3000",
            "--v-expression",
            "0 4000",
            "--u-labels",
            "Far Near Mid",
            "--v-labels",
            "NearV EndV",
        ],
    );
    ok(
        &f,
        "create-model-view",
        &[
            "--name",
            "Detail settings",
            "--depth",
            "10,20",
            "--projection",
            "perspective",
            "--opacity",
            "0.6",
        ],
    );
    let args = [
        "--name",
        "Batch",
        "--grid",
        "Axes",
        "--u-axes",
        "first",
        "--v-axes",
        "none",
        "--u-prefix",
        "Section",
        "--u-properties",
        "Detail settings",
    ];
    let result = ok(&f, "create-model-view", &args);
    let created = views(&result).last().unwrap();
    assert_eq!(text(created, "name"), "Section Near");
    assert_eq!(text(created, "projection"), "perspective");
    assert_eq!(created.get("opacity").unwrap().float().unwrap(), 0.6);
    let result = ok(&f, "create-model-view", &args);
    assert_eq!(
        text(views(&result).last().unwrap(), "name"),
        "Section Near (2)"
    );
    let bytes = std::fs::read(f.root.join("store/views.usda")).unwrap();
    assert!(bytes.starts_with(b"#usda 1.0"));
    let layer = std::str::from_utf8(&bytes).unwrap();
    assert!(layer.contains("def Camera"));
    assert!(layer.contains("xformOp:transform"));
    for invalid in [
        vec!["--name", "Bad", "--side", "front"],
        vec!["--name", "Bad", "--edge", "0,0,0,1,0,0"],
        vec!["--name", "Bad", "--grid", "Axes", "--u-axes", "invalid"],
        vec![
            "--name",
            "Bad",
            "--grid",
            "Axes",
            "--u-properties",
            "missing",
        ],
    ] {
        assert_eq!(f.run("create-model-view", &invalid).0, 2);
        assert_eq!(
            std::fs::read(f.root.join("store/views.usda")).unwrap(),
            bytes
        );
    }
    ok(
        &f,
        "create-level",
        &["--name", "Ground", "--elevation", "0"],
    );
    ok(
        &f,
        "create-level",
        &["--name", "Upper", "--elevation", "3000"],
    );
    let result = ok(
        &f,
        "create-model-view",
        &["--name", "Plan", "--levels", "all"],
    );
    assert_eq!(views(&result).len(), 5);
}
