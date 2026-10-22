# K1 plate contour validation and rectangular body frame

Focused implementation slice of issue #171, based on `master` `9291a96`. It addresses
F04-POS-18 / F06-POS-48 and the nonplanarity codes of F04-NEG-29, F04-NEG-38,
F06-NEG-38 and F06-NEG-49. It does not close #171 or claim K1 acceptance.

## Behaviour

The kernel's `plate_contour::admit` validates the submitted ordered 3D contour before deciding
whether its geometry is supported. Count, finiteness, coordinate/dimension/local-size caps and
scaled minima precede the ordered area vector, planarity, winding and projected simplicity.
Area-vector signs, dominant-axis ties (Z, Y, X), exact planarity, rectangular closure and
orthogonality use the existing bounded exact integer arithmetic over binary64 inputs.
Intersection and touching tests reuse the kernel's exact segment predicates. No point is
reordered, projected into the authored model, snapped or repaired.

`rectangular_body` evaluates an exact rectangle with its origin at the first point, X along the
first edge, Z along the ordered area vector and Y = Z cross X. Its eight element-local vertices
use F06 section 6 role-bit order; the existing winding, incidence and volume checks run before
sealing. The engine and CLI save this body through the existing atomic revision path. Contour
coordinates and element placement remain separate authored data. `--contour` accepts `X,Y` or
`X,Y,Z` per point. Existing `z=0` mesh paths, bytes and quantity signatures stay unchanged.

The rules follow F04 section 3, "Element-local frames", and F06 sections 4 and 6. The frozen
contract text still limits general non-`z=0` evaluation; the existing frozen POS-18/POS-48
rectangle cases explicitly require this supported frame. This slice implements those cases
without editing the contracts or fixtures.

## Verification and limits

`crates/kernel/tests/plate_contour.rs` covers ordered frames, canonical vertices, tilted and
vertical rectangles, independent signed volumes, exact winding ties, small-coordinate arithmetic,
scaled near-planarity, invalid topology and caps. CLI regressions save and reopen the frame,
preserve authored coordinates and check that a rejected nonplanar create leaves the head intact.
The F04/F06 drivers exercise the unchanged frozen cases and retain their partial-field labels.
The independent pxr regression reads materialized USD, compares authored coordinates and exact
mesh vertices, checks translated bounds and volume, and runs the existing compliance checker.

Local Windows verification passed: the full offline/locked workspace test suite with
`BEAMCRAFT_REQUIRE_PXR=1`, the final focused kernel/CLI regressions (including all five pxr
oracle tests), engine tests, formatting, workspace Clippy with warnings denied, Rustdoc with
warnings denied, module boundaries, F07 fixture verification and the independent Python
expectations plus their eight regression tests. OpenUSD 0.25.11 reported empty compliance
errors, failed checks and warnings for the new horizontal and oblique plate pair. The unchanged
F04 driver now reports 36 matching admission projections, one deviation (POS-28) and five
skips. F06 still reports partial projections, with only POS-11 in its explicit deviation list;
the face-table format discrepancy within POS-01 remains. WASM and cargo-deny are unavailable
locally and still require CI evidence.

Remaining work is explicit:

- Valid near-planar input is admitted by the kernel but still rejected by command application.
  Persisting an authored draft with unsupported geometry and no partial mesh/key is a separate
  slice (F04-POS-28 / F06-POS-56).
- Holes on framed rectangles, nonrectangular polygons outside `z=0`, rotated element placements
  (P10), exact face-table identifier agreement and missing protocol adapters remain open.
- Existing non-origin rectangles in `z=0` retain their G05 representation; their full body-frame
  and feature assertions remain unclaimed.

## Provenance

Contributor-original Rust, test logic and synthetic numeric inputs, MPL-2.0. The numerical
references follow elementary cross/dot products, rectangular volume and the divergence theorem.
No dependency, downloaded resource, catalogue or runtime network access is added. Synthetic
test data are covered by the existing contributor-original test-data entry in `ATTRIBUTION.md`.
