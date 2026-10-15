# Original fixture expectations, frozen before first candidate test

Contributor-original BeamCraft material, MPL-2.0, 2026-10-15. Fixture version 1.
Synthetic dimensions and identities, not steel profile/material catalogue data.
Expectations derive from F04 sections 1-4 and 8, architecture ownership rules,
and F11 N01-S01/S03/S04/S06/S07/S08/S10/S11 scenario descriptions.

- Model: two complete UUIDv7 identities; dimensions 1000.125, 500.25, 10.5 mm;
  `metersPerUnit=0.001`, `upAxis=Z`; unknown string, prim customData and layer dictionary;
  material relationship to a synthetic definition; second prim inactive.
- Round trip: USDA and USDC preserve every authored spec/field/value in this fixture.
  Second same-format write after reread has identical bytes. Supported edit preserves
  unknown content and identity; no broad unknown-type preservation is presumed.
- Composition: sublayers [strong, middle, model]; widths 1200.5 then 1100.5 override
  model 1000.125; removing strong selects middle. A root over wins at 1300.5.
  Inactive second element excluded from default traversal, included after activation.
- Foreign: reference maps `/Source` to the second element path; width 77.
  Source bytes remain identical after a stronger local override to 99. That override
  is legal USD but forbidden BeamCraft behavior: wrapper must reject it. Provenance
  allowlist excludes foreign source, even if its prim carries a valid BIM identity.
- Deltas: width and label changes replay on a fresh base; competing width edits have
  the same `(UUID, attribute)` key; width versus label has disjoint keys. This does not
  prove semantic independence, live ordering, idempotency or atomic publication.
- Hostile corpus: all byte truncations of a small USDA, malformed syntax, depths
  32/64, huge integer/count-like tokens, invalid UTF-8 and corrupted small binary
  inputs. Children get <=64 KiB input, 5 s deadline and a 512 MiB Windows Job commit
  cap. Timeout, crash, panic or memory termination is a finding, never safe parsing.
- Cost: 1000/10000 elements have full unique UUID-shaped names, explicit width/id
  fields. Time open separately from compose/traverse (force value reads) and save.
  Warm filesystem cache is allowed and must be reported. No benchmark gate or
  official-oracle conformance result follows from this sanity experiment.
