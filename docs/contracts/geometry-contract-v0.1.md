# F06: geometry and viewport contract, version 0.1.0

Status: **draft 0.1.0, not frozen; nothing implemented; K0 not passed**. Date: 2026-10-15.
Base: `9d40abc34b8a5686358b78adee5f9c52b9c830bd`.

This original contract fixes the logical interface between the engine and any geometry kernel,
and between the engine and any viewport or query consumer. It defines what goes in, what comes
out, which numbers are bounded, which identifiers are stable and which results may be called
current. It describes no algorithm and no code. [Acceptance
descriptions](geometry-contract-cases-v0.1.json) are future tests, not runnable fixtures. No
kernel, mesh, query or viewport exists, and no statement here is verified by a test.

## 1. Scope and status

**What this is.** A draft logical contract for the tasks G01-G09, P03, V01-V03 and D01. They can
work against the interfaces below without choosing an implementation. A change that conflicts
with it needs a reviewed amendment (section 12), not a local workaround.

**Relation to other documents.**

- [F04 model and command contract 0.1.0](model-and-commands-v0.1.md) (freeze candidate) owns
  identities, units, input caps, error codes (its section 8), quantity statuses (its section 5)
  and the open items this contract takes over (section 4 here). F06 never weakens an F04
  invariant and never widens an F04 cap.
- [F05 worker protocol 0.1.0](worker-protocol-v0.1.md) (draft) is consumed only for the host
  boundary: attachments, limits and cancellation semantics. F06 does not change it. Mesh bytes
  travel as opaque `u8` attachments, so no typed element negotiation is needed (section 6).
- The [first-scope package 0.1.0](../clean-room/handoff/first-scope-v0.1.md) admits a rectangular
  plate, a straight member, identities, units, rigid placement and declared failures. This
  contract stays inside that scope. Source-exposure admission of any implementer is a separate
  decision under the [admission policy](../clean-room/admission-policy.md).

**Assumptions recorded here (not hidden).**

- The owner decided G-01 and G-02 (F04 section 11). The detail proposals P-1 (plate local box),
  P-2 (member section frame) and P-3 (roll about the axis) are **assumed for this draft,
  pending owner confirmation**. Every rule below that relies on them says so in the words
  "assumed pending owner confirmation" and names the proposal. If the owner changes a proposal,
  F06 changes jointly with F04.
- F06 accepts ownership of G-06 (minimum nonzero dimension, geometric tolerance, the "parallel"
  test of P-3) and of the profile-resource-format question (F06/P02). Their policies in
  section 4 are marked **draft, to be validated by G04 and fixtures**, and are not frozen.

**Excluded from 0.1.0:** boolean operations and any cut, hole, fit or weld geometry; curved or
multi-segment members; contour or bent plates; imported meshes; scale, shear and reflection;
drawing projection and dimensions; rendering style, shading and lighting; persisted storage
formats (USD encoding belongs to the U-tasks, and this contract defines no second persisted model
format); and any claim of numerical accuracy beyond determinism (section 2).

**Sources and provenance.** Contributor-original text, MPL-2.0. Behaviour comes only from F04,
F05, the first-scope package, repository documents and these public references (rules are
referenced, not implementations copied):

- [IEEE 754-2019](https://standards.ieee.org/ieee/754/6210/) binary64 arithmetic and correctly
  rounded square root.
- [Euler characteristic](https://en.wikipedia.org/wiki/Euler_characteristic) of a closed polyhedral
  surface, and the
  [divergence theorem](https://en.wikipedia.org/wiki/Divergence_theorem) volume of a closed
  oriented triangle mesh, `V = (1/6) * sum(v0 . (v1 x v2))`.
- [UsdGeomMesh](https://openusd.org/release/api/class_usd_geom_mesh.html): `orientation`
  (`rightHanded` means counter-clockwise winding seen from the front, normals by the right-hand
  rule), `subdivisionScheme` and `faceVertexCounts`.
- [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259.html) for the logical JSON forms and
  [RFC 6234](https://www.rfc-editor.org/rfc/rfc6234.html) for SHA-256.

No commercial-product material, catalogue, third-party kernel code or test, or observation note
is used. Constants are proposed original BeamCraft rules, not vendor behaviour.

## 2. Kernel input

A kernel call takes one closed, finite, bounded `GeometryInput`. Unknown fields are rejected
(`UNSUPPORTED_PROPERTY`). It contains no element identity, label, appearance, camera, clock or
random seed: the result depends on this input only.

| Field | Rule |
|-------|------|
| `contract_version` | Exactly `0.1.0`; otherwise `UNSUPPORTED_VERSION` |
| `body` | Exactly one of the two bodies below |
| `placement` | `translation_mm` (3 numbers) and `orientation_xyzw` (4 numbers), F04 section 3 |
| `density_kg_m3` | Optional; a number or null. Absent or null means unknown (F04) |
| `outputs` | Nonempty set drawn from `mesh` and `quantities`, no duplicates |

**Plate** (P-1, assumed pending owner confirmation): `width_mm`, `length_mm`, `thickness_mm`.
The element-local solid is `[0, width] x [0, length] x [0, thickness]`: width along local +X
(the first edge, G-01), length along +Y, thickness along +Z. Origin corner at the local origin.

**Straight member** (P-2 and P-3, assumed pending owner confirmation): `axis_points_mm` (two
points `p1`, `p2` in element-local coordinates) and a `profile`. The first scope accepts the
profile kind `rectangle` with `width_mm` and `height_mm`; any other kind is unsupported
(section 3). The section is centred on the axis, with the width along `X_m` and the height along
`Y_m`; the end faces are perpendicular to the axis through `p1` and `p2` without extensions. The
frame `(X_m, Y_m, Z_m)` is built by the rule of section 4 (the roll rule of P-3).

**Profile resource (draft, F06/P02, to be validated by G04 and fixtures).** The kernel consumes
the logical content of a profile definition, not its file format. For `rectangle` that content
is the section kind, `width_mm`, `height_mm` and units `mm`, carried by an immutable definition
with a full content digest as F04 section 4 requires. The persisted encoding of that definition
is a USD/U02 decision. This contract defines no second persisted format and no catalogue.

**Caps (taken from F04, never widened; F06 additions marked).**

| Input | Rule | Origin |
|-------|------|--------|
| Plate dimensions, member axis length | greater than zero, at most 1,000,000 mm | F04 section 3 |
| Axis point coordinates, translation components | absolute value at most 1,000,000,000 mm | F04 section 3 |
| Density | greater than zero, at most 1,000,000 kg/m3, when present | F04 section 3 |
| Quaternion | components in [-1, 1]; squared norm within 1e-12 of one | F04 section 3 |
| Section `width_mm`, `height_mm` | at most 1,000,000 mm | F06 addition (draft) |
| Minimum nonzero dimension | at least 0.001 mm (section 4) | F06 addition (draft) |

F04's "greater than zero" is a lower limit that the minimum nonzero dimension raises; this is a
proposed F04 amendment (section 12). The input is fixed-size: at most 16 numbers (a member with its
placement and density), so no allocation depends on caller-controlled counts.

**Deterministic evaluation order.** A call runs these stages in order and stops at the first
stage that reports a failure; it reports all failures of that stage (at most 32) in input field
order:

1. **Support**: body kind, profile kind, transform form and mesh source (`unsupported`).
2. **Finiteness**: every f64 (`NON_FINITE_NUMBER`).
3. **Caps**: the table above (`LIMIT_EXCEEDED`).
4. **Ranges**: greater than zero, minimum dimension, axis length, density, quaternion
   (`INVALID_ARGUMENT`).
5. **Build** (derive frame, certify it, derive vertices, tables), **checks** (section 6
   self-checks) and **quantities** (section 8), with cancellation points between them (section 10).

Arithmetic uses binary64 with round-to-nearest-even, only `+ - * /` and `sqrt` (no
trigonometric or other library functions), no fused multiply-add contraction, and the operand
order written in this contract. Negative zero is normalized to positive zero in every output and
every key. No clock, random source, thread count, locale, hash-map iteration order or
environment variable influences a result. The same input and algorithm version give the same
bytes on every platform that follows IEEE 754 for these operations. This is a determinism rule,
not an accuracy promise; accuracy bounds are for G04 and fixtures.

**Order of evaluation for quantities and dot products.** Call the three box edge lengths `a`,
`b`, `c` (plate `width, length, thickness`; member `w, h, L`). Then `V_mm3 = (a*b)*c`,
`S_mm2 = 2*((a*b + a*c) + b*c)`, `V_m3 = V_mm3 / 1e9`, `S_m2 = S_mm2 / 1e6` (division by
constants that are exactly representable) and `mass = (V_mm3 * density) / 1e9`. A dot product
of three terms is `(r1*v1 + r2*v2) + r3*v3`; world coordinate `i` of a vertex `(x, y, z)` is
`(((R_i0*x + R_i1*y) + R_i2*z) + t_i)`. Bounds take the minimum and maximum of the eight computed
vertices with `<` and `>`, scanning vertex indices in ascending order.

**Thresholds, certification and conformance.** Every decision of this contract (caps, minimum
dimension, parallel test, status) is the written binary64 comparison on the binary64 values
defined above; there is no unspecified rounding band, and a value exactly at an inclusive limit
is accepted. Where a wrong decision would yield an invalid result rather than a different valid
one, the result carries a numeric certificate (for example the frame certificate of section 4,
rule 2): a build, mesh or query result whose certificate fails is `GEO_NUMERIC_UNCERTAIN`
(`UNSUPPORTED_GEOMETRY`) and is never returned as a partial or current result. The parallel test
needs no band because both of its branches give a certified frame on either side of the
threshold. The claim of identical bytes and decisions on every platform is a rule to be
demonstrated, not a verified property: G04 and the fixtures must supply conformance evidence on
each target, including `wasm32`. Any change that alters output bytes or a decision for some
admitted input raises `algorithm` and, with it, every key (section 7).

## 3. Result and diagnostics

Every kernel call returns one `GeometryResult`:

| Field | Rule |
|-------|------|
| `contract_version` | `0.1.0` |
| `status` | One of `ok`, `unsupported`, `invalid_input`, `limit_exceeded`, `cancelled` |
| `diagnostics` | Array of at most 32 records; empty exactly when `status` is `ok` |
| `versions` | `algorithm`, `tolerance_policy`, `feature_id_rule`, `mesh_layout` (section 7), each an integer |
| `shape_key` | Section 7; present only when `status` is `ok` |
| `mesh` | Descriptor and blob of section 6; present only when `ok` and requested |
| `quantities` | Records of section 8; present only when `ok` and requested |

**Diagnostic record.** Exactly `code` (a stable `GEO_` code below), `f04_code` (its F04
section 8 mapping), `field` (a fixed schema token of at most 64 ASCII bytes, never an input
path or value) and `stage` (`support`, `finiteness`, `caps`, `ranges`, `validation`, `build`,
`checks` or `quantities`). Messages use fixed English text and never echo input values, private
paths or exceptions. When a host forwards a record through F05 it uses `field`, a null
`target_id` and `reason_code = f04_code`; the `GEO_` code stays in the geometry result body.

**Status mapping.** `ok` has no code. `unsupported` maps to `UNSUPPORTED_GEOMETRY`.
`invalid_input` maps to `INVALID_ARGUMENT`, `NON_FINITE_NUMBER` or (blob structure only)
`INTEGRITY_ERROR`. `limit_exceeded` maps to `LIMIT_EXCEEDED`. `cancelled` maps to `CANCELLED`.
The overall status is the status of the first diagnostic.

| `GEO_` code | F04 code | Meaning |
|-------------|----------|---------|
| `GEO_NON_FINITE` | `NON_FINITE_NUMBER` | NaN or infinity in any number |
| `GEO_NOT_POSITIVE` | `INVALID_ARGUMENT` | Dimension, density or similar not greater than zero |
| `GEO_BELOW_MINIMUM` | `INVALID_ARGUMENT` | Dimension greater than zero but below the minimum |
| `GEO_ABOVE_LIMIT` | `LIMIT_EXCEEDED` | Input above its cap |
| `GEO_AXIS_TOO_SHORT` | `INVALID_ARGUMENT` | Axis length zero or below the minimum (G-03) |
| `GEO_QUATERNION_INVALID` | `INVALID_ARGUMENT` | Component range or norm rule violated |
| `GEO_UNSUPPORTED_PROFILE` | `UNSUPPORTED_GEOMETRY` | Profile kind other than `rectangle` |
| `GEO_UNSUPPORTED_TRANSFORM` | `UNSUPPORTED_GEOMETRY` | Scale, shear, reflection or a matrix form |
| `GEO_UNSUPPORTED_STAGE_FRAME` | `UNSUPPORTED_GEOMETRY` | Source unit or up axis other than mm and Z |
| `GEO_UNSUPPORTED_MESH_SOURCE` | `UNSUPPORTED_GEOMETRY` | Any imported mesh as kernel input |
| `GEO_BUFFER_LIMIT` | `LIMIT_EXCEEDED` | Vertex, triangle or byte cap exceeded |
| `GEO_TOPOLOGY_LIMIT` | `LIMIT_EXCEEDED` | Face, edge, shell or feature cap exceeded |
| `GEO_BLOB_INVALID` | `INTEGRITY_ERROR` | Blob structure disagrees with its header |
| `GEO_MESH_NOT_CLOSED` | `UNSUPPORTED_GEOMETRY` | Open shell: no exact status |
| `GEO_MESH_INWARD` | `UNSUPPORTED_GEOMETRY` | Closed but wound inward |
| `GEO_MESH_NOT_CERTIFIED` | `UNSUPPORTED_GEOMETRY` | No certificate of freedom from self-intersection |
| `GEO_SELF_CHECK_FAILED` | `UNSUPPORTED_GEOMETRY` | The kernel's own result failed section 6 checks |
| `GEO_NUMERIC_UNCERTAIN` | `UNSUPPORTED_GEOMETRY` | A numeric certificate failed (frame orthonormality, non-finite intermediate) in a build or query |
| `GEO_CANCELLED` | `CANCELLED` | Cancellation observed before the seal point |
| `GEO_FEATURE_NOT_FOUND` | `NOT_FOUND` | Well-formed FeatureId naming no feature |
| `GEO_FEATURE_ID_INVALID` | `INVALID_ARGUMENT` | Malformed FeatureId |
| `GEO_TRIANGLE_ADDRESS_FORBIDDEN` | `UNSUPPORTED_PROPERTY` | A triangle or vertex index used as an address |
| `GEO_STALE_TOKEN` | `STALE_WORKSPACE` | Snapshot token older than the retained history |
| `GEO_PICK_INPUT_INVALID` | `INVALID_ARGUMENT` | Bad ray or point query input |
| `GEO_PICK_LIMIT` | `LIMIT_EXCEEDED` | Hit count or radius above its cap |

The first 19 codes belong to the kernel and blob stages (`GEO_NON_FINITE` and
`GEO_NUMERIC_UNCERTAIN` also occur in queries); the last six to queries (section 9).
Every F04 code used here exists in F04 section 8.

**Failure never returns partial geometry as current.** A result whose status is not `ok` has no
`mesh`, `quantities` or `shape_key`. A failed or cancelled call changes nothing: the previous
current result of an element stays current and its generation does not advance (F04 section 8).
A mesh that fails self-checks or a numeric certificate is discarded, never displayed as
current. A mesh that is not an
analytic first-scope result may be shown by a viewport only if the host flags it
"not exact"; it never receives engineering quantities.

## 4. Tolerance and degeneracy policy (G-06, draft)

**Draft, to be validated by G04 and fixtures, not frozen.** All constants carry the integer
`tolerance_policy` version 1. Changing any constant raises that version and therefore every
shape key (section 7).

| Constant | Value | Use |
|----------|-------|-----|
| Minimum nonzero dimension `eps_dim` | 0.001 mm, inclusive | Plate dimensions, section dimensions, axis length |
| Parallel threshold `eps_par` | 1e-6, strict | The parallel test of P-3 |
| Coincidence | Two points coincide when their distance is below `eps_dim` | Axis endpoints only |

**Why these values.** 0.001 mm is far above the spacing of binary64 numbers at the largest
admitted coordinate (about 1.2e-7 mm at 1e9 mm), so a body of minimum size stays distinct from
its neighbours at every admitted placement. Subtracting two submitted f64 values gives one
rounding at most, so the axis direction suffers no cancellation beyond that. A threshold of
1e-6 keeps the amplification of rounding error in the normalized roll vector below about 1e-10
relative. These are reasons for a draft, not proofs; G04 validates them.

**Rules.**

1. A dimension (plate width/length/thickness, section width/height, axis length) of zero or less
   is `GEO_NOT_POSITIVE`; a value above zero and below `eps_dim` is `GEO_BELOW_MINIMUM`; a value
   equal to `eps_dim` is accepted. A zero or too-short axis is `GEO_AXIS_TOO_SHORT`
   (`INVALID_ARGUMENT`, as G-03 requires). The axis length is
   `L = sqrt((dx*dx + dy*dy) + dz*dz)` with `d = p2 - p1` per component.
2. **Roll rule and parallel test (P-3, assumed pending owner confirmation: P-3).** Let
   `d = p2 - p1`, `hh = dx*dx + dy*dy`, `LL = hh + dz*dz`, `L = sqrt(LL)`. The frame is built in
   this order:
   - a. `Z = (dx/L, dy/L, dz/L)`.
   - b. The first direction `Y`, by the first branch that applies. **Horizontal rule** when
     `hh >= 1e-12 * LL`: with `h = sqrt(hh)`,
     `Y = ((-dx*dz)/(h*L), (-dy*dz)/(h*L), hh/(h*L))`, which is local +Z projected onto the plane
     normal to `Z`. **Near-vertical rule** (also called the column rule) when `hh < 1e-12 * LL`
     (strict; anti-parallel counts; an exactly vertical axis is included): with `U = (0, 1, 0)`
     and `c = Z_y`, the vector `P = (-Z_x*c, 1 - Z_y*c, -Z_z*c)` is `U - Z*(U . Z)`, and
     `Y = P / |P|` with `|P| = sqrt((P_x*P_x + P_y*P_y) + P_z*P_z)`. An exactly vertical axis
     gives `c = 0` and `Y = (0, 1, 0)`, as P-3 states. For a tilted near-vertical axis `Y` stays
     perpendicular to `Z`; a fixed `(0, 1, 0)` would not be (it would leave the section off
     perpendicular by the tilt toward Y).
   - c. `X0 = Y x Z`, `X = X0 / |X0|`, then `Y = Z x X`. The cross product is
     `(a2*b3 - a3*b2, a3*b1 - a1*b3, a1*b2 - a2*b1)`. This applies to both rules and gives a
     right-handed frame (`X x Y = Z`).
   - d. **Frame certificate.** `| |X|*|X| - 1 |`, the same for `Y` and `Z`, the absolute values
     `|X . Y|`, `|X . Z|` and `|Y . Z|` of the three pairwise dot products, and
     `| (X x Y) . Z - 1 |` must each be at most 1e-12 (every bound is on an absolute value, so a
     negative error is rejected like a positive one); otherwise the build fails with
     `GEO_NUMERIC_UNCERTAIN` (`UNSUPPORTED_GEOMETRY`) and no partial result (`F06-NEG-31` injects
     `X . Y = +1e-9`, `F06-NEG-35` injects `X . Y = -1e-9`). In the near-vertical rule
     `|P|^2 = 1 - c^2 >= 1 - 1e-12`, so the normalization does not amplify rounding error.

   The decision `hh < 1e-12 * LL` is the written binary64 comparison (section 2). Cases keep a
   margin on both sides, and one case sits at a relative margin of 1e-12 (`F06-POS-09`,
   `F06-POS-10`, `F06-POS-46`). Y-tilted near-vertical axes are `F06-POS-41` and `F06-POS-42`.
3. The roll is generally discontinuous at the threshold by construction; the jump depends on the
   tilt direction. Compare the two branch frames on the **same nonvertical unit axis** `Z`
   near the threshold, rotating from the near-vertical frame to the horizontal frame by the
   right-hand rule about `Z`. In exact arithmetic, let `r = sqrt(Z_x^2 + Z_y^2)` and
   `n = sqrt(1 - Z_y^2)`. Then
   `cos(theta) = -Z_y*Z_z/(r*n)` and `sin(theta) = Z_x/(r*n)`. For tilts from local +Z:

   | Tilt toward | Exact branch rotation |
   |-------------|-----------------------|
   | +Y | 180 degrees (half turn) |
   | -Y | 0 degrees (no roll jump on this meridian) |
   | +X | +90 degrees (quarter turn) |
   | -X | -90 degrees (quarter turn) |

   These angles hold on either side of the threshold and give the one-sided limiting roll
   differences there. Frames on distinct finite axes also differ in axis direction, so their
   relative three-dimensional rotation is not solely a roll. For tilts from local -Z, the +Y
   and -Y entries exchange; the +X and -X entries keep their signs. Other tilt directions follow
   the formula above. Continuity stays an open owner decision (section 12).
   Anyone who needs another roll expresses it through the element placement (P-3). F04's text
   of P-3 names a fixed `+Y` for the column case; for a tilted near-vertical axis that is not
   perpendicular to the axis, so F04 P-3 needs a joint amendment (section 12).
4. The kernel never snaps, welds or merges vertices, and never heals geometry. Vertices are
   distinct by construction, not by a tolerance test, and every box edge is at least `eps_dim`
   long after the range checks.
5. **Quaternion handling (F04 section 3 left this to F06).** The accepted quaternion is
   normalized: `n = ((x*x + y*y) + z*z) + w*w`, `r = sqrt(n)`, each component divided by `r`.
   The rotation matrix uses only products and sums of those components: `R00 = 1 - 2*(y*y + z*z)`,
   `R01 = 2*(x*y - z*w)`, `R02 = 2*(x*z + y*w)`, `R10 = 2*(x*y + z*w)`,
   `R11 = 1 - 2*(x*x + z*z)`, `R12 = 2*(y*z - x*w)`, `R20 = 2*(x*z - y*w)`,
   `R21 = 2*(y*z + x*w)`, `R22 = 1 - 2*(x*x + y*y)`. The matrix is quadratic in the components, so
   `q` and `-q` give bit-identical matrices and meshes. **Canonical sign for keys:** after
   normalization negate all four components when `w < 0`, or when `w = 0` and the first nonzero
   of `x, y, z` is negative; then normalize negative zero. Scale, shear and reflection never
   reach this step (they are unsupported).
6. **Tolerances in signatures.** The first scope has no caller-supplied tolerance. The
   `tolerance_policy` version is an input of the shape key and the world-measurement key
   (section 7). A policy change is never silent and never leaves a current result keyed by the
   old policy.
7. **Separate from test comparison.** The comparison rule CR-TOL-001 (F04 section 10, G-05) is a
   rule for comparing test results only. The kernel never reads it, no signature contains it,
   and the geometric constants above never appear in a test comparison. A test that needs
   "equal" for a decision (a branch, a status, a FeatureId) compares exactly.

## 5. FeatureId

A FeatureId is a stable semantic name for a face, edge or vertex of a first-scope body. It
replaces triangle, vertex and buffer indices as the only address that persists outside a mesh
buffer.

**Form (rule version `fid1`, assumed pending owner confirmation: P-1, P-2).** A qualified
FeatureId is `fid1:` + ElementId + `:` + local id, with the ElementId in its canonical lowercase
36-character form (F04 section 2). The local id is `kind:token`, where `kind` is `plate` or
`member`, and `token` is one of:

- face: `f.` + axis role, where the role is one of `x0`, `x1`, `y0`, `y1`, `z0`, `z1`;
- edge: `e.` + two roles of different axes in axis order (`x` before `y` before `z`), for example
  `e.x0.y1`;
- vertex: `v.` + three roles, one per axis, in axis order, for example `v.x0.y1.z1`.

The axes are those of the body frame: for a plate, local X, Y, Z (width, length, thickness; the
digit 0 is the minimum side and 1 the maximum side); for a member, `X_m`, `Y_m`, `Z_m`, so
`z0` is the end face at `p1` and `z1` the end face at `p2`. A body has 6 faces, 12 edges and 8
vertices: 26 features. Allowed characters are `a-z`, `0-9`, `:`, `.` and `-`. The longest form,
a member vertex, is 59 bytes; the maximum length is **64 bytes**. The kernel produces local ids;
the engine prepends `fid1:` and the ElementId.

**Rules.**

1. **Independent of mesh order.** A FeatureId is built only from the rule version, the
   ElementId, the body kind and role tokens. It does not depend on dimensions, placement, vertex
   order, triangle order, buffer offsets or the platform.
2. **Uniqueness.** All FeatureIds of one result are pairwise different, and a FeatureId contains
   its owning element's identity, so two elements never share one.
3. **Rebuild with unchanged geometry** gives identical FeatureIds and identical mesh bytes.
4. **Edits that keep the topology** (dimension, placement, density, rename) keep every
   FeatureId. A placement or dimension change moves the feature, not its name.
5. **Copy.** A copy has a new ElementId (F04 `CopyElement`), so all its FeatureIds are new;
   the local parts are the same, and the shape key and element-local mesh bytes are shared until
   an edit. The source's ids never change.
6. **Axis reversal.** Swapping `p1` and `p2` swaps the physical faces that carry `z0` and `z1`
   (and `x0`/`x1`, because `X_m` changes sign): ids follow roles, not world positions. A
   reference to a physical face is therefore tied to the frame rule and survives only as long as
   that rule gives the same role.
7. **Rule changes.** A change of the naming rule raises the version tag (`fid2`). Old references
   are migrated only by an explicit reviewed step, never reinterpreted silently.
8. **Never a triangle index.** A drawing, dimension, annotation or any stored reference
   addresses geometry by `(ElementId, local id, rule version)` only. Triangle, vertex and buffer
   indices are valid only inside one mesh snapshot and are never stored, returned by pick
   results, or accepted by a lookup (`GEO_TRIANGLE_ADDRESS_FORBIDDEN`). A reference whose
   FeatureId no longer resolves is dangling and reported as `NOT_FOUND`, not repaired by index.
9. **Mapping inside a snapshot.** The face table (section 6) maps a triangle to the local id of
   its face; vertex index `i = xb + 2*yb + 4*zb` maps to the vertex role bits. Edge geometry
   comes from `LookupFeature` (section 9), not from the buffer.

## 6. Mesh and buffers

**Layout rules.** Positions are binary64 millimetres. Indices are u32. The mesh is a triangle
mesh whose winding is counter-clockwise seen from outside, so face normals computed by the
right-hand rule point out of the body. This matches
[UsdGeomMesh](https://openusd.org/release/api/class_usd_geom_mesh.html)
`orientation = rightHanded`; a USD adapter must author `orientation` as `rightHanded`,
`subdivisionScheme` as `none` and every `faceVertexCounts` entry as 3 (the default scheme
subdivides, so it is never left implicit). A proper rotation preserves winding; reflections are
unsupported.

**Closed-shell requirement.** A mesh may receive volume and area quantities only if all hold:
every vertex index is in range; every triangle has three distinct indices and nonzero area;
every undirected edge belongs to exactly two triangles, used once in each direction; the signed
volume `(1/6) * sum(v0 . (v1 x v2))` is positive (outward). For one genus-0 shell
`V - E + F = 2`; for `k` such shells it is `2k`. Euler's formula alone is not a closure test
(`F06-NEG-18` has `V - E + F = 1`, `F06-NEG-19` has 4 for two shells, and an inward mesh has 2).
The first-scope analytic builders also self-check this before sealing (`GEO_SELF_CHECK_FAILED`).

**Canonical box mesh (assumed pending owner confirmation: P-1, P-2, P-3).** Both bodies are
boxes in a right-handed body frame with origin corner `o` and edge vectors `e1, e2, e3`
(`e1 x e2` along `e3`): plate `o = (0, 0, 0)`, `e1 = (width, 0, 0)`, `e2 = (0, length, 0)`,
`e3 = (0, 0, thickness)`; member
`o = (p1 - (w/2)*X_m) - (h/2)*Y_m`, `e1 = w*X_m`, `e2 = h*Y_m`, `e3 = p2 - p1`. Vertex `i`
(`xb = i & 1`, `yb = (i >> 1) & 1`, `zb = i >> 2`) is `((o + xb*e1) + yb*e2) + zb*e3`
component by component. The 6 faces, in table order, are quads of vertex indices wound
counter-clockwise from outside:

| Face index | Local id token | Quad |
|------------|----------------|------|
| 0 | `f.x0` | 0, 4, 6, 2 |
| 1 | `f.x1` | 1, 3, 7, 5 |
| 2 | `f.y0` | 0, 1, 5, 4 |
| 3 | `f.y1` | 2, 6, 7, 3 |
| 4 | `f.z0` | 0, 2, 3, 1 |
| 5 | `f.z1` | 4, 5, 7, 6 |

Quad `[a, b, c, d]` gives the triangles `(a, b, c)` and `(a, c, d)`; triangle `2*f` and `2*f + 1`
belong to face `f`. Counts: 8 vertices, 12 triangles, 6 faces, 12 topological edges, 18
triangle-mesh edges (12 box edges and 6 face diagonals), 1 shell; `8 - 18 + 12 = 2` and
`8 - 12 + 6 = 2` (`F06-POS-01`).

**Mesh blob.** One body's mesh is one blob. All integers are **little-endian**, all positions
are little-endian binary64 with negative zero normalized. (F05's frame and prefix fields are
big-endian; they are transport, and this blob is the opaque payload.)

| Offset | Size (bytes) | Content |
|--------|--------------|---------|
| 0 | 8 | magic, ASCII `BCMESH01` |
| 8 | 4 | layout version, `1` |
| 12 | 4 | vertex count `V` |
| 16 | 4 | triangle count `T` |
| 20 | 4 | face table entry count `K` |
| 24 | 4 | space: 0 element-local, 1 world |
| 28 | 4 | reserved, zero |
| 32 | `24*V` | positions `x, y, z` per vertex |
| next | `pad8(12*T)` | indices, 3 u32 per triangle, zero padding to a multiple of 8 |
| next | `pad8(2*T)` | per-triangle face index (u16, an index into the face table), zero padding |

`pad8(n)` rounds `n` up to a multiple of 8. The total is `32 + 24*V + pad8(12*T) + pad8(2*T)`;
the first-scope body is 392 bytes (`F06-POS-02`). Section starts are multiples of 8 relative to
the blob start, but an attachment's start in memory is not aligned (F05 prefixes 44 bytes), so
consumers read with unaligned loads or copy. The face table (an ordered array of at most `K`
local ids) travels in the JSON descriptor, not in the blob. The encoded descriptor of one body
(face table and fields) is at most 32,768 bytes of UTF-8; a page's control object also obeys
the negotiated F05 `control_bytes`, `json_tokens` and `string_bytes`, and a page ends when any
of its budgets would be exceeded (section 10).

**Primitive limits and general validator limits.** The first-scope builders (plate and member)
always produce exactly 8 vertices, 12 triangles, 6 faces, 12 topological edges, 18
triangle-mesh edges, 1 shell and 26 features. These **primitive limits** are constants of the
builders, not caps. The **hard caps** below are the general structural-validator limits for any
blob (including a future body or a blob received from outside); a blob inside them is
structurally valid, not thereby an engineering result (section 8).

**Hard caps (draft).**

| Item | Cap |
|------|-----|
| Vertices per body | 4,096 |
| Triangles per body | 8,192 |
| Faces per body (topological) | 1,024 |
| Topological edges per body | 12,288 |
| Shells per body | 16 (the first scope produces 1) |
| Features per body | 17,408 (4,096 + 12,288 + 1,024) |
| Mesh blob size | 213,024 bytes (the value of the formula at the three caps) |

A validator checks the three counts against their caps before computing any size, computes
sizes with checked 64-bit arithmetic, compares the supplied length with the computed length
and only then reads sections. Zero counts, a wrong magic, an unknown layout version, a nonzero
reserved field, non-finite or negative-zero coordinates, indices out of range and a face index
outside the table are `GEO_BLOB_INVALID`. Counts above a cap are `GEO_BUFFER_LIMIT` (vertices,
triangles, bytes) or `GEO_TOPOLOGY_LIMIT` (faces, edges, shells, features). Exactly at a cap is
accepted (`F06-POS-27`); one above is rejected (`F06-NEG-11` to `F06-NEG-14`).

**Chunking over F05.** A mesh blob is **never split**: it is one attachment, whole. A page of a
snapshot or change delivery holds whole blobs, at most the negotiated `effective_limits`
`attachments` per bundle, each within `buffer_bytes`, within `bundle_bytes`, and at most
`batch_items` items per page and `delivery_pages` pages per delivery (F05 sections 4, 5 and 10;
the numbers are those of F05 and are not restated here). Because a blob is at most 213,024
bytes, it fits any F05 maximum. If a negotiated limit is smaller than a body needs, the body is
not delivered and the query fails with `GEO_BUFFER_LIMIT`; it is never fragmented across
attachments. `F06-POS-28` shows the page arithmetic. Transport element type stays `u8`; the blob
is self-describing, so F05's open question on element-type negotiation (F05 section 12) needs no
change.

**Digest rule.** The `mesh_digest` is the SHA-256 (RFC 6234) of the exact blob bytes, as 64
lowercase hexadecimal digits. For a blob sent as one F05 attachment it equals that attachment's
`sha256`. The digest identifies bytes; it is not the shape key (F04 section 5). The same shape
key, space and placement give the same bytes and the same digest (`F06-POS-02`). A world blob
differs from the local blob and has its own digest.

**Element-local versus world space.** The kernel's primary mesh is element-local (space 0),
shared by every element with the same shape key. A world mesh (space 1) is a derived result:
`world = R*local + t` per vertex (section 7). A viewport should keep the local mesh and apply
the element transform when drawing; the transform application for drawing is presentation, while
world bounds, measurements and picks come from the engine.

## 7. Transforms and measurement keys

**Spaces and transforms (rigid only in the first scope).**

| Space | Definition |
|-------|------------|
| Body | The box frame of section 6 where the body is axis-aligned from `o` |
| Element-local | F04 element-local coordinates; kernel meshes are in this space. Plate: identical to the body frame. Member: `p_local = o + B * p_body`, with `B = [X_m Y_m Z_m]` |
| World | `p_world = R(q) * p_local + translation_mm` (rotate, then translate; F04 section 3) |
| Unit | Lengths in mm, areas mm2, volumes mm3 internally; reports use m2 and m3 by division by 1e6 and 1e9 (section 2) |

The transform is a rigid pose and nothing else: a unit quaternion and a translation, with
determinant +1. Any matrix, scale, shear or reflection is `GEO_UNSUPPORTED_TRANSFORM`. A parent
transform is outside this contract (F04). For USD, F04 authors `metersPerUnit = 0.001` and
`upAxis = Z`, so native results are millimetres and Z up and the stage conversion is the
identity. **Imported meshes with another `metersPerUnit` or `upAxis` are unsupported until the
R-tasks** (`GEO_UNSUPPORTED_STAGE_FRAME`); in 0.1.0 no imported mesh is accepted as kernel input
at all (`GEO_UNSUPPORTED_MESH_SOURCE`), and the kernel never guesses a conversion.

**Canonical numeric encoding for keys (draft; F04 left it to F06/P04).** Key preimages are
byte strings: ASCII tags, unsigned integers big-endian, and every f64 as 8 bytes big-endian
binary64 after normalizing negative zero. NaN and infinity never occur (rejected earlier).
Decimal text, locale and JSON formatting are never part of a preimage. Keys are SHA-256 digests
shown as 64 lowercase hexadecimal digits; they are not content digests of stored bytes.

**Three separate dependency keys.** Each has its own input set; the sets of the shape key, the
world-measurement key and the presentation key do not overlap.

1. **Shape key**: the element-local geometry. Preimage: `BCGEO-SHAPE` + `0x00`, a body kind
   byte (1 plate, 2 member), then for a plate the three f64 `width, length, thickness`; for a
   member the six f64 of `p1, p2`, a section kind byte (1 rectangle) and the two f64 `width,
   height`; then four u16 big-endian: `algorithm`, `tolerance_policy`, `feature_id_rule`,
   `mesh_layout` (all 1 in 0.1.0). It excludes placement, density, material, label, appearance,
   camera, ElementId and the profile resource digest (two profiles with the same dimensions give
   the same geometry; the digest stays in the F04 footprint instead).
2. **World-measurement key**: everything measured in world space. Preimage: `BCGEO-WORLD` +
   `0x00`, the 32 raw bytes of the shape key, the three f64 of `translation_mm`, the four f64 of
   the canonical quaternion (section 4), then u16 `algorithm` and `tolerance_policy`. It
   excludes density, label, appearance and camera.
3. **Presentation key**: how an element is shown. Inputs: the resolved display appearance
   reference (digest), visibility and the camera description of the view. It contains no shape,
   placement or physical-material input. It is computed by the presentation layer (V01 fixes its
   preimage) and never by the kernel. Selection and hover are viewport state outside all three
   keys and are never engineering input.

**Mass signature.** Mass is a quantity with its own input signature (F04 section 5), not a
fourth key: SHA-256 of `BCGEO-MASS` + `0x00`, the shape key, a presence byte (0 or 1) and the f64
density (0 when absent), then u16 `algorithm`.

**Invalidation matrix.** "Inval." marks a changed key or signature; "-" marks an unchanged one.

| Change | Shape key | World key | Presentation key | Mass signature | Local mesh bytes | World bounds |
|--------|-----------|-----------|------------------|----------------|------------------|--------------|
| Rename | - | - | - | - | - | - |
| Camera change | - | - | Inval. | - | - | - |
| Selection, hover | - | - | - | - | - | - |
| Visibility or appearance change | - | - | Inval. | - | - | - |
| Density value change | - | - | - | Inval. | - | - |
| Placement change | - | Inval. | - | - | - | Inval. |
| Dimension, axis or profile-dimension change | Inval. | Inval. | - | Inval. | Inval. | Inval. |
| Copy (new ElementId, same inputs) | - | - | - | - | - | - |
| Algorithm or tolerance-policy version change | Inval. | Inval. | - | Inval. | Inval. | Inval. |

Reading the rows: a rename or a camera change does not invalidate shape (F04 section 5);
a density change invalidates mass only; a placement change invalidates world measurements but
not shape (`F06-POS-19` to `F06-POS-24`). A copy has the same keys as its source; only its
FeatureIds differ (`F06-POS-26`). Changing the material reference to a definition with the same
density and the same resolved display appearance changes nothing visible; changing only the
density value changes the mass signature and not the presentation key, because the presentation
key holds the resolved display reference, not the material. F04's wording "material-dependent
appearance" is read this way (a proposed clarification, section 12).

**Quantity signatures.** Volume, surface area, member length and local bounds carry the shape
key. World bounds carry the world-measurement key. Mass carries the mass signature. A quantity
is `current` only if its signature matches the key computed from the current authored inputs.

## 8. Quantities

Each quantity record has value (if any), unit, status, input signature and algorithm version
(F04 section 5). Status values are exactly F04's: `current`, `stale`, `unknown`, `unsupported`.

| Quantity | Unit | Definition for the first scope | Required inputs |
|----------|------|--------------------------------|-----------------|
| Volume | m3 (mm3 internally) | Plate `w*l*t`; member `w*h*L` where `L` is the axis length | Valid shape |
| Surface area | m2 (mm2 internally) | `2*(a*b + a*c + b*c)` over the three box edge lengths | Valid shape |
| Local bounds | mm | Minimum and maximum of the eight vertices in element-local space (P-1: plate `[0,w] x [0,l] x [0,t]`) | Valid shape |
| World bounds | mm | Minimum and maximum of the eight vertices after the rigid transform | Shape and placement |
| Member length | mm | `L = sqrt((dx*dx + dy*dy) + dz*dz)` of `d = p2 - p1` (members only) | Valid shape |
| Mass | kg | Volume in m3 times density in kg/m3 | Current volume and an explicit density |

The definitions of the member bounds and vertices follow P-2 and P-3 and are assumed pending
owner confirmation; the plate bounds follow P-1 likewise.

**Status rules.**

- `current`: computed analytically by the first-scope builders from a sealed build whose
  signature matches the current authored inputs.
- `stale`: authored inputs changed and the rebuild is not complete; the previous value may be
  retained, clearly marked, and is never presented as current (`F06-POS-38`).
- `unknown`: a required input is absent or unavailable. **Unknown density keeps mass unknown**
  even when volume is current; density is never defaulted, guessed from a catalogue or taken as
  zero (`F06-POS-03`, `F06-POS-13`). A dependency that cannot be read gives `unknown`
  (`DEPENDENCY_UNAVAILABLE` at the command level).
- `unsupported`: geometry outside the scope (a curved profile) or a mesh that is open, wound
  inward or not certified free of self-intersection (`F06-NEG-18` to `F06-NEG-20`). No value.
- Precedence for mass: `unsupported` if volume is unsupported; else `unknown` if density is
  absent; else `stale` if volume or density is stale; else `current`. Mass of an unsupported
  body is `unsupported` even when density is absent, because `unsupported` describes the
  geometry and supplying a density could never make the mass current; this keeps F04's
  statuses unchanged and is why `F06-NEG-18` expects mass `unsupported` (corrected in the
  2026-10-15 amendment).

**Bounds are not volume.** A bounding box is never reported as volume, mass or any exact
quantity, even when it happens to have the same size as an axis-aligned body.

**Open or self-intersecting meshes never receive exact status.** In 0.1.0 every engineering
quantity comes from the analytic builders. A blob from any other source gets no engineering
quantity at all, whether or not it passes structural validation; there is no certifier of
freedom from self-intersection (`F06-NEG-19`: two closed boxes overlapping give a divergence sum
of 2000 mm3 for a union of 1500 mm3, so the sum is never reported).

## 9. Query API (logical)

All queries are read-only and bounded. They name no transport, no USD path and no file path.
A query is pinned to one project, workspace, base revision and workspace generation (the F05
`context` tuple) and returns a `snapshot_token`: an opaque string of at most the F05
`cursor_bytes` limit, compared for equality only (ordering and structure are never inferred). A
result is answered at exactly one pinned revision; data from two revisions is never mixed. An
unknown, expired or older-than-retained token is `GEO_STALE_TOKEN` (`STALE_WORKSPACE`). A
successful result is `ok`; a failed one carries an F04 code and bounded diagnostics (section 3)
with no partial data. Request schemas are closed (unknown field: `UNSUPPORTED_PROPERTY`).

| Query | Request fields | Result |
|-------|----------------|--------|
| `QueryBounds` | `scope` (`element` with an ElementId, or `all`), `space` (`local` for one element or `world`) | Bounds with status and key; with scope `all`, paged like `ListElements` |
| `ListElements` | `cursor` (null or string), `page_size` | Elements in ascending canonical ElementId order: id, kind, shape key, world key, presentation key, mesh digest, counts, blob size |
| `GetMeshSnapshot` | `cursor`, `space` | Pages of whole blobs plus descriptors with face tables, and the `snapshot_token` |
| `LookupFeature` | `feature_id` | Feature kind, owning ElementId, role, geometry in local and world space (face: outward unit normal and four vertex ids; edge: two vertex ids; vertex: position) |
| `Pick` | Ray form or point form below | Ordered hits |
| `GetChanges` | `since_token` | Changed, added and removed elements with their keys; blobs for those whose shape key changed; new token; or `resnapshot_required` |

**Pick contract.**

- *Ray form:* `origin_mm` (3 finite numbers, absolute value at most 4,000,000,000), `direction`
  (3 finite numbers, absolute value at most 1,000,000,000, Euclidean norm at least 1e-9;
  normalized by the engine), `t_max_mm` (default and maximum 4,000,000,000), `targets` (a nonempty
  subset of `face`, `edge`, `vertex`), `radius_mm` (default 0, maximum 1,000,000), `max_hits`
  (default 16, 1 to 64) and `include_backfaces` (default false). The bound 4e9 covers every world
  coordinate of an admitted body: a local point has magnitude at most about 1.8e9 and the
  translation components are at most 1e9, so each world component stays below 3.5e9.
- *Point form:* `point_mm` (as `origin_mm`), `radius_mm`, `targets`, `max_hits`.
- *Face hits (ray):* faces are closed rectangles in their planes, enlarged by the pick band
  `eps_pick = 1e-6 mm` (one nanometre, draft) in each in-plane direction. With the unit ray
  direction `u` and the outward unit normal `n`, a face is **parallel** to the ray, and not
  hit, when `|u . n| < 1e-12`. Otherwise `t = ((p0 - o) . n) / (u . n)` for a point `p0` of the
  face and the ray origin `o`; the face is hit when `0 <= t <= t_max` and the in-plane
  coordinates of the hit point lie in the enlarged rectangle. A face is front-facing when
  `u . n < 0`; back-facing faces are skipped unless `include_backfaces` is true. A hit that lies
  on the exact rectangle's boundary or in its enlargement has `boundary` true, and a hit on a
  shared edge is reported for every non-parallel face that qualifies (`F06-POS-43`,
  `F06-POS-44`). A non-finite intermediate value is `GEO_NUMERIC_UNCERTAIN`, with no partial hits.
- *Edge and vertex hits:* the distance between the ray (or point) and the closed segment (or
  the point), accepted when at most `radius_mm + eps_pick`; for a ray, `t` is the ray parameter
  of the closest point, and if the minimum is attained on an interval the smallest such `t`.
  Zero radius is therefore exact up to the band: a feature touching the ray (distance 0 up to
  rounding) is accepted with radius 0 (`F06-POS-32`).
- *Point form distances:* the distance to a face is the Euclidean distance to the nearest point
  of the closed face rectangle (the foot of the perpendicular when it lies inside, otherwise a
  point of the rectangle's boundary); a point inside the solid still measures to surfaces, never
  zero. A face is accepted when that distance is at most `radius_mm + eps_pick`, front or back
  (`F06-POS-33`, `F06-POS-45`).
- *Hit record:* `element_id`, `feature_id`, `feature_kind`, `t_mm` (ray form), `distance_mm`
  (distance to the feature; 0 for a ray hit on a face), `point_world_mm` (the hit or closest
  point), `normal_world` (outward unit normal for a face, null otherwise), `back_facing` and
  `boundary` (faces; false otherwise). A hit never contains a triangle, vertex or buffer
  index.
- *Ordering and tie-breaking (draft, to be validated by G04):* sort ascending by the tuple
  (`k`, ElementId, kind rank, local id, `t_mm`), where `k = floor(t_mm / 1e-6)` for the ray form
  (`floor(distance_mm / 1e-6)` for the point form), the ElementId and local id compare as ASCII
  strings and the kind ranks are face 0, edge 1, vertex 2. Quantizing first makes the order a pure
  function of the inputs; hits whose `t` differs by less than one nanometre (`1e-6 mm`, the same
  quantum as `eps_pick`) may therefore be ordered by identity instead of by `t`. At most
  `max_hits` records are returned; if more qualify, the first `max_hits` of this order are
  returned and a `truncated` flag is true.
- *Tolerance:* `radius_mm` is the caller's query parameter. It is not the geometric tolerance of
  section 4 and not the test comparison rule CR-TOL-001. A viewport that converts pixels to a
  radius does so before calling; the engine uses the number it receives.
- *Limits:* `max_hits` above 64 or `radius_mm` above 1,000,000 is `GEO_PICK_LIMIT`; a zero or
  too-short direction, an empty or duplicated `targets`, a negative radius or `t_max_mm` is
  `GEO_PICK_INPUT_INVALID`; a non-finite number is `GEO_NON_FINITE`.
- *Work and deadline (draft):* a `Pick` considers at most 65,536 elements and at most 26
  features per element; a model with more elements fails before the scan with `GEO_PICK_LIMIT`
  and no partial hits (`F06-NEG-33`). The scan visits elements in ascending ElementId order and
  polls the cancellation token and the F05 request deadline every 1,024 elements. A cancelled
  query returns `CANCELLED` and a missed deadline follows F05 section 8; neither returns partial
  hits. Only a completed scan returns hits.

**Incremental update.** `GetChanges` answers from a bounded retained history. It lists at most
the F05 `batch_items` entries per page and at most `delivery_pages` pages; an entry reports the
element, the kind of change (`added`, `changed`, `removed`), the three keys and, only when the
shape key changed, the new blob. A placement-only edit therefore resends no mesh (`F06-POS-34`);
a dimension edit resends the blob (`F06-POS-35`). A token the engine cannot answer gives the
outcome `resnapshot_required` with `STALE_WORKSPACE` and no partial list (`F06-POS-36`); the
consumer then reads a fresh snapshot, as F05's invalidation event requires. A delivery that
would exceed the F05 delivery caps fails before dispatch instead of being truncated silently.

**The renderer never computes engineering geometry.** Bounds, quantities, normals used for
measurement, feature geometry, pick results and FeatureIds come from the engine. A viewport may
upload meshes, apply the element transform for drawing, compute display-only shading normals
from the supplied mesh and run a display-only GPU pick as a hint. Anything that selects an
element or feature for engineering use is resolved by the engine's `Pick` or `LookupFeature`;
a display hint is mapped to a FeatureId only through the snapshot's face table.

**Headless surface (logical only).** Every query has an engine API first and a CLI command with
the same fields and the same logical result: build and mesh, quantities, bounds, element list,
feature lookup, pick and changes. The CLI adds no computation and no private path to geometry
(`AGENTS.md` section 3; `F06-POS-39`). How the CLI delivers mesh bytes, for instance to a file,
is an I/O-edge decision of the CLI task; it does not alter the logical result.

## 10. Topology and bounds limits

| Item | Rule |
|------|------|
| Counts | Vertices, triangles, faces, topological edges, shells and features are capped by the table in section 6 |
| Arithmetic | Counts are compared with their caps before any product; sizes use checked 64-bit arithmetic; any overflow is `GEO_BUFFER_LIMIT` |
| Allocation | Sized only by validated input: the first-scope builders allocate for exactly 8 vertices, 12 triangles and 6 faces; a blob validator allocates only after the length check |
| Recursion | None in the first scope. A later traversal must state its own depth bound, at most 16 (the depth used elsewhere in F04 and F05), in an amendment |
| Input size | At most 16 numbers per kernel call (section 2) |
| Primitive versus validator limits | Primitive counts are exact constants (section 6); the hard caps bound any blob a validator accepts |
| Descriptor bytes | At most 32,768 encoded bytes per body descriptor; the page budget also obeys F05 `control_bytes`, `json_tokens` and `string_bytes` |
| Query output and work | At most 64 hits per pick; at most 65,536 elements and 26 features per element considered; cancellation and deadline polled every 1,024 elements; page and delivery bounds from F05 |
| Negotiated limits | Each cap is a maximum; where F05 `effective_limits` (`buffer_bytes`, `attachments`, `bundle_bytes`, `control_bytes`, `json_tokens`, `string_bytes`, `batch_items`, `delivery_pages`, `request_timeout_ms`) is smaller, the smaller value applies. A body that cannot be delivered whole under it fails with `GEO_BUFFER_LIMIT`; it is never split or truncated (`F06-NEG-34`). F06 never widens a negotiated or F04 limit |

**Cancellation points and deterministic result.** The kernel polls a host-supplied cancellation
token at these points only: P0 on entry, P1 before the build, P2 before the checks and P3 before
the quantities. The seal point follows the quantities; a result is sealed there. A request
observed at P0 reports stage `validation`; at P1 stage `build`; at P2 stage `checks`; at P3
stage `quantities` (the stage that would have run next). Work between two points is bounded by
a fixed number of operations, so cancellation latency is bounded. A cancellation observed at or
before P3 gives status `cancelled`, `GEO_CANCELLED`, and no mesh, quantities or key: the partial
work is discarded. A cancellation after the seal point is a no-op and returns the complete result
(`F06-POS-37`). For a fixed input and a fixed observing point the outcome is the same every
time (`F06-NEG-16`, `F06-NEG-17`). Cancellation never changes engineering state; F05 section 8
still decides what the host may report: `CANCELLED` only when the discard is proven.

## 11. Acceptance cases

The [case file](geometry-contract-cases-v0.1.json) holds 81 logical cases: 46 positive
(`F06-POS-01` to `F06-POS-46`) and 35 negative (`F06-NEG-01` to `F06-NEG-35`). They are
proposed future tests: acceptance needs a real kernel, real buffers and independent readers.
Numeric expectations use exact forms or decimal approximations checked with rational arithmetic
and high-precision square roots; round-3 checks use a throwaway script under ignored `target/`.
The derivation text is in each case. Numeric comparison in tests follows CR-TOL-001 (F04 section
10); statuses, codes, counts, FeatureIds and digests compare exactly. Cases that depend on P-1 to
P-3 list those proposals in `assumes`.
Existing F04 and first-scope IDs are referenced, not duplicated, and no existing ID is changed.

| Topic | F06 cases | Existing cases referenced |
|-------|-----------|---------------------------|
| Plate and member meshes: counts, Euler, closed shell, orientation | `F06-POS-01`, `F06-POS-04` to `F06-POS-08` | `FS-PLATE-A`, `FS-MEMBER-A`, `F04-POS-14` to `F04-POS-16` |
| Blob bytes and digest | `F06-POS-02` | none |
| Quantities, mass, unknown density | `F06-POS-03`, `F06-POS-13`, `F06-POS-38` | `F04-POS-01`, `FS-PLATE-A-MASS` |
| Rotated placement | `F06-POS-11`, `F06-POS-12` | `F04-POS-10`, `FS-POSE-A` |
| FeatureId stability, uniqueness, grammar | `F06-POS-16` to `F06-POS-18` | `F04-POS-03` |
| Three keys and invalidation matrix | `F06-POS-19` to `F06-POS-26` | `F04-POS-02`, `F04-POS-06` |
| Cap boundaries, exactly at limit accepted | `F06-POS-14`, `F06-POS-15`, `F06-POS-27` | `F04-POS-08`, `F04-POS-13`, `FS-PLATE-C`, `FS-MEMBER-B` |
| Cap boundaries, just above rejected | `F06-NEG-06`, `F06-NEG-11` to `F06-NEG-14` | `F04-NEG-19`, `F04-NEG-21`, `FS-NEG-PLATE-BOUND`, `FS-NEG-AXIS-BOUND` |
| Degenerate axis, minimum dimension | `F06-NEG-03` to `F06-NEG-05` | `F04-NEG-20`, `FS-NEG-AXIS-ZERO` |
| Parallel-up boundary, near-vertical frame, frame certificate | `F06-POS-05`, `F06-POS-06`, `F06-POS-09`, `F06-POS-10`, `F06-POS-41`, `F06-POS-42`, `F06-POS-46`, `F06-NEG-31`, `F06-NEG-35` | `F04-POS-16` |
| Non-finite input | `F06-NEG-01`, `F06-NEG-02`, `F06-NEG-27` | `F04-RAW-01`, `F04-RAW-02`, `F04-RAW-04` to `F04-RAW-06` |
| Buffer cap exceeded, invalid blob | `F06-NEG-11` to `F06-NEG-15`, `F06-NEG-30` | none |
| Cancellation | `F06-POS-37`, `F06-NEG-16`, `F06-NEG-17` | none |
| Unsupported profile, transform, stage frame | `F06-NEG-08` to `F06-NEG-10` | `FS-NEG-SECTION` |
| Open, self-intersecting, inward mesh not exact | `F06-NEG-18` to `F06-NEG-20` | none |
| Failure keeps previous result | `F06-NEG-21` | `F04-NEG-14` |
| Queries: pick, lookup, incremental, chunking | `F06-POS-28` to `F06-POS-36`, `F06-POS-39`, `F06-POS-40`, `F06-POS-43` to `F06-POS-45`, `F06-NEG-22` to `F06-NEG-29`, `F06-NEG-32` to `F06-NEG-34` | `F04-NEG-09`, `F04-NEG-15` |

The invalid-quaternion, translation-limit and density cases are already covered by
`F04-NEG-22` to `F04-NEG-26` and `FS-NEG-QUAT`, `FS-NEG-DENSITY`, and are not repeated. The file
also carries a `cross_references` list of these topics.

## 12. Freeze status and open questions

**Status.** Draft 0.1.0, not frozen. Nothing is implemented, K0 is not passed and no geometry
implementer is declared eligible. Only the owner freezes F04; F06 can freeze only after F04's
P-1 to P-3 are confirmed or replaced, because rules marked "assumed pending owner confirmation"
depend on them.

**Open owner decisions.**

| Item | Question |
|------|----------|
| P-1, P-2, P-3 | Confirm or replace the plate local box, the member section frame and the roll rule. F06 changes jointly with F04 |
| Policy constants | Accept, or replace after G04, the minimum dimension 0.001 mm, the parallel threshold 1e-6, the caps of section 6 and the pick constants of section 9 |
| Roll discontinuity | Accept the roll jump at the parallel threshold (section 4, rule 3), or choose a continuous rule. The near-vertical rule does not remove it |
| Open item A5 | Full-domain coordinate accuracy for cancellation-heavy inputs, subnormal mass and rounded-mesh cross-checks stay **open**: no certified accuracy bound exists yet, and widening a tolerance or narrowing an F04 input range to hide the gap is not allowed. G04 and fixtures must establish bounds on the full admitted domain |
| FeatureId on axis reversal | Accept that ids follow roles, not world faces (section 5, rule 6) |

**What later tasks must still decide.**

| Task | Decision left open here |
|------|-------------------------|
| F08 | Where the kernel, query and blob code live (module paths) within the one-way dependency rule |
| G01 | The kernel API in Rust types for section 2, the builders and the self-checks of section 6 |
| G04 | Validation of every draft constant with fixtures: accuracy bounds, thresholds, tie quantum 1e-6 mm |
| G02, G03 and other G-tasks | Mesh operations beyond boxes; their topology and FeatureId rules need a new contract version |
| P02 | The persisted encoding of the profile definition (section 2 fixes only its logical content) and any section kinds beyond `rectangle` |
| P04 | Dependency-signature storage, including how the keys of section 7 are authored and compared |
| V01 | The presentation-key preimage, GPU upload, display-only picking and the pixel-to-radius conversion |
| U-tasks | The USD authoring of meshes (`orientation`, `subdivisionScheme`, `faceVertexCounts`), layer placement and the `metersPerUnit`/`upAxis` handling |
| R-tasks | Imported meshes, unit and up-axis conversion, and measurement support for foreign geometry |
| D01 | Drawing projection built on FeatureIds (section 5, rule 8) |

**Proposed amendments to other contracts (not applied; those documents are read-only here).**

1. F04 section 3: add the minimum nonzero dimension 0.001 mm (draft) as a lower limit for plate
   dimensions, section dimensions and member-axis length, with `INVALID_ARGUMENT` for values
   between zero and the minimum, and add a section-dimension cap of 1,000,000 mm.
2. F04 section 5: clarify "material-dependent appearance" as the resolved display reference
   (section 7), so that a density change alone invalidates mass only. Also record that
   F06 defines the canonical numeric encoding and the key preimages (section 7).
3. F04 section 11: mark G-06 and the profile-resource-format question as accepted by F06 with
   the draft policy of sections 2 and 4.
4. F04 section 3, P-3 (joint amendment; the owner decides): replace the fixed `+Y` fallback of
   the near-vertical case by the projection of `(0, 1, 0)` onto the plane normal to the axis,
   normalized, followed by the re-orthogonalization of section 4, rule 2. An exactly vertical
   axis keeps `+Y`; `F04-POS-16` and the other exact examples are unchanged.
5. F05 section 12: record that F06 keeps the transport element type `u8` with a self-describing
   blob, so no element-type negotiation is required. F05 itself needs no change.

**Amendments log.**

- 2026-10-15, F06 0.1.0: first draft on base `9d40abc34b8a5686358b78adee5f9c52b9c830bd`
  (F04 0.1.0 freeze candidate, F05 0.1.0 draft, first-scope package 0.1.0). No existing F04, F05
  or first-scope ID, code, limit or case was changed.
- 2026-10-15, F06 0.1.0 correction round 1, after the independent kernel-design review (G04).
  Existing case IDs keep their meaning; the case count rises from 70 to 80 (46 positive, 34
  negative).
  - A1 (defect, resolved): the near-vertical roll fallback is now the normalized projection of
    `(0, 1, 0)` onto the plane normal to the axis, re-orthogonalized and certified (section 4,
    rule 2; `F06-POS-41`, `F06-POS-42`, `F06-NEG-31`). F04 P-3 needs a joint amendment
    (proposed amendment 4). The roll discontinuity stays an open owner decision.
  - A2 (clarified): evaluation order for quantities and dot products, written binary64
    decisions with no rounding band, conformance evidence and version bumps (section 2;
    `F06-POS-46`).
  - A3 (inconsistency, resolved): mass of an unsupported body is `unsupported` (section 8
    precedence kept); `F06-NEG-18` changed from mass `unknown` to `unsupported` and now removes
    the ninth triangle `(0, 2, 3)` of `F06-POS-01`, as its text says. New diagnostic
    `GEO_NUMERIC_UNCERTAIN` (`UNSUPPORTED_GEOMETRY`) for failed numeric certificates.
  - A4 (clarified): the tie quantum `1e-6 mm` is one nanometre; closed faces enlarged by the
    pick band, parallel threshold `1e-12`, point-to-face distance and zero-radius behaviour are
    defined (section 9; `F06-POS-43` to `F06-POS-45`).
  - A5 (open, not fixed): full-domain accuracy for cancellation-heavy inputs, subnormal mass and
    rounded-mesh cross-checks; see the open owner decisions above.
  - A6 (clarified): primitive limits versus validator caps, descriptor byte bound, pick work
    and deadline bounds, negotiated limits (sections 6 and 10; `F06-NEG-33`, `F06-NEG-34`).
- 2026-10-15, F06 0.1.0 correction round 2, after the Copilot review of PR #20. The frame
  certificate of section 4, rule 2, step d bounds the absolute value of each pairwise dot product
  (a signed bound accepted `X . Y = -1e-9`); `F06-NEG-35` is added beside `F06-NEG-31` so both
  signs of an injected error fail. The case count rises from 80 to 81 (46 positive, 35 negative).
- 2026-10-15, F06 0.1.0 correction round 3, after an independent exact-arithmetic review of A7
  and A8. The contract stays a draft; all 81 IDs and meanings remain (46 positive, 35 negative).
  - A7 (defect, resolved with a qualification): the branch rotation depends on tilt direction.
    From +Z, +Y gives 180 degrees, -Y gives 0 degrees, and +X/-X give +90/-90 degrees from the
    near-vertical to the horizontal frame. Section 4, rule 3 now gives the exact formula and
    distinguishes a branch comparison on one axis from frames on distinct axes. `F06-POS-10`
    and `F06-POS-42` prose is corrected; `F06-POS-41` frame mathematics is unchanged. The roll
    rule is unchanged and continuity stays an open owner decision. A half turn for every
    Y-directed tilt is not confirmed: -Y from +Z has no roll jump.
  - A8 (defect, resolved): `F06-POS-10` and `F06-POS-42` now use
    `L = sqrt(250000000001)/500 ~ 1000.0000000020000000`; their small normalized components
    and the `F06-POS-10` horizontal ratio have sufficient digits for CR-TOL-001.
    `F06-POS-45` uses `sqrt(10025) ~ 100.12492197250392864` for all four equal distances.
    The numeric scan also corrects the `F06-POS-41` squared horizontal ratio to
    `2.4999987500001562502e-13`; its exact rational frame and adequate decimals are retained.
    Other square-root, rotation and division expectations were checked; no other insufficient
    precision was found. These example corrections establish no full-domain accuracy bound
    and do not close A5.
