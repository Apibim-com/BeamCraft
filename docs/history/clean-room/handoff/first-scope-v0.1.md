# First-scope handoff package, version 0.1.0

Date: 2026-10-14. Author: F09 independent reviewer (contributor-original, MPL-2.0).
Repository base: `ebb5b928dda3d36d6b6570549c3f745bb1883ff7`.

## Scope statement

This package is admitted for **one scope only**: a rectangular plate, a straight member with a
synthetic rectangular section, identities, units, rigid placement and declared failures.
It is the whole of what a geometry implementer may treat as requirements from this review.
Everything else is **pending, unsupported or outside the geometry handoff**, whatever other
documents may suggest. Approval of this limited package says nothing about broader requirements.

The package is not an engine, not a frozen geometry specification and not evidence that any
behaviour is implemented. The [F04 contract](../../../contracts/model-and-commands-v0.1.md) it cites is a **review draft**
(version 0.1.0; reviewed at PR #6 head `b378d7b339dc5319b77892c5f95ed9ae168ae943`, now on the
default branch with identical content); this package does not freeze it. Case IDs such as
`F04-NEG-04` refer to its [case file](../../../contracts/model-command-cases-v0.1.json).
Source-exposure admission of a person or agent is separate and is decided under the
[admission policy](../admission-policy.md), never by reading this package.

## How each entry is justified

Every entry below names an **approved source**: a public standard, a public mathematical fact,
an owner decision recorded in this repository, or the F04 draft. No entry rests on observation
of a commercial product, a catalogue, a vendor manual or a third-party kernel. Expected results
are in [first-scope-cases-v0.1.json](first-scope-cases-v0.1.json). They were derived by hand and
re-derived with exact rational arithmetic that uses no BeamCraft code and no floating point, so
they do not depend on any implementation.

Entry status values:

| Status | Meaning |
|--------|---------|
| admitted | Approved source and independent expected results exist; implementers may rely on it |
| admitted, open gap | The stated part is admitted; the named gap is excluded until resolved |
| proposed | Original proposal that needs a named decision; implementers must not rely on it |

## Requirements

| ID | Requirement | Approved source | Status | Expected-result basis | F04 draft cases |
|----|-------------|-----------------|--------|-----------------------|-----------------|
| CR-UNIT-001 | Lengths and coordinates are finite f64 millimetres; angles are radians; the frame is right-handed with Z up; volume is reported in m3, density in kg/m3, mass in kg | Owner decision (architecture); SI prefix definition (1 m = 1000 mm, so 1 m3 = 10^9 mm3) | admitted | Exact unit-conversion arithmetic in every volume case | F04-POS-01 |
| CR-ID-001 | An element or project identifier is a full UUIDv7 in canonical lowercase 8-4-4-4-12 form: version nibble 7, variant nibble 8, 9, a or b; anything else is rejected | [RFC 9562 section 5.7](https://www.rfc-editor.org/rfc/rfc9562.html#section-5.7); owner decision (full identifiers) | admitted | Hand-chosen valid and invalid strings, FS-ID-VALID and FS-NEG-ID | F04-NEG-01, F04-NEG-02 |
| CR-ID-002 | An owned element's prim name keeps all 32 hexadecimal digits of its identifier | Owner decision in `docs/architecture.md` | admitted, open gap | The exact path form (prefix and parent path) is defined only by the F04 draft and is excluded until F04 is frozen | |
| CR-PLATE-001 | A plate takes explicit width, length and thickness; each is finite, greater than zero and at most 1,000,000 mm; nothing is defaulted | Owner first-slice decision; F04 draft section 3 limits | admitted, open gap | FS-PLATE-A, B, C and the boundary negatives; gap G-01 (local frame) is excluded | F04-NEG-04, F04-NEG-05, F04-NEG-06 |
| CR-PLATE-002 | Plate volume is width*length*thickness; surface area is 2*(wl + wt + lt) | Elementary solid geometry (rectangular cuboid) | admitted | Exact rational results in FS-PLATE-A, B, C | F04-POS-01, F04-POS-06 |
| CR-MEMBER-001 | A straight member axis is two explicit local points; its length is the Euclidean distance between them; the length must be greater than zero and at most 1,000,000 mm | Euclidean distance; owner first-slice decision; F04 draft section 3 | admitted | FS-MEMBER-A (3-4-12 triple), FS-MEMBER-B, FS-NEG-AXIS-BOUND | F04-POS-07 |
| CR-MEMBER-002 | A prismatic member with an explicit rectangular section b*h and axis length L has volume b*h*L | Elementary solid geometry (prism volume) | admitted, open gap | FS-MEMBER-A, B; the section's orientation about the axis (gap G-02) and the profile-definition resource format are excluded | F04-POS-07 |
| CR-PLACE-001 | A placement is a translation (three finite numbers, each at most 1,000,000,000 mm in absolute value) and a quaternion (x, y, z, w) with every component in [-1, 1] and squared norm within 1e-12 of one; the accepted quaternion is normalized and used as a proper rotation; world point = R*local + translation; lengths and volumes are unchanged | Standard rotation mathematics ([Quaternions and spatial rotation](https://en.wikipedia.org/wiki/Quaternions_and_spatial_rotation), Hamilton convention, active rotation); F04 draft section 3 limits; owner decision (rigid pose only) | admitted | FS-POSE-A to F; exact rational rotation matrices | |
| CR-QTY-001 | Volume is a current value when the shape inputs are valid; mass is current only when a density greater than zero (at most 1,000,000 kg/m3) was explicitly authored, otherwise it is unknown; bounding-box or approximate values are never promoted | Owner decision (unknown/stale/unsupported statuses); F04 draft section 5 | admitted | FS-PLATE-A (unknown), FS-PLATE-A-MASS, FS-NEG-DENSITY | F04-POS-01, F04-POS-07 |
| CR-FAIL-001 | Invalid input is rejected without crashing and without changing state: non-finite or overflowing numbers, numeric strings, values outside the limits above, degenerate axes, invalid quaternions, invalid identifiers, non-positive density | `AGENTS.md` section 4 (never crash on input); F04 draft sections 3 and 8 for the stable codes | admitted | Negative cases in the case file; the code names follow the draft and may change before freeze | F04-NEG-01, 02, 04, 05, 06, 14, RAW-01, RAW-02 |
| CR-TOL-001 | Test comparison rule: quantities pass when the relative error is at most 1e-12; coordinates pass when the absolute error is at most 1e-9 mm plus 1e-12 of the expected magnitude | Floating-point analysis below | admitted (tests only) | See the analysis below | |
| CR-EXCL-001 | The exclusions listed in the next section are unsupported or pending; implementers do not infer behaviour for them | This review | admitted | Not applicable | |

### Basis of the comparison rule

Binary64 has unit roundoff 2^-53, about 1.1e-16 (IEEE Std 754-2019, binary64).
A product of three operands followed by one division carries at most four roundings, a relative
error below 5e-16. A rotation from a quaternion needs about twenty operations, bounded by about
5e-15 of the coordinate magnitude, which is below 1e-9 mm for the case magnitudes used here.
The chosen limits leave a margin of more than three orders of magnitude over these bounds.
They apply to test comparison only; they are not a geometric tolerance policy, which F06 must set.

## Exclusions

Not part of this package, and not to be implemented from this review:

- Position presets and reference-line offsets of any kind (unsupported in the first slice).
- Profile catalogue lookup, standard profile families and their section properties.
- Cuts, fits, chamfers, holes, bolts, welds, assemblies, numbering and drawings.
- Curved or multi-segment members, contour plates and bent or lofted plates.
- Boolean operations, topology checking and any tolerance policy.
- Copy, delete and undo semantics (described only in the F04 draft).
- Import or conversion of foreign models, and any behaviour of foreign references.

## Open gaps

| Gap | Question | Effect until resolved |
|-----|----------|-----------------------|
| G-01 | The plate's local frame is not defined: which local axes carry width, length and thickness, and where the origin sits | Only frame-independent quantities (volume, area, mass) are admitted for plates; no local bounds |
| G-02 | The member section's rotation about the axis, its offset from the axis and the end-face definition are not defined; the profile-definition resource format is not specified | Only axis length and volume are admitted for members |
| G-03 | A zero-length axis could map to `INVALID_ARGUMENT` or `UNSUPPORTED_GEOMETRY` | FS-NEG-AXIS-ZERO states only that the input is rejected |
| G-04 | The F04 draft does not state the quaternion convention or the order "rotate, then translate" | This package states them (CR-PLACE-001); F04 should adopt the same wording before freeze |
| G-05 | The F04 draft cases give expected values without a comparison rule, and have no cases for degenerate axes, invalid quaternions, boundary values or the translation limit | CR-TOL-001 and the new FS cases cover them; F04 should adopt or replace them |
| G-06 | No minimum nonzero dimension or geometric tolerance exists | Not specified here; owned by F06 |

## For implementers and issuers

An issuing task lists this package by version and repository commit, never by pasting reviewer
inputs. Use the [task template](../implementation-task-template.md). The package version is
`0.1.0`; any change to an entry, case or status raises the version and is reviewed again.
