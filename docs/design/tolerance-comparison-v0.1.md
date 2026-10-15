# Tolerance comparison and owner decisions, version 0.1.0

Task: record owner decisions 1–6 of 2026-10-15 and compare the tolerances with the public
ifc-lite kernel. Branch: `docs/owner-decisions-2026-10-15`, stacked on PR #23. Round 1
recorded the contour plate, explicit roll and the tolerance policy; round 2 adds the ifc-lite
numbers below. No code or dependency is changed.

Status: documentation only, 2026-10-15. F04 remains a freeze candidate at contract_version 0.1.0;
F06 remains a draft. Nothing is implemented or admitted by this report. Constants are **confirmed by
the owner as first-slice values; draft, to be validated by fixtures**.

This report consumes [F04](../contracts/model-and-commands-v0.1.md),
[F06](../contracts/geometry-contract-v0.1.md), the read-only [first-scope
package](../clean-room/handoff/first-scope-v0.1.md) and [kernel design](kernel-design-v0.1.md). The
package and design retain their earlier policies. Only the two contracts and their cases apply the
owner amendment here.

## Interpretation and classification

The owner's “lower the precision to 1e-6” means relative error 1e-6, with a length floor of 1e-6 m =
0.001 mm. That floor equals the unchanged minimum nonzero dimension. It does not mean 1e-6 mm. This
interpretation relaxes admission/comparison/certificate bounds; it proves no algorithm accurate, and
it does not close the design's full-domain accuracy gate A5.

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
| eps_dim, F04 s.3 / F06 s.4 | G | 0.001 mm | unchanged, 1e-6 m | minimum thickness, edge, section or axis length, inclusive | Supported for distinct construction; no relative mesh accuracy guarantee |
| Coincidence, F06 s.4 | G | distance < 0.001 mm | unchanged | axis endpoints only; same minimum rejection | Supported, no general weld/merge rule |
| eps_par, member default frame, F06 s.4 | G | 1e-6 | unchanged | dimensionless strict horizontal/total length ratio | Supported to avoid tiny denominator; frame correction still required |
| Squared member parallel multiplier | E | 1e-12 | unchanged = (1e-6)^2 | dimensionless, hh < eps_par^2*LL | Square identity, not a certificate or comparison tolerance |
| Near-vertical projection lower bound | E | 1-c^2 >= 1-1e-12 | unchanged | dimensionless mathematical consequence of branch | Not an independently adjustable tolerance |
| Pick band eps_pick, F06 s.9 | G | 1e-6 mm = 1e-9 m | 0.001 mm = 1e-6 m | length, inclusive face/edge/vertex band | Original quantum supported for buckets, tight for robustness |
| Pick tie quantum, F06 s.9 | G | 1e-6 mm | 0.001 mm | deterministic length buckets, not exact equality | Wider buckets are intentional; identity can precede distance within bucket |
| Ray/face parallel threshold, F06 s.9 | G | 1e-12 | 1e-6 | dimensionless abs(unit direction dot unit normal), strict | Original design required certified near-zero handling; no sign guess |
| Ray direction norm minimum, F06 s.9 | G | 1e-9 | 1e-6 | dimensionless normalization input, inclusive | Supported normalization range; direction is not a length |
| Plate planarity, new F04 s.3 | G | absent | 0.001 mm + 1e-6*D | distance to p0 plane, D=max distance from p0 | Assumption; valid in-band nonplanar input is unsupported until A02 |
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

The owner explicitly retained eps_dim and eps_par. Both match the numerical scale 1e-6 after
converting lengths to metres or treating the ratio as dimensionless. Squaring eps_par still gives
1e-12. It would change the branch angle by a factor of 1000 to replace that square by 1e-6.

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
| Versions | contract 0.1.0; algorithm 2, tolerance_policy 2, feature_id_rule 1, mesh_layout 1 | Encoding and behaviour changed; no freeze |
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
| Length floor and `eps_dim`, 0.001 mm = 1e-6 m | plane clipping floor 1e-6 m = 0.001 mm | Equal (metre reading) | Yes; equals the ifc-lite floor and the 1e-6 policy |
| Pick band and pick tie quantum, 0.001 mm | same floor, 0.001 mm | Equal | Yes |
| Coordinate floor of CR-TOL-001, 0.001 mm | same floor, 0.001 mm | Equal | Yes |
| CR-TOL-001 relative part, 1e-6 | half-space cap relative factor 1e-5 of the mesh diagonal | Tighter by 10 | Yes (owner value); the ifc-lite factor is looser |
| Plate planarity, 0.001 mm + 1e-6*D | `PLANE_TOL` 1 mm; magnitude scaling 2^-22 (about 2.38e-7) | Absolute term tighter by 1000 than `PLANE_TOL`; relative term about 4.2 times looser than 2^-22 | Yes; `PLANE_TOL` serves void tests, see below |
| Member parallel threshold `eps_par`, 1e-6 | `PARALLEL_EPS` 1e-9 (2D, dimensionless) | Looser by 1000 | Yes; this is the owner-protected 1e-6 |
| Near-vertical branch, sine of axis angle below 1e-6 (about 5.7e-5 degrees) | `AXIS_THRESHOLD` cosine 0.999847695, sine about 0.01745 (about 1 degree) | Tighter by about 17,000 | Yes; different purpose (axis classification) |
| Ray/face parallel threshold, 1e-6 | `PARALLEL_EPS` 1e-9 | Looser by 1000 | Yes |
| Ray direction minimum norm, 1e-6 | none | No counterpart | Yes (owner value) |
| Quaternion norm admission, 1e-6 | none; `ORTHOGONAL_DOT_TOL` 0.02 is a dot-product bound | No counterpart | Yes (owner value) |
| Frame certificate, absolute 1e-6 on norms, dots and handedness | `ORTHOGONAL_DOT_TOL` 0.02; `PARALLEL_EPS` 1e-9 | Tighter than 0.02 by 20,000; looser than 1e-9 by 1000 | Yes |
| Exact rectangle identities (orthogonality, p3 = p0 + e1) | `ORTHOGONAL_DOT_TOL` 0.02 | BeamCraft is exact, so tighter | Not applicable; exact rational identities are never loosened |
| Roll cap, contour limits, resource caps, hashes | none | No counterpart | Not tolerances |
| Coincidence of axis endpoints, below 0.001 mm | floor 0.001 mm; `POSITION_DEDUP_GRID` 0.1 mm | Equal to the floor; 100 times tighter than the dedup grid | Yes; BeamCraft has no vertex dedup or weld rule |

Findings from the mapping:

- **Floor.** The ifc-lite plane clipping floor 1e-6 m equals the BeamCraft minimum dimension
  0.001 mm. Checked, not assumed: 1e-6 m times 1000 = 0.001 mm. This holds for metre files only (see
  the open unit point above).
- **Parallel.** Their 2D parallel epsilon 1e-9 is a thousand times tighter than the member parallel
  threshold 1e-6. The owner wants 1e-6 and `eps_par` is protected, so it stays. It is a different
  quantity: theirs is a 2D parallel test, ours is the ratio that keeps the default-frame denominator
  away from zero. The squared multiplier 1e-12 is an exact identity and has nothing to do with
  their 1e-9.
- **Magnitude scaling.** Their plane tolerance grows with coordinate magnitude (2^-22 relative);
  the BeamCraft floors are fixed. The open accuracy gate A5 (full-domain accuracy, including
  cancellation at large coordinates) is not closed by a fixed 0.001 mm. *Proposal for the owner,
  no contract change:* adopt magnitude scaling only for geometric decisions (class G and C, as
  planarity already does with D), as `max(0.001 mm, 2^-22 times magnitude)`. At 1e9 mm that would
  be 238 mm, which is unusably large for a coordinate that wide, so the scale term should use the
  local body size rather than the world origin, as planarity does. That addresses the
  large-coordinate part of A5 only. It does not address rounded-mesh error or subnormal mass, and
  the test rule CR-TOL-001 and the pick band should remain absolute. The decision is the owner's;
  the contract is unchanged.
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
policy or an exact identity. No contract value was changed by this comparison. The "take the same
ones" instruction is met where ifc-lite has a counterpart that agrees (the floor) and is overridden
by the explicit "lower the precision to 1e-6" where it does not (relative bounds, parallel
thresholds). That reading is the policy recorded here; the open items are the metre unit of the
floor and the magnitude-scaling proposal.

## Proposed policy

1. The owner's decision is the policy: relative 1e-6, length floor 1e-6 m = 0.001 mm, applied to
   classes T, G and C only. Class E identities are never loosened.
2. Where ifc-lite has an agreeing counterpart (the floor), the contract equals it. Where it has a
   different or looser value for another purpose (1 mm touch and plane tolerances, mesh weld
   values), the contract does not adopt it until a task needs that purpose.
3. Re-check each constant against fixtures before the freeze; the values stay "draft, to be
   validated by fixtures" while being confirmed by the owner as first-slice values.
4. Do not add magnitude scaling or any new constant without an owner decision.

## Changed boundaries and cases

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

All existing case IDs are retained. F04 now has 57 logical cases (24 positive, 33 negative) and
seven raw cases. F06 has 96 cases (52 positive, 44 negative). Amendments and coverage tables in the
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
  0.001 mm+1e-6*D; exactly planar rectangles alone receive first-slice solids. Near-planar polygons
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
  Source-exposure admission, contract freeze, fixture validation and design reconciliation
  remain separate gates.

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
attachments. Algorithm and tolerance versions are 2; mesh layout remains 1.

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
input about local +Z. Algorithm/tolerance versions are 2. Adopt the revised 1e-6
comparison/admission/certificate bounds and 0.001 mm length floor without changing exact
references, eps_dim, eps_par or mathematical error bounds. Prefer suitable audited permissive Rust
crates, including for bounded robust arithmetic; handwritten integer/sign algorithms are justified
only when no suitable crate exists. Reconcile the matrix with 96 F06 cases. Keep full-domain
accuracy, subnormal mass and cross-target trig conformance open until proved. The ifc-lite numeric
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

The ROADMAP progress-log entry and the ATTRIBUTION row for this report and the amended
contracts are applied directly (round 2), so they are no longer proposals. UI asset and patch
rows wait for their own original-source audits; the architecture paragraph adds no dependency.

## Checks and delivery

Round 1 changed the six contract, case, architecture and report files. Round 2 changed this report,
`ROADMAP.md` and `ATTRIBUTION.md`, and replaced the stale "comparison blocked" wording in
`docs/architecture.md`, F04, F06 and the F06 case file. No contract value was changed. Scratch
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

## Technical reservations

The owner-directed policy is applied. A shared epsilon alone cannot establish kernel accuracy. At an
expected coordinate of 1e9 mm, the amended comparison permits 1000.001 mm of error; it is a
deliberately loose comparison policy, not a suitable promised measurement accuracy. The design's
rounded-mesh and subnormal counterexamples remain unresolved. Exact expected values, topology and
identity checks remain intact. The winding hemisphere and missing roll cap are reviewable
assumptions; neither should be mistaken for a verbatim owner decision.

## Exposure note

Round 1 read this worktree's AGENTS.md, CLAUDE.md, ROADMAP.md, architecture, F04 and F06 with their
case files, the first-scope package and case file, the kernel design, the domain vocabulary and
module boundaries. Round 2 read this report, ROADMAP.md, ATTRIBUTION.md and the contract passages
that mention the comparison. The ifc-lite numbers in this report were supplied by the orchestrator;
the worker did not read, fetch or open the ifc-lite repository in round 2. Round 1 saw only search
excerpts and listings of that repository, and no source file; no code, structure, comment, fixture
or asset was copied. PdfCraft and craft-libs facts come from the orchestrator's supplied Cargo.toml
facts. No commercial-product material, oracle process or client project was accessed.

Earlier training or prior exposure that cannot be established remains unknown. This documentation
task does not declare any implementer eligible or amend the source-exposure admission policy.
