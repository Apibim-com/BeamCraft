# Architecture

BeamCraft is a Cargo workspace with an original Rust engineering and geometry core.
USD is the planned authoritative project state. The official OpenUSD SDK will handle scene
composition in an isolated local worker. These boundaries are accepted design decisions;
the worker, storage, commands and UI are not implemented yet.

## Current implementation

```
apps/
  beamcraft-cli     command-line entry point (binary: beamcraft)
  boundaries        module-boundary checker (development tool, not shipped)
crates/
  geom              points, vectors, coordinate systems   (no deps)
  model             parts, profiles, assemblies, F04 error codes  (-> geom)
  protocol          F05 frame header, limits, IPC codes   (-> model)
tools/
  fixtures          F07 fixture generator and verifier
```

`geom` currently provides points and distance. `model` provides a beam axis and its length and the
23 stable F04 error codes. `protocol` provides the F05 frame header and binary prefix, the
negotiable limits, a bundle budget and the transport codes; it does no I/O. The CLI reports its
version. These types are a scaffold, not a USD engine or profile catalogue.

[Module boundaries (F08)](contracts/module-boundaries-v0.1.md) give the exact crate and module path
of every contract area, the allowed dependency edges and which crates are planned. A test and CI
enforce them.

## Planned boundaries

- Extend the existing `geom` and `model` crates. Do not create duplicate geometry or domain cores.
- Keep domain mathematics, BIM identities, commands and invariants independent of UI, filesystem,
  scheduling runtimes and the C++ ABI. Pure domain crates must build for `wasm32-unknown-unknown`.
- The pure `protocol` crate holds the F05 wire rules (framing, limits, transport codes). Process
  supervision, pipes and deadlines stay in the native host.
- Place filesystem access, process supervision, storage and coordination in a native engine host.
  The host depends on domain crates, not the reverse.
- Use a thin Python/pxr worker for the first OpenUSD prototype, subject to the F03 package audit.
  Python serves USD operations; engineering rules and geometry computations remain in Rust.
- Build an original native UI. The initial viewport candidate is wgpu with egui/eframe;
  its dependencies, fonts and assets need an audit. Kit is excluded.
- Add steel, export and drawing modules when their contracts require them. F08 fixed the concrete
  module paths after F04–F07 and creates no empty crates: `kernel`, `engine`, `host` and `ui` are
  planned paths in the boundary document, created by their own tasks with their first real code.

Our geometry kernel will be implemented independently under the clean-room rules. IFC-lite, Manifold,
truck and OCCT are not production backends or source donors. Start with finite, bounded inputs,
analytic primitives and original fixtures; expand topology and operations through verified scopes.

## Project state and revisions

One `project.usda` is the project entry point. USD stores authored parameters, placements,
operations and relationships. Derived USD layers contain computed geometry and quantities.
Memory snapshots, indexes, caches and journals do not replace the authoritative model format.
JSON control messages describe operations, not a second persisted native model.

A published root references complete immutable generations of authored and derived layers.
Publishing replaces the root atomically after verifying assets and the expected base revision.
Recovery must expose either the previous or the new complete revision. U05–U07 define and test
the storage mechanism; a sequence of uncoordinated file replacements is insufficient.

Every placed element has a full UUIDv7. Prim names retain all 32 hexadecimal digits of the UUID;
renaming a user label does not change identity. Immutable resources use full SHA-256 digests
and verified bytes. Relationships are validated for duplicate identities and dangling targets.

Copies receive independent identities and share immutable geometry until a shape edit.
Personal workspaces preserve unpublished authored edits, derived results and journals across
restart. They are valuable draft state and are excluded from the published root, not deleted
with disposable caches.

## References, quantities and collaboration

Foreign USD references must remain read-only. Edits and overrides of foreign prims are forbidden,
including overrides authored in our layers. Our attachment wrapper
may have a placement; measurements and annotations are separate owned results. Reference updates
are explicit and preserve the original source bytes. IFC import and native conversion are outside
the initial scope; IFC is a later export adapter for an explicitly supported subset.

Shape, mass, appearance and world measurements have separate dependency signatures.
Renaming or moving the camera need not rebuild shape; changing density invalidates mass.
Measurements carry freshness and support diagnostics. Unknown density, stale results and
unsupported geometry must not be presented as current exact values.

A single worker owns the live USD stage. Independent calculations may run over immutable inputs;
stage mutation and publication are coordinated. Command footprints include reads, writes and
relationships so rebase can detect semantic conflicts. Conflicts preserve local edits rather than
silently accepting a server value.

Worker IPC is planned as versioned, bounded, length-prefixed control frames with separate binary
buffers over private pipes. F05 must specify limits, errors, cancellation and revision checks.
The [F05 worker protocol draft](contracts/worker-protocol-v0.1.md) specifies this private IPC
contract and its proposed acceptance cases.
The [F06 geometry and viewport contract draft](contracts/geometry-contract-v0.1.md) specifies the
kernel input, mesh, key and query contract in the same way.
The [F10 domain vocabulary and module contract draft](contracts/domain-vocabulary-v0.1.md)
specifies the shared element kinds, relationships, validation rules and discipline-module boundary
in the same way; only the steel module is in scope and nothing in it is implemented.
Coordination and control servers require explicit opt-in, loopback binding and token authentication.
Resource paths do not authorize outbound access; local resolution and containment are validated.

## Rules

- Dependencies point one way, from higher-level crates down to `model` and `geom`. No back-edges.
  The allowed edges are the module table of the boundary document; `cargo test --workspace` checks
  the manifests against it. Domain crates (`kind = domain`) have no UI, OS, filesystem, process or
  async-runtime dependency and build for `wasm32-unknown-unknown`; CI derives that crate list from
  the same table.
- Units: millimetres, radians internally, right-handed, Z up. Convert only at the IO/UI edge.
- Every feature is reachable headlessly: engine API → CLI command → end-to-end test.
- Input is untrusted; see `AGENTS.md` §4.
- Project-owned code uses MPL-2.0; each new workspace package needs its own `deny.toml` exception.
  Third-party dependency and asset policies remain unchanged. F03 audits the actual OpenUSD
  distribution, bundled libraries and licence obligations before integration.

## Decision provenance

This original design summary follows the owner's local `architecture-proposal.md` dated
2026-10-14 (§1, §3–7, §9–17) and `implementation-plan.md` revision 1.1
(§1–4, §5–7, §14). Detailed Russian plans and observation materials remain outside the repository.
Those local plans describe intended acceptance, not completed features. See [the roadmap](../ROADMAP.md)
for gates and [development](development.md) for the verified F01 baseline and remaining checks.

The [F04 model and command contract](contracts/model-and-commands-v0.1.md) is a review draft
of these logical boundaries. Its examples are proposed acceptance descriptions, not implemented
APIs or saved models. F09 admission precedes freezing it for kernel implementers.
