# U09 standard USD metadata

Date: 2026-10-17. Status: **experimental slice; closes the P01/P03 metadata gaps.**
Branch `feat/u09-standard-metadata`, based on `origin/master` at `06a2497`.

Plan row: U09 / S, depends on P01 and P03 (merged). Result: model `kind`, `displayName`
from `bim:name`, an authored Mesh `extent` and flat normals, so an ordinary USD reader shows the
element with its name, bounds and flat shading. Behaviour comes from the public UsdGeom and
Usd.ModelAPI documentation only; no third-party sample was used.

## What is implemented

| Area | What exists |
|---|---|
| Authoring (`authored/edit.rs`) | `/World` assembly, `/World/Steel` group, element component, `Geometry` subcomponent. `displayName` = `bim:name` at create and on every `Label` change. `extent` (`float3[]`, min and max of the float32 points) and `normals` (`normal3f[]`, `interpolation = "uniform"`, one unit Newell normal per face) are written with the mesh. |
| `authored/derive.rs` | `extent` and `face_normals`, shared by authoring and validation, with bad buffers refused (no panic, no indexing). |
| Reader (`Prim`, `Property`) | `Prim::kind` and `Prim::display_name` carry the composed metadata, `Property::interpolation` the composed `interpolation` of an attribute; all are charged to the snapshot budget. |
| Validator | An owned element must be a component with `displayName` equal to `bim:name`; an owned Mesh must be a subcomponent whose `extent` equals the point bounds and whose `normals` have `uniform` interpolation (missing or `vertex` is refused) and match the points within 1e-5 per component; an authored `/World` or `/World/Steel` kind must be the expected one. |
| Schema | `SCHEMA_DESCRIPTOR` lists the new fields, so the schema digest changes (experimental stores written before U09 are not readable by this schema pin). |

## Decisions

- Normals are per face (`uniform`), not per corner: the kernel meshes share vertices between
  faces, and a reader that averages shared normals would round the edges.
- The extent is exact over the float32 points the layer stores, so it also matches a pxr read of
  the same points. Normals tolerate 1e-5 because another tool may normalise in a different order.
- `kind` on `/World` and `/World/Steel` is checked only when authored, so hand-built layers
  without kinds still open; authoring always writes it.
- `displayName` is prim metadata, not an attribute, so the F04 footprint is unchanged: it follows
  the `Label` group, and the mesh fields follow the `Derived` group.

## Evidence

* `cargo test -p beamcraft-layer-engine`: the new tests check kinds, name, extent and normals in
  the composed snapshot, that a rename changes `displayName` and nothing under `Geometry`, and
  that a wrong name, kind, extent or normal is refused.
* The pxr oracle (`tests/pxr_oracle.py`, local, usd-core 25.11) now checks, for plates (with and
  without holes) and members: `Usd.ModelAPI` kinds and the model hierarchy, `GetDisplayName()`,
  the authored extent against `UsdGeom.PointBased.ComputeExtent`, uniform normals that point
  along each face's outward normal, and `ComplianceChecker` with no findings.
* `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test --workspace` pass.

## Not done

- Visual material and `displayColor` (U08), lighting (U13), instancing (U11), payloads (U10).
- Old stores are not migrated.
