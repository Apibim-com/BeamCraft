# F07 analytical expectations

This document derives every expected value in
[`tests/fixtures/expectations/f07.json`](../../tests/fixtures/expectations/f07.json) by hand from
the fixture inputs, using closed-form elementary geometry. The values were written before the
oracle compared them. The generator did not produce them, and no BeamCraft geometry
implementation was used. The only tool used was a calculator, Python's `math` module, to turn
closed forms such as 450√3 into decimals.

Contributor-original, 2026-10-14, `MPL-2.0`.

## Conventions

All conventions are defined by the `beamcraft.fixture/1` format in [README.md](README.md).

- **Units.** Inputs: millimetres (mm), degrees (deg), kg/m³. Results: mm, mm³, kg.
  Internally, angles are radians: θ_rad = θ_deg · π / 180 (90° = π/2, 30° = π/6).
- **Volume conversion.** 1 m = 1000 mm, so 1 m³ = 10⁹ mm³ and V[m³] = V[mm³] · 10⁻⁹.
- **Mass.** m[kg] = ρ[kg/m³] · V[mm³] · 10⁻⁹. Every density in this set is the synthetic test
  value ρ = 6000 kg/m³, a round number chosen for exact arithmetic. It does not describe a real
  material. When a fixture has no density, mass is **unknown** (`null`), never zero or a default.
- **Plate.** Its local box is [0, a] × [0, b] × [0, t], with a corner at the local origin. The
  placement rotates it about the unit vector of `axis` through the local origin by `angle`
  (right-hand rule), then translates it by `origin`. Volume is V = a·b·t; a rigid motion
  preserves it.
- **Member.** Axis vector d = end − start, axis length L = |d|, unit axis x̂ = d / L. Height
  direction ẑ = normalise(up − (up·x̂) x̂); width direction ŷ = ẑ × x̂, so (x̂, ŷ, ẑ) is
  right-handed. The rectangular w × h profile is centred on the axis, with width along ŷ and
  height along ẑ. The prism runs exactly from start to end, with no end extensions. Volume is
  V = w·h·L.
- **Bounds.** These are axis-aligned bounds of the solid: the min/max of the 8 corners. For a
  box with centre c, unit edge directions eᵢ and half-extents hᵢ, the half-size along world
  axis k is Σᵢ |eᵢ,ₖ| hᵢ.
- **Totals.** Total volume and mass are sums over objects. Total mass is unknown if any object
  mass is unknown. Total bounds are the union of object bounds.

## Tolerances

| Quantity | Tolerance | Reason |
|----------|-----------|--------|
| Lengths and bounds | absolute 1 × 10⁻⁶ mm | Far above f64 rounding at the 10⁷ mm coordinate limit (spacing ≈ 1.9 × 10⁻⁹ mm), so it covers cos(π/2) ≠ 0 in floating point |
| Volume | relative 1 × 10⁻¹² | Products of at most three exact integers plus one square root |
| Mass | relative 1 × 10⁻¹² | One extra multiplication by ρ·10⁻⁹ |

These tolerances apply to the analytical comparison only. A future mesh, USD or kernel feature
gate must set and justify its own tolerances.

## Supported cases

### E1. `f07.plate.axis-aligned`

Plate a × b × t = 640 × 410 × 15, origin 0, angle 0.

- V = 640 · 410 · 15 = 3 936 000 mm³ = 0.003936 m³.
- m = 6000 · 0.003936 = 23.616 kg.
- Bounds: [0, 0, 0] to [640, 410, 15].

### E2. `f07.plate.translated`

This is the E1 plate translated by (1250, −340, 2200). A translation preserves volume and mass
(3 936 000 mm³, 23.616 kg) and shifts the bounds:
[1250, −340, 2200] to [1890, 70, 2215].

### E3. `f07.plate.rotated-z-quarter`

The plate is 750 × 300 × 10. The axis (0, 0, 5) normalises to (0, 0, 1), the angle is +90°
(π/2), and the origin is (500, 0, 0).

Rz(π/2) maps (x, y, z) to (−y, x, z). The local x ∈ [0, 750] and y ∈ [0, 300] become
x′ ∈ [−300, 0] and y′ ∈ [0, 750]. After translation:

- Bounds: [200, 0, 0] to [500, 750, 10].
- V = 750 · 300 · 10 = 2 250 000 mm³, so m = 6000 · 0.00225 = 13.5 kg.

### E4. `f07.plate.rotated-z-30`

The plate is 900 × 260 × 22, rotated +30° (π/6) about +Z at the origin. It has no density.

With c = cos 30° = √3/2 and s = sin 30° = 1/2, Rz maps (x, y) to (x c − y s, x s + y c).
The corners are:

| Local (x, y) | x′ | y′ |
|--------------|----|----|
| (0, 0) | 0 | 0 |
| (900, 0) | 450√3 | 450 |
| (0, 260) | −130 | 130√3 |
| (900, 260) | 450√3 − 130 | 450 + 130√3 |

- Bounds: x ∈ [−130, 450√3], y ∈ [0, 450 + 130√3] and z ∈ [0, 22], or
  [−130, 0, 0] to [779.4228634059947, 675.166604983954, 22].
- V = 900 · 260 · 22 = 5 148 000 mm³.
- Mass is **unknown** (`null`) because the fixture omits density.

### E5. `f07.plate.rotated-x-quarter-translated`

The plate is 480 × 350 × 12, rotated +90° about +X, then translated to (−800, 2500, 300).

Rx(π/2) maps (x, y, z) to (x, −z, y). The local box becomes x ∈ [0, 480], y ∈ [−12, 0] and
z ∈ [0, 350]. After translation:

- Bounds: [−800, 2488, 300] to [−320, 2500, 650].
- V = 480 · 350 · 12 = 2 016 000 mm³, so m = 6000 · 0.002016 = 12.096 kg.

### E6. `f07.member.along-x`

The member runs from (0, 0, 0) to (4200, 0, 0), with up = (0, 0, 1) and a 120 × 260 profile.

Its frame is x̂ = (1, 0, 0), ẑ = (0, 0, 1) and ŷ = ẑ × x̂ = (0, 1, 0).

- L = 4200 mm.
- V = 120 · 260 · 4200 = 131 040 000 mm³ = 0.13104 m³, so m = 786.24 kg.
- Bounds: x ∈ [0, 4200], y ∈ [−60, 60] and z ∈ [−130, 130].

### E7. `f07.member.vertical`

The member runs from (300, 500, 0) to (300, 500, 3150), with up = (0, 1, 0) and a 140 × 220
profile.

Its frame is x̂ = (0, 0, 1). Because up ⊥ x̂, ẑ = (0, 1, 0), and
ŷ = ẑ × x̂ = (1, 0, 0). Width therefore runs along X and height along Y.

- L = 3150 mm.
- V = 140 · 220 · 3150 = 97 020 000 mm³, so m = 582.12 kg.
- Bounds: x ∈ 300 ± 70 = [230, 370], y ∈ 500 ± 110 = [390, 610] and z ∈ [0, 3150].

### E8. `f07.member.diagonal-5-12-13`

The member runs from (1000, 200, 450) to (2500, 3800, 450), with up = (0, 0, 1) and a
100 × 300 profile.

- d = (1500, 3600, 0) = 300 · (5, 12, 0). Since 5² + 12² = 13², L = 300 · 13 = 3900 mm.
- The frame is x̂ = (5/13, 12/13, 0), ẑ = (0, 0, 1) and ŷ = ẑ × x̂ = (−12/13, 5/13, 0).
- The centre is (1750, 2000, 450). The half-extents are L/2 = 1950 along x̂, w/2 = 50 along ŷ
  and h/2 = 150 along ẑ.
- Half-sizes:
  - X: (5/13)·1950 + (12/13)·50 = 750 + 600/13.
  - Y: (12/13)·1950 + (5/13)·50 = 1800 + 250/13.
  - Z: 150.
- Bounds:
  - x ∈ [12400/13, 33100/13] = [953.8461538461538, 2546.153846153846].
  - y ∈ [2350/13, 49650/13] = [180.76923076923077, 3819.230769230769].
  - z ∈ [300, 600].
- V = 100 · 300 · 3900 = 117 000 000 mm³ = 0.117 m³, so m = 702 kg.

### E9. `f07.copy.two-plates`

The fixture holds two complete, independent plates with the keys `plate-original` and
`plate-copy`. Each is 520 × 330 × 18, unrotated, with origins (0, 0, 0) and (700, 0, 0).

- Each plate: V = 520 · 330 · 18 = 3 088 800 mm³, so m = 6000 · 0.0030888 = 18.5328 kg.
- Bounds: [0, 0, 0] to [520, 330, 18] for the original, and [700, 0, 0] to [1220, 330, 18]
  for the copy.
- Totals: V = 6 177 600 mm³ and m = 37.0656 kg. The union bounds are [0, 0, 0] to
  [1220, 330, 18].
- Separation along X: 700 − 520 = 180 mm, so the bodies do not touch.

This case only describes an input with two complete objects. It does not show that any
application copy command works.

## Unverified case

### E10. `f07.plate.far-from-origin`

This is the E1 plate at origin (9 990 000, −9 990 000, 9 990 000), inside the ±10⁷ mm
coordinate limit.

- Bounds: [9 990 000, −9 990 000, 9 990 000] to [9 990 640, −9 989 590, 9 990 015].
- V = 3 936 000 mm³ and m = 23.616 kg.

The values are exact; every coordinate is an integer that f64 represents exactly. The case is
**unverified**: no BeamCraft implementation has yet shown that it meets the tolerances at this
magnitude, for example after meshing or with GPU single precision.

## Rejections (R)

Expected rejection codes come from the `beamcraft.fixture/1` rules in [README.md](README.md),
not from running the validator.

| Fixture | Code | Rule |
|---------|------|------|
| `f07.unsupported.profile-circle` | `unsupported_profile` | Only `rect` profiles exist in version 1 |
| `f07.unsupported.length-unit-inch` | `unsupported_unit` | Only mm, deg and kg/m3 are accepted |
| `f07.invalid.plate-zero-thickness` | `non_positive_dimension` | Dimensions must be greater than 0 |
| `f07.invalid.member-negative-width` | `non_positive_dimension` | Dimensions must be greater than 0 |
| `f07.invalid.plate-below-minimum-thickness` | `degenerate_geometry` | 0.0004 mm is below the 0.001 mm minimum |
| `f07.invalid.member-zero-length` | `degenerate_geometry` | The axis length is 0 |
| `f07.invalid.member-up-parallel` | `degenerate_geometry` | up = (−2, 0, 0) ∥ d = (4200, 0, 0) |
| `f07.invalid.rotation-zero-axis` | `degenerate_geometry` | The rotation axis is (0, 0, 0) |
| `f07.invalid.negative-density` | `non_positive_density` | Density must be greater than 0 |
| `f07.invalid.number-overflow` | `non_finite_number` | 1e400 exceeds the f64 maximum (about 1.8 × 10³⁰⁸) |
| `f07.invalid.number-nan` | `non_finite_number` | `NaN` is not a finite number |
| `f07.invalid.json-truncated` | `malformed_json` | The document ends inside an object |
| `f07.invalid.json-duplicate-member` | `malformed_json` | The member name `kind` is repeated |
| `f07.invalid.utf8` | `malformed_json` | Bytes C3 28 are not valid UTF-8 |
| `f07.invalid.wrong-type` | `invalid_field` | `size` is a string |
| `f07.invalid.unknown-field` | `invalid_field` | `colour` is not a plate field |
| `f07.invalid.unknown-format` | `unsupported_format` | `beamcraft.fixture/99` |
| `f07.invalid.empty-objects` | `invalid_field` | At least one object is required |
| `f07.invalid.duplicate-object-key` | `duplicate_object_key` | `plate-1` appears twice |
| `f07.invalid.limit-objects` | `limit_exceeded` | 17 objects exceed the limit of 16 |
| `f07.invalid.limit-depth` | `limit_exceeded` | Nesting depth 27 exceeds the limit of 16 |
| `f07.invalid.limit-coordinate` | `limit_exceeded` | 2 × 10⁷ mm exceeds the ±10⁷ mm limit |
| `f07.invalid.limit-dimension` | `limit_exceeded` | 250 000 mm exceeds the 100 000 mm limit |
