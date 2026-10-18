// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
// Contributor-original synthetic plates, beams and assemblies, MPL-2.0.

//! A01 end to end: `create-assembly`, `assembly-attach`, `assembly-detach` and `assemblies`
//! through the real CLI binary and committed revisions. An assembly has its own identity and no
//! body; the `part-of` record lives with the part; kinds, cardinality, self-links and cycles are
//! refused with nothing written; an assembly with parts cannot be deleted and a part with a
//! record cannot be copied.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use beamcraft_protocol::{
    json::{self, Json},
    message,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

const FRAME: &str = "019a7bf0-0000-7000-8000-000000200001";
const OUTER: &str = "019a7bf0-0000-7000-8000-000000200002";
const OTHER: &str = "019a7bf0-0000-7000-8000-000000200003";

struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("a01-tests");
        fs::create_dir_all(&base).unwrap();
        let root = base.join(format!(
            "e2e-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("store")).unwrap();
        Self { root }
    }
    fn run(&self, command: &str, extra: &[&str]) -> (i32, Json) {
        let store = self.root.join("store").to_str().unwrap().to_owned();
        let mut args = vec![command, "--store", store.as_str()];
        args.extend_from_slice(extra);
        let output = Command::new(env!("CARGO_BIN_EXE_beamcraft"))
            .args(&args)
            .output()
            .unwrap();
        let body = json::decode(&output.stdout, &message::read_limits()).unwrap();
        (output.status.code().unwrap(), body)
    }
    fn ok(&self, command: &str, extra: &[&str]) -> Json {
        let (code, body) = self.run(command, extra);
        assert_eq!(code, 0, "{command} {extra:?}: {body:?}");
        body
    }
    fn rejected(&self, command: &str, extra: &[&str]) -> String {
        let before = self.head();
        let (code, body) = self.run(command, extra);
        assert_eq!(code, 2, "{command} {extra:?}: {body:?}");
        // A refusal writes no revision.
        assert_eq!(self.head(), before, "{command} {extra:?}");
        text(&body, "code")
    }
    fn head(&self) -> String {
        text(&self.ok("revision-open", &[]), "revision_id")
    }
    fn plate(&self) -> String {
        let body = self.ok(
            "create-plate",
            &["--width", "1000", "--length", "500", "--thickness", "10"],
        );
        text(at(&body, &["plate"]), "element_id")
    }
    fn beam(&self) -> String {
        let profile = text(
            at(
                &self.ok("define-profile", &["--width", "100", "--height", "50"]),
                &["definition"],
            ),
            "digest",
        );
        let body = self.ok(
            "create-member",
            &[
                "--start",
                "0,0,0",
                "--end",
                "3000,0,0",
                "--profile-digest",
                &profile,
            ],
        );
        text(at(&body, &["member"]), "element_id")
    }
    fn assembly(&self, id: &str, name: &str) -> Json {
        self.ok("create-assembly", &["--name", name, "--new-id", id])
    }
    fn attach(&self, part: &str, assembly: &str) -> Json {
        self.ok(
            "assembly-attach",
            &["--id", part, "--assembly-id", assembly],
        )
    }
    fn assemblies(&self) -> Vec<Json> {
        list(&self.ok("assemblies", &[]), "assemblies").to_vec()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert!(self.root.is_absolute() && self.root.file_name().is_some());
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn text(json: &Json, key: &str) -> String {
    json.get(key).unwrap().string().unwrap().to_owned()
}
fn at<'a>(json: &'a Json, path: &[&str]) -> &'a Json {
    path.iter().fold(json, |j, k| j.get(k).unwrap())
}
fn list<'a>(json: &'a Json, key: &str) -> &'a [Json] {
    at(json, &[key]).array().unwrap()
}
fn strings(json: &Json, key: &str) -> Vec<String> {
    list(json, key)
        .iter()
        .map(|v| v.string().unwrap().to_owned())
        .collect()
}
fn assembly<'a>(items: &'a [Json], id: &str) -> &'a Json {
    items.iter().find(|a| text(a, "id") == id).unwrap()
}
fn groups(change: &Json, key: &str) -> Vec<(String, String)> {
    list(at(change, &["footprint"]), key)
        .iter()
        .map(|e| (text(e, "target"), text(e, "group")))
        .collect()
}

#[test]
fn create_attach_nest_and_detach_write_only_the_part() {
    let f = Fixture::new();
    let plate = f.plate();
    let beam = f.beam();
    let mesh_before = text(
        &f.ok("plate-inspect", &[])
            .get("plates")
            .unwrap()
            .array()
            .unwrap()[0],
        "mesh_digest",
    );

    let created = f.assembly(FRAME, "Frame A");
    assert_eq!(text(&created, "outcome"), "assembly_created");
    // An assembly is an element of its own: new identity, label, no body in the derived layer.
    assert_eq!(strings(&created, "changed_layers"), ["authored.usda"]);
    assert_eq!(text(&created, "element_id"), FRAME);
    assert_ne!(FRAME, plate);
    f.assembly(OUTER, "Outer");

    let attached = f.attach(&plate, FRAME);
    assert_eq!(text(&attached, "outcome"), "part_attached");
    assert_eq!(strings(&attached, "changed_layers"), ["authored.usda"]);
    // The record lives with its source: the part's relationship group is the only write; the
    // assembly is read (lifecycle and relationships) to decide kind and cycles.
    assert_eq!(
        groups(&attached, "writes"),
        [(plate.clone(), "relationship".to_owned())]
    );
    assert!(groups(&attached, "reads").contains(&(FRAME.to_owned(), "lifecycle".to_owned())));
    f.attach(&beam, FRAME);
    let nested = f.attach(FRAME, OUTER);
    assert_eq!(text(&nested, "outcome"), "part_attached");

    let all = f.assemblies();
    assert_eq!(all.len(), 2);
    let frame = assembly(&all, FRAME);
    assert_eq!(text(frame, "name"), "Frame A");
    assert_eq!(text(frame, "part_of"), OUTER);
    let mut parts = strings(frame, "parts");
    parts.sort();
    let mut expected = vec![plate.clone(), beam.clone()];
    expected.sort();
    assert_eq!(parts, expected);
    assert_eq!(strings(assembly(&all, OUTER), "parts"), [FRAME]);
    assert_eq!(at(assembly(&all, OUTER), &["part_of"]), &Json::Null);

    // Membership never touches the parts' bodies, and the verified head still inspects.
    let inspected = f.ok("plate-inspect", &[]);
    assert_eq!(
        text(
            &inspected.get("plates").unwrap().array().unwrap()[0],
            "mesh_digest"
        ),
        mesh_before
    );

    let detached = f.ok("assembly-detach", &["--id", &plate]);
    assert_eq!(text(&detached, "outcome"), "part_detached");
    assert_eq!(
        groups(&detached, "writes"),
        [(plate.clone(), "relationship".to_owned())]
    );
    let all = f.assemblies();
    assert_eq!(strings(assembly(&all, FRAME), "parts"), [beam]);
}

#[test]
fn kinds_cardinality_self_links_and_cycles_are_refused_with_nothing_written() {
    let f = Fixture::new();
    let plate = f.plate();
    let beam = f.beam();
    f.assembly(FRAME, "Frame");
    f.assembly(OUTER, "Outer");
    f.assembly(OTHER, "Other");
    f.attach(&plate, FRAME);
    f.attach(FRAME, OUTER);

    let attach = |part: &str, assembly: &str| {
        f.rejected(
            "assembly-attach",
            &["--id", part, "--assembly-id", assembly],
        )
    };
    // DV-REL-003: the target must be an assembly.
    assert_eq!(attach(&beam, &plate), "INVALID_ARGUMENT");
    // DV-REL-006: no self-link.
    assert_eq!(attach(OTHER, OTHER), "INVALID_ARGUMENT");
    // DV-REL-004: one assembly per part; no silent re-parenting.
    assert_eq!(attach(&plate, OTHER), "DEPENDENCY_CONFLICT");
    assert_eq!(attach(&plate, FRAME), "INVALID_ARGUMENT");
    // DV-REL-005: cycles of length two and three.
    assert_eq!(attach(OUTER, FRAME), "DEPENDENCY_CONFLICT");
    f.attach(OTHER, FRAME);
    assert_eq!(attach(OUTER, OTHER), "DEPENDENCY_CONFLICT");
    // Unknown identities.
    let missing = "019a7bf0-0000-7000-8000-0000002000ff";
    assert_eq!(attach(missing, FRAME), "NOT_FOUND");
    assert_eq!(attach(&beam, missing), "NOT_FOUND");
    assert_eq!(
        f.rejected("assembly-detach", &["--id", &beam]),
        "INVALID_ARGUMENT"
    );
    // The identity of a new assembly must be free, whatever the kind that holds it.
    assert_eq!(
        f.rejected("create-assembly", &["--name", "Again", "--new-id", &plate]),
        "DUPLICATE_ID"
    );
    assert_eq!(
        f.rejected("create-assembly", &["--name", "", "--new-id", missing]),
        "INVALID_ARGUMENT"
    );
}

#[test]
fn replay_reuse_delete_and_copy_follow_the_part_of_records() {
    let f = Fixture::new();
    let plate = f.plate();
    let create_op = "019a7bf0-0000-7000-8000-00000000a001";
    let attach_op = "019a7bf0-0000-7000-8000-00000000a002";
    let first = f.ok(
        "create-assembly",
        &["--name", "Frame", "--new-id", FRAME, "--op-id", create_op],
    );
    let again = f.ok(
        "create-assembly",
        &["--name", "Frame", "--new-id", FRAME, "--op-id", create_op],
    );
    assert_eq!(text(&again, "outcome"), "assembly_created_replayed");
    assert_eq!(
        text(at(&again, &["revision"]), "revision_id"),
        text(at(&first, &["revision"]), "revision_id")
    );
    assert_eq!(
        f.rejected(
            "create-assembly",
            &["--name", "Renamed", "--new-id", FRAME, "--op-id", create_op],
        ),
        "OP_ID_REUSE"
    );
    let args = [
        "--id",
        plate.as_str(),
        "--assembly-id",
        FRAME,
        "--op-id",
        attach_op,
    ];
    f.ok("assembly-attach", &args);
    let replay = f.ok("assembly-attach", &args);
    assert_eq!(text(&replay, "outcome"), "part_attached_replayed");
    assert_eq!(at(&replay, &["footprint"]), &Json::Null);

    // DV-REL-009: an assembly with parts is not deleted, and there is no cascade.
    assert_eq!(
        f.rejected("delete-element", &["--id", FRAME]),
        "DEPENDENCY_CONFLICT"
    );
    // DV-REL-011: a part with a record is not copied incompletely.
    assert_eq!(
        f.rejected("copy-element", &["--id", &plate]),
        "DEPENDENCY_CONFLICT"
    );
    // Deleting the part removes its record with it; then the empty assembly can go.
    f.ok("delete-element", &["--id", &plate]);
    assert!(strings(&f.assemblies()[0], "parts").is_empty());
    let deleted = f.ok("delete-element", &["--id", FRAME]);
    assert_eq!(text(&deleted, "kind"), "assembly");
    assert!(f.assemblies().is_empty());
}
