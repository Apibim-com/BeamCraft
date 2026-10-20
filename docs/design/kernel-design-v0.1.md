# G04: first-scope kernel design, version 0.1.0

Date: 2026-10-15. Status: **draft; nothing implemented; no implementer declared eligible**.
Contributor-original design, MPL-2.0. Task-card base:
`29d962e790ffd21ecd9a45143f052d4b4d53d3f7`, originally stacked on F06 PR #16. Round 4 consumes
F06 0.1.0 after correction round 4, task-supplied head `172f4b1`, with 83 cases. Git ownership
validation prevents independently verifying HEAD. This is neither a contract freeze nor a K0 gate.

## 1. Scope and status

The inputs are [F06 0.1.0](../contracts/geometry-contract-v0.1.md), its
[83 acceptance descriptions](../contracts/geometry-contract-cases-v0.1.json),
[F04 0.1.0](../contracts/model-and-commands-v0.1.md), the
[first-scope package 0.1.0](../history/clean-room/handoff/first-scope-v0.1.md), its
[cases](../history/clean-room/handoff/first-scope-cases-v0.1.json), and the
[admission policy](../history/clean-room/admission-policy.md). F04 and F06 were frozen at 0.1.0 on 2026-10-16 (P-1, P-2 and P-3 confirmed by the owner;
P-3c is erratum E-2); renewed admission is still pending.
This design identifies contradictions; it does not silently repair those contracts.

The only solids considered are a rectangular plate and a straight rectangular prism member,
with rigid placement. Results are volume, surface area, local/world axis-aligned bounds, member
length and mass with explicit density. Analytic values describe the ideal solid defined by the
submitted f64 parameters, not an exact representation of arbitrary real input or a rounded mesh.
This is **not an exact B-rep**. The proposed shell is a bounded derived representation.

- **S1: mathematics.** Adds: Finite scalar/vector operations, error enclosures, rigid pose,
  predicates. Gate and exclusions: No solids; independent arithmetic tests first.

- **S2: profile/extrusion/triangulation.** Adds: Rectangle, two end sections, six quads split into
  twelve triangles, semantic roles. Gate and exclusions: Only the two boxes; use corrected F06's
  projected frame and certificate. Joint F04 P-3 confirmation and admission remain open.

- **S3: topology/measurements.** Adds: Incidence checks, construction certificate, analytic
  quantities and mesh cross-checks, bounded box queries. Gate and exclusions: API then CLI then real
  E2E tests; no general mesh certification.

- **S4: pilot cuts.** Adds: A later, separately admitted experiment may add one plane clipping one
  convex box and a planar cap. Gate and exclusions: Not authorized by this draft; new topology,
  naming, bounds and cases required.


S4 is a boundary for planning, not an operation specified here. It must define coplanar/tangent
and empty results, orientation, volume conservation, work limits and FeatureId loss before code.
It does not authorize arbitrary booleans.

Explicitly `unsupported`: nonrectangular profiles, catalogues, curves, multi-segment axes, contour
or bent plates, holes, cuts (including S4 today), fits, chamfers, fillets, weld/bolt geometry,
assemblies as solids, disconnected/cavity/nonmanifold solids, arbitrary imported meshes,
foreign geometry conversion, scale/shear/reflection, parent transforms, general polygon
triangulation, topology healing, snapping, exact B-rep, drawings and persisted model formats.
USD remains the project's authoritative format; these documents introduce no saved-model format.

### Mathematical references and use

These are references to mathematics, not asset or source-code imports. No upstream text, figure,
software, test or numerical oracle output is included.

- [IEEE 754-2019](https://standards.ieee.org/ieee/754/6210/), arithmetic formats and operations:
  binary64 and round-to-nearest-even. Only the publisher's public description was accessible;
  the full normative text was not obtained.
- David Goldberg, *What Every Computer Scientist Should Know About Floating-Point Arithmetic*
  (1991), sections "Relative Error and Ulps", "Exactly Rounded Operations" and "Denormalized
  Numbers", [author's paper reprint][goldberg]:
  roundoff, cancellation and gradual underflow. The inequalities below are derived here.
- Jonathan Richard Shewchuk, *Adaptive Precision Floating-Point Arithmetic and Fast Robust
  Geometric Predicates* (1997), Discrete & Computational Geometry 18, 305-363,
  [author's publication page](https://www.cs.cmu.edu/~quake/robust.html): determinant signs can
  be wrong near zero in ordinary floating point. The page was read; paper downloads failed.
  No linked software was opened. No algorithm or implementation is adopted from that software.
- OpenStax, *Calculus Volume 3*, section 6.8, theorem 6.20,
  [divergence theorem][divergence]:
  the signed tetrahedron identity below is derived by applying flux to a triangle shell.
- Eric W. Weisstein, [Euler Characteristic](https://mathworld.wolfram.com/EulerCharacteristic.html)
  and [Polygon](https://mathworld.wolfram.com/Polygon.html), reference mathematics, consulted
  2026-10-15: Euler invariant and elementary planar triangulation. Only convex quadrilaterals
  are triangulated here; their diagonal proof is given below.

## 2. Representation

Keep immutable validated analytic parameters authoritative: plate dimensions, or the two member
axis points and rectangle dimensions; separately keep the normalized rigid pose and optional
density. Keep derived frame vectors, eight local corners, twelve indexed triangles and six face
roles. These are conceptual in-memory values, not a new schema or module layout.

For the plate use F06's P-1 box. For the member use its P-2 centred section and endpoints. Given
a certified orthonormal frame X, Y, Z and positive dimensions a, b, c, the map from the unit
cube is `o + a*s*X + b*t*Y + c*u*Z`, with `s,t,u` in `[0,1]`. Its determinant is positive
`a*b*c`; hence it is injective, convex and orientation preserving. This construction is the
certificate of a single embedded solid in ideal mathematics. F06's computed frame passes a
finite residual gate, not exact orthonormality; account for its construction error using
section 3 enclosures and W41. Rounded corners still require the checks below.

Use the exact face ordering, role names, vertex enumeration and diagonal choices of F06 section
6, with two triangles mapped to each `f.x0` through `f.z1` role. The faces carry FeatureId roles;
the engine qualifies them with ElementId. Twelve geometric edges and eight vertices are named
by intersections of those roles. Diagonals have no engineering FeatureId. All addresses outside
a snapshot use F06 FeatureIds; no triangle index becomes an address or an identity.

In a convex planar quad `[a,b,c,d]`, diagonal `ac` lies inside it and separates triangles `abc`
and `acd`; their interiors are disjoint, their common diagonal cancels in the oriented boundary,
and their union is the quad. This proves sufficiency without general triangulation. The shell
has 8 vertices, 18 triangle edges, 12 triangles: `8-18+12=2`. Removing six diagonals gives
`8-12+6=2` (W01). Each undirected triangle edge must occur twice with opposite directions.
Euler alone proves neither closedness, outward winding nor absence of self-intersection.

### Analytic versus shell checks

Analytic volume and area never come from bounds. Measure the shell independently:

1. Check all addresses, distinct triangle corners, directed incidence and one connected shell.
2. Check signed face orientation against the analytic role and nonzero triangle area.
3. Check the interval volume and area against the analytic enclosures, including vertex rounding.
4. Check each rounded vertex lies within its analytic corner enclosure. Bound corner displacement
   as in section 3. Reject any lost embedding certificate, including collapsed thin edges.

With an anchor r inside the analytic box, let `a=v0-r`, `b=v1-r`, `c=v2-r`. For a closed outward
shell, divergence with the vector field `x/3` gives
`V_mesh = sum(det(a,b,c))/6`. This also follows by summing signed tetrahedra with common apex r.
The area is `sum(norm((v1-v0) cross (v2-v0)))/2`. Exact rational tetrahedra in the independent
test oracle and analytic `a*b*c` are different methods; they share no production helper.

For each determinant use its six signed triple products, accumulate twelve determinants in a
fixed order with a correction term for lost low parts, and retain an enclosure from section 3.
Compensation improves the central estimate, not its certified sign. Exact dyadic integer sums
are reserved for discrete signs and independent tests. A possible future implementation of
compensated checks still needs explicit admission of auxiliary arithmetic under A2; corrected
F06 section 2 fixes analytic output order and written binary64 decisions.

A triangle shell cannot express analytic curves, general face intersections, persistent cut
history or exact adjacency after arbitrary edits. Construction certification applies only to
our two primitives. An externally supplied closed shell never receives engineering quantities.

## 3. Numeric model

### 3.1 Domain, order and elementary error model

Only finite f64 inputs and outputs; internal length mm, area mm2, volume mm3. Report area by
division by exact integer `1000000`, volume by division by `1000000000`. Corrected F06 section 2
fixes mass order as `fl(fl(V_mm3*density)/1e9)`, not multiplication of reported volume in m3.
Avoid rounded `1e-6`/`1e-9` multipliers adding another constant error.
These divisors are exactly representable integers. No fast-math, reassociation, contraction to
FMA, extended intermediates or flush-to-zero. Use F06 validation and component order, its
norm parenthesization, quaternion normalization/matrix and canonical sign, then its vertex
construction. Order the world dot product as `((Rk0*x + Rk1*y) + Rk2*z) + tk`.
New interval/check arithmetic is auxiliary; it never changes a branch silently.

Let `u=2^-53`, `eta=2^-1074` (smallest positive subnormal) and `g(n)=n*u/(1-n*u)`.
For a normal operation with normal exact result, `fl(z)=z*(1+d)`, `|d|<=u`. Uniformly, including
underflow, `|fl(z)-z|<=u*|z|+eta/2`. A chain of n normal multiplicative roundings has error at
most `g(n)`: expand the product of `(1+d_i)` and bound it by the geometric series in `n*u`.
Positive sums inherit the maximum term bound rather than a cancellation factor.

Inputs are the submitted binary64 values. Decimal-to-binary conversion is separate and can be
included by starting reference intervals at the exact decimal rational and enclosing conversion.
Subtracting close submitted f64 coordinates does not restore information lost before input.
It is often exact (Sterbenz cancellation), but never assume decimal intent was preserved.

### 3.2 Length, volume, area and mass derivations

For member length, set exact differences D from submitted points; compute d by three subtractions.
For a normal dominant component, each squared term has at most three rounding factors (two from
d and one from squaring). Two positive additions give a squared-length relative bound `g(5)`.
Square root and its rounding give
`sqrt(1-g(5))*(1-u) <= Lhat/L <= sqrt(1+g(5))*(1+u)`.
This is enclosed by `1 +/- g(6)`. Tiny underflowing components use section 3.3 enclosures;
the minimum admitted L keeps the norm itself normal. No relative bound is asserted for an
individual zero/tiny component. Their additive contribution is retained in the enclosure.

For volume `((a*b)*c)/1e9` has three roundings: plate bound `g(3)`. For a member include the
length bound to obtain `g(9)` (six length factors plus three).

For area evaluate `p=fl(a*b)`, `q=fl(a*c)`, `r=fl(b*c)`, `s=fl(p+q)`, `t=fl(s+r)`, then
`Shat=fl((2*t)/1e6)`. This is **three rounded products, two rounded additions and one rounded
division: six rounded operations**. Multiplication by 2 is exact in this range; it removes
none of those six roundings. Use the conservative plate-area bound **`g(6)`**. To see its
sufficiency, expand the positive sum: each exact area term acquires only a subset of the six
rounding factors. Each coefficient therefore lies within `1 +/- g(6)`, and positive weighting
preserves that enclosure. Underflow contributions still require section 3.3's absolute terms.

For a member take `c=Lhat`, with `Lhat/L` in `1 +/- g(6)`. The area expression still executes
six rounded operations, but a term's coefficient follows at most four: its product, two
additions and division. Specifically `a*b` and `a*Lhat` each traverse four roundings, while
`b*Lhat` traverses three. Thus each coefficient is in `1 +/- g(4)`. Only the latter two terms
also carry the length error; it is not squared or counted twice by adding positive terms.
Their combined factors satisfy
`(1-g(6))*(1-g(4)) >= 1-g(10)` and `(1+g(6))*(1+g(4)) <= 1+g(10)`.
Consequently the member-area bound remains **`g(10)`**. This termwise proof, rather than adding
six total area operations to six length factors, justifies retaining that bound.

Mass order is `(((a*b)*c)*density)/1e9`: three products and one division, so normal bounds
remain `g(4)` plate and `g(10)` member including six length factors. These bounds depend on
volume, not area; reported volume and mass must each follow their own F06 expression.

The following conservative bounds are relative to the ideal binary-input primitive. Add the
tiny absolute underflow terms through the recurrence in section 3.3 when they occur.

| Quantity | Normal relative bound | Worked absolute upper bound at maximum |
|----------|-----------------------|-----------------------------------------|
| Member length | `g(6) < 6.662e-16` | less than `6.662e-10 mm` at `1e6 mm` (W02) |
| Plate volume | `g(3) < 3.331e-16` | less than `3.331e-7 m3` at `1e9 m3` (W03) |
| Member volume | `g(9) < 9.993e-16` | less than `9.993e-7 m3` at `1e9 m3` (W04) |
| Plate area | `g(6) < 6.662e-16` | less than `3.9972e-9 m2` at `6e6 m2` (W05) |
| Member area | `g(10) < 1.111e-15` | less than `6.666e-9 m2` at `6e6 m2` (W06) |
| Plate/member mass | `g(4)` / `g(10)` | less than `0.4441` / `1.111 kg` at `1e15 kg` (W07) |

W05 exactly gives `g(6)=3/4503599627370493`; multiplying by the maximum `6000000 m2` gives
`18000000/4503599627370493 m2`, below the tabulated `3.9972e-9 m2`. W06 retains its `g(10)`
bound after the coefficient-factor derivation above. Neither reference area in W08 or W09
changes: this correction changes the error budget, not the analytic formulas or exact areas.

W08: a `1000*500*10` plate has `V=5000000 mm3=0.005 m3`,
`S=2*(500000+10000+5000)=1030000 mm2=1.03 m2`; at density `1000`, mass is `5 kg`.
W09: member `200*300*4000` has `V=240000000 mm3=0.24 m3`,
`S=2*(60000+800000+1200000)=4120000 mm2=4.12 m2`; at density `2000`, mass is `480 kg`.
All are exact rational references, not measurements of implemented code.

Density has no positive lower bound in F04. Thus normal relative mass bounds are conditional.
W10: density `eta` with volume `1e-18 m3` gives ideal mass `eta*1e-18`, which rounds to zero.
With F06's corrected mass order the density product and final division can each underflow.
Their absolute contribution is at most `(eta/2)*(1+(1+u)/1e9) < eta` (W10), in addition to
`g(10)*m` and any propagated input/length enclosure error; use section 3.3 for the actual input.
Never present zero as the current mass of a positive-density solid. Return a numeric diagnostic
and no sealed result if the required relative certificate is impossible. F06 now supplies
`GEO_NUMERIC_UNCERTAIN`; A5 still leaves the underflow certificate/accuracy requirement open.
This design proposal must not reject an authored density by a new
undocumented minimum. Absent density remains `unknown`, not zero and not a numeric failure.

### 3.3 Bounds, frames, dot/cross products and enclosures

Cancellation makes relative coordinate bounds inappropriate. Use absolute radii. If computed
centres A, B enclose ideal values with radii eA, eB, derive bounds as follows, rounding every
nonnegative radius upward (next representable value) after every operation:

```text
z = fl(A +/- B): eZ = eA + eB + u*(abs(A)+abs(B)) + eta/2
z = fl(A*B):    eZ = abs(A)*eB + abs(B)*eA + eA*eB + u*abs(A*B) + eta/2
z = fl(A/B):    eZ = (eA + abs(A/B)*eB)/(abs(B)-eB) + u*abs(A/B) + eta/2
```

The quotient formula requires `abs(B)>eB`; otherwise return uncertain. The exact unrounded
`A*B` and `A/B` in the rounding terms are bounded upward by enclosing operations, not computed
with an unchecked f64 expression. For sqrt form the nonnegative interval `[A-eA,A+eA]`, take
sqrt of each endpoint and widen each by one representable step. A strictly negative lower
endpoint caused by a square-sum enclosure is intersected with its proved nonnegative domain;
an unproved negative radicand is a failure. `min`/`max` of endpoints over eight corners gives
enclosures of the ideal bounds; extrema themselves add no arithmetic rounding.

These formulas follow by expanding `(A+da)*(B+db)`, by the triangle inequality for addition,
and by writing the quotient difference over `B*(B+db)`. They are a per-input derived error
bound for **every local and world bound**, even at cancellation. Identity plate local bounds
are the input dimensions exactly; other bounds propagate through F06's frame, vertex and pose
expressions. No unproved global coordinate accuracy is substituted for this recurrence.

For a dot product, three products and two additions give
`|fl(a dot b)-a dot b| <= g(3)*sum(abs(ai*bi))` for normal fixed inputs. Each cross component
is two products and a subtraction, bounded by `g(2)*(abs(ai*bj)+abs(aj*bi))`. Include input
radii using the recurrence. A triple product determinant uses six triple products and five
additions; bound by `g(7)*sum(abs(triple_products))` for fixed normal coordinates, or use the
recurrence when differences/underflow occur. Near cancellation, an absolute enclosure containing
zero cannot decide a sign. Compensation does not remove coordinate construction error.

For mesh cross-checks apply the recurrence to the triangle area and tetrahedron expressions
using corner radii, then sum enclosures. When using the table's normal analytic area bound,
set `k=6` for a plate and `k=10` for a member. Because that bound is relative to ideal area S,
`abs(Shat-S) <= g(k)*S` implies `S <= abs(Shat)/(1-g(k))`; the analytic radius must therefore
be at least `g(k)/(1-g(k))*abs(Shat)`, evaluated with upward rounding. Add any applicable
input-conversion and underflow contributions. A computed-centre radius `g(k)*abs(Shat)` alone
does not follow from the ideal-relative bound. The plate certificate must use `g(6)`, never
the former plate-area `g(4)` budget. The analytic and mesh intervals must overlap, their centres
must differ by at most the sum of their radii, and the independent topology/embedding tests
must pass. Overlap alone is not a certificate of correctness: a broad interval is a
diagnostic if it cannot establish positivity or preserve the construction's disjoint faces.
A rounded face can be slightly nonplanar; picking uses the ideal analytic face, never an
arbitrary plane fitted to rounded triangles.

### 3.4 Range, underflow and determinism

W11: the admitted edge range gives volume from `1e-9` to `1e18 mm3`, reported from `1e-18`
to `1e9 m3`; area from `6e-6` to `6e12 mm2`, reported from `6e-12` to `6e6 m2`.
Before rejecting an overlong axis, each difference is at most `2e9`; its squared sum is at
most `3*(2e9)^2=1.2e19` (W12). No overflow is possible there. After admission, frame denominators
on the horizontal branch have `h/L` near or above `1e-6`. Including rounding at the
branch boundary, `h*L > 9e-13 mm2` is a conservative lower bound at minimum L (W13).
Quaternion norm is near one; normalization cannot divide by zero. Subnormal quaternion/axis
components can exist and intermediate products can underflow: keep gradual underflow and the
absolute `eta/2` term. Never flush small nonzero input to zero. F04 parser underflow-to-zero
rejection remains an IO responsibility and is not relaxed.

An ideal member corner has norm at most `sqrt(3)*1e9 + 1e6`, using the axis endpoint norm
and two half-section offsets; it is less than `1.734e9`. Each world component is less than
`2.734e9`, below the `4e9` query cap (W14). Include frame construction/certificate radii when
bounding computed corners; the ideal-frame estimate alone is not a finite-precision certificate.
Fixed-coordinate determinant expressions have magnitudes far below binary64 overflow, but very
small terms require interval or dyadic treatment. Reject any nonfinite intermediate as
`GEO_NUMERIC_UNCERTAIN` rather than publishing infinity or NaN.

For one algorithm version, build profile, rounding environment and input, sequential iteration
and fixed operation order give repeatable bytes. Test `q` and `-q` with signed-zero normalization.
No claim of identical bytes across untested compilers, profiles, targets or floating-point
environments is made. F06's stronger cross-platform statement requires conformance evidence;
A2 records that gate without weakening any input or identity invariant.

## 4. Predicates, tolerance and failure contract

Only the following predicates are needed; do not add incircle, arbitrary boolean intersection
or general point-in-polyhedron routines for this scope.

- **Rectangle orientation.** Mathematical definition: Sign of the 2D determinant of two consecutive
  edges. Certification: Positive analytic dimensions prove sign; rounded coordinates checked by
  enclosure/exact dyadic sign.

- **Outward triangle.** Mathematical definition: Sign of cross(edge, edge) dotted with outward
  analytic role. Certification: Interval excludes zero, or bounded exact sign for dyadic
  coordinates; no sign guess.

- **Parallel branch.** Mathematical definition: `hh < tau*LL`, `tau` the submitted binary64 constant
  `1e-12`. Certification: Corrected F06 section 2 requires its written binary64 comparison.
  Inclusive limits are exact as written; no unspecified band remains. Reference arithmetic does not
  override that decision.

- **Closed shell.** Mathematical definition: Integer index range, two opposite edge occurrences, one
  connected component. Certification: Exact combinatorics; no length tolerance, no welding.

- **Point in box/face.** Mathematical definition: Six signed analytic half-space inequalities; face
  uses the other two coordinate intervals. Certification: Certified intervals; equality is boundary,
  unresolved sign is uncertain.

- **Ray front/parallel.** Mathematical definition: With normalized u and outward unit n, skip a
  face when `abs(u dot n) < 1e-12`; otherwise the sign determines front/back facing. Certification:
  use the written comparison, then enclose the quotient; a failed certificate gives no hits.

- **Pick radius.** Mathematical definition: Distance to the closed feature is at most
  `fl(radius_mm + eps_pick)`, with `eps_pick=1e-6 mm`. Certification: F06's written distance
  comparison decides; a squared-distance enclosure is an auxiliary check, not a replacement
  operation order or a silently changed boundary decision.


For a box face ray intersection solve F06's plane equation, restrict to `[0,t_max]`, and test
both closed in-plane intervals enlarged by eps_pick in each direction. Flag `boundary` when
on the original boundary or in the enlargement. Report every qualifying non-parallel face;
coplanar parallel faces never hit. Point picks use distance to the nearest point of each
original closed face rectangle (both front and back), not distance to the solid or its enlarged
box; interior points can have positive distance to every face. Accept point-face, edge and
vertex distances through `radius_mm + eps_pick`. New independent cases are in section 5.4.
For edges, squared distance is a convex quadratic over the closed feature segment and the
ray parameter domain. Interior stationary points and relevant domain boundaries exhaust its
minima. Applied A10 fixes the domain to the closed finite segment `[0,t_max]` for every ray
target. For vertices, clamp `(v-o) dot u` to that interval; for edges, minimize over both
closed segments, including endpoints. Parallel overlaps use the smallest minimizing t. The
distance band applies at both ends without extending the parameter interval; faces keep their
exact t interval test. W49 verifies endpoint inclusion and short-horizon rejection. These are
mathematical query requirements, not a general intersection engine. Face/edge/vertex results
are resolved by analytic FeatureId roles.

For integer sign fallback, a finite f64 is an integer times `2^-1074`. At query coordinate
cap `4e9 < 2^32`, its aligned numerator has fewer than 1107 bits; a difference uses at most
1108 bits. A degree-three determinant plus its six-term sum needs fewer than 3330 bits (W15).
A proposed 4096-bit signed fixed buffer suffices for those **dyadic degree-three signs only**.
Use checked limb operations; there is no variable-precision library or dependency. It does not
certify determinants of ideal irrational frame coordinates or arbitrary rational expressions.
For those use enclosures and return uncertain if they cannot decide. This fallback is a proposal
requiring A2, not permission to replace F06's declared arithmetic or branch result.

Return one of positive/negative/zero/uncertain internally. Exact zero is not uncertain. There is
no epsilon sign invented from CR-TOL-001. Use no adaptive unbounded refinement: one enclosure
pass and at most one bounded integer sign pass. No healing, best guess or perturbed inputs.

F06 validation order and existing codes remain authoritative. Invalid input, cap failure and
cancellation follow F06 sections 2, 3 and 10. Numeric certificate failures now use
`GEO_NUMERIC_UNCERTAIN`, mapped to F04 `UNSUPPORTED_GEOMETRY`, for both build and query, with no
partial/current data and no state/generation change. Frame failures occur at `build`
(F06-NEG-31); query intermediate faults return no hits (F06-NEG-32). Structural self-check
failures retain `GEO_SELF_CHECK_FAILED` or the specific mesh code. F06-NEG-18 now correctly
makes mass `unsupported` when volume is unsupported, even with absent density. Unknown density
on supported geometry remains a separate quantity status. Auxiliary certificate proposals in
this design do not declare that the unresolved full-domain accuracy gate A5 has passed.

## 5. Verdicts on every F06 draft constant

Here **supported** means numerically/resource justified for the stated use, **loose** means
larger than this scope requires, and **tight** means insufficient for a claimed guarantee.
No verdict is permission to change a contract. Draft amendments are listed in section 9.

### 5.1 Dimensions, coincidence and roll

- `eps_dim=0.001 mm` inclusive: **supported for distinct construction**, not for relative mesh
  quantity accuracy. W16: spacing at `1e9` is `2^-23=0.00000011920928955078125 mm`; below `4e9`
  it is `2^-21=0.000000476837158203125 mm`. A minimum edge spans at least `2097.152` of the
  latter spacings. A unit edge has a component at least `1/sqrt(3)`, still many spacings.
  Preserve the minimum; do not infer that world-mesh dimensions have `1e-12` relative accuracy.
- Coincidence `distance < 0.001 mm`, axis endpoints only: **supported** as the same declared
  minimum-length rejection, not as a merge rule. W17: `0.0009 < 0.001`, and `0.001` equals the
  literal minimum and is accepted for the axis-aligned case. Corrected F06 section 2 now fixes
  the written binary64 boundary comparison; it is not an ideal-real comparison or a merge rule.
- Section cap `1e6 mm`: **supported**; W11 bounds products/area safely. W18: at width and height
  `1e6` and L `1000`, volume is `1e15 mm3=1e6 m3`. Do not raise the cap.
- Parallel threshold `eps_par=1e-6`, squared multiplier `1e-12`, strict inequality: **changed
  from insufficient as a rigid-frame rule to supported as a branch policy with certification**.
  Corrected F06 section 4 projects +Y in the near-vertical branch, normalizes `X0=Y cross Z`,
  then recomputes `Y=Z cross X` in both branches. Exact vertical still uses +Y. The former
  `1e-9` relative exclusion band is removed, not retained as a loose operating constant.
  W19: `g(10)<1.111e-15<1e-9` still explains why that old band was unnecessarily large;
  F06-POS-46 now tests the specified rounded decision closer to the threshold.

W20 and W21 describe the **superseded fixed-Y rule only**. With `d=(0,0.0005,1000)`, its
horizontal squared ratio is `1/4000000000001 < 1e-12`. Fixed Y would give `Y dot Z=0.0005/L`,
nonzero, a relative determinant deficit `1/4000000000001` (about `2.5e-13`), and about `0.5 mm`
axial projection at height `1e6`. Those are regression counterexamples, not defects remaining
in corrected F06. The projected replacement and current certificate are derived in W40/W41.
The roll remains discontinuous; its jump depends on signed tilt direction (applied A7), not
universally on a quarter turn. From +Z, +Y gives 180 degrees, -Y gives 0, and +X/-X give
+90/-90 degrees. Joint F04 P-3 confirmation remains open under F06 section 12.

### 5.2 Section 6 resource caps

All caps remain enforced, including exact-at-cap acceptance by a structural header validator.
Structural validation is not permission for a primitive builder to emit larger solids.

- **4096 vertices.** Verdict: loose for 8-corner scope; supported validator limit. Worked resource
  evidence: 512 times 8 (W22).

- **8192 triangles.** Verdict: loose for 12 triangles; supported validator limit. Worked resource
  evidence: greater than 682 times 12 (W22).

- **1024 faces.** Verdict: loose for 6 roles; supported validator limit. Worked resource evidence:
  greater than 170 times 6 (W22).

- **12288 topological edges.** Verdict: loose for 12; supported validator limit. Worked resource
  evidence: 1024 times 12 (W22).

- **16 shells.** Verdict: loose for 1; supported validator limit. Worked resource evidence: 16 times
  1; never certifies multiple shells (W22).

- **17408 features.** Verdict: loose for 26; supported validator limit. Worked resource evidence:
  `4096+12288+1024=17408` (W23).

- **213024 blob bytes.** Verdict: supported, exact simultaneous count ceiling. Worked resource
  evidence: `32+98304+98304+16384=213024` (W24).


W25: first-scope blob is `32+192+144+24=392 bytes`. Corrected F06 sections 6 and 10 distinguish
these exact primitive counts from general structural caps. W26's `17408*64=1114112` is only a
hypothetical all-feature-string payload, not the actual six-face descriptor. F06 now bounds
that descriptor at 32768 encoded UTF-8 bytes; the derivation and verdict are W44 below. Page
control bytes, tokens and strings are checked separately against negotiated F05 budgets;
fitting a blob does not establish delivery feasibility.

### 5.3 Pick constants and comparison

- **Origin/point absolute cap `4e9 mm`.** Verdict: supported. Numeric or resource evidence: W14
  world corner bound is below it.

- **Direction component cap `1e9`.** Verdict: supported. Numeric or resource evidence:
  `3*(1e9)^2=3e18` squared norm cannot overflow (W27).

- **Direction norm minimum `1e-9`.** Verdict: supported as normalization range. Numeric or resource
  evidence: minimum squared norm `1e-18` is normal; scalar input scale does not set geometric
  resolution (W27).

- **`t_max` default/maximum `4e9 mm`.** Verdict: supported finite-segment horizon for every
  ray target, tight for all admitted origin-to-body pairs. Applied A10 resolves the former
  edge/vertex-domain qualification. Numeric or resource evidence: `-4e9` to `+1e9` requires
  `5e9 mm`; this cap is a search horizon, not universal reach (W28). The endpoint distance band
  never extends the parameter interval (W49); full-domain accuracy remains open under A5.

- **Radius default 0.** Verdict: supported with certified boundary decisions. Numeric or resource
  evidence: **changed from an exact-boundary interpretation to supported band acceptance**;
  distances up to eps_pick qualify even when positive. W42 quantifies the band.

- **Radius maximum `1e6 mm`.** Verdict: supported. Numeric or resource evidence: radius squared
  `1e12 mm2`, safely normal (W29).

- **Hits default 16, range 1..64.** Verdict: supported bounded output. Numeric or resource evidence:
  one body has 26 features; 64 need not include all bodies; 65 is rejected (W30).

- **Tie quantum `1e-6 mm`.** Verdict: supported for deterministic buckets, tight as a robustness
  margin. Numeric or resource evidence: at `4e9` it spans only `2.097152` ulps; bucket index `4e15 <
  2^52` (W31).

- **Kind ranks face 0, edge 1, vertex 2.** Verdict: supported discrete policy. Numeric or resource
  evidence: 3 distinct integer ranks; independent of floating point.

- **CR-TOL-001 quantity relative `1e-12`.** Verdict: loose for normal analytic values, tight for
  subnormal mass or rounded mesh quantities. Numeric or resource evidence: the corrected plate
  area `g(6)<6.662e-16` and member area `g(10)<1.111e-15` both remain below `1e-12`, so this
  verdict is unchanged. W10 underflows and W32 loses about `1.4e-4` relative.

- **Coordinate absolute `1e-9 mm` plus relative `1e-12`.** Verdict: supported for listed small
  examples, tight across the full admitted domain. Numeric or resource evidence: W33 gives about
  `2.22e-7 mm` at an ideal zero after cancellation.


W31 uses `1e-6` as exact decimal for the spacing ratio; the contract's f64 divisor is slightly
different. Bucket computation must specify `floor(fl(t / f64(1e-6)))`, not claim equality with
mathematical real quantization. For example adjacent buckets can contain values less than a
quantum apart. If an interval straddles a boundary, exact identity ordering is still repeatable
for identical rounded t, but the order is not robust to small numeric changes. Corrected F06
sections 2 and 9 establish this written floating-point bucket policy; test neighboring
representable values. The tie quantum and eps_pick have equal numeric values but different uses.
For a nonnegative finite quotient below the integer cap, checked truncation to u64 realizes
floor without adding a transcendental arithmetic operation. Face backface inclusion default
`false` is **supported** as a discrete query policy; it has no accuracy implication. The inclusive
lower pick parameter bounds `t=0` and radius `0` remain **supported**; zero radius now admits
positive feature distance through the explicit eps_pick band.

W32: `fl(1e9 + f64(0.001))-1e9 = 0.00100004673004150390625 mm`. For a cube whose three edges
are rounded this way the relative volume error is
`(0.00100004673004150390625/0.001)^3-1`, between `0.0001401` and `0.0001403`.
With only two translated thin edges the relative error is between `0.0000934` and `0.0000936`.
Even one such edge has error between `0.0000467` and `0.0000468`; thus neither the minimum
dimension nor compensated mesh summation establishes `1e-12` relative mesh volume accuracy.
The one-, two- and three-edge bounds here define the example. Mesh checks must use section 3 radii.

W33: let `s=f64(0.7071067811865476)`, q=(0,0,s,s). F06 computes norm square
`1+2^-52` (displayed as `1.0000000000000002`) and sqrt `1`, so `R00=1-2*s*s=-2^-52`.
For local point `(1e9,0,0)`
the ideal normalized rotation has x=0, but the computed x is
`-0.0000002220446049250313080847263336181640625 mm`. An admitted column centred on x=`1e9`
inherits that error in its small world x bounds. CR-TOL-001 near zero permits only `1e-9 mm`.
A5 requests a cancellation-aware coordinate accuracy gate, keeping the existing test rule and
input caps unchanged; no acceptance case is weakened to conceal the issue.

W39: `1e-6 mm=1e-9 m` is one nanometre; `0.001 mm=1e-6 m` is one micrometre. Corrected
F06 section 9 now uses the former unit correctly for both band and tie quantum.

Other fixed constants are discrete layout/protocol policies, not geometric tolerances:
contract 0.1.0, integer versions 1, 32-byte header, 8-byte alignment, u32/u16 indices, offsets
and space tags, FeatureId 64-byte cap (longest current form 59), 26 primitive features,
16 maximum numeric inputs, 32 diagnostics, 64-byte diagnostic field tokens, recursion 0 and
later maximum depth 16, cancellation points P0..P3. **Supported**: W25 establishes current
blob offsets `32,224,368,392`; W34 gives `5+36+1+7+10=59 <=64`, `6+12+8=26` and
`6+2+3+4+1=16` numeric inputs. A capped diagnostic uses fixed schema tokens only. These
constants add no geometric precision and are not approvals for future geometry. Quaternion
norm tolerance `1e-12` is **loose but supported as admission**, since normalization follows;
it is about 9007 unit roundoffs (W35). It is not a permission for a nonrigid rotation matrix.

### 5.4 Corrected F06 rules and new derivations

W40 (near-vertical frame): use the rational Pythagorean construction of F06-POS-41 with
`m=4000001`. Then `dy=2*m/1e8=0.08000002`, `dz=(m*m-1)/1e8=160000.08` and
`L=(m*m+1)/1e8=160000.08000002`. Put `s=4000001/8000004000001` and
`c=8000004000000/8000004000001`: `s*s+c*c=1`, `s*s<1e-12`. The corrected POS-41
squared-ratio approximation is `2.4999987500001562502e-13`; rational cross-multiplication bounds
its relative error below `1e-19`, well inside CR-TOL-001. Projection of U=+Y gives
`P=(0,c*c,-c*s)` of norm c, hence `X=(1,0,0)`, `Y=(0,c,-s)`, `Z=(0,s,c)`.
All squared norms and the determinant equal 1 and all pairwise dots equal 0 exactly.
In ideal arithmetic the general near-vertical branch has `norm(P)^2=1-Z_y^2 >= 1-1e-12`;
rounding requires the actual certificate, not assuming that identity holds exactly for f64 Z.
The small-offset Y-tilted case of W20 likewise now has perpendicular projected Y.
At the same positive Y tilt the horizontal formula gives `X=(-1,0,0)` and `Y=(0,-c,s)`,
exactly opposite to the near-vertical formula's section axes: the branch rotation is 180 degrees.
To cover signed meridians, put `r=sqrt(Z_x^2+Z_y^2)` and `n=sqrt(1-Z_y^2)`. Independently
projecting the two reference directions gives `cos(theta)=-Z_y*Z_z/(r*n)` and
`sin(theta)=Z_x/(r*n)` for rotation from the near-vertical to the horizontal frame about Z.
For `Z=(0,s,c)`, `(0,-s,c)`, `(s,0,c)`, `(-s,0,c)`, the exact (cos,sin) pairs are respectively
`(-1,0)`, `(1,0)`, `(0,1)`, `(0,-1)`: +Y gives 180 degrees, -Y gives 0, +X gives +90, -X gives
-90. Replacing c by -c exchanges the Y-tilt outcomes and retains the X-tilt signs. Thus the
earlier unqualified Y-tilt half-turn wording was too broad; it applies to +Y from +Z. These
are comparisons on one common axis and one-sided limiting roll differences at the threshold.
Frames on distinct finite axes also differ in axis direction. Applied A7 changes the wording,
not the frame formula, the certificate bounds or the supported branch-policy verdict.

W41 (frame certificate): the new residual limit `e=1e-12` is **supported and loose as a
roundoff/fault gate**, not an exact-rigidity or full-domain quantity accuracy promise. For
fixed vectors with squared norms at most `1+e`, Cauchy-Schwarz and section 3 give dot evaluation
error at most `g(3)*(1+e) < 3.331e-16`; a six-term determinant has error at most
`6*g(7)*(1+e)^2 < 4.663e-15`. These bounds concern evaluation of the computed vectors only;
frame construction radii are separate. Thus e is well above ordinary residual evaluation noise,
and the injected dot magnitude `1e-9` is `1000*e`, decisively outside the certificate.
Check squared-norm residuals, magnitudes of all three dots, and handedness. F06 requires
GEO_NUMERIC_UNCERTAIN on failure with no mesh/quantities/key; a successful 1e-12 residual gate
alone does not establish a 1e-12 relative error for an anisotropic mesh. A9 is now applied in
F06 section 4 rule 2 step d; NEG-31 and NEG-35 reject both dot-error signs. W41's bounds and
verdict are unchanged. In NEG-35's exact-decimal reference, injected Y=(-1e-9,1,0) has
norm-squared excess 1e-18 and determinant 1, but abs(X dot Y)=1e-9=1000*e fails. The nearest
f64 fault component also fails the dot limit while passing the norm-residual limit.

W42 (band/boundary policy): `eps_pick=1e-6 mm` is **supported as explicit inclusion policy,
tight as a universal numeric margin**. It spans `2.097152` ulps below 4e9 (W31), so it cannot
absorb every cancellation error left open in A5. For the plate's x interval `[0,1000]`, ray
face inclusion uses `[-0.000001,1000.000001]`, both ends included. A ray hitting x=0 has
boundary true; at x=-0.0000005 it also has boundary true and qualifies under the band.
Parallel neighboring faces remain skipped; all qualifying non-parallel shared faces are kept.
For point picks and edges/vertices the ideal radius threshold is `radius+eps_pick`, followed
by the specified f64 evaluation. At maximum radius its ideal square is
`(1000000.000001)^2=1000000000002.000000000001 mm2`, safely finite. A point at (500,250,5)
in the example plate has distance 5 to z0/z1, not zero merely because it is inside the solid.
**Changed verdict:** radius zero now means band-limited positive-distance inclusion, not exact
contact alone. The radius cap itself remains supported. Boundary semantics are now specified.

W43 (ray parallel threshold): `abs(u dot n)<1e-12` is **supported as a strict classification
policy, tight as an accuracy guarantee**. Zero and either sign below the threshold skip the
face; equality does not. A retained denominator has reciprocal at most `1e12` in the written
arithmetic. For exact unit operands the normal dot bound is `g(3)`; relative denominator
uncertainty near the threshold can approach `g(3)/1e-12 < 0.0003331`. The quotient enclosure
must therefore include denominator sensitivity, not assume the pick band absorbs it. Finite
input does not by itself certify the entire query; failed numeric certificates discard all hits.
This replaces the former exact-zero-only predicate, a **changed predicate/verdict**. It does
not certify a new threshold sufficient for A5 or silently alter F06's written comparison.

W44 (descriptor bytes): the new `32768` byte cap is **supported as an independent encoded-size
budget**. A conservative six-entry face-ID JSON array with 64 ASCII characters per ID takes
at most `6*(64+2)+5+2=403` bytes, leaving `32365` for the remaining descriptor fields. This is
not a proof those fields fit: count the complete UTF-8 encoding before delivery. A hypothetical
1024-entry array at that string bound takes `1024*66+1023+2=68609` bytes, beyond the descriptor
cap. A structurally admissible generic header may therefore lack a deliverable descriptor;
structural acceptance is not transport acceptance. The prior missing-descriptor-budget finding
is **resolved**, and the earlier hypothetical W26 payload is not a first-scope allocation.

W45 (pick work): `65536` elements, `26` features and polling every `1024` elements are
**supported bounded-work policies**. Maximum candidates are `65536*26=1703936`; each full
polling block has `1024*26=26624` candidates, with `65536/1024=64` full blocks at the cap.
Preflight rejects `65537` elements before the feature scan. Keep only the ordered top 64 hits;
never return them before the complete scan. Include cancellation/deadline tests at block
boundaries and the final completion check. **Changed verdict:** Pick now has an explicit work
bound; the old missing-cap finding is resolved. No milliseconds of latency are proved here.

W46 (negotiated limits): **supported**, with every budget checked in its own units. The
first-scope blob needs `392` bytes: negotiated `391` fails, while `392` fits the buffer budget.
Passing that check alone does not pass descriptor/control/token/string/attachment/bundle,
page-count, batch or timeout budgets. Eight maximum-size descriptors already contribute
`8*32768=262144` bytes before page JSON overhead; eight 392-byte blobs contribute `3136` bytes.
Whole-body delivery either satisfies every smaller applicable effective limit or fails before
dispatch with no split/truncation. A smaller deadline remains subject to F05's deadline outcome,
not an invented byte comparison. **Changed verdict:** the missing negotiated-page accounting
finding is resolved by F06 sections 6 and 10; runtime budget enforcement still needs tests.

W47 (corrected case numbers, applied A8): rational endpoint squaring proves
`100.12492197250392 < sqrt(10025) < 100.12492197250394`. The former POS-45 E=`100.124921973`
failed because `E-100.12492197250394 > 1e-12*E`. Its corrected E=`100.12492197250392864`
satisfies the comparison throughout that enclosure; all four equal distances use this value.
Also square `1000.00000000199999` and `1000.00000000200001` around `1000000+0.002^2`:
they enclose the POS-42/POS-10 axis length, consistent with their corrected approximation
`1000.0000000020000000`. Dividing 0.002 and 1000 by these positive endpoints proves the
corrected small component `1.9999999999960000000e-6` and large component `0.999999999998`
satisfy CR-TOL-001. POS-42's squared ratio is exactly `1/250000000001`, consistent with
`3.9999999999840000000e-12`. W40 separately checks the corrected POS-41 squared ratio.
The previously deficient expectations are resolved; the rational enclosures and numeric-budget
verdicts are unchanged, and none of these examples closes A5. F06 cases remain read-only.

W48 (regression for the former signed-certificate ambiguity): take `m=2000000000`, `s=2*m/(m*m+1)`
and
`c=(m*m-1)/(m*m+1)`. The exact vectors `X=(1,0,0)`, `Y=(-s,c,0)`, `Z=(0,0,1)` have unit
norms and `X dot Y=-s < -1e-12`, but a literal signed test `X dot Y <= 1e-12` passes.
The determinant residual is `1-c=2/4000000000000000001 < 1e-12`, so the other stated tests
do not rescue that former signed interpretation. Applied A9 now rejects this frame because
abs(X dot Y)=s>1e-12. F06 section 4 rule 2 step d and NEG-31/NEG-35 explicitly require that
magnitude test. All W48 rational values are unchanged; the counterexample is now a regression
for a resolved ambiguity, not an outstanding defect or a frame emitted by the normal builder.

W49 (finite-segment regression, applied A10): use the plate [0,1000] x [0,500] x [0,10],
`o=(0,-100,10)`, unit direction `u=(0,1,0)`, radius=0, targets edge and vertex.
For vertex `v=(0,0,10)`, squared distance to r(t) is `(t-100)^2`. On the unbounded ray its
minimum is 0 at t=100. On [0,50] it decreases to `(50-100)^2=2500 mm2`, so the minimum
distance is 50 mm, greater than eps_pick=1e-6 mm. Edge x0.z1 has the same segment minimum.
Every feature has y>=0, whereas this segment has y<=-50: no edge/vertex qualifies, exactly
as F06-NEG-36 requires. On [0,100], vertex x0.y0.z1 and its three incident edges x0.y0,
x0.z1, y0.z1 all attain distance 0 at the endpoint t=100; every other edge/vertex is at least
10 mm away. The four hits sort with edges first in that ASCII order, then the vertex, as
F06-POS-47 requires. The written binary64 bucket is `floor(100/1e-6)=100000000` for each.
The inclusive distance band applies to endpoint distances: nearby features beyond an end may
qualify, but reported t stays in [0,t_max]. A10 selects the finite segment; the old unbounded
alternative is retained solely as a regression contrast. The exact distances and prior bounds
are unchanged; the formerly unresolved expected outcomes are now fixed by F06 section 9.

W50 (shared key inputs, applied A11): set algorithm=1 and tolerance_policy=1, as F06 section 7
specifies for 0.1.0. Both fields occur in both shape and world preimages. Each is u16 big-endian:
the pair is exactly `00 01 00 01` (4 bytes) in each preimage. World also encodes the 32-byte
shape key and depends transitively on all shape inputs; the other directly encoded fields are
disjoint. Domain tags `BCGEO-SHAPE` plus NUL and `BCGEO-WORLD` plus NUL first differ at byte
offset 6 (`S` versus `W`), so their complete preimages differ across domains. This does not
assert collision-free SHA-256. The previous disjoint-input wording is resolved; no numeric
value, encoding vector, digest or dependency invalidation changes.

## 6. Failure, allocation and work bounds

The primitive arithmetic work/scratch budgets below are proposals for later code review;
F06's count, byte and Pick limits are contract rules. None is a measured timing guarantee.
Use fixed-size records for validated numbers, corners, tables and analytic results. Reject
unsupported variants before numerical work. Never allocate from an unvalidated count.

- **S1 input/range/pose.** Proposed bound per body/call: 16 scalars; at most 32 diagnostics; 4096
  elementary scalar operations. Cancellation: P0 entry, P1 before build.

- **S2 construction.** Proposed bound per body/call: 8 corners, 6 quads, 12 triangles, 26 roles;
  4096 scalar operations. Cancellation: P2 before checks.

- **S3 primitive checks.** Proposed bound per body/call: 36 directed edge occurrences; pairwise scan
  at most `36*36=1296`; 12 area and volume terms; 100000 scalar/enclosure operations. Cancellation:
  P3 before quantities.

- **S3 quantities/seal.** Proposed bound per body/call: six quantity kinds including optional mass;
  4096 scalar operations. Cancellation: seal after quantities; later cancellation has no effect.

- **S3 blob validator.** Proposed bound per body/call: 213024 bytes, at most 24576 directed triangle
  edges; bounded structural scan. Cancellation: host polls between validated chunks before
  publication.

- **S3 query.** Proposed bound per body/call: 26 feature candidates per body, retain at most 64
  hits. F06 additionally caps the scan at 65536 elements, in ElementId order, and polls
  cancellation/deadline every 1024 elements; no partial result on failure.

- **S4 pilot.** Proposed bound per body/call: no admitted work/allocation budget. Cancellation:
  unsupported until amended.


W36: directed edges at the generic triangle cap are `3*8192=24576`. If topology validation is
later admitted at that cap, an ordered edge-record sort uses at most 15 bottom-up merge passes
(`24576<32768=2^15`), with checked iterative indices and fallible allocation. This is only a
work-budget proposal; structural blob acceptance does not imply general topology certification.
Primitive checking needs no recursion and no general search tree.

S1-S3 arithmetic scratch has a proposed 256 KiB per-body ceiling including enclosures, tables
and one integer predicate workspace: at most 16 simultaneously live 4096-bit signed values
(`16*512=8192` magnitude bytes, W37). Implementers must demonstrate layout/temporary counts
and operation counts within those budgets; exhaustion returns a limit diagnostic, never grows
the buffer. A bounded integer school product uses at most `128*128=16384` 32-bit limb pairs
per multiplication (W38), with checked carry and capacity. At most one fallback per predicate;
total primitive integer work is capped at 10000000 limb-pair steps. If insufficient, fail closed.
Actual operation instrumentation and memory accounting are acceptance work, not checked here.

Corrected F06 sections 9 and 10 now bound Pick at 65536 elements and 26 features per element,
with cancellation/deadline polling every 1024 elements and pre-scan rejection above the cap.
Use a fixed bounded top-64 accumulator rather than allocating all candidate hits; compare its
result to an independent full-sort oracle in tests. W45 derives the work/poll counts. This is
a per-request work limit, not a project element limit. Complete the scan before exposing hits;
expired/cancelled requests return no partial prefix. Polling bounds candidate work between
polls, not measured wall-clock latency. Page/delivery controls bound the other logical queries.

F06 allocation size arithmetic uses checked u64, then fallible conversion to platform size;
count checks precede products, supplied blob length precedes reads, and all readers use checked
accessors/iterators. Match role enums exhaustively; never index from caller-provided integers.
No unsafe alignment casts, no input-derived slicing, no unwrap/expect/panic/todo/unreachable.
Use fallible reservation for variable buffers and propagate failure without state mutation.
Avoid formatting user input into diagnostics. Retain current state outside the sealed candidate.
Recursion depth is **0** for S1-S3; no later traversal exceeds 16 without reviewed amendment.
These structural measures satisfy the later no-panic design requirement, not a proof about code
that does not yet exist. A discovered crash requires a small original regression fixture.

## 7. Independent verification plan

The [validation matrix](kernel-validation-plan-v0.1.json) contains **44 planned rows**:
S1 **10**, S2 **8**, S3 **25**, S4 **1**. Oracle kinds: **analytic 24**, **invariant 14**,
**metamorphic 6**. It references **all 83 distinct F06 cases** (47 positive, 36 negative).
All G04-V01..G04-V44 IDs are retained. No row is added in round 4: G04-V31 now also references
F06-POS-47 and F06-NEG-36. G04-V31/G04-V33/G04-V37/G04-V40 reflect applied A7/A8/A10/A11;
the resolved A7/A8/A10 validation gates are removed. W40/W47/W49/W50 give their exact checks.
Rows can reference several cases; this is not 83 executable tests. S4's reference is an existing
unsupported-geometry boundary case, not a cut acceptance case. All rows are `planned`.
Rows labelled amendment-dependent cannot be marked passed until the cited amendment is resolved.

Expected values must be authored independently of production geometry or mesh tables. Use
exact integers/rationals, rational orthogonal matrices from integer quaternions, integer
Pythagorean triples and bounded rational intervals for square roots (square rational endpoints
to prove enclosure). Distinguish ideal decimal values, their f64 conversion and exact dyadic
inputs. Never take production output or a third-party kernel as expected truth.

Analytic classes: origin boxes, translated boxes, minimum/maximum dimensions, short axes at large
origins, anisotropic boxes, all signed axis directions, horizontal/sloped/near-vertical members,
quaternions with rational ideal matrices, density absent/normal/subnormal. Predicate classes:
exact boundary, one representable step inside/outside, tied distance buckets, tiny nonzero ray
components, exact parallel ray, and f64 input whose intermediate component products underflow.
At each cap exercise below, equal, above and malformed count encodings before allocation.

Invariants: independent incidence enumeration, one shell, Euler 2, outward signs, positive
volume, analytic volume versus exact tetrahedra, independent triangle area versus six analytic
rectangles, bounds containing every corner. Mutations deleting/reversing a triangle, duplicating
an edge or changing a role must make the checker fail; Euler passing is not enough.

Metamorphic classes: translate and rotate without changing scalar quantities; q versus -q
gives identical canonical pose/bytes; reordering test-only triangles preserves sums and semantic
roles; scale all lengths by a positive exactly representable factor within caps, then test
`V/L^3` and `S/L^2` invariance (no runtime scale transform is authorized); change density alone;
repeat build and compare byte identity. Rigid invariance is tested on analytic quantities, and
mesh deviations are bounded by corner enclosures rather than assumed zero.

Negative classes: nonfinite input, malformed/oversized blobs, unsupported profile/transform,
zero/subminimum axis, invalid density/quaternion, unclosed/inward/overlapping meshes, uncertain
predicates, underflowed mass, stale tokens, invalid FeatureId, cap exceedance, cancellation at
every declared point. Verify no current partial result, no generation change on rejection, and
unknown mass remains distinct from unsupported volume. Engine API -> CLI E2E parity must use
actual primitive builders/readers; mocks cannot pass that gate.

The matrix is a specification, not an implementation or test result. Its structural checks and
the scratch rational/decimal calculations validate this document only. Existing repository
tests say nothing about a future kernel. F07's explicit-up member format and its tolerance
rule differ from F06; only a separately reviewed adapter can consume those fixtures.

## 8. Work breakdown for implementers

Task IDs below denote consumers requested by the G04 card; implementation-plan files were not
inputs. The exact allocation between their future cards needs the orchestrator's confirmation.

- **G01.** Receives sections: 1, 3, 4, 6, 7. Must establish; must not infer: Typed finite
  mathematics and enclosures; no general kernel or new dependency.

- **G02.** Receives sections: 1, 2, 3, 5, 7. Must establish; must not infer: Rectangle/profile
  extrusion and canonical triangulation using corrected F06's frame, subject to P-1..P-3
  confirmation/admission; no contours.

- **G03.** Receives sections: 2, 4, 6, 7. Must establish; must not infer: Bounded incidence and
  topology checks for constructed boxes; no general mesh certifier.

- **G05.** Receives sections: 2, 3, 5, 7. Must establish; must not infer: Analytic measurements,
  mass/status handling and independent mesh checks; no bounds-volume substitution.

- **G06.** Receives sections: 1 S4, 4, 6, 7. Must establish; must not infer: Issue new pilot-cut
  requirements/limits/FeatureIds first; no cut implementation from this draft.

- **G09.** Receives sections: 3, 4, 5, 6, 7. Must establish; must not infer: Numeric robustness,
  query/E2E validation and determinism evidence; no claim from mock-only checks.


An issuer may give a fresh implementer only an explicit allowlist of reviewed versions pinned
to actual repository commits: admitted first-scope package/cases, owner-confirmed F04/F06,
independently admitted portions of this draft/matrix, named public mathematical references and
explicitly listed BeamCraft source. Follow admission-policy sections 4, 5 and 8 and the
[task template](../history/clean-room/implementation-task-template.md). No private observations,
restricted summaries, oracle outputs or third-party kernel material may cross the boundary.
This draft is not yet an admitted implementation handoff. No person's eligibility is decided.

## 9. Open decisions, limitations and exposure

### Reconciliation of A1 through A11

Status is checked against F06 0.1.0 after correction round 4 (83 cases), not inferred from its log.
"Partly" includes the still-open portion of the original G04 request; it does not imply that
F06's adopted part is unimplemented contract text. All runtime behavior remains unimplemented.

1. **A1, frame: partly applied.** F06 section 4 rule 2 adopts the projected +Y fallback,
   normalization/re-orthogonalization and 1e-12 frame certificate; section 3 supplies its
   diagnostic. Section 12 explicitly leaves joint F04 P-3 amendment, owner confirmation,
   admission and the continuity decision open. Use the corrected frame now as the draft
   requirement; do not repeat the fixed-Y defect as current. W40/W41 validate its mathematics.
2. **A2, arithmetic: partly applied.** F06 section 2 fixes quantity, mass, dot/world-coordinate
   order, ascending bounds scan, binary64 boundary decisions with no exclusion band, target
   conformance evidence and algorithm bumps for changed outputs; section 7 defines affected
   keys. This changes this design's mass order while preserving its normal error budgets.
   Auxiliary interval, compensated-sum and bounded integer production checks are still not
   specified/admitted by F06's f64-only rule. They remain proposals, not substituted arithmetic.
3. **A3, uncertainty: partly applied.** F06 sections 2, 3, 4 and 9 adopt
   GEO_NUMERIC_UNCERTAIN with UNSUPPORTED_GEOMETRY, atomic discard and no partial hits.
   Section 8 and corrected F06-NEG-18 resolve unsupported-mass precedence and the ninth-triangle
   wording. Section 12's A5 still leaves finite underflow-to-zero mass and its required
   accuracy certificate open; the existence of a diagnostic alone does not define that gate.
4. **A4, picking: applied.** F06 section 9 defines closed face enlargement, eps_pick=1e-6 mm,
   boundary flags/shared-face inclusion, the strict parallel test abs(u dot n)<1e-12,
   nearest-point distance to a closed face and zero-radius band acceptance. Sections 2 and 9
   make bucket arithmetic the written binary64 evaluation; section 12 corrects nanometre units.
   This is a deterministic policy, not proof of full-domain query accuracy. Applied A10 below
   additionally fixes the finite-segment domain for edge/vertex candidates.
5. **A5, accuracy: open.** F06 section 12 explicitly leaves cancellation-heavy coordinates,
   subnormal mass and rounded-mesh cross-check accuracy unresolved. W10/W32/W33 remain relevant.
   No input cap or CR-TOL-001 rule is weakened here, and no conformance claim is promoted.
6. **A6, resources: applied.** F06 sections 6 and 10 separate exact primitive counts from
   validator caps and bound descriptors to 32768 bytes. Sections 9 and 10 bound Pick work and
   polling; section 10 applies smaller negotiated buffer, attachment, bundle, control, token,
   string, batch, delivery and request-timeout limits. Bodies remain whole or fail with
   GEO_BUFFER_LIMIT; cancellation/deadline behavior still follows F05. W44..W46 check the limits.

7. **A7, roll-jump wording: applied with a qualification.** F06 section 4 rule 3 gives the
   signed branch-angle formula and corrects POS-10/POS-42 prose (sections 11 and 12). From +Z,
   +Y gives 180 degrees, -Y gives 0, +X gives +90 and -X gives -90. My proposal's unqualified
   half turn for Y tilt was only partly right: it holds for +Y, not -Y, from +Z. W40 now
   verifies all four signed meridians and distinguishes common-axis branch comparisons from
   distinct-axis finite rotations. F06 adopts direction dependence without changing the roll
   rule; the owner continuity decision and joint F04 P-3 amendment remain open in section 12.
8. **A8, numerical expectations: applied.** F06 sections 11 and 12 and POS-45 now use
   `100.12492197250392864` for sqrt(10025); POS-42/POS-10 use
   `1000.0000000020000000` for L and sufficient digits for the small components and ratios.
   The additional POS-41 squared-ratio correction is `2.4999987500001562502e-13`.
   W40/W47 check all these against exact rational references. F06 chose more digits than the
   minimal POS-45 decimal I proposed; both satisfy the unchanged CR-TOL-001. No comparison
   tolerance, algorithm, key or full-domain accuracy verdict changes; A5 stays open.
9. **A9, certificate magnitudes: applied.** F06 section 4 rule 2 step d now explicitly requires
   abs(X dot Y), abs(X dot Z), abs(Y dot Z) at most 1e-12. NEG-31 (+1e-9) and NEG-35 (-1e-9)
   both require GEO_NUMERIC_UNCERTAIN, stage build, with no partial geometry or state change.
   The section 12 review-round-2 log records the change. W41 and W48 already used dot
   magnitudes, so no worked number or frame-certificate verdict changes. W48 now documents a
   resolved signed-test regression, and the corresponding validation gate is removed.

10. **A10, finite pick domain: applied.** F06 section 9 now uses the same closed finite
    segment 0<=t<=t_max for edge/vertex candidates as for faces, clamps the vertex parameter,
    minimizes over both closed segments for edges and chooses the smallest minimizing t.
    The inclusive endpoint distance band never extends that parameter interval; faces keep
    their exact t test. Section 11 and POS-47/NEG-36 assert endpoint hits and short-horizon
    rejection. W49 derives the outcomes. The t_max verdict now covers every ray target; its
    inability to reach every admitted origin/body pair and the A5 accuracy gap remain.
11. **A11, dependency-set wording: applied.** F06 section 7 now explicitly shares algorithm
    and tolerance_policy between shape and world preimages, lists otherwise disjoint directly
    encoded fields, transitive shape dependence and distinct domain tags. Sections 11 and 12
    retain existing key vectors and invalidations. This matches the requested wording repair;
    W50 verifies shared bytes and the first domain-tag difference. No encoding changes.

### Final independent consistency read

The final read of F06 sections 4, 7, 9, 11 and 12 and its case file made **no new finding
(A12 or later)**. This is a bounded document review, not proof of complete consistency or
implemented behavior. A5, the partly applied portions of A1/A2/A3 and the owner/admission gates
remain open; applied amendments do not close them.

Open gates: owner confirmation/admission of frames; independent design review; numeric-budget
implementation proof; resource-budget instrumentation; matrix materialization as tests; real
API/CLI E2E evidence; compiler/target reproducibility; F06 freeze; source-exposure admission.
No software dependency, network service, asset catalogue or persisted model definition is added.

### Exposure note

Round 4 re-read F06 sections 4, 7, 9, 11 and 12 and the 83-case file, including the corrected
POS-10/POS-41/POS-42/POS-45 and new POS-47/NEG-36. It read the existing design/matrix, local
checker and repository rules/roadmap/architecture. No new external sources or other checkouts
were read. Earlier rounds read their then-current F06 inputs, the same authorized design
inputs and local scratch checker. F08 boundaries are integrated; this reconciliation does not
select or change module paths or manually read implementation sources. The following earlier
exposure history is retained, not attributed anew to round 4.

The original design run read the task card, AGENTS.md, CLAUDE.md, ROADMAP.md, docs/architecture.md,
F06 draft
and its entire case file, F04's scope/limits/frames/quantities/errors/comparison sections,
every public file under docs/clean-room (README, policy, templates, review result, first-scope
package and cases), and docs/fixtures/README.md and expectations.md. Some initial combined
tool output was truncated; relevant inputs were then read in smaller sections. It enumerated
input filenames and attempted read-only Git status/branch/HEAD. It read only this worktree's
repository inputs and its own two outputs/scratch verification. Prescribed Cargo checks may
read toolchain/dependency files as part of the tools; no other checkout was opened as input.

Public browser reads were the mathematical reference pages listed in section 1, plus failed
attempts to retrieve the robust-predicates paper as PDF/PostScript and a LibreTexts mirror of
the divergence theorem. These failed retrievals are not claimed as read. The academic
publication page includes links to software; none was followed. No third-party kernel source,
tests or documentation, commercial detailing material, private folder or observation note was
opened, copied or summarized during this run. Existing document links to such material were
not followed. No external oracle software was run.

Earlier model training and any prior exposure that cannot be established are **unknown (EU)**
for all kernel areas. A fresh task context does not prove independence. No implementer is
declared eligible; no private exposure record was filed outside the assigned worktree, since
that was not authorized. The public note records the limitation; independent admission and any
owner risk acceptance remain separate decisions under the policy. This task is design only.

[goldberg]: https://docs.oracle.com/cd/E19957-01/806-3568/ncg_goldberg.html
[divergence]: https://openstax.org/books/calculus-volume-3/pages/6-8-the-divergence-theorem
