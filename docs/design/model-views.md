# Local model views

This implementation adds local presentation tools to the existing properties and interactive
creation workbench. It does not change engineering geometry, numbering, element identities or
immutable model revisions. It does not claim completion of the W01 shared-view acceptance gate.

## Creation and properties

The Two points, Three points and Surface commands immediately arm viewport picking. Point
creation uses model/grid snaps, step prompts, captured-point markers and a live construction
line. The final valid click creates the view; no XYZ entry or separate picking button is
required. Coincident/collinear points can be re-picked without losing valid prior points.
Esc cancels, Backspace removes the last point, and Ctrl+P aligns the camera with the active view
plane while preserving the point-selection command and the input frame. Explicit planes in the
native UI are defined by three picked points; coordinate expressions remain a headless input.
Work areas fit the model or selection through buttons, and grids/axes are selected from model
records rather than entered as identities.

The engine admits coordinate planes with offsets, two-point sections relative to the current
work plane, three-point planes, explicit frames, model levels, straight regular or drawn grid
axes, planar surfaces and surface-edge alignment. Part views use evaluated geometry and its
placement; four-view sets contain front, top, end and spatial views. Assembly sets include nested
assembly members. Undeformed inspection currently accepts straight members, whose authored axis
is already undeformed; unsupported member geometry returns a typed refusal.

Grid batches independently select none, the nearest axis, the farthest axis, or all axes in each
U/V family. Each family has its own naming prefix and can reuse a saved view's display settings.
Horizontal plan batches use the model's separate levels. Duplicate generated names receive a
numeric suffix; explicit duplicate names are refused. Batch saves are all-or-nothing.

Each view stores its plane, projection, visible span, above/below depth, camera rotations, work
area, render mode, grid visibility, hidden element kinds, optional colour and opacity. Camera
rotation leaves the input plane fixed. CPU picking and snapping honour the active view's kind
and depth restrictions; GPU fragments use the same depth slab and work area. Visibility and
presentation changes preserve the engineering snapshot token.

## Browser and windows

The project browser has Views, Drawings and Schedules tabs. Only Views is implemented here.
Views support named folders and subfolder paths, search, favourites, quick access, previews,
Ctrl/Shift selection, duplicate, cut/copy/paste and drag to a folder. Closing a view retains its
definition. Temporary views remain in the current UI session and are omitted from persistence.

The main viewport and up to eight auxiliary windows share geometry buffers. Views retain
independent cameras, visibility and display settings. Auxiliary windows support navigation,
tiling and docking; modelling input is handled by the active main viewport. Ctrl+Tab cycles open
views. Ctrl+P toggles between a plane-aligned camera and its previous orientation. Six directional
camera commands leave the input plane fixed. Refresh clears transient overlays; redraw reloads
the model, and the all-views variant invalidates auxiliary display caches.

## Storage and headless access

`model-views`, `create-model-view`, `update-model-view`, `delete-model-view`,
`create-view-folder` and `delete-view-folder` use the engine's validated definitions. The UI runs
these commands through its existing bounded CLI worker. `beamcraft --help` documents the full
argument set, including surface rays, per-family grid settings and global appearance updates.

The store-local `views.usda` is a standalone presentation layer with native USD Camera prims,
camera transforms, projection and aperture, plus explicitly typed application properties.
Native projection and aperture units follow the
[OpenUSD Camera specification](https://openusd.org/release/api/class_usd_geom_camera.html).
It is project-bound and read by the layer engine's bounded virtual reader. It is not inserted
into the model's immutable revision stack. Saves take an exclusive local lock, validate the whole
collection, sync a fresh temporary file and atomically replace the old layer. Invalid existing
data is never overwritten. Limits are 256 views, 512 folders, eight folder levels and 1 MiB.

This local layer does not implement shared view publication, conflict resolution, access roles,
review author/time/comment/revision metadata or drawings generated from views. Parametric
components, deformation results and curved grid axes require corresponding model capabilities;
assembly views and straight members use the geometry that exists today. No proprietary assets,
documentation text, examples, tables or UI graphics are included.

## Verification

Engine tests cover frame degeneracy, finite/range admission, depth/work-area containment,
visible picking and camera/input-plane independence. CLI integration tests create real model
geometry and exercise view constructors, batches, persistence, folders, refusal atomicity and
unchanged model revisions. Native GPU tests verify orthographic rays and actual fragment clipping.
Windowless UI controller tests exercise view switching and independent windows and render the
browser and properties panel in light and dark themes. Optional QA images are generated only
from contributor-original synthetic scenes under `target/`.
