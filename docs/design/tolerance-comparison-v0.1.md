# Tolerance comparison and owner decisions, version 0.1.0

Task: record owner decisions 1–6 of 2026-10-15 and compare the tolerances with the public
ifc-lite kernel. Branch: `docs/owner-decisions-2026-10-15`, stacked on PR #23. Round 1
recorded the contour plate, explicit roll and the tolerance policy; round 2 adds the ifc-lite
numbers below. The local-size amendment is on intended branch
`docs/tolerance-magnitude-scaling`, stacked on PR #26; Git ownership prevents verification.
No code or dependency is changed.

Status: documentation only, 2026-10-15. F04 remains a freeze candidate at contract_version 0.1.0;
F06 remains a draft. Nothing is implemented or admitted by this report. Constants are **confirmed by
the owner as first-slice values; draft, to be validated by fixtures**.

This report consumes [F04](../contracts/model-and-commands-v0.1.md),
[F06](../contracts/geometry-contract-v0.1.md), the first-scope requirements 0.1.0 (PR #10) and [kernel design](kernel-design-v0.1.md). The
requirements and design retain their earlier policies. Only the two contracts and their cases apply the
owner amendment here.

## Interpretation and classification

The owner's “lower the precision to 1e-6” means relative error 1e-6, with a length floor of 1e-6 m =
0.001 mm. The local-size amendment below scales the geometric floor while retaining the
absolute test and pick floors. The length floor does not mean 1e-6 mm. This
interpretation relaxes admission/comparison/certificate bounds; it proves no algorithm accurate, and
it does not close A5 in full: only the large-coordinate policy part is resolved by the
new owner decision; rounded-mesh cross-checks and subnormal mass remain open.

Every tolerance below has one of four classes:

1. **T**: test comparison rule CR-TOL-001, independent of kernel decisions.
2. **G**: geometric tolerance, admission or degeneracy threshold.
3. **C**: numeric certificate acceptance bound. The bound may relax; its proof must remain sound.
4. **E**: exact identity, mathematical bound, conversion, version or resource invariant. These are
   not adjustable tolerances. No exact rational expected value is loosened or replaced.

Lengths in CR-TOL-001 use `0.001 mm + 1e-6*abs(expected)`. Other quantities use relative 1e-6. Frame
components use absolute 1e-6 at zero and relative 1e-6 otherwise. The kernel never reads this test
rule. Coordinate magnitude does not enlarge the pick band: it is an absolute query length tolerance,
now 0.001 mm. Planarity has its own explicit scale D, not the world origin. No unproved global
“accuracy certificate” is introduced. Numeric enclosures in the design remain derived per operation;
their mathematical error bounds cannot be replaced by guessed 1e-6 errors.

## Precision, units and design verdicts

“Old” records the pre-amendment F04/F06 and unchanged handoff/design. “Now” records these contracts.
All dimensionless values have no conversion to millimetres. Design verdicts refer to its section 5;
they are evidence and limitations, not owner adoption of implementation algorithms.

| Constant / source | Class | Old | Now | Unit / meaning | Design verdict / reason |
|-------------------|-------|-----|-----|----------------|-------------------------|
| CR-TOL-001 quantity comparison | T | 1e-12 | 1e-6 | relative, dimensionless | Loose for normal analytic values, tight for subnormal/rounded mesh; A5 stays open |
| CR-TOL-001 coordinate absolute floor | T | 1e-9 mm | 0.001 mm = 1e-6 m | length | Supported on listed small cases, tight at cancellation; owner explicitly relaxes |
| CR-TOL-001 coordinate relative part | T | 1e-12 | 1e-6 | dimensionless | No full-domain guarantee follows |
| Quaternion squared-norm admission, F04 s.3 / F06 s.2 | G | 1e-12 | 1e-6 | dimensionless deviation from 1 | Loose but supported as admission; normalize accepted q |
| Frame unit norms, pairwise absolute dots, handedness, F06 s.4 | C | 1e-12 each | 1e-6 each | dimensionless absolute deviations | Certificate added after original design; negative and positive errors both bounded |
| eps_dim = eps_geom, F04 s.3 / F06 s.4 | G | fixed 0.001 mm | max(0.001 mm, 2^-22 * L) | minimum thickness, edge, section or axis length, inclusive; local size L | Owner local-size decision; no relative mesh accuracy guarantee |
| Coincidence, F06 s.4 | G | distance < 0.001 mm | distance < eps_geom | axis endpoints; contour edge minima also use eps_geom | No weld/merge rule; exact duplicates remain exact |
| eps_par, member default frame, F06 s.4 | G | 1e-6 | unchanged | dimensionless strict horizontal/total length ratio | Supported to avoid tiny denominator; frame correction still required |
| Squared member parallel multiplier | E | 1e-12 | unchanged = (1e-6)^2 | dimensionless, hh < eps_par^2*LL | Square identity, not a certificate or comparison tolerance |
| Near-vertical projection lower bound | E | 1-c^2 >= 1-1e-12 | unchanged | dimensionless mathematical consequence of branch | Not an independently adjustable tolerance |
| Pick band eps_pick, F06 s.9 | G | 1e-6 mm = 1e-9 m | 0.001 mm = 1e-6 m | length, inclusive face/edge/vertex band | Original quantum supported for buckets, tight for robustness |
| Pick tie quantum, F06 s.9 | G | 1e-6 mm | 0.001 mm | deterministic length buckets, not exact equality | Wider buckets are intentional; identity can precede distance within bucket |
| Ray/face parallel threshold, F06 s.9 | G | 1e-12 | 1e-6 | dimensionless abs(unit direction dot unit normal), strict | Original design required certified near-zero handling; no sign guess |
| Ray direction norm minimum, F06 s.9 | G | 1e-9 | 1e-6 | dimensionless normalization input, inclusive | Supported normalization range; direction is not a length |
| Plate planarity, F04 s.3 | G | 0.001 mm + 1e-6*D before local-size amendment | eps_geom + 1e-6*D | distance to p0 plane, D=max distance from p0 | Assumption; valid in-band nonplanar input is unsupported until A02 |
| Normal orientation, simplicity and exact rectangle identities | E | positive cuboid dimensions | exact rational signs/zeros | no tolerance for winding, self-intersection, e0 dot e1=0, p3=p0+e1 | Never heal, snap, infer topology or loosen exact identities |
| Binary64 unit roundoff u | E | 2^-53 | unchanged | dimensionless mathematical constant | Error analysis input, not an acceptance epsilon |
| Smallest positive subnormal eta | E | 2^-1074 | unchanged | numerical representation quantum in the operand's unit | Underflow requires eta/2 absolute error propagation |
| g(n)=n*u/(1-n*u) | E | n=2,3,4,5,6,7,9,10 as derived | unchanged | dimensionless analytic rounding bound | Not a replaceable policy tolerance |
| Design normal relative budgets: length / plate volume / member volume | E | g(6) / g(3) / g(9) | unchanged derivations | <6.662e-16 / <3.331e-16 / <9.993e-16 | Conditional normal arithmetic bounds, not implemented certificates |
| Design normal relative budgets: plate area / member area / plate mass / member mass | E | g(6) / g(10) / g(4) / g(10) | unchanged derivations | dimensionless; area <6.662e-16 / <1.111e-15 | Do not replace the corrected plate-area budget with g(4) |
| Design absolute maxima: length / plate and member volume | E | <6.662e-10 mm / <3.331e-7 and <9.993e-7 m3 | unchanged | exact unit scaling of normal budgets | These are analysis results, not blanket tolerances |
| Design absolute maxima: plate/member area and mass | E | <3.9972e-9 / <6.666e-9 m2; <0.4441 / <1.111 kg | unchanged | conditional normal-value bounds | Underflow and rounding-heavy mesh counterexamples still apply |
| Design spacing at 1e9 / below 4e9 | E | 2^-23 / 2^-21 mm | unchanged | binary64 spacing, minimum spans >=2097.152 latter spacings | Supported distinctness; no analytic/mesh equivalence proof |
| Former proposed exclusion band in design s.5.1 | C | 1e-9 relative | no exclusion band adopted | design discusses an older draft, not current F06 | Design calls it loose; F06 written comparisons remain authoritative |
| Design denominator lower estimate | E | h*L > 9e-13 mm2 | unchanged analysis | from minimum length and retained branch threshold | Not a degeneracy cutoff to relax |
| Design W32 mesh error examples | E | >0.0001401 and <0.0001403 (three edges); >0.0000934 and <0.0000936 (two); >0.0000467 and <0.0000468 (one) | unchanged | relative rounding counterexamples | All still exceed 1e-6; A5 remains open |
| Design cancellation example W33 | E | -0.0000002220446049250313080847263336181640625 mm | unchanged | exact error from rounded rotation/cancellation | May fit new test floor; this does not certify an algorithm |
| Exact reference arithmetic and unit conversions | E | 1 m=1000 mm; 1 m2=1e6 mm2; 1 m3=1e9 mm3 | unchanged | exact SI identities, no error allowance | Divide by exact integers; do not substitute rounded constants |

The earlier decision retained eps_dim and eps_par; the later local-size decision replaces only
the fixed geometric length floor with eps_geom. eps_par stays dimensionless 1e-6, and its
written squared multiplier stays 1e-12. It would change the branch angle by a factor of 1000 to
replace that square by 1e-6.

## Operating, resource and layout constants

These are class E operating invariants, not precision knobs. They remain unchanged except for the
explicitly listed new contour/roll bounds and the algorithm/tolerance versions. The design section 5
calls the generic structural limits loose for boxes but supported as validator limits.

| Constant / source | Value and unit | Meaning / design verdict |
|-------------------|----------------|--------------------------|
| F04 command bytes / depth / strings | 1 MiB / 16 levels / 4096 UTF-8 bytes | Bound parsing; no precision meaning |
| F04 label | 1–256 UTF-8 bytes; U+0000–001F and U+007F forbidden | Exact input policy |
| F04/F06 dimension and axis cap | 1,000,000 mm = 1000 m | Supported finite products; preserve existing maximum rectangle |
| F04/F06 coordinate/translation cap | abs <=1,000,000,000 mm = 1,000,000 m | Authored inputs, not derived world extrema |
| F04/F06 section cap | 1,000,000 mm = 1000 m | Supported; no cap increase |
| F04/F06 density | 0 < density <=1,000,000 kg/m3 | No positive minimum; unknown when absent; subnormal mass still open |
| Quaternion component range | [-1,1], dimensionless | Normalize accepted q; antipodes equal |
| USD linear-unit metadata | metersPerUnit=0.001, upAxis=Z | Exact mm conversion, not accuracy floor |
| F04 relationships / footprint | 1024 targets / 4096 entries | Exact bounded command work |
| New contour points / D | 3–1024 points / D <=2,000,000 mm = 2000 m | Assumptions; 2e6 retains old 1e6-by-1e6 rectangle |
| New roll cap | abs <=6.283185307179586 rad (binary64 2*pi) | Assumption; earlier F04/F06 have no angle cap; no modulo rewriting |
| Kernel numeric input size | old 16; now plate <=3081, member <=17 scalars | 3*1024+1+7+1; 6+2+1+7+1 |
| Contour pair-work ceiling | 523,776 pairs | 1024*1023/2, inclusive upper bound before adjacent exclusions |
| F06 diagnostics / field token | 32 records / 64 ASCII bytes | Fixed safe schema fields |
| F06 result codes | old 25, now 27 GEO codes | Two contour codes; both map to existing F04 codes |
| F04 stable codes | 23 | Unchanged; no implementation enum change |
| Versions | contract 0.1.0; algorithm 3, tolerance_policy 3, feature_id_rule 1, mesh_layout 1 | Encoding and behaviour changed; no freeze |
| fid1 length / longest form | 64 / 59 bytes | Exact name bounds; rule follows body roles |
| Primitive box counts | 8 vertices, 12 triangles, 6 faces, 12 topological edges, 18 triangle edges, 1 shell, 26 features | Exact topology; 8-18+12=2 and 8-12+6=2 |
| Mesh integer/position encoding | u32 indices, u16 face indices; f64 positions; little endian | Binary layout, not tolerances |
| Mesh header | magic BCMESH01; 32 bytes; offsets 0,8,12,16,20,24,28; 8-byte magic, 4-byte fields | Exact wire layout |
| Mesh section sizing | 24*V; pad8(12*T); pad8(2*T); pad8 to 8 bytes | Exact checked arithmetic |
| Primitive blob / offsets | 392 bytes; positions 32, indices 224, face table 368 | Supported exact size |
| Generic vertices / triangles / faces | 4096 / 8192 / 1024 | Loose for 8/12/6 scope, supported validator caps |
| Generic topological edges / shells / features | 12288 / 16 / 17408 | Loose for primitive; features=4096+12288+1024 |
| Generic blob bytes | 213024 | Exact simultaneous count ceiling, supported |
| Descriptor / size arithmetic | 32768 encoded UTF-8 bytes / checked 64-bit sizes | Additional descriptor budget; never infer from mesh bytes |
| Query origin/point cap | abs <=4e9 mm = 4e6 m | Supported to cover native world points; not an error floor |
| Direction component cap | abs <=1e9, dimensionless | Supported normalization input; no mm conversion |
| Pick horizon | default and maximum 4e9 mm = 4e6 m | Design notes it cannot cover every origin/body pair; finite interval is intentional |
| Pick radius | default 0; maximum 1e6 mm = 1000 m | User query parameter, separate from tolerance |
| Pick output / ranks | default 16; 1–64 hits; face/edge/vertex ranks 0/1/2 | Exact result order and bound |
| Pick work / polling | <=65536 elements, <=26 features each; poll every 1024 elements | Bounded scan, no partial hits on failure |
| Recursion / cancellation | no recursion; future depth <=16; P0–P3 before seal | Exact resource/control policy |
| Shape key encoding | kind byte 1/2, u16 contour count, f64 coordinates/thickness or axis/roll/profile; four u16 versions | Ordered authored contour is part of shape, not placement |
| Digests / mass signature | SHA-256, 32 raw bytes / 64 lowercase hex digits; density presence byte 0/1 | Exact identities; never compare with tolerance |
| World/mass tags and space | BCGEO-SHAPE, BCGEO-WORLD, BCGEO-MASS plus zero byte; mesh space 0 local / 1 world | Exact domain separation |
| Host transport limits referenced by F06 | negotiated F05 limits, smaller applicable bound wins | No F05 number is changed or restated as a geometry tolerance |
| Design dyadic fallback proposal | <1107/1108/3330 bits; proposed 4096-bit buffer; degree <=3 | Analysis/proposal only, not a required homemade integer library |
| Design resource proposals | 24576 directed edges; <=15 merge passes; <=10000000 limb-pair steps | Separate design proposal, not implemented or admitted here |

F04 UUID version 7, RFC variant, UUID length 36, digest length 64, u64 generation range and exact
string/type rules remain identity/representation invariants. F06 face/edge/vertex role bits,
triangle winding, outward positive signed volume, Euler identities and closed incidence remain exact
class E rules. No tolerance changes them. The public design's mathematical working examples and
error-analysis constants remain exact references or qualified estimates, not new kernel cutoffs.

## Public ifc-lite comparison

Provenance: the numbers were compared with the public ifc-lite repository (MPL-2.0), numbers only;
no code or text was copied.

The values below were supplied by the orchestrator from the public repository `LTplus-AG/ifc-lite`
(licence MPL-2.0, default branch `main`) at the pinned commit
`a3148f11f644bf3d0237a34eadce0f665ff2d104`, numbers only. This worker did not read the repository.
Where a unit is not stated in the source, the table says so. Where the unit is metres, the value in
millimetres is the metre value times exactly 1000. The ifc-lite kernel works in metres for the
cases described here.

### Verified table (supplied, pinned commit)

| Name | Value | Unit as stated there | Millimetres | File (at the pinned commit) |
|------|-------|----------------------|-------------|-----------------------------|
| `PARALLEL_EPS` | 1e-9 | dimensionless (2D parallel test) | not a length | `rust/geometry/src/geom2d.rs` |
| `EPS` | 1e-9 | not stated | unknown | `rust/geometry/src/router/voids/geom.rs` |
| `TOUCH_EPS` | 1.0e-3 | comment says 1 mm, so metres | 1 mm | same |
| `PLANE_TOL` | 1e-3 | not stated; metres in context | 1 mm | same |
| `ORTHOGONAL_DOT_TOL` | 0.02 | dimensionless dot product | not a length | same |
| `AXIS_THRESHOLD` | 0.999_847_695 | dimensionless cosine, about 1 degree | not a length | same |
| `MAX_OFFSET_JITTER` | 50.0e-6 | metres in context (50 micrometres) | 0.05 mm | `rust/geometry/src/facet_weld.rs` |
| `MAX_VERTEX_MOVE` | 200.0e-6 | metres in context (200 micrometres) | 0.2 mm | same |
| `POSITION_DEDUP_GRID` | 1.0e-4 | metres in context | 0.1 mm | same |
| `EDGE_SLACK` | 4.0e-4 | metres in context | 0.4 mm | same |
| plane clipping floor | 1e-6 | metres (1 micrometre) | 0.001 mm | `rust/geometry/src/csg/plane_eps.rs` (doc comment) |
| plane distance scaling | 2^-22 of the coordinate magnitude | relative, dimensionless | magnitude times 2^-22 | same |
| floor for file units of millimetres | 1e-6 mm | stated in the same comment (a nanometre) | 1e-6 mm | same |
| half-space cap on-plane tolerance | `(diag * 1e-5).max(1e-6)` | mesh diagonal based, floor 1e-6 | floor 0.001 mm when metres | `rust/geometry/src/processors/boolean/halfspace_cap.rs` (as described in the `plane_eps.rs` comment) |
| AABB clip plane scale | 1e-6 | scale factor | not a length | `rust/geometry/src/router/voids/aabb_clip.rs` (as described in the `plane_eps.rs` comment) |

All rows: supplied by the orchestrator from the public repository, numbers only, at the commit
above. The last two rows are second-hand descriptions taken from a comment in `plane_eps.rs`.

**Excluded from the table:** profile clearance and join bounds of 1e-9 in `docs/api/rust.md`. That
excerpt is unpinned and its unit is "file length units", so it is not verified evidence and is not
used in the mapping.

One point needs the owner: the `plane_eps.rs` comment gives the floor as 1e-6 in the file's length
unit, which is 1 micrometre for metre files but 1 nanometre (1e-6 mm) for millimetre files. The
BeamCraft floor of 0.001 mm equals the metre case only. BeamCraft is millimetre-based internally, so
a literal "same number in file units" would be 1e-6 mm, a thousand times tighter than the contract.
The contract keeps the metre reading chosen in round 1 (see Interpretation); the owner should
confirm it.

### Mapping of BeamCraft constants to the closest ifc-lite value

“Equal”, “tighter” and “looser” describe the BeamCraft contract value against the ifc-lite value,
after converting to the same unit. “Follows owner” asks whether the contract value is the owner's
1e-6 ("take the same ones, lower the precision to 1e-6").

| BeamCraft constant (now) | Closest ifc-lite value | Relation | Follows owner decision |
|--------------------------|------------------------|----------|------------------------|
| Absolute minimum of eps_geom, 0.001 mm = 1e-6 m | plane clipping floor 1e-6 m = 0.001 mm | Equal minimum (metre reading); geometric floor can grow | Yes; owner adopted local-size scaling |
| Pick band and pick tie quantum, 0.001 mm | same floor, 0.001 mm | Equal | Yes |
| Coordinate floor of CR-TOL-001, 0.001 mm | same floor, 0.001 mm | Equal | Yes |
| CR-TOL-001 relative part, 1e-6 | half-space cap relative factor 1e-5 of the mesh diagonal | Tighter by 10 | Yes (owner value); the ifc-lite factor is looser |
| Plate planarity, eps_geom + 1e-6*D | `PLANE_TOL` 1 mm; magnitude scaling 2^-22 (about 2.38e-7) | Different purposes; geometric floor now scales with local D, with retained relative term | Yes; owner decision recorded below |
| Member parallel threshold `eps_par`, 1e-6 | `PARALLEL_EPS` 1e-9 (2D, dimensionless) | Looser by 1000 | Yes; this is the owner-protected 1e-6 |
| Near-vertical branch, sine of axis angle below 1e-6 (about 5.7e-5 degrees) | `AXIS_THRESHOLD` cosine 0.999847695, sine about 0.01745 (about 1 degree) | Tighter by about 17,000 | Yes; different purpose (axis classification) |
| Ray/face parallel threshold, 1e-6 | `PARALLEL_EPS` 1e-9 | Looser by 1000 | Yes |
| Ray direction minimum norm, 1e-6 | none | No counterpart | Yes (owner value) |
| Quaternion norm admission, 1e-6 | none; `ORTHOGONAL_DOT_TOL` 0.02 is a dot-product bound | No counterpart | Yes (owner value) |
| Frame certificate, absolute 1e-6 on norms, dots and handedness | `ORTHOGONAL_DOT_TOL` 0.02; `PARALLEL_EPS` 1e-9 | Tighter than 0.02 by 20,000; looser than 1e-9 by 1000 | Yes |
| Exact rectangle identities (orthogonality, p3 = p0 + e1) | `ORTHOGONAL_DOT_TOL` 0.02 | BeamCraft is exact, so tighter | Not applicable; exact rational identities are never loosened |
| Roll cap, contour limits, resource caps, hashes | none | No counterpart | Not tolerances |
| Coincidence of axis endpoints, below eps_geom | floor 0.001 mm; `POSITION_DEDUP_GRID` 0.1 mm | Equal absolute minimum; local-size floor may grow | Yes; no vertex dedup or weld rule |

Findings from the mapping:

- **Floor.** The ifc-lite plane clipping floor 1e-6 m equals the BeamCraft minimum dimension
  absolute minimum 0.001 mm. Checked, not assumed: 1e-6 m times 1000 = 0.001 mm. This holds for
  metre files only (see
  the open unit point above).
- **Parallel.** Their 2D parallel epsilon 1e-9 is a thousand times tighter than the member parallel
  threshold 1e-6. The owner wants 1e-6 and `eps_par` is protected, so it stays. It is a different
  quantity: theirs is a 2D parallel test, ours is the ratio that keeps the default-frame denominator
  away from zero. The squared multiplier 1e-12 is an exact identity and has nothing to do with
  their 1e-9.
- **Magnitude scaling (round-2 open item 2, resolved for the large-coordinate part).** The
  owner decided on 2026-10-15: "Scale the tolerance by coordinate magnitude." The accepted
  proposal uses local body size, never world-coordinate magnitude: eps_geom is
  max(0.001 mm, 2^-22 * L). Plate L is D; member L is max(axis length, section half-diagonal).
  Geometric length decisions use this floor; relative 1e-6 bounds/certificates, CR-TOL-001,
  absolute pick bands and exact checks remain unchanged. At a world coordinate of 1e9 mm,
  world-based scaling would be about 238 mm; moving an element cannot change its tolerance.
  Only A5's large-coordinate part is closed. Rounded-mesh cross-checks and subnormal mass remain
  open. This is a contract policy decision, not implemented accuracy evidence.
- **Touch and plane tolerances.** `TOUCH_EPS` and `PLANE_TOL` (1 mm) serve void and adjacency tests.
  The first slice has no voids, cuts, clashes or connection tests, so they map to nothing in the
  first slice. They become relevant for the later assembly, cut and hole work; the contract
  tolerances (0.001 mm) are three orders of magnitude tighter and are not a replacement for them.
- **Mesh weld values.** `MAX_OFFSET_JITTER`, `MAX_VERTEX_MOVE`, `POSITION_DEDUP_GRID` and
  `EDGE_SLACK` belong to facet welding and cleanup of imported meshes. The first-slice kernel builds
  exact eight-vertex solids and has no welding step, so there is no counterpart.
- **Dimensionless thresholds.** `ORTHOGONAL_DOT_TOL` and `AXIS_THRESHOLD` classify imported or
  user geometry roughly. BeamCraft either demands exact identities or applies 1e-6 to authored
  input, so it is tighter in every comparable case.

Contradictions with the owner decision: none found. Every contract value is either the owner's 1e-6
policy or an exact identity. Round 2 changed no contract value; the later owner local-size amendment
changes the
geometric floor as recorded here. The "take the same
ones" instruction is met where ifc-lite has a counterpart that agrees (the floor) and is overridden
by the explicit "lower the precision to 1e-6" where it does not (relative bounds, parallel
thresholds). The metre-unit interpretation remains an earlier review item. Round-2 open item 2
is resolved for its large-coordinate part by the local-size decision; the remainder of A5 is
exactly rounded-mesh cross-checks and subnormal mass.

## Proposed policy

1. The owner policy retains relative 1e-6 and the test/pick floor 0.001 mm. Geometric length
   floors use eps_geom from local body size; dimensionless certificates keep 1e-6. Class E
   identities are never loosened.
2. Where ifc-lite has an agreeing counterpart (the floor), the contract equals it. Where it has a
   different or looser value for another purpose (1 mm touch and plane tolerances, mesh weld
   values), the contract does not adopt it until a task needs that purpose.
3. Re-check each constant against fixtures before the freeze; the values stay "draft, to be
   validated by fixtures" while being confirmed by the owner as first-slice values.
4. The 2026-10-15 owner decision authorizes local-size scaling as specified below. Further
   tolerance changes still need a reviewed owner decision.

## Earlier boundary changes (before the local-size amendment)

| Boundary | Effect on old inputs | Amendment preserving intent |
|----------|----------------------|--------------------------------|
| Quaternion norm 1e-12 -> 1e-6 | F04-NEG-25 with w=0.999999999 would pass | w=0.999999: 1-w^2=0.000001999999 >1e-6 |
| Accepted norm example | F04-POS-11 still passes, but loses its near-boundary purpose | w=0.99999995: 1-w^2=0.0000000999999975 <1e-6; rotation remains identity |
| Frame certificate 1e-12 -> 1e-6 | F06-NEG-31/35 with dot +/-1e-9 would pass | inject +/-2e-6; negative-frame norm deviation 4e-12 remains admissible |
| Ray direction minimum 1e-9 -> 1e-6 | Norms in [1e-9,1e-6) become invalid | No old case in this interval; zero direction still invalid |
| Ray/face parallel 1e-12 -> 1e-6 | abs(dot) in [1e-12,1e-6) becomes parallel | Old face cases use exact 0 or unit-normal hits; no old hit expectation flips |
| Pick band 1e-6 -> 0.001 mm | Features in (old band,new band] can newly qualify | No old input lies in the changed interval; old hit sets remain |
| Pick bucket 1e-6 -> 0.001 mm | Different near-equal distances can now tie | Existing equal-distance ties and POS-45 ordering unchanged; explanatory divisors updated |
| Test relative 1e-12 -> 1e-6; coordinate floor 1e-9 ->0.001 mm | Actual output comparisons can pass with larger errors | No expected analytic value changes; old cases specify no measured implementation output |
| Member squared parallel multiplier 1e-12 | No change | POS-09/10/41/42/46 unchanged; it is eps_par squared |
| Minimum 0.001 mm | No change | F04 POS-17/NEG-27 and F06 POS-14/NEG-03/04 retain intent |

These are all old described-case outcome flips identified by the input scan: F04-NEG-25 and
F06-NEG-31/35. Other moved boundaries change the permitted domain but no existing expected outcome.
F04-POS-11 moves to preserve purpose, not because its outcome flips. First-scope FS-NEG-QUAT's
fourth entry would also flip under a future package amendment; it is read-only here. Its counterpart
F04-NEG-25 no longer claims identical input. FS-POSE-F remains an accepted small-deviation example.
The read-only design's existing counterexamples retain their exact values.

All existing case IDs are retained. After the local-size amendment F04 has 69 logical cases (30
positive, 39 negative) and
seven raw cases. F06 has 112 cases (59 positive, 53 negative). Amendments and coverage tables in the
contracts enumerate new contour/roll cases. Every origin rectangle is rewritten from width/ length
to four points, preserving width=|p1-p0| and length=|p2-p1| and all exact quantities. F06-POS-25's
seven digests and the antipodal equality assertion change because contour encoding and
algorithm/tolerance versions change. The canonical box mesh, fid1 tokens, layout version and
baseline mesh digest do not change.

## Assumptions and remaining decisions

- Contours close implicitly, have 3–1024 points, bounded coordinates, bounded edges and D <=2e6 mm.
  The separate D cap preserves the old maximum square. No repeated closure point is accepted.
- Clockwise means negative dominant component of the area vector (tie priority Z,Y,X). Reject rather
  than reorder. The owner did not supply an independent normal; a hemisphere convention is necessary
  to distinguish clockwise in 3D without secretly changing X or point order.
- p0 is the body-frame origin and bottom face; extrusion is only along +Z_p. Planarity admission is
  eps_geom+1e-6*D; exactly planar rectangles alone receive first-slice solids. Near-planar polygons
  remain valid authored input with unsupported evaluation, without moving their points.
- No numeric angle cap exists in the read F04/F06. Use inclusive abs(roll_rad)<=binary64(2*pi). This
  is a documented new assumption, not a claim of an earlier cap. Preserve authored roll.
- Apply 1e-6 to the dimensionless direction minimum and face parallel threshold, retaining the
  separately owner-protected member eps_par and its exact square. Use absolute 0.001 mm pick bands
  and buckets; relative world-coordinate scaling would make selection depend on origin.
- Sin/cos require a pinned audited math choice and cross-target evidence. Omitted/zero roll bypasses
  trig; no backend, crate, Git revision or vendor patch is selected in this record.
- The ifc-lite numbers are supplied facts, not read by this worker. The floor is read as 1e-6 m
  (0.001 mm); the owner should confirm it against the millimetre-file reading (1e-6 mm).
  Contract freeze, fixture validation and design reconciliation remain separate gates.

## Proposed edits to read-only files (not applied)

The following is exact proposed text for their owners, subject to review and package versioning.

**AGENTS.md section 5, after the pure-Rust preference:**

> Prefer suitable well-maintained permissively licensed Rust crates for math, geometry primitives
and robust predicates over handwritten equivalents. Write own algorithms only where no suitable
crate exists. Copyleft and weak-copyleft third-party packages, including MPL-2.0, require an
explicit owner exception for each package. Audited dependency use does not authorize copying
implementation text or restricted commercial-product material.

**module-boundaries-v0.1.md section 3, after the empty-list statement:**

> The owner decision of 2026-10-15 prefers suitable audited permissive Rust math and geometry
crates. It does not populate this allowlist. Add each approved domain dependency here and update
its manifest and dependency audit together. Keep the PdfCraft frontend stack in the planned
beamcraft-ui app; craft-libs requires a pinned revision, licence verification and a deny.toml
unknown-git-source exception before integration. The domain/host/app edges and engine independence
remain unchanged.

**F05 section 12 clarification:**

> F04/F06 owner amendments of 2026-10-15 replace plate width/length with contour_points_mm and add
member roll_rad. Model commands keep their existing transport envelope; numeric overflow,
duplicate-key and logical control bounds remain enforced. Geometry blobs remain opaque u8
attachments. Algorithm and tolerance versions are 3 after the local-size amendment; mesh layout
remains 1.

**F10 sections 2.2 and 6.5 (replacing the corresponding old fields/rules):**

> A steel plate requires steel:plate:contourPointsMm and steel:plate:thicknessMm, validated under
F04 section 3. A valid nonrectangular contour is authored model input; its geometry remains
unsupported until A02. A beam/column optionally authors steel:member:rollRad, default zero, as
shape input rotated about member local +Z after its default frame. DV-ATTR-001/004/005 and
DV-LIM-001/002/003 must validate those fields and F04's contour, roll and 1e-6
quaternion-admission limits. Replace the old rectangular width/length cases with equivalent
contours without changing their meanings. Review vocabulary compatibility and raise the version as
its policy requires before admitting this field replacement.

**kernel-design-v0.1.md sections 2–9 reconciliation notice:**

> F04/F06 owner amendments of 2026-10-15 supersede the rectangular input and roll assumptions:
exact rectangle contours keep the canonical box mesh; P-2 is confirmed; optional roll is shape
input about local +Z. Algorithm/tolerance versions are 3. Adopt the revised 1e-6
comparison/admission/certificate bounds and the local-size eps_geom of F04 section 3/F06
section 4. eps_dim equals eps_geom; retain eps_par=1e-6, exact references and mathematical
error bounds. Prefer suitable audited permissive Rust
crates, including for bounded robust arithmetic; handwritten integer/sign algorithms are justified
only when no suitable crate exists. Reconcile the matrix with 112 F06 cases. Mark only A5's
large-coordinate part resolved by
the policy; keep rounded-mesh cross-checks, subnormal mass and cross-target trig conformance
open until proved. The ifc-lite numeric
comparison is in tolerance-comparison-v0.1.md; no source code is adopted.

**Other design documents, where applicable, append this cross-reference:**

> The owner-decision record of 2026-10-15 in F04/F06 and tolerance-comparison-v0.1.md supersedes
earlier plate-input, roll and tolerance assumptions. This document is not updated or admitted by
that record; reconcile its affected rules and cases before implementation.

**first-scope package and cases, for a separately reviewed version amendment:**

> Replace CR-PLATE-001's rectangular dimensions with F04's contour/thickness input while retaining
rectangle-only first-slice solid support. Record P-1 replacement, P-2 confirmation and P-3 axial
roll. Amend CR-PLACE-001 to 1e-6 norm admission and CR-TOL-001 to relative 1e-6 with a 0.001 mm
length floor. Preserve exact rational results; rewrite FS-NEG-QUAT's fourth quaternion to
w=0.999999 so it remains rejected. Raise the package version and review admission before relying
on those changes; no current 0.1.0 entry is silently amended.

UI asset and patch rows wait for their own original-source audits; the architecture paragraph adds
no dependency.

## Earlier-round checks and delivery (historical)

Round 1 changed the six contract, case, architecture and report files. Round 2 changed this report,
`ROADMAP.md` and `ATTRIBUTION.md`, and replaced the stale "comparison blocked" wording in
`docs/architecture.md`, F04, F06 and the F06 case file. That round changed no contract value.
Scratch
verification scripts are under the ignored `target/owner-decisions/`.

| Command | Exact result |
|---------|--------------|
| `python -c "import json; ..."` on both case files | Exit 0; json ok |
| `cargo fmt --all -- --check` | Exit 0; no output |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0; no warnings |
| `cargo test --workspace` | Exit 0; 120 tests passed, 0 failed |
| `python target/owner-decisions/verify.py` | Exit 0; IDs, counts, cross-references, error-code mappings, links and anchors, 100-column prose, exact Fractions expectations and digests passed |
| `git diff --check` | See the final report |
| WASM check | Not run; target absent |
| cargo-deny | Not run; tool absent |

These tests exercise the existing scaffold; they do not execute the contract cases or demonstrate a
contour or roll kernel. Tables and code blocks are exempt from prose wrapping.

## Local-size amendment, 2026-10-15

Owner decision: "Scale the tolerance by coordinate magnitude." This accepts round-2 open item 2
for the large-coordinate part of A5 only. The exact operative definition is in
[F04 section 3](../contracts/model-and-commands-v0.1.md#3-units-types-and-initial-limits) and
[F06 section
4](../contracts/geometry-contract-v0.1.md#4-tolerance-and-degeneracy-policy-g-06-draft):

> `eps_geom = max(0.001 mm, 2^-22 * L)`.

L is in millimetres in the element-local frame before placement. Plate L is contour radius D;
member L is max(axis length, section half-diagonal). F04/F06 specify norm/subtraction/max order,
binary64 round-to-nearest-even, the absolute minimum, unchanged caps and failure diagnostics.
Compute the floor only after finite input, derived-norm and cap checks. Finite excessive L is a
cap failure; non-finite derived L is numeric uncertainty, never a fallback floor. Planarity is
eps_geom+1e-6*D, preserving the earlier relative term. No world-origin scaling is allowed.

The floor applies to geometric length minima, coincidence/degeneracy, planarity and distance bands.
Parallel/orthogonality comparisons expressed as lengths use it as their absolute floor. Member
eps_par remains dimensionless 1e-6 with written squared multiplier 1e-12; relative bounds and
numeric certificates keep 1e-6. CR-TOL-001, absolute eps_pick/tie quantum and exact identities
remain unchanged. Pick boundary flags still use section 9's own band. No new runtime API is added.

### New cases and dependency keys

| Topic | F04 cases | F06 cases |
|-------|-----------|-----------|
| Scaled thickness: above / equal / below | POS-25, POS-26, NEG-34 | POS-53, POS-54, NEG-45 |
| Small body, floor unchanged at a large placement | POS-27 | POS-55 |
| Scaled planarity: valid unsupported / invalid | POS-28, NEG-38 | POS-56, NEG-49 |
| Member half-diagonal: axis above / below | POS-29, NEG-36 | POS-57, NEG-47 |
| Member axis: section dimension above / below | POS-30, NEG-37 | POS-58, NEG-48 |
| L above plate / member cap | NEG-35, NEG-39 | NEG-46, NEG-51 |
| Non-finite derived L (fault injection) | none | NEG-50 |
| Pick band stays absolute on a large body | none | POS-59 |

New totals are 69 F04 logical cases (30 positive, 39 negative), seven raw; 112 F06 cases
(59 positive, 53 negative). All existing IDs and meanings remain. The existing-input scan finds
no new outcome flip: no input rewrite is needed. In particular F04-POS-17 and F06-POS-14 retain
their minimum-floor intent because their local sizes keep 2^-22*L below 0.001 mm. The new negative
scaled-minimum cases would pass under the fixed floor; POS-28/POS-56 instead changes old rejection
to valid authored input, still unsupported evaluation. No unsupported geometry is promoted to solid.

Algorithm and tolerance_policy change from 2/2 to 3/3; feature_id_rule and mesh_layout stay 1/1.
Section 7 preimage fields/tags do not change. L and eps_geom are derived shape policy, not
additional
encoded inputs. F06-POS-25 regenerates two shape keys, two world keys and three mass signatures,
and rechecks antipodal equality. The baseline mesh digest stays unchanged. The final task report
lists exact digests and actual check results; scratch verification is in ignored
`target/tolerance-magnitude/` and uses independent Fraction binary64 rounding and hashing.

### Proposed kernel-design edits (not applied)

- Sections 2, 3.1, 3.2 and 3.4: consume F04/F06's amended local-size/minimum domain and ordered
  binary64 scale derivation. Distinguish axis length ell from tolerance L. Preserve all proved
  error-analysis formulas and operation-derived enclosures; a policy epsilon is not an error bound.
- Section 4 and section 5.1 (W16/W17): replace fixed geometric distance/minimum floors by
  eps_geom; specify length-based boundary bands without changing exact signs, combinatorics or
  rectangle identities. Keep member eps_par and its squared multiplier. Section 5.3/5.4 pick
  rules keep the earlier-amended absolute 0.001 mm band and quantum, not eps_geom.
- Section 5.4 W50 and version references: shared version bytes become `00 03 00 03`; shape
  versions are 3/3/1/1, world 3/3 and mass algorithm 3. Reconcile POS-25's seven digests.
- Sections 7 and 9: reconcile coverage with all 112 F06 cases; A5 is partly resolved, with only
  the large-coordinate policy part closed. Rounded-mesh cross-checks (W32) and subnormal mass
  (W10) remain open. Keep original counterexample arithmetic and conformance/admission gates.
  Update the read-only validation matrix in its own reviewed task, not by this amendment.

### Amendments log

- 2026-10-15: recorded the owner's local-size decision; resolved round-2 open item 2 only for
  A5's large-coordinate part. Updated current policy tables, F04/F06 links, counts and proposed
  read-only reconciliation. Rounded-mesh cross-checks and subnormal mass remain open. The five
  authorized documents/case files alone change; Git metadata, code and dependencies are untouched.

### Local-size amendment checks

Cargo ran with `CARGO_NET_OFFLINE=true`; no network access was used. These are scaffold checks,
not execution of the logical acceptance descriptions or evidence of a working geometry kernel.

| Command | Exact result |
|---------|--------------|
| `python -c "import json; [json.load(open(p, encoding='utf-8')) for p in ('docs/contracts/model-command-cases-v0.1.json','docs/contracts/geometry-contract-cases-v0.1.json')]; print('json ok')"` | Exit 0; `json ok` |
| `git diff --check` | Exit 129; `warning: Not a git repository. Use --no-index to compare two paths outside a working tree`; no usable diff check. Earlier `git status` refused dubious ownership; no workaround attempted |
| `cargo fmt --all -- --check` | Exit 0; no output |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0; no warnings |
| `cargo test --workspace` | Exit 0; 120 tests passed, 0 failed |
| `python target/tolerance-magnitude/verify.py` | Exit 0; `verification ok`: unique/contiguous IDs, counts, references, F04 codes, links/anchors, 100-column prose, LF/no trailing whitespace, retained original lines, unchanged old cases and exact Fraction/digest derivations passed |
| WASM | Not run; target absent |
| cargo-deny | Not run; tool absent |

The independent before/after text comparison confirms no whole-file rewrite. Git branch, HEAD and
the PR #26 stack cannot be verified under the ownership block. Status is **uncommitted**; no Git
write command was run. The design proposals above remain unapplied.

## Technical reservations

The owner-directed policy is applied. A shared epsilon alone cannot establish kernel accuracy. At an
expected coordinate of 1e9 mm, the amended comparison permits 1000.001 mm of error; it is a
deliberately loose comparison policy, not a suitable promised measurement accuracy. Local-size
scaling resolves only the owner-selected large-coordinate policy part. The design's
rounded-mesh and subnormal counterexamples remain unresolved. Exact expected values, topology and
identity checks remain intact. The winding hemisphere and missing roll cap are reviewable
assumptions; neither should be mistaken for a verbatim owner decision.
