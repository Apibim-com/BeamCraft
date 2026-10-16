# Architecture

BeamCraft is a Cargo workspace with an original Rust engineering and geometry core.
USD is the planned authoritative project state. By the owner decision of 2026-10-16, the
pure-Rust `openusd` crate (0.8.x) is the primary layer engine behind a thin BeamCraft wrapper,
with the aim of no Python/C++ in the product path. The official SDK worker remains only an
explicit fallback and an external test oracle where available, subject to F03 admission.
[F11](design/layer-engine-decision-v0.1.md) records the evidence and open integration gates;
no production USD backend is integrated. U05 implements immutable byte storage only.

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
- Use the thin BeamCraft wrapper around the pinned Rust layer engine for primary USD operations;
  engineering rules and geometry computations remain in the independent Rust core. In-process
  module versus separate Rust process remains open under F11/U01. F05 continues to govern the
  SDK fallback and any reviewed separate-process route; in-process semantics need explicit review.
  [Wrapper requirements](design/layer-engine-wrapper-requirements-v0.1.md) cover foreign writes,
  numeric/input validation, deterministic saves and deltas. F08 dependency/placement review and
  package/file-level licence admission precede integration; this record adds no dependency.
- Use the sibling PdfCraft UI stack only under the
  [V01.0 audit](audits/ui-stack-audit-v0.1.md). That audit admits eframe/egui 0.36.2, wgpu 30.0.1,
  the PdfCraft-patched winit 0.30.13, egui-wgpu 0.36.2 and vello_common 0.0.8, plus craft-ui and
  craft-quit from storytold/craft-libs at rev `83bf4cb25c01ed46e7645c4172974239868aaf06`, all with
  conditions. `default_fonts` and craft-print are rejected. No UI crate, vendored patch or PdfCraft
  asset is added by that audit. Keep frontend crates above the engine, in F08's planned app; F08's
  audited domain allowlist stays empty until an explicit reviewed table amendment.
- Add steel, export and drawing modules when their contracts require them. F08 fixed the concrete
  module paths after F04–F07 and creates no empty crates: `kernel`, `engine` and `ui` are
  planned paths in the boundary document, created by their own tasks with their first real code.

Prefer well-maintained permissively licensed Rust crates for math, geometry primitives and robust
predicates, preferably pure Rust; write own algorithms only where no suitable crate exists. Each
dependency needs the AGENTS.md section 5 audit, deny.toml checks and any F08 allowlist amendment
before integration. Copyleft or weak-copyleft third-party crates, including MPL-2.0, need an
explicit owner exception for each package. No code is copied from other projects; audited dependency
use is distinct from copying implementation text. The restricted commercial-material and clean-room
rules remain unchanged. The layer-engine choice is recorded in F11; this paragraph admits no
additional math dependency. Start with bounded primitives and original fixtures, expanding through
reviewed scopes.

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

A single backend owner behind the BeamCraft wrapper owns the live USD stage; its execution
topology remains open under F11. Independent calculations may run over immutable inputs;
stage mutation and publication are coordinated by the host. Command footprints include reads, writes
and relationships so rebase can detect semantic conflicts. Conflicts preserve local edits rather than
silently accepting a server value.

For a separate-process backend, worker IPC follows the reviewed F05 profile: versioned, bounded,
length-prefixed control frames with separate binary buffers over private pipes. An in-process
route still needs F11's reviewed semantic mapping of limits, cancellation and revision checks. The
[F05 worker protocol draft](contracts/worker-protocol-v0.1.md) specifies this private IPC contract
and its proposed acceptance cases. The [F06 geometry and viewport contract
draft](contracts/geometry-contract-v0.1.md) specifies the kernel input, mesh, key and query contract
in the same way. The [F10 domain vocabulary and module contract
draft](contracts/domain-vocabulary-v0.1.md) specifies the shared element kinds, relationships,
validation rules and discipline-module boundary in the same way; only the steel module is in scope
and nothing in it is implemented. Coordination and control servers require explicit opt-in, loopback
binding and token authentication. Resource paths do not authorize outbound access; local resolution
and containment are validated.

## Part representations and view filters (planned requirement)

Owner wish of 2026-10-16, to be planned with A03/A04 (cuts, holes), U02 (USD authoring) and V02
(view snapshot and query). It changes no frozen contract: F04/F06 0.1.0 have no operations.

- A part with operations (holes, cuts) is published with two representations in the same revision:
  a **full** one with the operations applied and a **simplified** one without them. A part with no
  operations has one mesh referenced by both. Identical content is stored once (SHA-256).
- The full representation is the default geometry of the revision, so an ordinary USD reader sees
  the complete part without BeamCraft. The simplified one is a lightweight stand-in (USD `proxy`
  purpose is the candidate mechanism). A payload for the heavy mesh is a candidate for fast opening
  of very large models. The USD encoding is decided by the U-tasks.
- Each representation has its own shape key and freshness; an operation change makes both stale. The
  representation is an enumeration, not a flag, so coarser levels can be added later.
- The simplified representation is flagged not exact. Volume, mass, bills of materials and snapping
  use only the full one, and the two are never summed (see "References, quantities and
  collaboration" and F06 section 3).
- View settings choose what to load, not what exists. A per-view filter can hide whole element
  kinds that are separate objects (welds, bolts, references) and can switch parts between the
  simplified and the full representation. The filter belongs to user or workspace settings, not to
  the shared revision, and changes no authored or derived data.
- V02 must carry both parameters: an element-kind filter and a representation selector. F06 gains a
  `representation` field in the version that introduces operations.
- The gain is measured, not assumed: V01 reports load time, memory and frame time on the 10 000-part
  scene with each setting.

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

F04 and F06 were frozen at 0.1.0 by the owner on 2026-10-16. See the [tolerance
comparison](design/tolerance-comparison-v0.1.md). Nothing is implemented by this record, and
existing admission requirements remain in force.

### Owner decisions of 2026-10-16

1. F04 and F06 are frozen at 0.1.0; P-1a to P-1d, P-2, P-3a and P-3b are confirmed. P-3c (roll jump
   at the parallel threshold) is accepted as erratum E-2 until a black-box observation of the
   reference behaviour for a column whose axis moves is recorded.
2. A future `section_anchor` (nine positions plus offsets) is reserved in F04 section 3.
3. Part representations and per-view filters are a planned requirement (section above).
4. The core is extensible by discipline modules (F10); new element kinds, commands and rules are
   registered by a module, not hard-coded. Importing foreign IFC (E04) remains an open owner decision.
