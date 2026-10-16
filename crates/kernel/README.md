# beamcraft-kernel

Contributor-original BeamCraft code and documentation, MPL-2.0.

G02 implements an allocation-free rectangular plate primitive in millimetres,
right-handed, Z up. It depends only on the existing `beamcraft-geom` math types
and `beamcraft-model` error codes and SHA-256. No new third-party dependency,
external asset, catalogue, network capability or unsafe code is introduced.

## API

`rectangular_plate(RectangularPlate { width_mm, length_mm, thickness_mm })`
returns a sealed `PlateResult` or typed `diagnostic::Failure`. The dimensions
describe an origin plate spanning `[0,width]`, `[0,length]`, `[0,thickness]`.
The raw dimension input is a primitive convenience API, not F04 authored input.
The caller must perform F04 contour admission before using this API for authored
geometry. No supplied contour is accepted, inferred, repaired or rotated here.

The result exposes eight G01 points, twelve canonical triangles, six outward
unit face normals, the ordered local face table, the exact 392-byte BCMESH01
blob and its full SHA-256. Normals travel separately: F06's blob has no normals
section. Subdivision is explicitly `none`, orientation `rightHanded`. Local
face, edge and vertex ids and edge endpoint mappings are in `feature_id`;
vertex roles follow `i=xb+2*yb+4*zb`. Engine qualification with ElementId is
outside this crate's G02 slice. Indices are snapshot-local, not stable addresses.

Volume uses `(width*length)*thickness`; cubic metres divide that result by 1e9.
It is returned only after mesh self-checks. This is analytic primitive volume,
not a complete F04 quantity record or a cache freshness claim.

Validation runs finiteness for all dimensions first, then one-sided submitted
upper caps, local D and geometric floor, then positivity and inclusive minima.
It returns all failures of the first failing stage in width/length/thickness
order (at most three, below F06's 32-record bound). Failure contains no geometry.
Nonpositive prospective width/length contribute zero to scale; no huge negative
input is squared. Dimensions above 1,000,000 mm are rejected before scale work.
For admitted dimensions D cannot exceed sqrt(2)*1,000,000, below the plate cap.
The geometric minimum is G01's `max(0.001,2^-22*D)`, never a test tolerance.

`rectangular_plate_with_cancel` accepts a host callback polled only at P0 entry,
P1 before build, P2 before checks and P3 before quantities. Cancellation discards
the candidate; no callback occurs after sealing. The host owns callback safety.

Construction follows the written F06 corner sum order without FMA. Positive,
orthogonal origin extents certify a non-self-intersecting convex box by
construction. Before sealing, bounded checks verify index validity, distinct
triangle indices, finite positive outward area, opposite paired edge incidence,
all vertices used, Euler characteristic and positive finite signed volume.
There are exactly 36 directed edge occurrences and at most 36*36 pairing
comparisons, with no recursion or allocation. All output zeros are positive.

## Verification and case coverage

The integration tests call the real public builder and read its actual buffers.
Their scalar cross/dot/volume calculations do not call production vector math.
Independent maps check edge incidence and connectivity; the frozen F06-POS-02
digest checks every buffer byte against an independently published vector.
An admitted dimension grid checks outward winding, connectedness, closedness,
Euler, analytic versus mesh volume and area, canonical zero and repeat bytes.
Volume/area comparisons use CR-TOL-001 relative 1e-6, without feeding that rule
into admission. Counts, statuses, ids, digest and minimum decisions are exact.

| F06 case | G02 obligation exercised |
|----------|--------------------------|
| POS-01 | Exact origin plate, counts, outward normals, closure, Euler and volume |
| POS-02 | Independent buffer reader, offsets, little endian and frozen SHA-256 vector |
| POS-03 | Analytic volume/unit-conversion projection; density, area records and bounds queries excluded |
| POS-14 | Plate inclusive absolute minimum and adjacent smaller float |
| POS-16, POS-17, POS-18 | Local role stability, uniqueness, grammar and qualification length; engine identity operations excluded |
| POS-37 | No cancellation poll after seal |
| POS-53, POS-54 | Dimension/scale projections of the large rectangle, above/at floor; authored origin and placement excluded |
| NEG-01 | Every dimension rejects NaN and both infinities before caps/ranges |
| NEG-03 | Every dimension below the absolute floor |
| NEG-16, NEG-17 | Cancellation at all four observing points, no partial result |
| NEG-18, NEG-20 | Internal corrupt-table injections fail self-check; imported meshes are covered by the G06 section below |
| NEG-45 | Large-rectangle thickness below scaled minimum and adjacent lower float |

Additional tests cover zero, signed zero, negative and huge finite negative
dimensions; exact upper caps and adjacent upper floats; same-stage aggregation
and first-stage precedence; edge endpoint/role consistency; malformed internal
indices and collapsed triangles; and all diagnostic mappings to existing F04 codes.

The remaining F06 cases require member frames/roll, authored contour validation
(including rectangle start/direction POS-48/49), placement, density and engine
quantity status, cache keys, queries, engine state or
transport. They cannot be fulfilled by a dimensions-only origin builder and
are not claimed as completed cases. The general F06 result/request adapter,
runtime feature lookup, complete quantities and shape
keys remain reserved at the F08 paths; the generic blob validator and mesh
checks are the G06 section below. Numerical certification of arbitrary
rounded/translated contours remains with G03/G04 and the F06 E-1 erratum.

## Integration and provenance

The kernel call is the headless API. The CLI command and engine/CLI end-to-end
gate are owned by P01, not G02. No worker protocol, USD, UI, hole, cut, contour
evaluation, member or quaternion is implemented. Member sin/cos reproducibility
remains OPEN and is not needed here. Native tests alone do not demonstrate
cross-target byte conformance. K0 and the complete geometry feature remain open.

Implementation and synthetic test inputs come from frozen F06 0.1.0 sections
2-6, 7 (encoding conventions), 10 and its case file. Mathematical provenance is
F06's public IEEE 754-2019 arithmetic reference, the divergence theorem for
signed triangle volume, and Euler's characteristic for a closed genus-zero shell.
These are original scalar implementations of public mathematical rules; no
external implementation, fixture, commercial software material or asset is copied.
Existing model SHA-256 is reused, without duplicating its implementation.

Registration changes are restricted to Cargo.lock (the root workspace glob
already discovers the crate), the package-specific MPL-2.0 exception in deny.toml
and the F08 table/creation note. The boundary checker and CI derive the portable
crate list from that table, so neither their code nor CI needs an edit. G01,
frozen F04/F06, ROADMAP.md, root Cargo.toml and other crates stay untouched.

## G06: topology, orientation and measurements

G06 adds a verification and measurement foundation that works on **any** BCMESH01
blob or vertex/index buffer of finite input, not only on G02 boxes. It is original
scalar code from public mathematics, adds no dependency, and keeps every G02 file,
test and result unchanged. Plan row (owner): own checks of shells, orientation,
volume and a bounded spatial index; an open or unsupported mesh never gets an exact
status; independent invariants and transforms; headless API. (G04 design section 8
lists a different, unconfirmed G06 allocation, "issue pilot-cut requirements"; the
owner's row wins and pilot cuts belong to G09. Its G05-style items "analytic
measurements, mass/status handling and independent mesh checks" are covered in part
here: status/mass rules, an independent mesh cross-check of sealed plates and exact
mesh measurements; contours, extrusion and triangulation remain G05.)

| Module | Content |
|--------|---------|
| `trimesh` | `TriMesh`: F06 section 6/10 structural gate for `BCMESH01` blobs (`from_blob`, `to_blob`) and buffers (`from_buffers`), caps (4096 vertices, 8192 triangles, 1024 face-table entries, 16 shells, 213,024 bytes, coordinates at most 4e9 mm), rigid `transformed`, local scale `tolerance` |
| `topology` | `analyze` / `analyze_with` / `analyze_with_tolerance`: sealed `Analysis` with `ShellStatus`, `Defects`, per-shell `ShellReport` (Euler characteristic, genus, orientation, signed-volume enclosure); `classify_point` for certified meshes; `Limits` (explicit caps) |
| `measure` | `measure`: volume, area, centroid enclosures and exact vertex bounds, gated by the analysis; F06 section 8 `QuantityStatus`, `mass_status`, `analytic_mass_kg`; `cross_check_plate` (independent re-read of a sealed G02 plate) |
| `spatial` | `Aabb` and `BoxIndex`: static median-split box tree (leaf size 4, at most 8192 items and 16,384 nodes, depth under 16, no recursion), overlap queries and overlapping-pair enumeration with explicit pair/result caps |
| `verdict`, `enclosure` | `MeshCode` (stable `GEO_` tokens with F04 mapping), `Fault`, `MeshError`, `ShellStatus`, `Uncertainty`; outward-rounded `Enclosure` |

`bigint`, `exact` and `intersect` are crate-private: fixed-capacity (72 x u64, no heap,
no recursion) integers, exact `orient2d`/`orient3d` and closed-set segment/triangle
contact.

**Method.** (1) Structural gate, then undirected-edge incidence by sorting directed
edges (`O(T log T)`, no welding, no healing): boundary, three-or-more-triangle,
same-direction and duplicate-triangle detection, vertex fans by corner union-find,
shells by shared vertices, Euler characteristic `V - E + F` and genus `(2 - chi)/2`
per closed shell. (2) Degeneracy under the F06 section 4 policy: a repeated index, an
edge strictly shorter than `eps_geom`, exactly collinear corners or an unused vertex.
(3) Exact signed volume per shell: every binary64 coordinate is an integer multiple
of a power of two, so `sum(v0 . (v1 x v2))` is computed exactly in integers and
enclosed once; its sign is the shell orientation (G04 section 2: exact sums, no
epsilon sign). (4) Self-intersection by two closed triangles meeting exactly when an
edge of one meets the other, decided from exact orientation signs (a floating-point
filter whose error bound is derived in `exact`, with exact integer fallback), over
candidate pairs from the box index; pairs sharing an edge or one vertex are classified
by that shared feature, so touching, crossing, folding and coincident faces are all
certified defects, and a one-binary-step gap is certified disjoint. (5) A shell inside
another is detected by exact crossing parity of axis rays from up to eight vertices
(six directions each); a grazing probe is skipped, never guessed.

**Verdict and precedence.** `Certified` needs every proof. Otherwise, in order:
`Degenerate`, `NonManifold`, `Open`, `InconsistentOrientation`, `Inward` (negative
shell volume, including a cavity shell), `Degenerate` (exactly zero volume),
`Overlapping`, `Uncertain`. `Uncertain` carries its reason (pair cap, index cap,
exact-predicate budget, numeric range, containment) and is never exact. Codes:
open/non-manifold/misoriented -> `GEO_MESH_NOT_CLOSED`, inward -> `GEO_MESH_INWARD`,
degenerate/overlapping/uncertain -> `GEO_MESH_NOT_CERTIFIED` (numeric range ->
`GEO_NUMERIC_UNCERTAIN`), all `UNSUPPORTED_GEOMETRY`; structural failures are
`GEO_NON_FINITE`, `GEO_ABOVE_LIMIT`, `GEO_BUFFER_LIMIT`, `GEO_TOPOLOGY_LIMIT`,
`GEO_BLOB_INVALID`, `GEO_CANCELLED`. These live in `verdict`; `diagnostic::Code` (G02)
is intentionally not edited and can absorb them later.

**Measurements and statuses.** Only a `Certified` analysis yields volume, area and
centroid, as outward-rounded enclosures of the polyhedron the given binary64
vertices define (area and centroid use exact cross/moment integers, one rounding
enclosure per triangle). They are mesh-computed values, never engineering
quantities: `MeshMeasurement::quantity_status()` is always `unsupported` (F06
section 8: a mesh from another source gets no engineering quantity), its mass is
`unsupported` even with an absent density (F06-NEG-18), and bounds are exact vertex
extents, not a volume. `mass_status` implements the section 8 precedence (unsupported,
then unknown for absent density, then stale, then current); `analytic_mass_kg` keeps
`fl(fl(V*density)/1e9)` and refuses an underflowed zero. Transforms: `TriMesh::transformed`
applies the production `RigidTransform` per vertex (approximate frame certificate, so
exact rigidity is not claimed); exact motions (dyadic translation, quarter turns,
axis permutations) preserve volume bit for bit in tests, power-of-two and integer
scale factors scale V and S as `k^3` and `k^2`, and a mirrored mesh is `Inward`.

**Tolerance.** `TriMesh::tolerance` derives the F06 `eps_geom = max(0.001, 2^-22 L)`
with `L` the largest distance from vertex 0 (G01 norm, capped at the 2,000,000 mm
plate cap) because a bare mesh has no body inputs. A caller that knows the body
passes its own `Tolerance` to `analyze_with_tolerance`; `cross_check_plate` does.
Known consequence, covered by a test: for a plate at exactly its inclusive minimum
thickness the plate rule (thickness excluded from `D`) is sound while the generic rule
is slightly larger and reports `Degenerate`.

**Work bounds and cancellation.** No recursion; allocations are sized by validated
counts only. `Limits::DEFAULT`: 2^20 candidate pairs, 16,384 index nodes, 2^18 exact
fallbacks; exceeding any gives `Uncertain`. Polls (stage that would run next):
`Validation` at entry, `Build` before incidence, `Checks` before degeneracy and
orientation, then for a clean closed outward mesh a `Checks` poll before the index,
one per index build and about every 1024 units of pair work, one per nested-shell
probe pair, and a final `Quantities` poll before sealing. `measure_with_cancel`
polls `Validation`, then `Quantities` before the pass and every 512 triangles. A
cancelled call returns only `MeshError` (no partial result); nothing polls after
sealing.

**Tests** (all original; oracles are voxel counting, closed forms, `std` maps and
integer/f64 scalar code, never production math): `tests/g06_shells.rs` (28: box,
prism, tunnel genus 1 and 2 by Euler, disjoint shells, 16/17 shell cap, shuffled and
relabelled copies, repeat identity, 120 seeded voxel sets against an independent
verdict, open, duplicate, three-face edge, edge/vertex contact, flipped triangle,
inverted and cavity shells, collapsed/collinear/short/unused, overlap, touching,
nested, budgets, large slab, point classification, blob and buffer validation at
and one above every cap, cancellation at every poll), `tests/g06_measure.rs` (16:
closed-form volume/area/centroid, refusal without values, mass rules, exact and
inexact rigid motions, scale, far-from-origin cancellation, measurement
cancellation, G02 cross-check), `tests/g06_fuzz.rs` (5 seeded hostile-input suites:
mutated blobs, random soups, surgery on valid shells, perturbed vertices, maximum
size soups; no panic, deterministic output, and every `Certified` verdict checked by
the independent oracle), `tests/g06_fixtures.rs` and 20 unit tests of the
integer, interval, predicate, contact and index code.

| Case or fixture | G06 obligation exercised |
|-----------------|--------------------------|
| F06-POS-27 | Blob exactly at every cap is structurally valid and round-trips |
| F06-NEG-11 to 14 | Counts above caps and 32-bit-overflowing counts rejected before any size |
| F06-NEG-15, 30 | Length disagreement and empty blob are `GEO_BLOB_INVALID` |
| F06-NEG-18 | Open mesh: three boundary edges, Euler 1, no value, mass unsupported |
| F06-NEG-19 | Overlapping closed boxes: Euler 4, `Overlapping`, the sum 2000 never reported |
| F06-NEG-20 | Fully inverted box: signed volume -5e4 enclosed, `Inward` |
| G03 `g03.plate.*` with expectation `ok` (22) | Sealed G02 blob re-read, certified under the plate tolerance, counts/Euler/bounds exact, exact rational volume inside the mesh enclosure |
| G03 invalid and limit rows (26) | Not run: the real builder rejects them, so no mesh exists |
| G03 `large-positive`, `large-negative`, `large-mixed`, `large-minimum-cube` (4, `unverified`, no expectations) | Not run: origin placement is outside the dimension-only builder and no reference exists |
| G03 `through-hole`, `cavity`, `thin-wall`, `nonrectangular` | Not run: need G05 contours and G09 cuts; not faked. A hand-made hollow body is refused as `Inward` by `tests/g06_shells.rs`, which is not a fixture claim |

**Not implemented, stated plainly.** Ray first-hit and ray-based picking (point
classification of a certified mesh exists; F06 pick queries stay analytic and belong
to later tasks); healing, welding or repair; cavities and holes as valid solids
(negative shells are refused); exact area/centroid (enclosures only); engineering
quantities for non-analytic meshes (by contract); member and contour meshes (G05);
cross-target byte conformance, which native tests do not show; the `cargo deny` and
`wasm32-unknown-unknown` checks, which were not available in the authoring
environment. The crate uses only `std` plus `geom`/`model`, so no new licence or
WASM-incompatible dependency is introduced, but the WASM claim awaits CI.

**Headless and provenance.** The kernel calls are the engine API; the CLI command and
the engine/CLI end-to-end gate stay owned by P01, as for G02. References are public
mathematics: the divergence theorem (signed tetrahedron volume), Euler-Poincare for
closed orientable surfaces, standard orientation-determinant predicates and the
two-closed-triangles-meet-iff-an-edge-meets-the-other fact, IEEE 754 rounding
(correctly rounded operations bound each widening by one representable step), and
F06 sections 4, 6, 8 and 10 with G04 sections 2-4. No third-party geometry crate,
source, test or data, and no commercial-product material, was used.
