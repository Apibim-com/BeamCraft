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
| NEG-18, NEG-20 | Internal corrupt-table injections fail self-check; no imported-mesh evaluator is claimed |
| NEG-45 | Large-rectangle thickness below scaled minimum and adjacent lower float |

Additional tests cover zero, signed zero, negative and huge finite negative
dimensions; exact upper caps and adjacent upper floats; same-stage aggregation
and first-stage precedence; edge endpoint/role consistency; malformed internal
indices and collapsed triangles; and all diagnostic mappings to existing F04 codes.

The remaining F06 cases require member frames/roll, authored contour validation
(including rectangle start/direction POS-48/49), placement, generic blob
validation, density and quantity status, cache keys, queries, engine state or
transport. They cannot be fulfilled by a dimensions-only origin builder and
are not claimed as completed cases. The general F06 result/request adapter,
generic blob validator, runtime feature lookup, complete quantities and shape
keys remain reserved at the F08 paths. Numerical certification of arbitrary
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

Steps 4 to 6 are one F06 stage (ranges and contour validity): their failures are reported together in
input order (thickness; each contour's edges, area/winding and simplicity; contour pairs; containment).
A check is skipped only when its prerequisite failed (simplicity needs usable edges and a non-zero
area; a pair check needs both contours simple; containment needs no crossing between the pair), and the
nested-island support decision is made only when nothing invalid was found.

1. Counts, before any traversal: at least 3 points per contour, at most 1024 points per
   contour, at most 256 cavities and 2048 points in total (twice that is the F06 vertex cap).
2. Finiteness of every coordinate and the thickness.
3. Caps: coordinates at most 1e9 mm, thickness at most 1e6 mm, every contour edge (closing and cavity edges included) at most 1e6 mm, local size D (G01
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
Distances are written binary64 evaluations from local coordinate differences (so a lossless translation
near the 1e9 cap cannot move a clearance across the floor), used only for the floor comparisons and never change
a topology decision (F06 section 4). Public vertices and the blob both carry positive zero. Diagnostics use F06 `GEO_` codes with the F04 mapping, a
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
