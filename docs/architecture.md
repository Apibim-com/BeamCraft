# Architecture

BeamCraft is a Cargo workspace with an original Rust engineering and geometry core.
USD is the planned authoritative project state. The official OpenUSD SDK will handle scene
composition in an isolated local worker. These boundaries are accepted design decisions;
the worker, revision publication, commands and UI are not implemented yet. U05 implements
immutable byte storage only.

## Current implementation

```
apps/
  beamcraft-cli     command-line entry point (binary: beamcraft)
  boundaries        module-boundary checker (development tool, not shipped)
crates/
  geom              points, vectors, coordinate systems   (no deps)
  model             beam axis, F04 error codes, SHA-256 digest  (-> geom)
  protocol          F05 frame header, limits, IPC codes   (-> model)
  host              native immutable blob storage        (-> model)
tools/
  fixtures          F07 fixture generator and verifier
```

`geom` currently provides points and distance. `model` provides a beam axis and its length and the
23 stable F04 error codes plus `digest::ContentDigest` and incremental `digest::Sha256`.
`host::storage::BlobStore` provides publish-if-absent and verified retrieval of immutable bytes.
`protocol` provides the F05 frame header and binary prefix, the
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
- Use the sibling PdfCraft UI stack: Rust workspace, edition 2024, Rust 1.95; an engine independent
  of UI; egui/eframe 0.36 with egui_extras, rfd and egui_kittest for UI tests; craft-ui, craft-quit
  and craft-print from storytold/craft-libs pinned by git revision. PdfCraft (storytold/pdfcraft,
  MIT OR Apache-2.0) vendors patched winit, egui-wgpu and vello_common. These are
  orchestrator-supplied public Cargo.toml facts, not independently verified here. The patches and
  craft-libs licence remain unverified. Adopting craft-libs requires a licence audit and a deny.toml
  exception for its unknown git source; neither is done. Record an actual revision before
  integration. No dependency or vendored patch is added by this documentation. Keep frontend crates
  above the engine, in F08's planned app; F08's audited domain allowlist stays empty until an
  explicit reviewed table amendment.
- Add steel, export and drawing modules when their contracts require them. F08 fixed the concrete
  module paths after F04–F07 and creates no empty crates: `kernel`, `engine` and `ui` are
  planned paths in the boundary document, created by their own tasks with their first real code.

Prefer well-maintained permissively licensed Rust crates for math, geometry primitives and robust
predicates, preferably pure Rust; write own algorithms only where no suitable crate exists. Each
dependency needs the AGENTS.md section 5 audit, deny.toml checks and any F08 allowlist amendment
before integration. Copyleft or weak-copyleft third-party crates, including MPL-2.0, need an
explicit owner exception for each package. No code is copied from other projects; audited dependency
use is distinct from copying implementation text. The restricted commercial-material and clean-room
rules remain unchanged. No library or backend is selected here. Start with bounded primitives and
original fixtures, expanding through reviewed scopes.

## Project state and revisions

One `project.usda` is the project entry point. USD stores authored parameters, placements,
operations and relationships. Derived USD layers contain computed geometry and quantities. Memory
snapshots, indexes, caches and journals do not replace the authoritative model format. JSON control
messages describe operations, not a second persisted native model.

A published root references complete immutable generations of authored and derived layers.
Publishing replaces the root atomically after verifying assets and the expected base revision.
Recovery must expose either the previous or the new complete revision. U05–U07 define and test the
storage mechanism; a sequence of uncoordinated file replacements is insufficient.

Every placed element has a full UUIDv7. Prim names retain all 32 hexadecimal digits of the UUID;
renaming a user label does not change identity. Immutable resources use full SHA-256 digests and
verified bytes. Relationships are validated for duplicate identities and dangling targets.

U05 stores exact bytes at `<authorized-root>/blobs/<first-two-hex>/<all-64-lowercase-hex>`.
The root must be an existing absolute directory. Resource operations accept a `ContentDigest`,
never an asset path or URI. Publication flushes and syncs a temporary file within the store,
then installs it with a non-replacing hard link; unsupported filesystems fail without a rename
fallback. Concurrent identical publishers return one `Created` and otherwise `AlreadyPresent`.
Existing bytes are compared in full; differences return `IntegrityConflict` without overwriting.
Retrieval, presence and verification check the complete SHA-256. A digest is not a mathematical
proof that collisions are impossible. The 32 MiB F05 sections 3/4 bundle budget is the upper
bound for each blob and in-memory read; a store may choose a lower cap.

Every existing path component is checked for symbolic links/reparse points, and non-regular
blob entries and canonical containment violations are rejected. The host must protect the root
namespace from hostile concurrent local writers: portable safe std checks cannot prevent a
directory replacement between validation and opening. Windows additionally opens blob reparse
points without following them. These checks are not an OS sandbox. U05 promises neither
directory-entry crash durability nor crash-atomic multi-file publication; crashes may leave
temporary files. U06/U07 own revision publication and recovery. Garbage collection is deferred
until retained revisions and drafts are accounted for. The asset CLI belongs to U04.

Copies receive independent identities and share immutable geometry until a shape edit.
Personal workspaces preserve unpublished authored edits, derived results and journals across
restart. They are valuable draft state and are excluded from the published root, not deleted
with disposable caches.

## References, quantities and collaboration

Foreign USD references must remain read-only. Edits and overrides of foreign prims are forbidden,
including overrides authored in our layers. Our attachment wrapper may have a placement;
measurements and annotations are separate owned results. Reference updates are explicit and preserve
the original source bytes. IFC import and native conversion are outside the initial scope; IFC is a
later export adapter for an explicitly supported subset.

Shape, mass, appearance and world measurements have separate dependency signatures. Renaming or
moving the camera need not rebuild shape; changing density invalidates mass. Measurements carry
freshness and support diagnostics. Unknown density, stale results and unsupported geometry must not
be presented as current exact values.

A single worker owns the live USD stage. Independent calculations may run over immutable inputs;
stage mutation and publication are coordinated. Command footprints include reads, writes and
relationships so rebase can detect semantic conflicts. Conflicts preserve local edits rather than
silently accepting a server value.

Worker IPC is planned as versioned, bounded, length-prefixed control frames with separate binary
buffers over private pipes. F05 must specify limits, errors, cancellation and revision checks. The
[F05 worker protocol draft](contracts/worker-protocol-v0.1.md) specifies this private IPC contract
and its proposed acceptance cases. The [F06 geometry and viewport contract
draft](contracts/geometry-contract-v0.1.md) specifies the kernel input, mesh, key and query contract
in the same way. The [F10 domain vocabulary and module contract
draft](contracts/domain-vocabulary-v0.1.md) specifies the shared element kinds, relationships,
validation rules and discipline-module boundary in the same way; only the steel module is in scope
and nothing in it is implemented. Coordination and control servers require explicit opt-in, loopback
binding and token authentication. Resource paths do not authorize outbound access; local resolution
and containment are validated.

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

This original design summary follows the owner's local `architecture-proposal.md` dated 2026-10-14
(§1, §3–7, §9–17) and `implementation-plan.md` revision 1.1 (§1–4, §5–7, §14). Detailed Russian
plans and observation materials remain outside the repository. Those local plans describe intended
acceptance, not completed features. See [the roadmap](../ROADMAP.md) for gates and
[development](development.md) for the verified F01 baseline and remaining checks.

The [F04 model and command contract](contracts/model-and-commands-v0.1.md) is a review draft of
these logical boundaries. Its examples are proposed acceptance descriptions, not implemented APIs or
saved models. F09 admission precedes freezing it for kernel implementers.

### Owner decisions of 2026-10-15

1. Math/dependencies: prefer suitable permissive Rust crates, retaining per-package audit and
   restricted-material rules — recorded; no dependency added.
2. UI: adopt the PdfCraft stack above — recorded; craft-libs licence, git revision and vendor patch
   audit remain open.
3. Plate: contour, first-edge X, normal/thickness Z and Y=Z x X — recorded; winding, limits and
   planarity assumptions noted in F04/F06; nonrectangle evaluation waits for A02.
4. Member frame: P-2 confirmed unchanged, with Y derived through a cross product — recorded.
5. Roll: optional explicit rotation about member local +Z after the default frame — recorded;
   missing prior angle cap addressed by an explicit assumption in F04.
6. Constants: owner-confirmed first-slice draft values, relative 1e-6 and length floor 1e-6 m=0.001
   mm — assumption noted; fixture validation open;
   ifc-lite comparison recorded in the tolerance report.

F04 remains a freeze candidate at 0.1.0; F06 remains a draft. See the [tolerance
comparison](design/tolerance-comparison-v0.1.md). Nothing is implemented by this record, and
existing admission requirements remain in force.
