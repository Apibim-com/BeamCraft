# BeamCraft UI

V03: the native window and viewport. It shows the accepted (published) revision of a store.

```text
beamcraft-ui --store ROOT [--cli PATH]
beamcraft-ui --store ROOT --metrics FRAMES [--width W] [--height H] [--cli PATH]
beamcraft-ui --synthetic COUNT --metrics FRAMES [--width W] [--height H]
```

- The model comes from the engine, through the headless CLI in a separate process:
  `beamcraft view-export --store ROOT --out -` writes the pinned view snapshot as one bundle, and
  `beamcraft_engine::ViewSnapshot::from_bundle` rebuilds and re-validates it here. The UI never
  opens USD and never depends on the CLI crate (F08: an app may not depend on an app). The CLI is
  looked up next to the UI executable unless `--cli` names it.
- Each distinct mesh is uploaded once and drawn with one instance per element placement.
  Coordinates are millimetres, re-centred on the engineering bounds in f64 before f32 conversion.
- Controls: primary drag orbits, secondary drag pans, the wheel zooms, F fits, R reloads.
- `--metrics` renders offscreen without a window and prints one JSON object (adapter, scene size,
  upload, per-frame median and p95). `--synthetic` measures synthetic kernel plates for scale.
- The UI font is the admitted Inter, read at run time from `assets/fonts/inter`; without it the
  window runs without text. `default_fonts` is never enabled.

Linux needs a Vulkan driver and the X11 or Wayland client libraries (including
`libxkbcommon-x11`) at run time; a missing window-system library is reported, not a crash.
