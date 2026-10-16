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

## G05: profiles, extrusion and triangulation (`profile` module)

G05 is the base for BeamCraft's own round and oval holes: a planar profile (one outer
contour plus cavities) is admitted, its caps are triangulated and it is extruded straight
into a sealed closed mesh. It adds no dependency (no triangulation or geometry crate), no
asset and no I/O, and stays pure and WASM-compatible. The existing G02 plate API is untouched.

### API (engine API; CLI is owned by P01)

`profile::extrude(outer, cavities, thickness_mm)` and `extrude_with_cancel` return a sealed
`Extrusion` (mesh, volume, status, tolerance) or a `ProfileFailure`. `Profile::admit`
validates a profile alone, `Profile::triangulate` returns the cap triangles, and
`extrude_profile` extrudes an admitted profile. `Cavity::polygon`, `Cavity::circle`,
`Cavity::slot` and `Cavity::ellipse` build cavities. As in G02, the kernel call is the
headless API: the CLI command and the engine/CLI end-to-end gate are owned by P01, not G05.

Frame: millimetres, right-handed, the profile plane is XY and the solid runs from `z = 0` to
`z = thickness` (F04 P-1: the contour is the bottom face). The 3-D contour, placement,
rotation and ElementId qualification belong to the engine/P01 and A02. The outer contour must
be counter-clockwise and every cavity clockwise (material on the left of each directed edge);
F06 section 2 rejects wrong winding and never reorders points, and this module extends that
rule to cavities. Z offsets of the contour (the z origin of the G03 large fixtures) are not
modelled.

### Admission (F06 stage order, all failures of the first failing stage, at most 32)

1. Counts, before any traversal: at least 3 points per contour, at most 1024 points per
   contour, at most 256 cavities and 2048 points in total (twice that is the F06 vertex cap).
2. Finiteness of every coordinate and the thickness.
3. Caps: coordinates at most 1e9 mm, thickness at most 1e6 mm, local size D (G01
   `Tolerance::for_plate`, from the first outer point) at most 2e6 mm.
4. Ranges: thickness `> 0` and `>= eps_geom`; every edge (including repeated points and the
   closing edge) at least `eps_geom`; exact non-zero area; exact winding; area at least
   `eps_geom^2`.
5. Simplicity, per contour: exact segment relations for all non-adjacent edge pairs (touch,
   overlap and crossing are all rejected), no adjacent backtracking, and no two non-adjacent
   edges closer than `eps_geom`.
6. Relations: no touching or crossing between contours and no two contours closer than
   `eps_geom` (inclusive minimum: exactly `eps_geom` is accepted), every cavity inside the outer
   contour (else invalid), no cavity inside another cavity (a valid island, reported
   `GEO_UNSUPPORTED_CONTOUR`, after every invalid finding).

All signs (orientation, winding, intersection, containment) are exact decisions about the
rational values of the submitted binary64 numbers: a rounding-error filter decides clear cases,
a fixed-capacity integer evaluation decides the rest, and nothing is ever decided by an epsilon.
Distances are written binary64 evaluations used only for the floor comparisons and never change
a topology decision (F06 section 4). Diagnostics use F06 `GEO_` codes with the F04 mapping, a
fixed schema field, the F06 stage and a fixed defect class, never user text or values.

### Curve cavities (explicit approximation, no contract change)

F06 0.1.0 has no curved geometry. `Cavity::circle`, `slot` (obround) and `ellipse` therefore
produce clockwise **polygons with vertices on the curve**, flagged by `CurveApproximation`
(kind, edges per full turn, requested sagitta, proven bound). The achieved bound is the
linear-interpolation error of a C2 curve, `h^2/8 * max|curve''|` with `h = 2*pi/n`, i.e.
`R*(2*pi/n)^2/8` with `R` the circle radius, the slot radius or the larger ellipse semi-axis. It excludes
the few-ulp rounding of vertex coordinates. The edge count `n` is the smallest multiple of four
in `[min_segments, max_segments]` (rounded up/down to multiples of four, 8 to 1020) that meets the
requested sagitta; if none does the call fails (`GEO_ABOVE_LIMIT`, defect `SagittaUnreachable`)
and nothing is coarsened silently. Sine and cosine use only `+ - * /`, an exact integer octant
reduction and a fixed Taylor series, so the points are bit-identical on every target and exactly
symmetric. Orientation of slots and ellipses is given by an axis vector, never an angle.
A **contract amendment is required** before the engine may treat these as exact curved features:
F06 needs a curve/face kind, the `subdivisionScheme` meaning of an approximated wall, and quantity
status rules (this module reports `VolumeStatus::Approximated` with a bound, where F06 section 8
would say `unsupported` for a curved profile).

### Triangulation, extrusion and seal

Cap triangulation joins each cavity to the outer polygon by one exact-visibility bridge (cavities
in descending right-most-x order, candidates by squared distance then position), then clips strictly
convex ears whose triangle holds no other vertex, boundary included, except copies of its own
corners. The step is not trusted: before sealing, the extrusion self-check proves a tiling (every
cap triangle exactly positive and every directed edge paired with its opposite exactly once, so
by the winding-number argument there is no overlap, gap or T-junction), index validity, all
vertices used, one connected component, Euler characteristic `2 - 2*cavities` (one genus per
cavity), a positive signed volume and agreement of the mesh volume with the analytic volume.
Work is bounded by caps (contours, cavities, F06 section 6 mesh caps checked before any
allocation) and a fixed budget of 150,000,000 exact-sign evaluations; exhausting it is a typed
failure, never a hang. No recursion, `unsafe` or panicking path is used.

Output: the F06 section 6 BCMESH01 blob (layout 1, element-local, negative zero normalised),
its SHA-256 digest, the vertex/triangle/face tables, per-face outward unit normals (`None` for the
curved wall of an approximated cavity) and `subdivisionScheme` `none`, orientation `rightHanded`.
Vertices: bottom points in contour order (outer, then each cavity in input order), then top points.
Faces: bottom cap, top cap, one wall per outer edge, then cavity walls; triangles are grouped by
face. A polygonal cavity has one planar wall per edge; an approximated cavity has a single wall
face, so a future hole feature finds all its walls through `ExtrusionMesh::cavity_walls`.
The volume is `net_area * thickness` (shoelace relative to each contour's first point; cubic metres divide
by 1e9). `VolumeStatus::Exact` holds when every contour is authored; otherwise `Approximated`
with `volume_error_bound_mm3`. The mesh/analytic agreement is a build certificate with its own
relative bound (1e-9 of the magnitude of the cancelling terms), not CR-TOL-001: F06 section 4
rule 7 forbids the kernel from reading the test comparison rule.

### Local feature ids (provisional)

F06 section 5 defines `fid1` roles only for the plate and member boxes and forbids inventing box
roles for other solids. G05 therefore uses the local kind `extrusion` with the same character
set and length bound: faces `f.z0`, `f.z1`, `f.o<k>`, `f.c<j>.s<k>` or `f.c<j>`; edges
`e.z0.<t>`, `e.z1.<t>`, `e.v.<t>`; vertices `v.z0.<t>`, `v.z1.<t>` with contour token
`<t>` = `o<k>` or `c<j>.<k>`. Indices follow the authored contour and cavity order, never mesh
order; a cyclic start change moves roles (F06 section 5 rule 9). A contract amendment must adopt
this grammar before the engine qualifies and persists these ids.

### Verification and case coverage

`tests/g05_profile_extrusion.rs` and `tests/g05_negative_and_fuzz.rs` call the public API and
read the actual buffers with `tests/common` (independent blob reader, edge-incidence and
connectivity maps, divergence volume, shoelace area, closed-form regular-polygon, obround and
ellipse areas; no production math). Covered: rectangle (equal to the G02 plate), one and many
round holes, slot and ellipse holes at several axes, L-shape and comb outlines with holes, flat
(collinear) vertices with aligned holes, cavities and thickness at the exact minimum distance and
dimension (dyadic floor `eps_geom = 0.25`) and one float below, the 1024-face cap exactly and one above, the triangle cap (8184 accepted, 8220 rejected), every contour defect class, non-finite values in every position, cap/limit boundaries,
cancellation at all four points with no poll after sealing, mapping of all `GEO_` codes, exact
translation and quarter-turn invariance, cyclic start changes, reversed winding rejected, byte
identity of repeated builds, a seeded valid-profile generator (120 cases, always accepted, then moved
exactly), seeded hostile contours (1500) and perturbations (200) and maximum-size inputs. Unit tests
check the exact predicates against an integer reference and corrupt build candidates (inward,
missing, duplicated, out-of-range, collapsed, displaced) that never seal.

G03 fixtures: `g03.plate.through-hole`, `g03.plate.cavity`, `g03.plate.thin-wall` and
`g03.plate.nonrectangular` (placeholders for 23 x 47 x 3 plates, recorded `unsupported`) get real
G05 solids (round hole, rectangular void, wall at the minimum thickness, L outline); their G03
expectation rows are unchanged `not_run`. `g03.plate.large-positive`, `large-negative` and
`large-mixed` (status `unverified`) run on the x/y translation: with the origin at +1e9 a positive
extent leaves the 1e9 coordinate cap and is rejected `GEO_ABOVE_LIMIT`, and the inward placement is
accepted; `large-minimum-cube` is decided by the submitted binary64 edge, which is below the floor
when the 0.001 mm decimal is added to 1e9. The z origin is not modelled, so those four stay
`unverified` for z. Other G03 ids are G02 dimension cases and are not run here.

F06 cases projected: NEG-37 (winding), NEG-40 (self-intersection), NEG-41/42 (point count, repeated
point), NEG-46 (D cap), NEG-01 (non-finite), POS-53/54 (scaled floor, at and above, for thickness),
NEG-16/17 (cancellation), NEG-18/20 (corrupted candidates), POS-01 (closed shell, Euler, volume,
extended to genus `n`), POS-27/NEG-12/13 (builder-side caps; no generic blob validator), POS-48/49
(cyclic start and quarter turn, projected to polygons). Not run: NEG-38 (no 3-D planarity: the
profile is planar by construction), NEG-39 (the valid-pentagon-unsupported decision is the F06 plate
request path, which G05 does not change; wiring it is A02), NEG-19 (no certifier for arbitrary
meshes), blob-validator cases, quantities beyond volume, keys, queries, density and transport.

### Not implemented (design section 8 G05 scope, owner row wins)

The kernel design's G05 allocation lists member prisms and frames with roll; those are not part of
the owner's G05 row (profile with cavity and straight sweep) and are not implemented here. Also
not implemented: rotation/placement and the 3-D contour adapter, curve-exact geometry, nested
islands, non-planar contours, bevelled or non-straight sweeps, hole features (A03/A04/G09), area and
bounds quantities, shape keys, generic mesh certification and spatial indexes (G06), cross-target
byte evidence (no `wasm32` target or `cargo deny` is installed in this environment, so neither was
run), and CLI wiring (P01).

### Provenance

Original scalar code from public mathematics only: shoelace area; divergence-theorem volume; Euler
characteristic of a closed surface; winding-number and tiling argument; exact sign of the rational
orientation determinant (with a standard rounding-error filter derived here); ear clipping with hole
bridging as publicly described by D. Eberly ("Triangulation by Ear Clipping", method only, no code
read); Taylor series for sine and cosine; the linear-interpolation error bound; F06 0.1.0 sections 2-6,
10; F04 section 3 P-1. No third-party code, crate, fixture or asset, and no Tekla/Trimble material.

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
