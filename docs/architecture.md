# Architecture

BeamCraft is a Cargo workspace. Library crates are pure Rust with no UI or OS dependencies and must
build for `wasm32-unknown-unknown`.

```
apps/
  beamcraft-cli     command-line entry point (binary: beamcraft)
crates/
  geom              points, vectors, coordinate systems   (no deps)
  model             parts, profiles, assemblies           (-> geom)
```

Planned crates (see `ROADMAP.md`): `profiles`, `ifc`, `dstv`, `render`, `ui`, `connections`,
`drawings`, `automation`.

## Rules

- Dependencies point one way, from higher-level crates down to `geom`. No back-edges.
- Units: millimetres, radians internally, right-handed, Z up. Convert only at the IO/UI edge.
- Every feature is reachable headlessly: engine API → CLI command → end-to-end test.
- Input is untrusted; see `AGENTS.md` §4.
