# P02.2 section properties in definitions

Date: 2026-10-17. Status: **experimental slice; follow-up card of the merged P02.**
Branch `feat/p02-2-section-properties`, based on `origin/master` at `06a2497`.

Plan: revision 1.3, section 7.14. A profile definition carries the section data that a bill of
materials reads, and a material carries its grade, so P04 can compute a nominal mass and a surface
without a second source of numbers. Only fields and checks are added: no catalogue data is
included, and no value is copied from a commercial table (AGENTS.md section 1.2).

## What is implemented

| Area | What exists |
|---|---|
| `beamcraft_model::definition` | `SectionProperties` (designation, standard, `area_mm2`, `mass_per_meter_kg`, `surface_per_meter_m2`, all optional); `MaterialSpec.grade` and `.standard`. Admission: labels by the element-label rule, numbers finite (`NON_FINITE_NUMBER`), greater than zero (`INVALID_ARGUMENT`), capped (`LIMIT_EXCEEDED`). For the rectangle, the declared area and surface per metre must agree with the family formula within 0.1 percent (`DECLARED_RELATIVE_TOLERANCE`). |
| `beamcraft_layer_engine::authored` | `Definition` carries the new fields; they are written as plain optional attributes (`steel:profile:designation`, `steel:profile:standard`, `steel:profile:areaMm2`, `steel:profile:massPerMeterKg`, `steel:profile:surfacePerMeterM2`, `bim:materialGrade`, `bim:materialStandard`) only when present, so existing definitions keep their digest. The snapshot validator reads the same fields, re-admits them and recomputes the digest. The schema descriptor lists them. |
| `beamcraft_cli` | `define-profile --designation --standard --area-mm2 --mass-per-meter-kg --surface-per-meter-m2`, `define-material --grade --standard`; `definitions` and the define output show them; a layer edited after publication is `INTEGRITY_ERROR`. |

## Decisions

- The values are part of the definition's identity: a profile with a designation is another
  digest than the same profile without one, and an element pins the exact content it was built on.
- A declared value that contradicts the family formula is refused, not stored. The plan asks for
  a diagnostic, not a silent choice; refusal at publication is the strict form of that for the one
  family that exists. The declared mass per metre cannot be checked without a density, so that
  check belongs to the quantity that uses both (P04).
- `h`, `b`, `tw`, `tf` and `r` of the plan card are not added: a rectangle has `height`/`width`
  only, and the web and flange fields belong to the I-section families (A07).
- A grade is a label. It never selects a density.
- The schema digest changes (the descriptor lists the new attributes). Layers pinned to the
  previous digest contain nothing the new mapping refuses, so `PREVIOUS_DESCRIPTOR` (the mapping
  of master with U09; stores from before U09 lack the kind, name, extent and normals) is accepted
  next to the current one when a store is opened or edited, and the next edit rewrites the pin.
  Checked with a store created by the pre-change binary and edited by this one. A later mapping
  change that is not backward compatible must not extend this list.

## Evidence

`cargo test -p beamcraft-model` (admission, codes and the formula agreement),
`cargo test -p beamcraft-cli` (decode and digest round trip, a layer with a dropped designation is
`INTEGRITY_ERROR`, and a real-binary end-to-end test: values stored as plain USD attributes,
listed in a fresh process, a mismatched area refused with nothing published),
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`.

## Not done

- No consumer: the engine still reads only width, height and density. P04 uses `mass_per_meter_kg`
  and `surface_per_meter_m2` for `nominalMassKg` and `surfaceAreaM2`.
- No I-section family and no `tw`, `tf`, `r` (A07). No catalogue lookup.
- The pxr oracle is unchanged; the new attributes are plain USD types.
