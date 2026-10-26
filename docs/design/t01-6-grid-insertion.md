# T01.6 regular-grid insertion

Experimental insertion slice, not completion of T01.6 or the parent T01 acceptance gate.

## Dependencies and merge order

This implementation builds on verified `origin/master` `65728b0`, which contains the regular-grid
model and drawn-line commands integrated through #417. The native form uses those commands and
their stored grid records. It does not require the unmerged regular-family editing slice #419.

The local development branch also carries the accepted command-navigation UI changes. For review,
integrate those first or use their branch as the base so the insertion PR shows only its own
changes. This note does not change the implementation-plan status or claim the parent gate.

## Behaviour

The Model ribbon's Grid tool opens an original BeamCraft inspector. It creates a regular grid
through the existing `create-grid` command: name, U/V coordinate expressions, explicit or
automatic labels, optional line limits, XY origin and Z rotation in degrees. Empty line limits
use the engine defaults. The form uses the shared model parsers; all engineering admission
remains with the engine. Creating is one atomic revision. Cancel runs no command. A refusal
leaves the model unchanged and is shown as a readable message.

Explicit label input may contain more words than coordinates produce lines. The shared model
parser uses only the first label per existing line; unused trailing words are ignored by both the
UI and CLI before building a canonical command. Missing labels continue from the last supplied
label: letters advance in the same alphabet and case; trailing numbers increment. Other names
gain a numeric suffix. Already-used labels in the family are skipped. Blank input in the form
selects the default automatic labels. This does not add axes or retain unused words in the
model. The accepted labels retain their validation and grid-wide uniqueness requirements.

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
