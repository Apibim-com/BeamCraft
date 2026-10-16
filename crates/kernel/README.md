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
