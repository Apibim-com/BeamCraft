# F08: module boundaries, version 0.1.0

Status: **draft 0.1.0, not frozen; K0 not passed**. Date: 2026-10-15.
Base: `29d962e790ffd21ecd9a45143f052d4b4d53d3f7` (stacked on the F06 draft).

This document fixes where the code of each contract area lives, which way dependencies point,
which crates exist now and which are created later by their own tasks. It consumes the
[F04 model and command contract](model-and-commands-v0.1.md) (freeze candidate), the
[F05 worker protocol](worker-protocol-v0.1.md) and the
[F06 geometry and viewport contract](geometry-contract-v0.1.md) (both drafts) and the F07
fixtures. It changes none of their semantics. It implements no feature: the planned paths below
are reservations, not code, and a path marked `planned` is created only by the task named for it.

The module table in section 2 is **machine-read**. The `beamcraft-boundaries` tool and its test
(section 6) check the real workspace manifests against it, and CI derives its list of portable
crates from it. Change the table and the manifests together.

## 1. Rules

1. **One direction.** `geom` <- `model` <- everything else. A crate may depend only on crates
   with a strictly lower layer number, and only on those listed in its row. A new edge needs a
   reviewed change to the table.
2. **Kinds.**
   - `domain`: pure and portable. Builds for `wasm32-unknown-unknown`. No UI, operating-system,
     filesystem, process, network or async-runtime dependency, and no use of `std::fs`,
     `std::process`, `std::net`, `std::thread`, `std::env` or `std::os` outside test code. May
     depend only on other `domain` crates and on the third-party crates of section 3 (none today).
   - `host`: native engine host. Owns process supervision, pipes, deadlines, filesystem access,
     storage, the outcome ledger and the worker client. Depends on domain crates, never the
     reverse. Third-party dependencies need the audit that `AGENTS.md` section 5 requires.
   - `app`: a binary entry point (CLI, later UI). Calls the host and the engine API. Nothing
     depends on an app.
   - `tool`: development tool, `publish = false`, never shipped. Nothing depends on a tool. The F07
    fixture tool lives in `tools/fixtures`; the boundary checker lives in `apps/boundaries` because
    F08 may write only to `crates/` and `apps/`. Moving it is a table edit plus a path move.
3. **Headless first.** Every feature is an engine API first, then a CLI command (`AGENTS.md`
   section 3). The CLI computes nothing that the engine does not expose.
4. **Contracts do not move.** Where a contract places a rule in one layer (for example F06: the
   renderer never computes engineering geometry), this document follows it.
5. **No empty crates.** A crate is created only together with real, tested code taken from a
   contract. Until then its path is documented here and nowhere else.

## 2. Module table

Columns: package; path; kind; layer; allowed internal dependencies (`-` for none); status
(`now` = a workspace member today, `planned` = created by its own task); the task that creates a
planned crate or owns a present one. Dependencies are checked in `[dependencies]`,
`[dev-dependencies]`, `[build-dependencies]` and target-specific tables alike.

<!-- module-table:begin -->
| Package | Path | Kind | Layer | Allowed dependencies | Status | Owner task |
|---------|------|------|-------|----------------------|--------|------------|
| beamcraft-geom | crates/geom | domain | 0 | - | now | G01 |
| beamcraft-model | crates/model | domain | 1 | beamcraft-geom | now | P01 |
| beamcraft-protocol | crates/protocol | domain | 2 | beamcraft-model | now | U01 |
| beamcraft-kernel | crates/kernel | domain | 2 | beamcraft-geom, beamcraft-model | planned | G01 |
| beamcraft-engine | crates/engine | domain | 3 | beamcraft-geom, beamcraft-model, beamcraft-kernel | planned | P01 |
| beamcraft-host | crates/host | host | 4 | beamcraft-model, beamcraft-protocol, beamcraft-engine | planned | U01 |
| beamcraft-cli | apps/beamcraft-cli | app | 5 | beamcraft-model, beamcraft-engine, beamcraft-host | now | U04 |
| beamcraft-ui | apps/beamcraft-ui | app | 5 | beamcraft-model, beamcraft-engine, beamcraft-host | planned | V01 |
| beamcraft-fixtures | tools/fixtures | tool | 9 | - | now | F07 |
| beamcraft-boundaries | apps/boundaries | tool | 9 | - | now | F08 |
<!-- module-table:end -->

No package has a third-party dependency today: `Cargo.lock` holds project-owned packages only.

## 3. Third-party crates allowed in domain crates

A domain crate may use a third-party crate only if it is listed here with the reason, after the
`AGENTS.md` section 5 review (permissive licence, pure Rust, `wasm32-unknown-unknown`, no
network, no OS access). The list is empty: no domain crate has a third-party dependency, and
a test fails if one appears.

<!-- third-party:begin -->
| Crate | Reason |
|-------|--------|
<!-- third-party:end -->

## 4. Where each contract area lives

"Now" means present and tested in this change; "planned" means a path reserved here and created
by the named task with real code. Allowed dependencies are those of the owning row in section 2.

| Contract area | Contract | Exact path | Status |
|---------------|----------|------------|--------|
| Rigid transform, vectors, quaternion, tolerance constants, canonical f64 encoding for keys | F06 s.2, 4, 7 | `beamcraft_geom` (modules `vec`, `quat`, `tolerance`) | planned, G01. Today `geom` holds `Point3` only |
| Kernel input and result, diagnostics, `GEO_` codes and their F04 mapping | F06 s.2, 3 | `beamcraft_kernel::{input, result, diagnostic}` | planned, G01 |
| Analytic box builders, self-checks, cancellation points | F06 s.4, 6, 10 | `beamcraft_kernel::build` | planned, G01 |
| Mesh blob builder, validator, face table, digest rule | F06 s.6 | `beamcraft_kernel::mesh` | planned, G01 |
| FeatureId (`fid1`) | F06 s.5 | `beamcraft_kernel::feature_id` (uses `ElementId` from `model::id`) | planned, G01 |
| Three dependency keys, mass signature, invalidation | F06 s.7 | `beamcraft_kernel::keys` | planned, G01 |
| Quantities and statuses | F06 s.8, F04 s.5 | `beamcraft_kernel::quantity` | planned, G01 |
| Pick and feature geometry for one body | F06 s.9 | `beamcraft_kernel::pick` | planned, G01 |
| Pinned queries (`ListElements`, `GetMeshSnapshot`, `LookupFeature`, `GetChanges`, `QueryBounds`), snapshot tokens | F06 s.9 | `beamcraft_engine::query` | planned, P01 |
| Presentation key preimage | F06 s.7 | `beamcraft_engine::presentation` (V01 may move it to the UI if it needs no engine state; never into the kernel) | planned, V01 |
| Identity (UUIDv7 `ElementId`), units, input limits | F04 s.2, 3 | `beamcraft_model::{id, limits}` | planned, P01 |
| SHA-256 digest type and hasher, for F04 resources, F06 keys and F05 buffers | F04 s.2, F06 s.6, 7, F05 s.3 | `beamcraft_model::digest` | planned, first task that needs it |
| F04 command and envelope types | F04 s.6, 7 | `beamcraft_model::command` | planned, P01 |
| F04 stable error codes | F04 s.8 | `beamcraft_model::error::ErrorCode` | **now** (23 codes, tested) |
| Bounded JSON decoder and RFC 8785 canonical bytes of the validated envelope | F04 s.3, F05 s.6 | `beamcraft_model::{json, canonical}` | planned, P01 |
| Command application, footprints, transactions, replay decisions | F04 s.8, F05 s.7 | `beamcraft_engine::{command, footprint, transaction}` | planned, P01 |
| Frame header, binary prefix, bundle budget | F05 s.3, 10 | `beamcraft_protocol::frame` | **now** (tested, bounded) |
| Negotiable limits and their maxima | F05 s.4, 10 | `beamcraft_protocol::limits` | **now** |
| Transport `IPC_` codes and their F04 mapping | F05 s.9 | `beamcraft_protocol::code::TransportCode` | **now** |
| Control-message schemas, handshake, correlation and event state machines (pure, no I/O) | F05 s.4, 5, 7 | `beamcraft_protocol::{message, session}` | planned, U01 |
| Worker supervision, pipes, deadlines, restart limits, diagnostic drain | F05 s.2, 8 | `beamcraft_host::worker` | planned, U01 |
| Filesystem, storage, atomic publish, personal workspace | F04 s.2, U05-U07 | `beamcraft_host::storage` | planned, U05-U07 |
| Durable outcome ledger and recovery | F05 s.7, 8 | `beamcraft_host::ledger` | planned, U07 |
| USD worker (thin Python/pxr) | F05 s.1 | `workers/usd/`, not a Cargo package | planned, U01 |
| CLI | AGENTS s.3, F06 s.9 | `apps/beamcraft-cli` | **now** (`--version` only) |
| Native UI and viewport | F06 s.9, V-tasks | `apps/beamcraft-ui` | planned, V01 |

The F05 sections 3 and 10 also require SHA-256 verification of attachments. The protocol crate
checks the declared lengths, counts, indices and prefix fields and exposes the claimed digest; the
streaming digest comparison needs a hasher and waits for `model::digest`.

## 5. Crates created now, and why

- **`beamcraft-protocol` (`crates/protocol`, domain).** Created because it holds real, tested code
  taken directly from F05 sections 3, 4, 9 and 10: the 16-byte header and 44-byte binary prefix
  with bounded encode and decode, the 14 negotiable limits with their maxima and the
  per-field minimum, a bundle size budget and the 16 transport codes with their F04 mapping. It
  depends on `model` only for `ErrorCode`, and has no I/O. Every decoder validates before it
  trusts a length, uses checked arithmetic, never slices by an input-derived position and has a
  regression test for each rejected input class.
- **`beamcraft_model::error::ErrorCode`** (a module in the existing crate, not a new crate): the
  23 stable codes of F04 section 8, shared by F05 and, later, F06.
- **`beamcraft-boundaries` (`apps/boundaries`, tool).** Created because it holds the real, tested
  checker of section 6. It is a development tool with no dependencies; it is not part of the
  product.

**Not created, on purpose:** `kernel`, `engine`, `host` and `ui`. Each would be empty today; the
tasks named in section 2 create them with their first real code. No placeholder crate, `todo!` or
unreachable stub is added.

## 6. Mechanical enforcement

`cargo test --workspace` runs the tests of `apps/boundaries`; CI also runs
`cargo run -p beamcraft-boundaries -- check`. The checks read the workspace and member
`Cargo.toml` files and this document, using only the standard library and no network:

1. every workspace member has a `now` row with the same path, and every `now` row is a member;
2. package names and paths in the table are unique, every dependency named in a row exists in the
   table and has a lower layer;
3. every internal dependency of a member (in any dependency table) is listed in its row. A
   dependency is checked by its real package name: `package = "..."` is resolved whether written
   inline, dotted (`x.package = "p"`, `x.version = "1"`) or in a sub-table, and an inherited
   `workspace = true` entry is resolved against the root `[workspace.dependencies]`;
4. a `domain` crate depends on no `host`, `app` or `tool` crate, and on no third-party crate that
   is not in section 3;
5. no package depends on an `app` or a `tool`, and `tool` packages set `publish = false`;
6. source of a `domain` crate does not mention `std::fs`, `std::process`, `std::net`,
   `std::thread`, `std::env` or `std::os` (also inside a `use std::{...}` group, including
   `use std::{fs as f}`). Comments and string literals are ignored, and only items annotated
   `#[cfg(test)]` are exempt, wherever they are declared. An inner `#![cfg(test)]` exempts the
   whole file only at file level before any item; inside `mod tests { ... }` it exempts only that
   module. Code placed after a test module is still scanned;
7. every CI job that mentions `wasm32-unknown-unknown` calls
   `cargo run -p beamcraft-boundaries -- portable` (all `now` `domain` rows) and selects no
   package by hand: continuation lines are joined, and any `-p`, `-pNAME` or `--package`
   argument other than the tool's own `cargo run -p beamcraft-boundaries` is rejected, whether
   in a variable assignment, a continuation or a separate run line;
8. the loader fails closed: a symbolic link or Windows reparse point at a member, along its path,
   at its manifest or inside its `src`, a manifest that is not a regular file, and a source
   directory nested deeper than eight levels are errors, never silently skipped.

Known limits: the scan is lexical, so a `#[cfg_attr(...)]`-generated or macro-generated path is
not seen, and an alias such as `use std as s;` is not followed. The manifest reader is line
based: a multi-line inline table is not read, and an inherited key that the root does not define
keeps its key as the name (Cargo itself rejects such a manifest). The wasm check and code review
cover what a lexical scan cannot.

The failing paths are proved by tests on an in-memory model (a synthetic manifest set), so no
broken manifest is committed. The wasm check is therefore as strict as before: today it lists
`geom` and `model`; it now also covers `protocol`, and every future `domain` crate joins it
automatically, with no second list to forget.

## 7. Amendments proposed to other contracts

None is required. Two clarifications are proposed for the owners of the named documents, as no
shared contract is edited here:

- F05 section 11, row F08: the "canonical-byte comparison" obligation is placed at
  `beamcraft_model::canonical` (pure) with the host as its only caller; it is not implemented yet.
- F05 section 12, open item "U02/F08 must publish the schema-specific change and snapshot item
  fields": F08 leaves this to U02. The message schemas will live in
  `beamcraft_protocol::message`, and the item fields in `beamcraft_engine`, not in the host.

## 8. Provenance

Contributor-original BeamCraft text, MPL-2.0, derived from F04, F05, F06 and the repository
[architecture](../architecture.md) and [development guide](../development.md). No third-party
material is used. The code it describes is original and implements only byte layouts, numbers and
code tables stated in F04 section 8 and F05 sections 3, 4, 9 and 10.
