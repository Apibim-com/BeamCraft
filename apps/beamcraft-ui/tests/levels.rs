// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
// Contributor-original synthetic levels; no commercial model or catalogue data.

//! The levels form runs the CLI's own commands: one gesture is one revision and one undo unit.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use beamcraft_json::{Json, Limits};
use beamcraft_ui::levels::{self, LevelForm};
use beamcraft_ui::stats::Stats;
use beamcraft_ui::tools::LevelChoice;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
const ACTOR: &str = "0199e7a0-0000-7000-8000-0000000000a1";

struct Fixture {
    root: PathBuf,
    cli: PathBuf,
    workspace: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let cli = Path::new(env!("CARGO_BIN_EXE_beamcraft-ui"))
            .parent()
            .unwrap()
            .join(format!("beamcraft{}", std::env::consts::EXE_SUFFIX));
        assert!(
            cli.is_file(),
            "build beamcraft-cli before running level UI acceptance"
        );
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("level-ui-tests")
            .join(format!(
                "{}-{}",
                crate::support::boot_nonce(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&root).unwrap();
        let workspace = root.join("draft");
        let f = Self {
            root,
            cli,
            workspace,
        };
        f.ok(&[
            "create-project",
            "--parent",
            f.root.to_str().unwrap(),
            "--name",
            "ui",
            "--actor-id",
            ACTOR,
            "--config-dir",
            f.root.join("config").to_str().unwrap(),
        ]);
        f.ok(&[
            "create-workspace",
            "--store",
            f.root.join("ui").to_str().unwrap(),
            "--workspace-dir",
            f.workspace.to_str().unwrap(),
        ]);
        f
    }

    fn ok(&self, args: &[&str]) -> Json {
        let out = Command::new(&self.cli).args(args).output().unwrap();
        let json = beamcraft_json::decode(&out.stdout, &Limits::DEFAULT).unwrap();
        assert_eq!(out.status.code(), Some(0), "{args:?}: {json:?}");
        json
    }

    fn head(&self) -> Json {
        self.ok(&["open-revision", "--store", self.workspace.to_str().unwrap()])
    }

    fn history(&self, command: &str) {
        self.ok(&[command, "--workspace-dir", self.workspace.to_str().unwrap()]);
    }

    fn levels(&self) -> Vec<LevelChoice> {
        let reply = self.ok(&["levels", "--store", self.workspace.to_str().unwrap()]);
        reply
            .get("levels")
            .unwrap()
            .array()
            .unwrap()
            .iter()
            .map(|level| LevelChoice {
                id: level.get("id").unwrap().string().unwrap().to_owned(),
                name: level.get("name").unwrap().string().unwrap().to_owned(),
                elevation_mm: level.get("elevation_mm").unwrap().float().unwrap(),
                elements: level.get("elements").unwrap().array().unwrap().len(),
            })
            .collect()
    }

    fn commit(&self, args: &[String]) -> Result<(), beamcraft_ui::tools::ToolError> {
        levels::commit(&self.cli, args, &Stats::default())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn form(name: &str, elevation: &str) -> LevelForm {
    LevelForm {
        name: name.into(),
        elevation: elevation.into(),
    }
}

#[test]
fn a_level_is_created_edited_undone_and_deleted_through_the_form() {
    let f = Fixture::new();
    let store = f.workspace.clone();
    f.commit(
        &form("Level 1", "3000")
            .prepare_create(&store, Some(ACTOR))
            .unwrap(),
    )
    .unwrap();
    let created = f.levels().remove(0);
    assert_eq!(
        (created.name.as_str(), created.elevation_mm),
        ("Level 1", 3000.0)
    );

    f.commit(
        &form("Storey 1", "3300.5")
            .prepare_update(&created, &store, Some(ACTOR))
            .unwrap(),
    )
    .unwrap();
    let edited = f.levels().remove(0);
    assert_eq!(
        (edited.name.as_str(), edited.elevation_mm),
        ("Storey 1", 3300.5)
    );
    assert_eq!(edited.id, created.id);

    // The whole edit is one undo unit, although it wrote the name and the elevation.
    f.history("undo-workspace");
    assert_eq!(f.levels(), std::slice::from_ref(&created));
    f.history("redo-workspace");
    assert_eq!(f.levels(), std::slice::from_ref(&edited));

    let before = f.head();
    assert_ne!(
        f.commit(
            &form("Storey 1", "1e12")
                .prepare_update(&edited, &store, Some(ACTOR))
                .unwrap(),
        )
        .unwrap_err()
        .code,
        ""
    );
    assert_eq!(f.head(), before, "a refusal writes nothing");

    f.commit(&levels::prepare_delete(&edited, &store, Some(ACTOR)).unwrap())
        .unwrap();
    assert!(f.levels().is_empty());
    f.history("undo-workspace");
    assert_eq!(f.levels(), [edited]);
}
