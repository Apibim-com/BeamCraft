# T01.6 regular-grid insertion

Experimental insertion slice, not completion of T01.6 or the parent T01 acceptance gate.

## Dependencies and merge order

This local implementation builds on the accepted UI work in `feat/ui-modeling` and the existing
grid stack: contract #415 (`ba9a855`), T01.1 #416 (`9150947`), T01.2 #417 (`29f90d5`) and
T01.3 #419 (`78df3d8`). They are preserved in the local integration base
`chore/t01-ui-prerequisites`, based on verified `origin/master` `f5f3d3e`. Publish the insertion
diff against that base only while its prerequisites are open; after they land, rebase and retarget
to `master`. No prerequisite is merged into `master` by developing this slice.

## Behaviour

The Manage ribbon's Rectangular grid tool opens the restored original BeamCraft inspector. It creates a regular grid
through the existing `create-grid` command: name, U/V coordinate expressions, explicit or
automatic labels, optional line limits, XY origin and Z rotation in degrees. Empty line limits
use the engine defaults. The form uses the shared model parsers; all engineering admission
remains with the engine. Creating is one atomic revision. Cancel runs no command. A refusal
leaves the model unchanged and is shown as a readable message.

The creation and cancel buttons stay below the scrollable fields. Grid visibility is session
view state; its switch never authors a revision. Levels remain separate from plan grids.

## Pinned display transport

`export-view --with-grids true --out -` adds the engine-computed world endpoints and labels from
the same verified inspection as the mesh snapshot and definitions. The opt-in `BCVXFRM2` envelope
has three bounded length-prefixed sections (definitions, selected element, grids), then the
ordinary engine bundle. Existing exports retain `BCVXFRM1` or their unframed format. The UI reads
both envelopes, using one CLI process per successful refresh.

The UI bounds and validates guides before displaying them, frames the camera to their union with
the engineering bounds, and paints guides/labels as a construction overlay. It does not create
engineering meshes, quantities or materials. A failed scene/frame update keeps the previously
displayed scene and its guides together. Guide visibility and label appearance are not authored
model properties.

## Verification and limits

Actual-binary acceptance compares form insertion with the direct CLI, checks rotated endpoints,
labels, one pinned refresh, framing, and refusals without publication. Native egui input tests
cover opening the form, hiding guides, cancel, and preserving the snapshot. Both legacy and new
frame readers remain bounded.

This slice does not add grid selection/property editing, drawn-line gestures, grid snaps or
intersection placement (T01.4), magnetic behaviour, radial grids, or derived USD guide tubes
(T01.7). New preview builds use the prerequisite grid schema; older stores are not silently
migrated. No new dependency or asset is introduced. Code and synthetic tests are contributor
original under MPL-2.0; no commercial screenshot or UI asset is included or traced.
