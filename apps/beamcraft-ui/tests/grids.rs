// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
// Contributor-original synthetic grids; no commercial model or catalogue data.

//! Grid insertion uses the same CLI command as a script, and the pinned refresh carries guides.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use beamcraft_json::{Json, Limits};
use beamcraft_ui::camera::Camera;
use beamcraft_ui::grid::{GridForm, GridScene};
use beamcraft_ui::source::{self, Known, Refreshed};
use beamcraft_ui::stats::Stats;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
const ACTOR: &str = "0199e7a0-0000-7000-8000-0000000000a1";

struct Fixture {
    root: PathBuf,
    cli: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let cli = Path::new(env!("CARGO_BIN_EXE_beamcraft-ui"))
            .parent()
            .unwrap()
            .join(format!("beamcraft{}", std::env::consts::EXE_SUFFIX));
        assert!(
            cli.is_file(),
            "build beamcraft-cli before running grid UI acceptance"
        );
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("grid-ui-tests")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&root).unwrap();
        let f = Self { root, cli };
        for name in ["ui", "script"] {
            f.ok(&[
                "create-project",
                "--parent",
                f.path().to_str().unwrap(),
                "--name",
                name,
                "--actor-id",
                ACTOR,
                "--config-dir",
                f.root.join("config").to_str().unwrap(),
            ]);
        }
        f
    }
    fn path(&self) -> &Path {
        &self.root
    }
    fn run(&self, args: &[&str]) -> (i32, Json) {
        let out = Command::new(&self.cli).args(args).output().unwrap();
        (
            out.status.code().unwrap(),
            beamcraft_json::decode(&out.stdout, &Limits::DEFAULT).unwrap(),
        )
    }
    fn ok(&self, args: &[&str]) -> Json {
        let (exit, json) = self.run(args);
        assert_eq!(exit, 0, "{args:?}: {json:?}");
        json
    }
    fn head(&self, store: &Path) -> Json {
        self.ok(&["open-revision", "--store", store.to_str().unwrap()])
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn at<'a>(json: &'a Json, keys: &[&str]) -> &'a Json {
    keys.iter().fold(json, |j, k| j.get(k).unwrap())
}

#[test]
fn inserted_grid_matches_the_cli_and_is_displayed_from_one_pinned_read() {
    let f = Fixture::new();
    let store = f.root.join("ui");
    let other = f.root.join("script");
    let form = GridForm {
        name: "Frame grid".into(),
        origin: ["1250".into(), "-750".into()],
        rotation: "20".into(),
        ..GridForm::default()
    };
    // Filling and validating the form is read-only; cancelling before insertion writes nothing.
    let before = f.head(&store);
    let args = form.prepare(&store, Some(ACTOR)).unwrap();
    assert_eq!(f.head(&store), before);
    let created = beamcraft_ui::grid::commit(&f.cli, &args, &Stats::default()).unwrap();
    f.ok(&[
        "create-grid",
        "--store",
        other.to_str().unwrap(),
        "--name",
        "Frame grid",
        "--origin",
        "1250,-750",
        "--rotation",
        "20",
        "--u-expression",
        "0 2*4000",
        "--v-expression",
        "0 2*5000",
        "--actor-id",
        ACTOR,
    ]);
    let listed = f.ok(&["list-grids", "--store", store.to_str().unwrap()]);
    let script = f.ok(&["list-grids", "--store", other.to_str().unwrap()]);
    for family in ["u", "v"] {
        let left = at(&listed, &["grids"]).array().unwrap()[0]
            .get(family)
            .unwrap()
            .get("lines")
            .unwrap()
            .array()
            .unwrap();
        let right = at(&script, &["grids"]).array().unwrap()[0]
            .get(family)
            .unwrap()
            .get("lines")
            .unwrap()
            .array()
            .unwrap();
        assert_eq!(left.len(), 3);
        for (a, b) in left.iter().zip(right) {
            for key in ["label", "start_mm", "end_mm"] {
                assert_eq!(a.get(key), b.get(key));
            }
        }
    }
    let stats = Stats::default();
    let refreshed = source::reload(&f.cli, &store, &Known::default(), &stats);
    assert_eq!(stats.cli_runs.get(), 1);
    let guides = refreshed.grids.unwrap();
    assert_eq!(guides.lines.len(), 6);
    let Refreshed::New(scene) = refreshed.scene.unwrap() else {
        panic!("expected a first scene");
    };
    assert_eq!(
        scene.0.pin().revision().to_string(),
        created.revision.manifest_digest
    );
    let summary = scene.1.summary();
    let (target, extent) = guides.framing(&summary);
    let view = Camera::new(target, extent, 1.5).unwrap().view(1.5).unwrap();
    for line in &guides.lines {
        for point in line.points {
            let local = std::array::from_fn(|i| (point[i] - summary.origin_mm[i]) as f32);
            let [x, y, z] = view.project(local);
            assert!(x.abs() <= 1.0 && y.abs() <= 1.0 && (0.0..=1.0).contains(&z));
        }
    }
    assert_eq!(
        f.head(&store).get("revision_id").unwrap().string().unwrap(),
        created.revision.revision_id
    );
}

#[test]
fn invalid_input_and_engine_refusals_publish_no_grid() {
    let f = Fixture::new();
    let store = f.root.join("ui");
    let before = f.head(&store);
    let mut form = GridForm::default();
    for expression in ["NaN 4000", "0 0", "0 201*1", "0 -1"] {
        form.u.expression = expression.into();
        assert!(form.prepare(&store, None).is_err());
    }
    form = GridForm::default();
    form.u.labels = "1 2 3 4 5 6 7 8 9".into();
    let mismatch = form.prepare(&store, None).unwrap_err();
    assert_eq!(mismatch.field, "U labels");
    assert!(mismatch.message.contains("3 axes"));
    assert!(mismatch.message.contains("9 labels"));
    form.u.labels = "1, 2, 3".into();
    form.v.labels = "A B C D E F".into();
    let mismatch = form.prepare(&store, None).unwrap_err();
    assert_eq!(mismatch.field, "V labels");
    assert!(mismatch.message.contains("3 axes"));
    assert!(mismatch.message.contains("6 labels"));
    form.v.labels = "A B C".into();
    assert!(form.prepare(&store, None).is_ok());
    form.v.labels = "A A C".into();
    let duplicate = form.prepare(&store, None).unwrap_err();
    assert_eq!(duplicate.field, "V labels");
    assert!(duplicate.message.contains("unique"));
    assert_eq!(f.head(&store), before);
    form = GridForm::default();
    form.u.labels = "A B C".into();
    form.v.labels = "A B C".into();
    let args = form.prepare(&store, Some(ACTOR)).unwrap();
    let refused = beamcraft_ui::grid::commit(&f.cli, &args, &Stats::default()).unwrap_err();
    assert_eq!(refused.code, "INVALID_ARGUMENT");
    assert!(refused.message.contains("unique"));
    assert!(!refused.message.contains("GRID_LABEL_DUPLICATE"));
    assert_eq!(f.head(&store), before);
    assert!(GridScene::from_reply(br#"{"outcome":"grids","grids":[{"u":{"lines":[{"label":"A","start_mm":[0,1],"end_mm":[0,1,0]}]},"v":{"lines":[]}}]}"#).is_err());
}
