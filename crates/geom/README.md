# beamcraft-geom

Contributor-original BeamCraft code and documentation, MPL-2.0.

Pure fixed-size f64 mathematics for G01; no dependencies, I/O, allocations,
quaternions, general element orientation or solids. Units are millimetres and
radians, right-handed, Z up. The public Rust API is the headless engine-facing
math API. Pure mathematics needs no CLI command under the G01 task scope.

## API and validation

- `vec::{Point3, Vec3}`: checked point differences, distances, translations,
  vector norms, normalization, dot/cross products, addition and scaling.
- `tolerance::{Tolerance, MemberScale}`: F06 local body size and geometric floor,
  dimension admission, coincidence and plate planarity band. Counts, finiteness
  and submitted caps are checked before scale; minima follow scale.
- `frame::{Frame3, FrameCertificate, MemberFrame}`: the written default member
  frame, strict parallel comparison, absolute seven-residual certificate and
  optional member roll about local +Z. Both default and rolled frames certify.
- `plane::Plane3`: a finite origin and normalized normal, signed distance and
  inclusive planarity distance band. This does not validate contour topology.
- `transform::{RigidTransform, Translation3}`: derived body-basis mapping then
  translation; element placement supports translation only. There is no matrix
  or quaternion placement input. A plate remains as drawn.
- `canonical_f64_bytes`: finite big-endian binary64 encoding with positive zero
  for both signed zeros, as required by F06 section 7. Key construction is later work.
- `error::MathError`: typed errors without a reverse dependency on `model`.

All checked operations reject non-finite submitted scalars as `NonFinite` and
non-finite derived results as `NumericUncertain`. Normalization rejects zero
axes; a nonzero vector whose ordered norm underflows to zero is uncertain.
Finite vectors and certified frames have private fields. Certificate residuals
are inspectable; a frame always computes its own certificate from its axes.

Compatibility: `Point3` retains the scaffold's public fields, raw const `new` and
unchecked `distance_to` so existing model consumers do not require unrelated
changes. These are raw-input conveniences, not a validated kernel boundary.
Use `try_new`/`validated` and `checked_distance_to`; every new checked operation
revalidates raw points. Checked outputs canonicalize negative zero. The existing
model scaffold still calls unchecked distance; it is outside G01 integration.

Arithmetic follows F06's written parentheses, direct division for normalization,
strict branch comparison and final translation order. No `mul_add`, reciprocal
substitution, implicit repair, snapping or modulo reduction is used. F06's frame
certificate permits residuals up to 1e-6; it is not a rigorous accuracy enclosure
or proof of exact mathematical rigidity. The CR-TOL-001 comparison lives only
in tests. Picking is not implemented and never reads the scaled geometric floor.

## Case coverage and scope

`tests/f06_math.rs` tests math projections of the frozen F06 cases. These do not
claim full kernel statuses, diagnostics aggregation, meshes, quantities or keys.

| Case IDs | Math obligation |
|----------|-----------------|
| POS-04 through POS-10 | Horizontal, up/down vertical, sloped, diagonal and near-threshold default frames |
| POS-14, POS-15 | Inclusive dimension floor and section cap |
| POS-41, POS-42, POS-46 | Projected near-vertical +Y, outside branch and close binary64 decision |
| POS-50 through POS-52 | Omitted/signed-zero roll equivalence, positive roll, roll after fallback |
| POS-53 through POS-55 | Scaled plate floor above/at limit and small body at large world placement |
| POS-56 | Large near-planar distance admitted by the scaled plane band |
| POS-57, POS-58 | Section- and axis-driven member floors |
| NEG-01 through NEG-07 | Nonfinite contour/translation, dimension/axis minima, section cap and positivity |
| NEG-09 | Scale/shear/reflection fail the body-frame numeric gate; no authored matrix parser exists |
| NEG-31, NEG-35 | Positive and negative pairwise dot injections both fail |
| NEG-43 through NEG-49 | Nonfinite/excess roll, plate/member floors, plate D cap and outside plane band |
| NEG-50 | Unit test injects nonfinite derived L; no fallback floor |
| NEG-51 through NEG-53 | Excess member L and negative/huge-negative section dimensions |

The remaining F06 cases cover excluded mesh/topology/quantities/query/identity,
engine state, exact contour admission, or the superseded quaternion placement
scope. POS-11/POS-12 and quaternion-dependent parts of POS-25 need a reviewed
contract amendment before their former rotation expectations can apply.

Additional tests check every certificate residual, inclusive certificate bound,
strict coincidence, exact branch equality and adjacent floats, signed-zero/key
bytes, overflow/underflow rejection, finite-input precedence, body transform
order and a deterministic grid of synthetic axes.

## Open items and proposed errata

- Nonzero roll uses one isolated `roll_sin_cos` function with separate
  `f64::sin` and `f64::cos` calls. The reproducible trig profile is an OPEN owner
  decision. No identical bytes across untested targets are claimed.
- F06-POS-09 says final Y is exactly `(0,1,0)`. Its required step c instead
  computes Y.y = `1.0000000000000002` (`0x3ff0000000000001`) on the written
  binary64 expressions. The regression asserts those bits; the frozen case is
  unchanged. Its exact-Y assertion needs amendment to follow step c. The
  independent scalar Python calculation agrees; other references keep their
  original comparison rules.
- F04/F06 general quaternion placement text and F08's reserved `quat` module
  conflict with the later owner decision. No quaternion is implemented; a
  reviewed documentation amendment should record translation-only placement,
  derived plate frames and member-only axial roll.
- Full numeric accuracy enclosures, contour exact predicates, kernel result/error
  mapping, solids, mesh certification, quantities and CLI integration belong to
  later tasks. The broad historic G01 allocation in F08 is not completed here.

## Provenance

Original implementation of scalar vector algebra and rigid body-basis mapping;
no external code or assets were copied. Normative expressions, caps, certificate,
encoding and test inputs come from F06 0.1.0 sections 2, 4, 7 and 10 and its case
file. Binary64 arithmetic and sqrt follow F06's public IEEE 754-2019 reference;
cross products and basis maps are elementary right-handed vector algebra. Owner
scope comes from the owner decision of 2026-10-15 (no external math crates, plates unrotated,
members roll about their local Z axis; see `docs/plan/decisions.md`) and the task.
No dependency, catalogue, external fixture or licence text is added.
