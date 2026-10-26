# T01.6: native grid workbench

The Grids ribbon action opens a native panel for grid creation, regular-family/frame edits,
drawn-line creation/reshape/removal and line/family relabelling. The pure `GridForm` maps fields to
the existing CLI commands; it computes no engineering geometry. Preview stages the exact argument
list, Commit runs it through the existing bounded process runner, and the result refreshes the
accepted model. Editing a field invalidates Commit until a new matching preview. Cancel and Escape
clear the staged command and two-click input without publishing. Unchanged name/frame/family fields
are omitted from updates, preserving the stored quaternion rather than round-tripping it through
angles on an unrelated edit.

An empty U expression at creation starts a drawn U family from its first line; an empty V
expression leaves V drawn. The line tools use world XY fields or two clicks on the Z=0 work plane.
Engine snaps are used for those clicks and for member placement. The snap switch and per-grid
visibility select which grids participate; hidden grids are neither drawn nor snapped to. Labels
are native UI text. Visibility, label display, radius and snap state live only in the window until
saved-view support arrives. They create no revision or snapshot token.

`list-grids` now includes its immutable `model_manifest` and additive exact frame, local endpoint
and regular-span fields. The panel decodes bounded typed grid records, admits them through model
validation, and draws/snaps only when the listing's manifest equals the engineering snapshot's pin.
Refresh runs asynchronously on a worker. Empty engineering scenes are framed around the grid
bounds, with an f64 display origin; the engineering snapshot itself is unchanged. Native lines
and labels are viewport overlays; the T01.7 USD guide mesh remains for other viewers.

Pure form/cancel tests are complemented by a native-adapter panel paint test and real-binary tests
of UI/headless geometry equivalence, retained AxisIds, cancellation and visibility/snap controls.
The real-binary tests use `BEAMCRAFT_TEST_CLI`, a nextest binary path when supplied, or the sibling
workspace CLI executable; they explicitly skip if no CLI binary is available. In this cloud session
an explicit freshly built CLI path is provided, so neither integration scenario is skipped.

Follows #422/#421 and the contract proposal #415. Shared-scope rights, pinned coordination grids,
saved views and magnetic placement remain their separately gated tasks. This slice does not claim
the parent T01 acceptance gate.

Contributor-original MPL-2.0. No dependencies or assets added.
