# F04: model and command contract, version 0.1.0

Status: **frozen 0.1.0** by the owner on 2026-10-16 (see [section 11](#11-freeze-status)), with the
owner-approved additive amendment A-2 of 2026-10-19 (operation records, hole and plane-cut
commands, section 12), whose element validity state (`degenerate`) and the command arguments
beyond the owner's decisions are marked "proposal for owner review". Nothing in it
is implemented. Draft base: `b34205281ab8a1f95052c4dc7d84f0995633e179`.
Freeze-candidate amendments (task F04.2) are made on `478f330e15819b574c62227bc23009729adb4fdc`; the
F06 amendments (task F04.3) are made on `b996c58fe850642f12b92b25b23c441fbd41ed47`; see [section
12](#12-amendments-log). F01 is merged. F02 is available locally: 35 source files, 160 occurrences
and 156 unique IDs; source hashes were independently checked against its pinned manifest. F02 is not
an input to this contract's amendments. The first-scope requirements 0.1.0
(rectangular plate, straight member, identities, units, rigid placement, declared failures) were
integrated in PR #10. K0 is not passed. The [F06 geometry
contract](f06-geometry-contract-v0.1.md) is frozen 0.1.0 and nothing in it is implemented; task
F04.3 applied the amendments it proposed to this contract (section 12).

This specifies domain semantics before F05 transport, F06 geometry and F08 module implementation. No
command described here is implemented by this PR. The current CLI still reports its version. JSON
examples are control messages and acceptance descriptions, never a second saved model format.

## 1. Authority and ownership

USD contains the authoritative authored parameters, placement, operations and relationships.
Published revisions also include corresponding derived geometry and quantities. Rust owns
engineering validation, commands, rights, dependency tracking and transactions; a USD adapter
handles composition and serialization. UI and CLI call the same engine API.

An owned element, immutable definition and foreign reference are different targets. Mutation
commands address owned elements by ID. The engine derives ownership from validated composition
provenance, not an `owner` string submitted by a caller. Unknown ownership is not write permission.
Edits and overrides of foreign prims are forbidden even in our own layers. Own reference wrappers
and measurement annotations have separate identities; they never make their foreign source writable.
R01 defines attachment commands after this contract.

## 2. Identity, revision and paths

| Type | Representation and invariant |
|------|------------------------------|
| ElementId, ProjectId, WorkspaceId, OpId | Full UUIDv7, canonical lowercase `8-4-4-4-12` form; validate version 7 and RFC variant |
| ActorId | Opaque identity from authenticated execution context; request claim must match that context |
| RevisionId | Full UUIDv7 identifying one immutable accepted project revision; never a shortened timestamp or array index |
| ContentDigest | SHA-256 of exact published bytes, all 64 lowercase hexadecimal digits |
| WorkspaceGeneration | Unsigned 64-bit counter; logical JSON form is a canonical decimal string, without leading zeros except `"0"` |
| FeatureId | Opaque stable identifier within a versioned geometry result; representation and loss rules belong to F06 |

UUID layout follows [RFC 9562 section 5.7](https://www.rfc-editor.org/rfc/rfc9562.html#section-5.7).
Creation checks actual uniqueness. UUID clock order gives neither authority nor conflict priority.
Same-millisecond elements must retain different complete IDs.

An owned steel element maps to `/World/Steel/E_<all 32 UUID hex digits without hyphens>`. `bim:id`
is the full hyphenated ID; `bim:name` is a label. Rename does not change ID or prim path. Prim paths
are adapter/index results, not caller-selected mutation destinations. The initial namespace does not
support arbitrary element reparenting.

A revision ID and a content digest have different roles. Each immutable revision has one validated
layer/asset manifest; reusing its ID for different content is an integrity error. U06 fixes the root
publication/CAS mechanism, and U05 verifies complete blob bytes. Commands never accept shortened
digests, absolute asset paths or remote resource URIs instead of a ContentDigest. The host resolves
authorized project-relative resources with containment checks.

## 3. Units, types and initial limits

Lengths and coordinates use finite f64 millimetres, rotations use radians where expressed as angles,
and the coordinate system is right-handed with Z up. Volume is m3, density kg/m3 and mass kg. The
USD adapter authors `metersPerUnit=0.001` and `upAxis=Z` explicitly. [USD stage linear
units](https://openusd.org/release/api/group___usd_geom_linear_units__group.html) are an IO concern;
standard USD angular operations use their documented units at that boundary.

Logical messages follow [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259.html): reject duplicate
object keys, invalid UTF-8 and non-JSON NaN/Infinity literals. Numeric fields accept numbers only,
not booleans or numeric strings. Reject f64 overflow/underflow that changes a nonzero submitted
number to infinity/zero. Do not silently round or clamp engineering inputs. Ordinary nearest-f64
conversion is the numeric representation; no extra dimension rounding is allowed. Positive and
negative zero are equivalent for signatures; normalize only their sign for that purpose.

These are original initial operating caps, not vendor tolerances or promises of kernel accuracy:

| Input | Initial constraint |
|-------|--------------------|
| Encoded logical command | At most 1 MiB before parsing; nesting depth at most 16 |
| Single string | At most 4096 UTF-8 bytes unless a narrower field cap applies |
| Element label | 1–256 UTF-8 bytes; reject control characters U+0000–001F and U+007F |
| Plate thickness/edge / supported member-axis length | Greater than zero and at most 1,000,000 mm |
| Minimum nonzero dimension (draft, to be validated by G04 and fixtures) | Plate thickness/edge lengths, section dimensions and member-axis length are at least `eps_geom = max(0.001 mm, 2^-22 * L)`, inclusive; a value above zero and below eps_geom is `INVALID_ARGUMENT` ([F06 section 4](f06-geometry-contract-v0.1.md#4-tolerance-and-degeneracy-policy-g-06-draft)) |
| Section dimensions (draft, to be validated by G04 and fixtures) | At most 1,000,000 mm; above that is `LIMIT_EXCEEDED` (F06 section 2) |
| Local size L / contour radius D | Plate L = D <= 2,000,000 mm; member L = max(axis length, section half-diagonal) <= 1,000,000 mm; see definition below |
| Plate contour count / roll angle (assumptions) | 3–1024 points; `abs(roll_rad) <= 6.283185307179586` rad |
| Coordinates / translation components | Absolute value at most 1,000,000,000 mm |
| Material density, when authored in a definition | Greater than zero and at most 1,000,000 kg/m3; absence is unknown |
| Element relationship targets | At most 1024; require valid pinned targets and reject forbidden cycles |
| Derived command footprint entries | At most 4096; fail before publishing if the footprint cannot be bounded |

The owner confirmed these first-slice constants on 2026-10-15 and froze them with this contract on
2026-10-16; fixture validation (G03/G04) may only produce a versioned erratum. The minimum and the section cap were added by the F06 amendments (section 12). They are
the policy of F06 sections 2 and 4 (frozen 0.1.0); the existing limits and codes above are
unchanged except for the owner-directed scaled minimum. Fixture validation and the remaining A5
accuracy work stay open as errata E-1 (F06 section 12).

**Geometric length floor (owner decision, 2026-10-15).** `eps_geom = max(0.001 mm, 2^-22 * L)`.
Here `L` is an element-local body size in mm, evaluated before placement: for a plate `L = D`,
the maximum distance from contour point p0 to any contour point; for a member `L = max(ell, H)`,
where `ell` is the axis length and `H` is the section half-diagonal. World coordinates and
translation magnitude never enter this scale. Thickness does not enter plate D; roll and rigid
placement do not enter member L.

Use binary64 round-to-nearest-even at every written operation, without reassociation or fused
multiply-add. Define `norm(v) = sqrt((v_x*v_x + v_y*v_y) + v_z*v_z)`. Subtract submitted
element-local point components first, then evaluate each norm in that order. For a plate scan
`norm(p_i - p0)` in ascending point index, starting with D = 0 and retaining the larger value
with `>`; set L = D. For a member compute `ell = norm(p2 - p1)`, `hw = width_mm/2`,
`hh_section = height_mm/2`, `H = sqrt(hw*hw + hh_section*hh_section)`, then L = max(ell, H). A
section dimension that is not greater than zero contributes 0 to H (`hw = 0` or
`hh_section = 0`): it is rejected in the range stage as `INVALID_ARGUMENT`
(`GEO_NOT_POSITIVE`), never as an above-limit or numeric-uncertainty result of this derivation.
Finally multiply L by the exactly representable binary64 constant `2^-22 = 1/4194304`, then
take max with binary64(0.001 mm). Thus eps_geom is never smaller than binary64(0.001 mm).
All length operands and results are in mm; the scale factor is dimensionless.

Check submitted finiteness and F04 caps before using the floor. Plate L must be at most
2,000,000 mm (the existing D cap); member L must be at most 1,000,000 mm (the existing axis
and section caps imply this). An excessive finite L is `LIMIT_EXCEEDED` (`GEO_ABOVE_LIMIT`
in F06), before minima or rectangle-only contour support decisions. Non-finite submitted input is
`NON_FINITE_NUMBER` (`GEO_NON_FINITE`). A non-finite derived L or norm intermediate from
otherwise finite capped input is numeric uncertainty: `UNSUPPORTED_GEOMETRY`
(`GEO_NUMERIC_UNCERTAIN`), with no floor, partial geometry or current key. Never clamp L,
substitute the absolute floor for a failed scale, or expand any F04 cap. Under the stated
caps, with nonpositive dimensions contributing 0, these square sums cannot overflow; the
diagnostic also covers arithmetic faults.

F05 must enforce input byte/depth/string bounds before unsafe allocations and cannot widen these
limits silently. F06 fixes mesh/topology bounds and tolerance policy separately. Host limits for
stage traversal and immutable assets are additional, not replaced by this table.

Placement is a rigid pose: `translation_mm` (three numbers) and `orientation_xyzw` (four quaternion
numbers). Components of orientation are finite in [-1, 1]; squared norm must be within 1e-6 of one.
F06 must validate and construct a proper rigid transform, normalize that accepted quaternion and
define deterministic antipodal/sign handling for geometry signatures. The F09 review integrated the
rule for the first scope (CR-PLACE-001). It remains a proposed rule until the owner freezes this
contract (section 11); F06 still owns rigid-transform construction, deterministic sign handling and
tolerance policy (G-06, accepted by F06 under its draft policy). Scale, shear, reflection and
commercial position presets are unsupported in the initial mutation API. Generic foreign transforms
remain read-only and use a separate measurement-support decision.

**Quaternion convention (G-04, adopted; matches CR-PLACE-001 of the first-scope package).**

- Component order is `(x, y, z, w)`: `q = x*i + y*j + z*k + w`, with Hamilton's rule `i*j = k`. `[0,
  0, 0, 1]` is the identity.
- The rotation is **active** and acts on element-local coordinates. For the normalized quaternion
  `q`, a local point `v` becomes `R(q) * v`, where `R(q)` is the proper rotation matrix of `v' = q v
  q^-1`. For a unit axis `u` and angle `a` the quaternion is `(u*sin(a/2), cos(a/2))`, a
  counter-clockwise turn seen from the tip of `u` in a right-handed frame.
- The pose maps a local point to the world as `world = R(q) * local + translation_mm`: rotate first,
  then translate. Rotation is about the local origin. Composition with any parent transform is
  outside this contract.
- The accepted quaternion is normalized (`q / |q|`) before use. The authored components are stored
  as submitted; this contract does not rewrite them.
- `q` and `-q` denote the same rotation and are both accepted. A canonical sign for signatures is
  not defined here; it stays with F06 (and P04).
- The component range and the 1e-6 squared-norm rule above are unchanged. This contract does not fix
  the f64 evaluation order of the squared norm, so no case sits exactly on that limit; the cases
  keep a clear margin on both sides.

Worked example (exact; checked with rational arithmetic): `q = (0, 0, 1/sqrt(2), 1/sqrt(2))` is a
+90 degree turn about +Z and maps the local point `(3000, 0, 0)` to `(0, 3000, 0)`. With
`translation_mm = [100, 200, 300]` the world point is `(100, 3200, 300)`. See `F04-POS-10` and
first-scope case `FS-POSE-A`. With the f64 literal `0.7071067811865476` for both components, the
squared norm differs from one by about 2.2e-16, inside the limit.

**Degenerate member axis (G-03, adopted by the owner 2026-10-15).** The two `axis_points_mm` points
must give a length greater than zero. Identical points (distance exactly zero) are rejected with
`INVALID_ARGUMENT`; a length above 1,000,000 mm is `LIMIT_EXCEEDED`; a non-finite coordinate is
`NON_FINITE_NUMBER` or `MALFORMED_COMMAND` as for any number. Reasons: a zero axis is a value that
violates the "greater than zero" range of an authored field, exactly like zero plate thickness
(`F04-NEG-04`, `INVALID_ARGUMENT`) and a negative thickness (`F04-NEG-14`). `UNSUPPORTED_GEOMETRY`
means a well-formed shape or transform outside the supported scope (for example a curved member or a
scaled transform); a zero-length straight axis is not a well-formed supported shape that merely
lacks a feature. This keeps every "dimension must be positive" rejection under one code. This is the
only error-code choice changed by the freeze-candidate amendments; the owner adopted it on
2026-10-15. A nonzero but very short axis is governed by the draft minimum nonzero dimension of
`eps_geom` in the limits table above: a length above zero and below that minimum is also
`INVALID_ARGUMENT` (G-06, F06 section 4).

**Element-local frames (G-01 and G-02).** These fix where the authored dimensions sit in the
element-local frame, before the placement of the previous subsection is applied. They define local
bounds for the first scope. The profile-definition resource format, minimum dimensions and any
geometric tolerance are F06's policy (G-06 and F06/P02), frozen with it.

*Owner decisions, 2026-10-15 (verbatim intent):*

- **G-01, plate.** The plate's local X runs along its first edge, from its first corner point to its
  second. Thickness runs along local Z.
- **G-02, member.** The profile section lies in the member's local XY plane. Local Z runs along the
  member length (the axis). The section height is in that XY plane.

*Owner decisions recorded on 2026-10-15.* P-1 is replaced, P-2 is confirmed unchanged, and P-3
retains the deterministic zero-roll frame and adds explicit axial roll. Details not stated by the
owner are assumptions below. On 2026-10-16 the owner confirmed P-1a to P-1d, P-2 (with the reserved
section anchor below), P-3a and P-3b, and froze the contract; P-3c is recorded as erratum E-2.

- **P-1, contour plate (replaces rectangular-only input).** `contour_points_mm` is an ordered array
  of 3 to 1024 distinct three-component points in element-local millimetres. Closure is implicit
  from the last point to the first; do not repeat the first point at the end. Each coordinate has
  absolute value at most 1,000,000,000 mm. Thickness and each edge length are in
  [eps_geom, 1,000,000] mm, inclusive; the maximum distance `D` from the first point to another
  point is at most 2,000,000 mm. These contour bounds and the winding convention below are recorded
  assumptions, not verbatim owner words. The body-frame origin is `p0`; `X_p = unit(p1 - p0)`.
  Compute the polygon area vector `N = sum((p_i - p0) x (p_(i+1) - p0))` in ascending index order,
  including the closing edge. Exact signs and zero tests use the rational values of the submitted
  binary64 coordinates; only normalization rounds to binary64. A zero area vector is degenerate and
  invalid. To make clockwise rejection unambiguous in 3D, choose the largest absolute component of N
  (ties: Z, then Y, then X) and require it to be positive. Otherwise reject clockwise winding with
  `INVALID_ARGUMENT`; never reorder the author's points. `Z_p = unit(N)` is the normal from which
  the contour is counter-clockwise when seen from +Z_p; `Y_p = Z_p x X_p`, so `X_p x Y_p = Z_p` for
  an exactly planar contour. This normal-sign convention is an explicit assumption. The contour
  plane passes through p0 with normal Z_p. Every point must be within `eps_plane = eps_geom +
  1e-6*D` of that plane. Contours outside the planarity band or degenerate contours are invalid. No
  input point is silently moved. A contour within this admission band but not exactly planar is
  valid model input and unsupported for solid evaluation until A02 defines it; it has no certified
  orthonormal body frame or evaluated solid in the first slice. A solid is the planar contour
  extruded `thickness_mm` along +Z_p; the contour is its bottom face, body z = 0, and the top face
  is body z = thickness. Element-local z need not be zero. First-slice solid evaluation supports
  exactly four points with e0=p1-p0, e1=p2-p1, p3=p0+e1 and e0 dot e1=0, all in the contour plane.
  Thus its edges are parallel to the derived plate X/Y axes, width=|p1-p0| and length=|p2-p1|. The
  old origin rectangle produces exactly the old box. Amendment A-2 (owner, 2026-10-19; A02.1):
  any other simple polygon (including a pentagon) lying in the plane `z = 0` is evaluated as the
  prism extruded along +Z as authored (F06 section 2). A valid polygon outside that plane, or
  within the planarity band but not exactly planar, is an accepted authored input with geometry
  status `unsupported`, mapped to existing `UNSUPPORTED_GEOMETRY`; it is not an invalid model.
  Within engineering validation, check types/count and caps before allocation, finiteness,
  coordinate/dimension/L caps, eps_geom, thickness and edge minima, repeated points, zero N,
  planarity, winding, then simplicity, in that order. For simplicity, drop the dominant coordinate
  of N (same tie rule),
  and test non-adjacent closed segments in ascending index-pair order; any
  intersection/touch/overlap, nonconsecutive duplicate or adjacent backtracking is invalid. Adjacent
  edges may share only their endpoint. Exact rational orientation signs distinguish touch from
  separation without loosening topology. Any ambiguous numeric certificate fails evaluation as
  unsupported, never as a guessed valid solid. Fewer than three points, repeated consecutive points
  (including repeated closure), subminimum edges, degeneracy, non-planarity, clockwise winding or
  self-intersection map to `INVALID_ARGUMENT`; non-finite numbers map to `NON_FINITE_NUMBER` (bare
  NaN remains malformed JSON), and caps map to `LIMIT_EXCEEDED`. Structural field/type errors follow
  F04 section 8. No new F04 error code is needed.
- **P-2, member frame.** For axis points `p1 -> p2` in element-local coordinates, the member's local
  `Z_m = unit(p2 - p1)`. The section is a `w x h` rectangle centred on the axis, with the width `w`
  along `X_m` and the height `h` along `Y_m`. Amendment 2026-10-18 (A07.1): any supported profile
  section is placed the same way, with the centre of its bounding box on the axis, its width along
  `X_m` and its height along `Y_m`. The end faces are perpendicular to the axis through
  `p1` and `p2`; there are no end extensions.
- **P-3, roll about the axis.** The command carries no "up" vector. The first direction `Y_m` is the
  element-local +Z direction projected onto the plane normal to `Z_m` and normalized, which makes
  the height of a horizontal member point up. If the axis is parallel to element-local Z (a column,
  including an anti-parallel axis), `Y_m` is instead the element-local +Y direction projected onto
  the plane normal to `Z_m` and normalized; an exactly vertical axis keeps `Y_m = +Y`. Then `X_m =
  normalize(Y_m x Z_m)` and `Y_m = Z_m x X_m`, which is right-handed (`X_m x Y_m = Z_m`) and keeps
  the section perpendicular to the axis. After that default frame is built, optional `roll_rad`
  (finite, default 0) rotates the section about local +Z_m by the right-hand rule: `X_r = cos(r)*X_m
  + sin(r)*Y_m`, `Y_r = -sin(r)*X_m + cos(r)*Y_m`; Z_m is unchanged. No other roll mechanism exists;
  other orientation is element placement. Roll belongs to `shape_inputs`, never placement. No
  existing numeric angle cap was found in F04/F06; assumed cap: `abs(roll_rad) <= 2*pi` radians,
  inclusive (binary64 cap 6.283185307179586). Above it is `LIMIT_EXCEEDED`; non-finite is
  `NON_FINITE_NUMBER`. Updates may set roll only on members; a plate roll patch is
  `UNSUPPORTED_PROPERTY`. An omitted create roll means zero; an omitted patch roll leaves the old
  value unchanged. Authored roll is preserved, without modulo reduction or hidden orientation
  presets. The tolerance for "parallel" is F06's draft policy ([F06 section
  4](f06-geometry-contract-v0.1.md#4-tolerance-and-degeneracy-policy-g-06-draft), rule 2); an exactly
  vertical axis always uses the column rule. The roll is discontinuous at that threshold, which is
  retained as the default-frame rule; continuity remains open in F06.
- **Owner confirmation, 2026-10-16.** P-1a to P-1d (contour limits, winding rejection without
  reordering, planarity band, rectangle-only first-slice evaluation), P-2, P-3a (default frame:
  horizontal axis gives height up, vertical axis gives height along +Y) and P-3b (roll cap 2*pi) are
  confirmed as written. P-3c (the roll jump at the parallel threshold) is accepted for 0.1.0 as the
  default-frame rule; the owner asked for a black-box observation of the reference behaviour. Any
  change to the rule is erratum E-2 and raises the F06 algorithm version, so stored shape keys
  change visibly and no 0.1.0 ID or case is rewritten.
- **Reserved section anchor (owner, 2026-10-16).** P-2 "centred on the axis" is the default anchor. A
  future optional `section_anchor` places the section by one of nine points (top, middle, bottom by
  left, centre, right) plus optional offsets `dx`, `dy` in millimetres along `X_m`, `Y_m`. The name
  and shape are reserved only: 0.1.0 has no such command argument, patch field or USD attribute, and
  a request carrying it is `UNSUPPORTED_PROPERTY`. Adding it later needs a new contract version and
  a new F06 algorithm version, not a rewrite of P-2.

Worked examples (exact; each vector and bound was checked with rational arithmetic, and the case
file carries them as `F04-POS-14` to `F04-POS-16`). The corrected near-vertical rule leaves all
three unchanged: the horizontal member projects +Z onto the plane normal to its axis, and the
exactly vertical column keeps +Y. Tilted near-vertical zero-roll axes are F06 cases (`F06-POS-41`,
`F06-POS-42`); the new rolled case is F04-POS-23 / F06-POS-52.

| Example | Frame | Local bounds (mm) |
|---------|-------|-------------------|
| Plate 1000 x 500 x 10, identity pose | local X, Y, Z carry width, length, thickness | x in [0, 1000], y in [0, 500], z in [0, 10] |
| Horizontal member `p1 = (0, 0, 0)`, `p2 = (4000, 0, 0)`, `w = 200`, `h = 300` | `Z_m = (1, 0, 0)`, `Y_m = (0, 0, 1)`, `X_m = Y_m x Z_m = (0, 1, 0)` | x in [0, 4000], y in [-100, 100], z in [-150, 150] |
| Column `p1 = (0, 0, 0)`, `p2 = (0, 0, 3000)`, `w = 200`, `h = 300` | `Z_m = (0, 0, 1)`, `Y_m = (0, 1, 0)`, `X_m = (1, 0, 0)` | x in [-100, 100], y in [-150, 150], z in [0, 3000] |

For the horizontal member the width 200 lies along `X_m = (0, 1, 0)`, hence the y extent, and the
height 300 along `Y_m = (0, 0, 1)`, hence the z extent. The F07 fixture format orients members with
an explicit `up` vector; that is a different input and is not an input of this command.

The first-scope requirements 0.1.0 (PR #10) listed G-01 and G-02 as open gaps; the frames above
close them.

## 4. Authored model fields

| Field / USD mapping | Required semantics |
|---------------------|--------------------|
| `bim:id` | Immutable owned element identity |
| `bim:name` | Explicit caller label; no environment-dependent hidden name |
| `bim:elementKind` | `plate`, `beam` or `column` for the first slice; independent of USD model kind |
| `bim:geometryMode` | `parametric` for initial create/update operations; preserve other loaded modes and report unsupported edits |
| `bim:schemaVersion` | Explicit `0.1.0` on owned elements; project/schema package also declare compatibility |
| `bim:createdBy` | Creation actor from trusted context; does not authorize future edits |
| `bim:revision` | Last accepted authored revision, absent on a never-published draft element; workspace generation tracks draft edits separately |
| Plate contour / thickness | Explicit `steel:plate:contourPointsMm` array and double `steel:plate:thicknessMm`; no schema fallback supplies missing input |
| Member roll | Optional double `steel:member:rollRad`, radians, default zero; section 3 |
| Straight member axis | Explicit two `bim:axisPointsMm` points in element-local coordinates |
| Profile / material | `bim:profile` / `bim:material` relationships resolve to pinned immutable definitions |
| Placement | Standard USD Xform operations on the owned element root, in authored content layers |
| Engineering operations | Explicit owned operation records and relationships (amendment A-2 below: holes and plane cuts). A create command carries no operations; they are added by their own commands |

Immutable profile/material definitions carry their full content digest and explicit units/schema
version. A member's profile is required; material is optional. Plate shape does not require a
profile definition. Missing material/density never selects a guessed catalogue grade or density.
Physical material and display shader are separate references and signatures.

Profile lookup by commercial catalogue name, inferred environment defaults, position presets, curved
members, direct-shape authoring and fabrication operations are outside this first slice. Do not
implement their behavior from unapproved source notes. Preserve unknown USD namespaces, attributes
and relationships when editing supported fields. Unsupported schema versions may be
inspected/preserved, but cannot be mutated without an explicit reviewed migration.

### Operation records (amendment A-2)

Owner-approved 2026-10-19 (decisions D-1, D-4, D-5; [F06 A-2](f06-geometry-contract-v0.1.md#12-freeze-status-and-open-questions)).
Text marked **proposal for owner review** goes beyond what the owner decided.

An engineering operation of a plate is an owned **operation record**, replacing the A04 pilot's two
plate attributes (`steel:plate:holePointsMm`, `steel:plate:holeVertexCounts`), which were a recorded
deviation from the rule above and are deprecated by this amendment; stores written with them need
an explicit reviewed migration before they can be mutated (section 4 above), owned by the A04
acceptance task. A record has:

| Field | Rule |
|-------|------|
| `operation_id` | Full UUIDv7 of the record, unused in the project (`DUPLICATE_ID`); proposal for owner review: a new identity type of the same form as ElementId |
| `kind` | `hole` or `plane_cut` (first slice) |
| `ordinal` | Assigned by the engine from a per-element counter in creation order; never reused after a removal, never renumbered, never caller supplied. It names the operation's FeatureIds (`cut:f.h<ordinal>...`, F06 section 5). The kernel's present limit of 999 on it is an implementation limit (G10) |
| parameters | `hole`: the clockwise polygon in plate-local XY, 3 to 1024 points, finite, within the coordinate cap, plus optional generator parameters (`round`: centre and diameter; `rectangle`: corner, width, height) kept for editing (proposal for owner review: the polygon is authoritative and the generator is informative). `plane_cut`: a point and a normal in element-local coordinates and the kept side (`positive` or `negative`) |

The operation records of one element are ordered by ordinal and are always replayed from the base
shape in that order (F06 section 2, "Operations"); there is no stored intermediate mesh that an edit
could leave stale. A round hole is stored as its regular `N`-gon (F06 section 2, D-2). There is no
designed limit on the number of operations of an element (D-5); the only limits are the F06
section 6 mesh caps and the implementation capacity of F06 section 2, enforced as below.

**Capacity before the kernel (D-5).** Before an operation command runs the kernel, the engine checks
the request against the kernel's implementation limits (hole and edge counts, triangle and
certification budget estimates). Above them the command is refused with the existing code
`LIMIT_EXCEEDED`, diagnostic `field` token `kernel_capacity` and the F06 diagnostic
`GEO_KERNEL_CAPACITY` (a new GEO token, no new F04 code; naming `LIMIT_EXCEEDED` rather than
`UNSUPPORTED_GEOMETRY` is a proposal for owner review). It is not `INVALID_ARGUMENT` and not
"invalid geometry". It changes no authored state, derived state or generation and publishes
nothing (section 8). A kernel budget refusal that passes the pre-check maps to the same result. The
numeric limits are not part of this contract ("implementation limit, to be lifted by G10").

**Geometry refusals.** `GEO_OP_UNSUPPORTED` maps to `UNSUPPORTED_GEOMETRY` and `GEO_OP_NO_EFFECT`
to `INVALID_ARGUMENT`; an invalid hole polygon is `INVALID_ARGUMENT` (`GEO_CONTOUR_INVALID`),
as the A04 pilot already does. No new F04 code is added.

### Element validity state (D-7; proposal for owner review)

The owner decided (2026-10-19, D-7): a part wrongly created by the user (for example contour points
in the wrong order, so the polygon is clockwise, self-intersecting or invalid) is kept as a
degenerate or invalid element without derived data, and when the user repairs it, geometry is
evaluated and volume and quantities are computed and recorded as for a normally created element.
Everything below, apart from that decision, is a proposal for the owner to review and amend; the
existing refusals of this contract stay in force until the owner accepts it.

| Topic | Proposal |
|-------|----------|
| State | Every owned element has a lifecycle validity `valid` or `degenerate`. A `degenerate` element keeps its authored inputs exactly as submitted (nothing reordered, merged, snapped or repaired) and carries up to 32 diagnostics (code and fixed field token) that say why. `valid` is the only state that has derived data |
| Derived data | A degenerate element has no mesh, shape key, FeatureIds, volume, surface area, bounds or mass. Each quantity record has status `unknown` with reason `shape_degenerate` and no value; nothing is computed from a guess, a bounding box or a partial shape |
| Which defects qualify | Only a geometric validity failure of well-typed, finite, in-cap shape inputs: a clockwise, self-intersecting, repeated-point, zero-area or non-planar contour (`INVALID_ARGUMENT`), a thickness, edge, section or axis below its minimum, a zero-length axis, an operation record that no longer fits after an edit. Never eligible, always rejected as today: malformed commands, non-finite numbers (`NON_FINITE_NUMBER`), cap violations (`LIMIT_EXCEEDED`), unknown or missing targets and definitions, permission and revision failures. Whether a contour with fewer than 3 points qualifies is an open detail (owner) |
| Creation | The refusal of section 7 stays the default: `CreatePlate` and `CreateStraightMember` with an invalid shape return `INVALID_ARGUMENT` and create nothing. They gain an optional argument `on_invalid_shape` with values `reject` (default, today's behaviour) and `keep_degenerate`; with the second the element is created in the `degenerate` state, one transaction, one undo unit. Interactive tools pass `keep_degenerate`; scripted callers keep the default unless they ask. Whether the default should change is an owner decision |
| Update | `UpdateElement` takes the same argument. On a valid element `reject` is the default as today; on a `degenerate` element a patch that leaves it degenerate is accepted (so a user can repair step by step), and a patch that makes it valid evaluates geometry and quantities in the same transaction, as for a new element. Rename, placement and material patches are always allowed. A patch never applies a valid half (section 7) |
| Operations on it | `AddHole`, `UpdateHole`, `RemoveHole`, `CutByPlane` on a degenerate target are refused with the existing `DEPENDENCY_UNAVAILABLE` (the base shape is not available). `RemoveHole` of an existing record is allowed so that a user can clean up |
| Repair | Repair is an ordinary edit. The result is exactly what creating the repaired element would give, with the same quantities, keys and FeatureIds; no history of the defect remains in them. If a repair makes an existing operation record unfit (a hole now outside the outline) the element stays degenerate with a diagnostic that names the record; no record is dropped silently |
| Queries | `InspectElement` shows `validity`, the diagnostics, the explicit inputs and `unknown` quantities. List and snapshot queries report the element with null keys, digest and counts and deliver no mesh blob. A viewport may draw the authored outline as an error marker; that is a presentation, not geometry, and cannot be picked as a solid. No FeatureId exists for it (`NOT_FOUND`) |
| Numbering | A degenerate element gets no part number, joins no equivalence group and holds no approved number; becoming valid enters it into numbering like any new element (A08/A09 own the mechanics) |
| IFC and BOM | Excluded from IFC export (reported as an omitted element with its reason, never exported as an empty or approximate solid) and from BOM and mass totals, whose reports state the number of excluded elements and list them with `unknown` quantities; a quantity is never reported as zero |
| Undo and redo | Creating a degenerate element, a failed-state edit and a repair are each one undo unit (section 8). Undo restores the exact previous state and validity: undoing a repair returns to `degenerate` with no derived data, never to a stale result. Derived data always follow the validity state |
| Publication | Open detail for the owner. The A04/P01 publish-time verifier re-evaluates each plate and today refuses a revision with an invalid one. Proposal: `CommitWorkspace` keeps refusing a workspace that contains a `degenerate` element, with an existing code to be chosen by the owner, until the owner decides otherwise |
| Other commands | `CopyElement` copies a degenerate element as degenerate; `DeleteElement` works as for any element; relationships to a degenerate element follow the existing dependency rules |
| Footprint | Validity is a lifecycle fact: commands that can change it read and write the lifecycle and shape-input groups (section 8) |

This state does not turn an invalid model into a supported one: a valid contour that the geometry
cannot evaluate (for example outside the plane `z = 0`) stays `valid` with geometry `unsupported`,
as in section 3.

## 5. Derived results and signatures

Each quantity has its own record: value (if any), unit, status, input signature and algorithm
version. Status is `current`, `stale`, `unknown` or `unsupported`. `current` requires a supported
computation over matching authored inputs and dependencies. `stale` may retain a clearly marked
previous value; consumers cannot treat it as current. `unknown`/`unsupported` have no calculated
value. A declared external value is a separate record. No bounding-box volume, approximate mesh or
missing density is promoted to exact mass/volume. Amendment A-2: a volume taken from a certified
closed shell is `current` and carries its enclosure width ([F06 section
8](f06-geometry-contract-v0.1.md#8-quantities)); an element in the `degenerate` state (section 4,
proposal for owner review) has every quantity `unknown`.

| Change | Dependencies and invalidation |
|--------|-------------------------------|
| Rename | Label only; retain shape and engineering quantities |
| Placement | Retain local immutable shape; invalidate world bounds/measurements |
| Plate contour/thickness, member axis/roll, profile or supported operation | Invalidate shape and its dependent quantities/features |
| Material/density | Retain shape; invalidate mass; the resolved display reference changes only if a different display reference is selected |
| Camera / selection | No authored element or engineering-signature mutation |

"Material-dependent appearance" means the resolved display reference (digest) of the element, not
the physical material: a change of density value alone invalidates mass only, and changes no
presentation input. Display references and physical-material references stay separate (section 4).

F06 defines the canonical numeric encoding and the key preimages ([F06 section
7](f06-geometry-contract-v0.1.md#7-transforms-and-measurement-keys), frozen 0.1.0); P04 still owns
signature storage. A blob digest identifies saved bytes; it is not the signature of normalized
engineering inputs. No arbitrary dimension rounding is allowed to manufacture shared hashes.

For the original synthetic example, 1000 x 500 x 10 mm gives volume 0.005 m3. If an explicitly
authored synthetic material has density 1000 kg/m3, mass is 5 kg. These are analytic project
examples, not catalogue records or a shipped USD fixture. F07/U02 must provide complete original
fixtures and real resources before runtime acceptance.

## 6. Logical mutation envelope

The envelope is shared by the future engine API, CLI and host. It does not fix F05 framing.

| Field | Rule |
|-------|------|
| `contract_version` | Exactly `0.1.0`; unsupported version is diagnostic, not best-effort parsing |
| `project_id`, `workspace_id` | Full IDs selecting an authorized project and owned draft |
| `op_id` | Full UUIDv7 for one atomic logical command |
| `actor_id` | Must match trusted caller context; never a credential |
| `base_revision` | Revision pinned by the selected workspace |
| `expected_generation` | Exact decimal WorkspaceGeneration precondition |
| `command` | Closed command name from the table below |
| `arguments` | Typed command-specific object; unknown envelope/argument fields are rejected |

Draft execution compares `base_revision` with the workspace base, not automatically with the latest
published head. This permits offline drafts. Publication additionally checks the current accepted
head. A changed head requires explicit rebase/conflict handling, never silent draft loss.

Within a workspace, retrying an accepted op_id with identical canonical request content returns the
recorded result without another generation increment. Different content under that op_id is
`OP_ID_REUSE`. Authentication/authorization still apply to a replay. Canonical request content
includes project/workspace/actor/base/generation, command and all arguments; exclude transport
metadata. The byte/number canonicalization profile must be fixed in F05 before implementation. C01
additionally defines project-wide publication idempotency; this draft rule is not its proof.

## 7. Command surface

| Command / arguments | Atomic behavior | Planned feature owner |
|---------------------|-----------------|-----------------------|
| `CreatePlate`: new_id, name, contour_points_mm, thickness_mm, placement, optional material_digest | Validate contour/thickness and unused ID; author one parametric owned plate; a simple polygon in `z = 0` is evaluated, other valid contours evaluate as unsupported (A-2); a shape that fails validation is refused (default) or kept `degenerate` (D-7 proposal) | P01 |
| `CreateStraightMember`: new_id, element_kind, name, axis_points_mm, profile_digest, placement, optional roll_rad, optional material_digest | Validate nonzero supported local axis and pinned profile; beam/column is an explicit kind | P03 |
| `UpdateElement`: target_id, changes | Typed patch of supported name/contour_points_mm/thickness_mm/axis_points_mm/roll_rad/profile_digest/material_digest/placement; no raw USD attribute/path setter | P04/P08 |
| `CopyElement`: source_id, new_id, name | Create an independent owned root with a fresh ID; share immutable definitions/shape until an edit | P05 |
| `DeleteElement`: target_id | Reject unresolved incoming dependencies; no implicit cascade or foreign-source mutation | P08 |
| `AddHole`: target_id, operation_id, hole (`polygon_points_mm`, or a `round` or `rectangle` generator) | A-2. Validate a valid plate target, a clockwise simple polygon and an unused operation_id; assign the next ordinal; check capacity (section 4); replay the base shape and all operation records through the kernel; on success append the record and update derived data in one transaction | A04 |
| `UpdateHole`: target_id, operation_id, changes (`polygon_points_mm` or generator parameters) | A-2. Replace the parameters of one existing hole record, keeping its ordinal and `operation_id`; the whole replay must succeed or nothing changes | A04 |
| `RemoveHole`: target_id, operation_id | A-2. Remove one hole record; its ordinal is retired and its FeatureIds become dangling (F06 section 5); the remaining records are replayed | A04 |
| `CutByPlane`: target_id, operation_id, plane (`point_mm`, `normal`), `keep` (`positive` or `negative`) | A-2. Add one `plane_cut` record to a valid target; a plane that is tangent, coplanar with a face, makes several section loops or misses the body is refused (`UNSUPPORTED_GEOMETRY` or `INVALID_ARGUMENT`, section 4) and changes nothing | A03 |

Plate `width_mm` and `length_mm` are no longer command or patch fields; derive them only for a
supported rectangle. No compatibility alias or silent persisted-schema migration is defined.

`changes` must be nonempty, every property must be supported by that kind, and all requested changes
form one transaction. `material_digest=null` explicitly clears a material reference; absence from a
patch leaves the material unchanged. Clearing a required profile is invalid. No caller-supplied
`bim:id`, generated path, derived mesh, quantity, actor, revision or numbering approval can be
patched through this API. A diagnostic never applies the valid half of a patch.

**Notes on the A-2 commands.** Each is one atomic command and one undo unit regardless of the
number of records it touches, with the footprints of section 8. `UpdateElement` of a contour or
thickness replays every operation record and is refused (or, for the D-7 proposal, kept
`degenerate`) if one no longer fits. `CopyElement` copies the operation records with fresh
`operation_id`s and the same ordinals. Proposals for owner review: (a) the pilot's optional
`CreatePlate.holes_mm` becomes shorthand for `CreatePlate` followed by ordered `AddHole` records in
one transaction, decided by the A04 acceptance task; (b) `AddHole` targets plates in the first
slice, and a hole in a beam flange or web is refused with `UNSUPPORTED_GEOMETRY` until the kernel
supports it (the kernel's through hole needs a prism along the cutter direction); (c) update and
removal of a plane cut follow the same pattern as `UpdateHole` and `RemoveHole` and are defined by
A03. The command names and argument names above are the logical surface; F05 framing is unchanged.

The first `CopyElement` takes one source. Internal self/owned operation-record relationships remap
to fresh destination entity IDs; shared external immutable references remain shared. Local geometry
FeatureIds remain scoped to their immutable result and are addressed through the new ElementId.
Relationships outside its supported copy set cause a diagnostic rather than an incomplete copy.
Copying a foreign source, inventing a density or retaining a live reference to the mutable source is
forbidden. Multi-element/assembly copy and feature-ID remap rules need later reviewed extensions.

Read API `InspectElement` returns a revision/generation-stamped view, ownership, explicit inputs,
relationships and per-quantity status. It never reconstructs missing engineering input silently.
Preview validates/calculates without publishing or advancing a draft generation; it is not an
accepted op_id journal entry. `CommitWorkspace` is a separate host operation under U06/U07; it
publishes a complete USD revision and does not turn one command into a separate saved model.

## 8. Footprints, transactions and errors

The engine derives read/write sets from command semantics and validated dependencies. Caller claims
cannot narrow them. Each entry identifies an owned ID or immutable digest plus a semantic group:
label, shape_inputs, placement, material_ref, relationship, lifecycle or derived signature.

| Operation | Minimum semantic footprint |
|-----------|----------------------------|
| Create | Read ID absence and pinned definitions; write new lifecycle, complete authored fields and relationships |
| Update contour/thickness/axis/roll/profile | Read complete shape-input group and relevant immutable definitions; write that group and invalidate dependent results |
| Update material | Read geometry-result signature and new material definition; write material_ref and mass/appearance invalidation |
| Update placement | Read local shape and placement; write placement and world-measurement invalidation |
| Copy | Read source lifecycle, full supported authored state, copy-set relationships and definitions; write new lifecycle and remapped owned state |
| Delete | Read lifecycle and complete incoming-relationship index at the expected generation; write lifecycle only if all dependencies permit deletion |
| Add/update/remove hole, cut by plane (A-2) | Read target lifecycle and validity, the complete shape-input group, all operation records of the target and the pinned definitions; write the operation records and the shape-input group, invalidate shape, quantities, features and mass signature, and re-evaluate derived data. A capacity refusal writes nothing |
| Change of validity (D-7 proposal) | Read and write lifecycle (validity) and the shape-input group; derived data are written or cleared together with the state |

Different property names alone do not prove independence. A later cut/bolt command must declare its
dependencies on related element shape/profile/placement. If an index or required payload is
unavailable, return `DEPENDENCY_UNAVAILABLE`; do not assume no incoming links or a current result.

Validation order: bounded decode/version/types -> trusted context and authorized workspace ->
existing op_id replay -> for new operations, workspace/base/generation -> ownership/ID/relationships
-> complete semantic footprint -> engineering validation -> apply to isolated draft -> validate
resulting composition -> record draft/journal. All state checks and commit occur under the host's
workspace transaction serialization mechanism. U06 also serializes publication with expected-head
verification; plain read-then-rename is not CAS.

An accepted mutation advances generation once and constitutes one future undo unit, regardless of
changed-property count. Rejection leaves authored state, derived state and generation unchanged. P07
defines durable undo/redo and restored identity/relationships; no fake undo is introduced here.
Cancellation before the apply/commit point discards work. After that point, report the recorded
outcome or `OUTCOME_UNKNOWN`; a timeout is not proof of rollback. U07/F05 define recovery/query.

| Stable code | Meaning |
|-------------|---------|
| `MALFORMED_COMMAND` | Invalid encoding/JSON, duplicate keys or structurally invalid envelope |
| `UNSUPPORTED_VERSION`, `UNSUPPORTED_COMMAND`, `UNSUPPORTED_PROPERTY` | Version/operation/property outside the accepted contract |
| `INVALID_ID`, `DUPLICATE_ID`, `NOT_FOUND` | Invalid full identity, already-owned creation ID, or missing target/definition |
| `INVALID_ARGUMENT`, `NON_FINITE_NUMBER`, `LIMIT_EXCEEDED` | Bad field/type/range, non-finite f64 or bounded-input cap violation |
| `PERMISSION_DENIED`, `FOREIGN_READ_ONLY` | Untrusted/mismatched caller or attempted foreign edit/override |
| `BASE_REVISION_MISMATCH`, `STALE_WORKSPACE`, `OP_ID_REUSE` | Failed revision/generation precondition or changed request under accepted op_id |
| `DANGLING_RELATIONSHIP`, `DEPENDENCY_CONFLICT`, `DEPENDENCY_UNAVAILABLE` | Invalid link, forbidden delete/change, or incomplete validation inputs |
| `UNSUPPORTED_GEOMETRY`, `INTEGRITY_ERROR` | Shape/transform outside scope, or revision/blob bytes inconsistent with their identity |
| `CANCELLED`, `OUTCOME_UNKNOWN`, `BACKEND_UNAVAILABLE` | Cancelled before commit, uncertain committed outcome, or required backend unavailable |

A zero-length member axis returns `INVALID_ARGUMENT` (G-03, adopted by the owner 2026-10-15; see
[section 3](#3-units-types-and-initial-limits)).

**Operation-command failures (amendment A-2).** The geometry diagnostics of the operation commands
map to the existing codes above and no code is added: `GEO_OP_UNSUPPORTED` and a body outside the
kernel's supported class are `UNSUPPORTED_GEOMETRY`; `GEO_OP_NO_EFFECT`, an invalid hole polygon
(`GEO_CONTOUR_INVALID`) and a plane that leaves the body unchanged or empty are `INVALID_ARGUMENT`;
`GEO_BUFFER_LIMIT` and `GEO_TOPOLOGY_LIMIT` are `LIMIT_EXCEEDED`. **Kernel capacity exceeded**
(`GEO_KERNEL_CAPACITY`, F06 section 2 "Operations") is `LIMIT_EXCEEDED` with the `field` token
`kernel_capacity`, checked in the engineering-validation step before the kernel runs (and
reported the same if a kernel budget ends the run). It is distinct from `INVALID_ARGUMENT` and
from invalid geometry, applies no change and publishes nothing; the choice of `LIMIT_EXCEEDED` is a
proposal for owner review. A budget refusal is never worded as an invalid shape.

Errors return code, safe message, field/target diagnostics and the observed workspace token when
known. Never include credentials, private paths or client data. Multiple failures follow this
validation order; runtime tests must not rely on whichever untrusted collection entry is visited
first.

## 9. F02 gaps resolved for this contract

- No automatic environment/default/catalogue resolution in the first slice. Explicit pinned
  definitions are required; environment resolver scope belongs to later packages.
- Initial member placement is explicit rigid pose plus local axis. Commercial position presets and
  unverified position cells are unsupported, not inferred.
- Quantities use geometric volume and explicit density; vendor gross/net tables and rounding are
  outside this contract. Unknown density remains unknown.
- Foreign-source annotations are separate owned result objects, never edited foreign UDAs.
- One accepted command is one atomic local undo unit; cancellation/rollback do not leave half a
  patch.
- Numbering/equivalence is not specified by F04. Placement-sensitive fabrication cases, tolerance
  and approved-number invalidation must be resolved by A08/A09 before numbering implementation.

## 10. Acceptance handoff and provenance

[Original command cases](../../tests/fixtures/contracts/f04-model-command-cases-v0.1.json) define positive and negative acceptance
descriptions. They are not a runnable model, geometry fixture, engine mock or evidence of a working
command dispatcher. No placeholder blob is offered as an actual saved resource. Case IDs are
proposed future tests; acceptance requires real API -> CLI -> USD -> independent reader. There are
69 logical cases (30 positive, 39 negative) and seven malformed/non-finite raw JSON cases.
`F04-POS-01` to `F04-POS-07`, `F04-NEG-01` to `F04-NEG-18` and `F04-RAW-01` to `F04-RAW-03` are the
original draft cases and keep their IDs and meaning; `F04-POS-08` to `F04-POS-30`, `F04-NEG-19` to
`F04-NEG-39` and `F04-RAW-04` to `F04-RAW-07` were added by the amendments in [section
12](#12-amendments-log). The member cases that need a profile (`F04-POS-07`, `F04-POS-10`,
`F04-POS-13`, `F04-POS-15`, `F04-POS-16`, `F04-POS-21` to `F04-POS-23`, `F04-NEG-20`,
`F04-NEG-21`, `F04-NEG-33`, `F04-POS-29`, `F04-POS-30`, `F04-NEG-36`, `F04-NEG-37`,
`F04-NEG-39`) require the harness to bind an
actual validated original profile digest; they do not provide a fake resource or an executable
fixture.

**Test comparison rule (G-05, adopted).** Expected numeric values in the case file are compared with
the owner amendment to CR-TOL-001 dated 2026-10-15: quantities pass when `|actual - expected| <=
1e-6*|expected|`; lengths and coordinates use `|actual - expected| <= 0.001 mm + 1e-6*|expected|`.
Dimensionless frame values use absolute 1e-6 at zero and relative 1e-6 otherwise. Exact rational
expected values never change; identities, strings, booleans, counts, generations and codes compare
exactly. This is a test comparison rule only, separate from geometric decisions and numeric
certificates. The test length floor remains 1e-6 m = 0.001 mm; it does not scale with body size.
The read-only first-scope package 0.1.0 retains the earlier 1e-12/1e-9 mm rule; it needs a
separately reviewed version amendment. References to its cases preserve mathematical meanings, not
an
assertion that its comparison policy has already changed. F07 has its own comparison rule, unchanged
here. See the [tolerance comparison](../design/tolerance-comparison-v0.1.md) for classification,
assumptions, design verdicts and the recorded 15-value ifc-lite comparison supplied by the
orchestrator from the public repository. The 2026-10-15 owner decision applies local-size scaling
to geometric length decisions (section 3; F06 section 4), closing only the large-coordinate part
of A5. Rounded-mesh
cross-checks and subnormal mass remain open. CR-TOL-001, the absolute pick band and exact
identities do not change; relative bounds and numeric certificates retain the 1e-6 policy.

**Coverage and cross-references (G-05).** New cases refer to the first-scope cases by ID through
`first_scope_refs` (package version 0.1.0, field `first_scope_package`) where a corresponding
first-scope case exists; the expected codes agree with those cases except for the one G-03 choice.
`F04-POS-12`, `F04-POS-14` to `F04-POS-17`, `F04-NEG-27` and `F04-RAW-04` to `F04-RAW-06` are
F04-only cases with no first-scope counterpart and carry no `first_scope_refs`.

| Topic | F04 cases | First-scope cases |
|-------|-----------|-------------------|
| Degenerate (zero-length) axis | `F04-NEG-20` | `FS-NEG-AXIS-ZERO` |
| Axis and plate dimension at the limit, accepted | `F04-POS-08`, `F04-POS-13` | `FS-PLATE-C`, `FS-MEMBER-B` |
| Axis and plate dimension just above the limit, rejected | `F04-NEG-19`, `F04-NEG-21` (coarser: `F04-NEG-05`) | `FS-NEG-PLATE-BOUND`, `FS-NEG-AXIS-BOUND` |
| Translation at the limit, accepted / just above, rejected | `F04-POS-09` / `F04-NEG-26` | `FS-POSE-E` / `FS-NEG-TRANSLATION-BOUND` |
| Invalid quaternion: norm, component range, zero, just outside tolerance | `F04-NEG-22` to `F04-NEG-25` | `FS-NEG-QUAT` |
| Non-finite quaternion or translation component | `F04-RAW-04`, `F04-RAW-05`, `F04-RAW-06` | none (F04-only cases; they follow the parsing pattern of `F04-RAW-01` and `F04-RAW-02`) |
| Rotation convention and rotate-then-translate | `F04-POS-10` | `FS-POSE-A` |
| Quaternion norm slightly off one, accepted and normalized | `F04-POS-11` | `FS-POSE-F` |
| `q` and `-q` | `F04-POS-12` | none (F04-only case) |
| Plate and member local frames (P-1 to P-3) | `F04-POS-14`, `F04-POS-15`, `F04-POS-16` | none (F04-only cases; the package lists G-01 and G-02 as open). Tilted near-vertical axes: F06 `F06-POS-41`, `F06-POS-42` |
| Minimum nonzero dimension (draft): exactly 0.001 mm accepted / just below, rejected | `F04-POS-17` / `F04-NEG-27` (zero: `F04-NEG-04`) | none (F04-only cases). F06: `F06-POS-14` / `F06-NEG-03`; member axis `F06-NEG-04`, `F04-NEG-20` |
| Contour rectangle rotation/start and valid pentagon | `F04-POS-18` to `F04-POS-20` | none; F06 `F06-POS-48`, `F06-POS-49`, `F06-NEG-39` (superseded by F06 A-2; `F04-POS-20` keeps its case text, see section 12) |
| Clockwise, non-planar, self-intersection, count, repetition | `F04-NEG-28` to `F04-NEG-32` | none; F06 `F06-NEG-37`, `F06-NEG-38`, `F06-NEG-40` to `F06-NEG-42` |
| Zero, quarter-turn and near-vertical roll; shape footprint | `F04-POS-21` to `F04-POS-24` | none; F06 `F06-POS-50` to `F06-POS-52` |
| Roll cap and non-finite input | `F04-NEG-33`, `F04-RAW-07` | none; F06 `F06-NEG-43`, `F06-NEG-44` |
| Local-size scaled floor, exact boundary, small-body floor, planarity and caps | `F04-POS-25` to `F04-POS-30`, `F04-NEG-34` to `F04-NEG-39` | none; F06 `F06-POS-53` to `F06-POS-58`, `F06-NEG-45` to `F06-NEG-49`, `F06-NEG-51` |

Non-finite quaternion and translation components follow the existing raw-JSON rules: a bare `NaN` is
`MALFORMED_COMMAND` and an overflowing literal such as `1e400` is `NON_FINITE_NUMBER`.

U03 must reject duplicate/short IDs and preserve identity through rename. P01/P03 must round-trip
explicit contour/thickness, local axis/roll and placement. P04 must distinguish shape/mass/world
invalidation and unknown density. P05 must preserve the source through copy/edit/reopen. P08 must
reject dangling delete and partial patches. U06/U07 must prove complete publication, retries and
crash recovery. F05/F06 must add transport/geometry negative cases under these invariants, not
weaken them.

Sources: original owner decisions in local architecture-proposal.md sections 1, 3–7, 9–14 and
17; implementation-plan.md revision 1.1 sections 1–4, 7.1 and 12; local F02 gap analysis; public
RFC/IO references linked above. No private source specification, manual, catalogue, UI label,
screenshot or third-party implementation is published by this document. Constants and command
choices above are proposed original BeamCraft rules, not commercial-product behavior.

Freeze conditions are listed in [section 11](#11-freeze-status). F05/F06 fix their implementation
details within these logical invariants; they are follow-up tasks, not circular prerequisites for
F04 acceptance. F04 review does not permit geometry implementers to inherit source-exposed contexts
or declare unsupported features implemented. The owner permits TOST-1.0 only for official OpenUSD
and its SDK resources; the scoped policy is recorded in [policy PR
#7](https://github.com/Apibim-com/BeamCraft/pull/7). SDK integration still requires provenance,
complete notices, native dependency closure and runtime checks. The remaining dependency, asset and
unsafe-code policies continue to apply.

## 11. Freeze status

This contract is **frozen at 0.1.0**, by the owner's explicit decision of 2026-10-16. Nothing in it is
implemented, K0 is not passed and no geometry implementer is declared eligible. A frozen contract
changes only by an erratum (0.1.x, owner-approved, listed in section 12 and in F06 section 12) or a
new contract version; no review, task or merge changes it implicitly. Freezing does not admit an
implementer: the first-scope package still needs its update to the contour plate (separate F09
admission step).

Adopted by the owner on 2026-10-15 (F09 review findings G-03 to G-05):

| Finding | Resolution | Where |
|---------|------------|-------|
| G-03 zero-length axis code | `INVALID_ARGUMENT` | [section 3](#3-units-types-and-initial-limits), `F04-NEG-20` |
| G-04 quaternion convention | `(x, y, z, w)`, Hamilton, active rotation, `world = R*local + translation`, normalized before use, `q` and `-q` equal | [section 3](#3-units-types-and-initial-limits) |
| G-05 comparison rule and cases | CR-TOL-001 as a test comparison rule only; 17 logical and three raw cases added | [section 10](#10-acceptance-handoff-and-provenance) |

Decided by the owner on 2026-10-15:

| Item | Recorded resolution | Status |
|------|---------------------|--------|
| G-01 / P-1 | Contour plate; X along first edge, Z thickness/normal, Y=Z x X; rectangle-only evaluation until A02 (a simple polygon in `z = 0` is evaluated since A02.1, amendment A-2) | Replaced; confirmed 2026-10-16 (P-1a to P-1d) |
| G-02 / P-2 | Centred member section and perpendicular ends, Y through a cross product | Confirmed unchanged; `section_anchor` reserved (section 3) |
| P-3 | Deterministic zero-roll frame followed by optional roll about local +Z | P-3a and P-3b confirmed 2026-10-16; P-3c accepted, erratum E-2 |
| G-06 | Constants confirmed for the first slice; relative 1e-6 and geometric floor eps_geom | Frozen 2026-10-16; fixture validation yields errata only; 15-value comparison recorded in the [tolerance report](../design/tolerance-comparison-v0.1.md); A5 large-coordinate part closed by local-size scaling; rounded-mesh cross-checks and subnormal mass open |
| Profile resource | Logical rectangle content in F06 section 2 | Accepted by F06 under draft policy; persisted encoding open |
| Freeze | Explicit owner freeze | **Done 2026-10-16** (F04 and F06 0.1.0); implementer admission remains separate |

The first-scope package 0.1.0 still has the old input and gaps; its update is a separate admission
step. Freezing does not declare an implementer eligible. The `contract_version` remains `0.1.0`. K0
is not passed and nothing is implemented.

## 12. Amendments log

- 2026-10-14, F04.2: status changed to freeze candidate; stale F09 statements updated to the
  integrated review 0.1.0 and first-scope package 0.1.0 (PR #10).
- 2026-10-14, F04.2: G-04 quaternion convention stated in section 3, consistent with CR-PLACE-001.
- 2026-10-14, F04.2: G-03 zero-length axis mapped to `INVALID_ARGUMENT` (proposed for owner
  adoption); recorded in sections 3 and 8. No new error code was added.
- 2026-10-14, F04.2: G-05 comparison rule adopted (section 10); cases `F04-POS-08` to `F04-POS-13`,
  `F04-NEG-19` to `F04-NEG-26` and `F04-RAW-04` to `F04-RAW-06` added with first-scope
  cross-references. Existing case IDs and meanings are unchanged.
- 2026-10-14, F04.2: sections 11 and 12 added. Caps, identity rules, envelope, command surface and
  the other error-code semantics are unchanged. `contract_version` remains `0.1.0`.
- 2026-10-15, F04.2: the owner adopted G-03 (`INVALID_ARGUMENT` for a zero-length axis), G-04 (the
  quaternion convention) and G-05 (the test comparison rule and added cases) as written. "Proposed
  for owner adoption" was removed from sections 3, 8 and 11.
- 2026-10-15, F04.2: the owner decided G-01 (plate local X along the first edge, thickness along
  local Z) and G-02 (section in the local XY plane, local Z along the axis, height in that plane).
  Recorded in sections 3 and 11.
- 2026-10-15, F04.2: added proposals P-1 to P-3 (plate local box, member section frame, roll about
  the axis), marked "proposed for owner adoption", with worked examples and cases `F04-POS-14` to
  `F04-POS-16`. G-06 and the profile resource format remain open; the freeze decision remains with
  the owner.
- 2026-10-15, F04.3, on base `b996c58fe850642f12b92b25b23c441fbd41ed47`: amendments proposed by F06
  section 12 are applied; the document stays a freeze candidate, not frozen, with `contract_version`
  `0.1.0`. K0 is not passed and no runtime command is implemented. Caps, identity rules, the
  envelope, the command surface and every existing error-code meaning are unchanged.
  - Section 3 limits (F06 proposed amendment 1; F06 sections 2 and 4): added the draft minimum
    nonzero dimension of 0.001 mm (plate dimensions, section dimensions, member-axis length;
    `INVALID_ARGUMENT` between zero and the minimum) and the draft section-dimension cap of
    1,000,000 mm (`LIMIT_EXCEEDED`), both marked draft, to be validated by G04 and fixtures. The
    G-03 sentence on very short axes now points to the minimum. New cases `F04-POS-17` and
    `F04-NEG-27`; the counts in section 10 are now 44 logical cases (seventeen positive,
    twenty-seven negative) and six raw cases.
  - Section 3, P-3 (F06 proposed amendment 4; F06 section 4, rule 2): the fixed +Y fallback is
    replaced by the projection of +Y onto the plane normal to the axis, then `X_m = normalize(Y_m x
    Z_m)` and `Y_m = Z_m x X_m`; an exactly vertical axis keeps +Y. The worked examples and
    `F04-POS-14` to `F04-POS-16` are unchanged (their frames are the same); P-1 to P-3 remain
    "proposed for owner adoption" and the owner has not confirmed them.
  - Section 5 (F06 proposed amendment 2; F06 section 7): "material-dependent appearance" is the
    resolved display reference, so a density change alone invalidates mass only; F06 defines the
    canonical numeric encoding and the key preimages.
  - Section 11 (F06 proposed amendment 3): G-06 and the profile-resource-format question are
    accepted by F06 under its draft policy (F06 sections 2 and 4), still not frozen; G-01 and G-02
    stay as the owner decided; P-1 to P-3 await owner confirmation.
  - Status text updated: F06 is a draft contract that exists, not frozen and not implemented. F06
    proposed amendment 5 concerns F05 only and is not applied here.

- 2026-10-15, owner-decision record (stack intended on PR #23; Git ownership prevented branch
  verification). P-1 replaced by contour input; P-2 confirmed unchanged; P-3 retains the default
  frame and adds finite, bounded axial roll. Clockwise rejection against the positive dominant
  normal, 1024-point cap, 2,000,000 mm contour radius, planarity band and 2*pi roll cap are explicit
  assumptions. Nonrectangles are valid authored input with unsupported evaluation until A02.
  Relative comparison/certificate bounds become 1e-6; length comparison/pick floor becomes 0.001
  mm=1e-6 m, equal to the unchanged minimum dimension. Member parallel threshold remains 1e-6
  (squared multiplier 1e-12); exact topology/identities and rational references are unchanged. The
  ray/face angular parallel threshold becomes 1e-6; direction minimum becomes 1e-6 (dimensionless).
  Draft constants are confirmed by the owner for the first slice, still to be validated by fixtures.
  The numeric comparison with the public ifc-lite kernel is in the [tolerance
  report](../design/tolerance-comparison-v0.1.md). No runtime feature is implemented.

  - Old rectangular input cases retain their IDs, meanings and exact volume/area/bounds.
    Width/length fields become four contour points; the width-only patch becomes a whole-contour
    patch. Raw malformed/non-finite tests now use contour coordinates; RAW-03 duplicates the contour
    field. Changed existing descriptions: `F04-POS-01`, `F04-POS-04`, `F04-POS-06`, `F04-NEG-01`,
    `F04-NEG-02`, `F04-NEG-03`, `F04-NEG-04`, `F04-NEG-05`, `F04-NEG-06`, `F04-NEG-07`,
    `F04-NEG-08`, `F04-NEG-09`, `F04-NEG-10`, `F04-NEG-11`, `F04-NEG-17`, `F04-NEG-18`,
    `F04-POS-08`, `F04-POS-09`, `F04-POS-11`, `F04-POS-12`, `F04-NEG-19`, `F04-NEG-22`,
    `F04-NEG-23`, `F04-NEG-24`, `F04-NEG-25`, `F04-NEG-26`, `F04-POS-14`, `F04-POS-17`,
    `F04-NEG-27`, `F04-RAW-01`, `F04-RAW-02`, `F04-RAW-03`, `F04-RAW-04`, `F04-RAW-05`,
    `F04-RAW-06`.
  - Boundary preservation: F04-NEG-25 changes w=0.999999999 (would now pass) to 0.999999 (deviation
    0.000001999999, still rejected). F04-POS-11 moves the accepted-near-boundary example to
    w=0.99999995 (deviation 0.0000000999999975); its identity rotation is unchanged. First-scope
    references remain thematic; that read-only package retains its original inputs.
  - Added POS-18/19 (rectangle frame/start), POS-20 (valid unsupported pentagon), POS-21 to POS-23
    (zero, pi/2 and near-vertical roll), POS-24 (roll update footprint), NEG-28 to NEG-32
    (clockwise, non-planar, self-intersection, too few/repeated points), NEG-33 (roll cap), RAW-07
    (overflow roll). Counts: 57 logical, 24 positive, 33 negative, seven raw. No existing ID or
    error-code meaning is removed. contract_version remains 0.1.0, freeze candidate, not frozen. The
    normal-sign and cap assumptions need owner review.

- 2026-10-15, tolerance-magnitude amendment (intended branch
  `docs/tolerance-magnitude-scaling`, stacked on PR #26; Git ownership blocks verification):
  section 3 applies `eps_geom = max(0.001 mm, 2^-22 * L)` in the element-local frame, with
  ordered binary64 evaluation and existing caps. F06 algorithm/tolerance versions become 3/3.
  Planarity retains its relative 1e-6*D term. CR-TOL-001, pick bands, exact identities and
  dimensionless relative/certificate bounds are unchanged. Only A5's large-coordinate part is
  closed; rounded-mesh cross-checks and subnormal mass stay open. Added POS-25 to POS-30 and
  NEG-34 to NEG-39: 69 logical cases (30 positive, 39 negative), seven raw. No previous case
  outcome flips or input rewrites are needed; existing minimum cases remain below the scaling
  crossover. F04 stays a freeze candidate 0.1.0, not frozen; no runtime change is made.

- 2026-10-16, F04 freeze: the owner confirmed P-1a to P-1d, P-2, P-3a and P-3b, accepted P-3c as
  erratum E-2 (black-box observation requested), reserved `section_anchor` (section 3) and froze F04
  and F06 at 0.1.0. Status lines, section 11 and the cases file status were updated; no case ID, error
  code, limit or meaning changed. The first-scope package update and implementer admission are not
  part of this change.

- 2026-10-18, A07.1 profile amendment (owner decision): the rectangle is no longer the only member
  profile. The F06 amendment A-1 adds the nine parametric section families and custom outlines of
  `docs/design/a07-section-library.md` as immutable, content-addressed profile definitions; their
  values are admitted with the existing section 3 codes (`NON_FINITE_NUMBER`, `LIMIT_EXCEEDED`,
  `INVALID_ARGUMENT` naming the parameter). P-2 places every section by its bounding-box centre.
  This is additive: the rectangle, every existing case ID, error code and limit keep their meaning,
  and `contract_version` stays 0.1.0 with this amendment recorded.

- 2026-10-19, F04 side of F06 amendment A-2 (owner decisions D-1 to D-7; task G08-docs, docs only).
  Owner-decided: holes, plane cuts and polygonal cavities are in scope; a simple polygon in `z = 0`
  is evaluated (section 3, A02.1); no designed limit on holes, with a capacity check before the
  kernel and a distinct refusal (D-5); an invalid user-authored part is kept as a degenerate element
  without derived data and is computed on repair (D-7). Edited: header status, section 3 (contour
  support), section 4 (operation records replacing the A04 plate-attribute deviation; element
  validity state), section 5 (one sentence), section 7 (`AddHole`, `UpdateHole`, `RemoveHole`,
  `CutByPlane`, notes, `CreatePlate` row), section 8 (footprints, operation-command failure
  mapping), section 10 (one reference note) and this log. No existing error code, limit, case ID or
  meaning is changed; no F04 code is added (`GEO_KERNEL_CAPACITY` is an F06 token mapped to
  `LIMIT_EXCEEDED`); `contract_version` stays 0.1.0. **Proposals for owner review** (everything
  outside D-1 to D-7 as worded): the `operation_id` identity, the generator parameters, the
  `on_invalid_shape` argument and its defaults, the degenerate-state rules for operations, queries,
  numbering, IFC, BOM, undo and publication, the choice of `LIMIT_EXCEEDED` for the capacity
  refusal, beam holes being refused until the kernel supports them, and the shorthand
  `CreatePlate.holes_mm`. Case-file edits needed and not made here (the case file is read by
  `apps/beamcraft-cli/tests/f04_cases.rs`): `F04-POS-20` still expects `geometry_status:
  unsupported` for the pentagon, which A02.1 now evaluates; new cases for the A-2 commands and the
  degenerate state come with A04 acceptance and P-Degenerate. No runtime change is made.
