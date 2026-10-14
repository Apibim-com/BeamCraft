# Roadmap

**Stage:** pre-alpha, scaffolding. Last updated 2026-10-14.

This is a one-page plan. Milestones are intentionally coarse; agents break them into tasks.

| # | Milestone | Status |
|---|-----------|--------|
| M0 | Repository, licence, CI, agent rules | done |
| M1 | Geometry kernel basics: vectors, planes, local coordinate systems, transforms | next |
| M2 | Profile library from public standards (EN, AISC): I/H, channel, angle, tube, plate | planned |
| M3 | Model core: parts, assemblies, grids, numbering, undo/redo, save/load | planned |
| M4 | IFC import and export (steel subset) | planned |
| M5 | Viewer: native window, 3D view, selection, orbit/pan/zoom | planned |
| M6 | Modelling tools: beams, columns, plates, polygons, cuts, fittings | planned |
| M7 | Connections: parametric bolted and welded joints | planned |
| M8 | Drawings and reports: GA and assembly drawings, BOM, DSTV NC export | planned |
| M9 | Web build (WASM) and 1.0 polish | planned |

## Progress log

- 2026-10-14: switched the current source and documentation to MPL-2.0; updated Cargo metadata,
  source notices, contribution rules and release packaging. Third-party licence policy remains
  permissive, with explicit MPL-2.0 exceptions for project-owned workspace crates.

- 2026-10-14: scaffold created: workspace (`geom`, `model`, `beamcraft-cli`), dual MIT/Apache-2.0
  licence, CI on Windows and Linux, release workflow, agent rules.
