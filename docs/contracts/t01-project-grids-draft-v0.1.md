# T01 contract draft: project grids (axes)

Status: **draft for owner review (2026-10-23).** [F04](f04-model-and-commands-v0.1.md) 0.1.0 and the
domain vocabulary stay unchanged until the owner accepts this text; on acceptance the new element
kind, commands and limits are recorded as an F04 amendment (name to be confirmed) and the sections
they extend get a dated pointer. It is the contract for the slices T01.1 to T01.6 in
[`tasks.md`](../plan/tasks.md) and answers [issue #192](https://github.com/Apibim-com/BeamCraft/issues/192).
The owner decided that development starts with grids ([decisions](../plan/decisions.md)); the
behaviour below is a proposal for that work, with the points that need the owner listed in
section 11. Nothing here is implemented.

Today the repository only says that the `Grids` layer holds "axes and levels"
([architecture](../architecture.md)), that levels are elements ([D2](../design/d1-d2-project-level.md))
and that shared grids change only by an explicit authorized command (F10 0.2 `DV-SCOPE-004`,
F12 `PC-OWN-002`). This document says what a grid is, how it is stored, built, edited and used.

## 1. Summary

| Question (issue #192) | Answer of this draft | Section |
|-----------------------|----------------------|---------|
| Kinds of grid; one object or many | Three kinds: `rectangular`, `radial`, `line`. A grid is **one element** that owns its axes as ordered records; an axis has a stable AxisId but is not an element | 2, 5 |
| Labels | Numeric, alphabetic or custom, unique per grid, numbering direction chosen at creation, stored explicitly; what a view shows is view state | 3 |
| Spacing and later edits | A list of distances, or a count and a pitch that expand to positions; positions are authoritative; axis commands edit them | 4, 8 |
| Storage and identity | Element of kind `grid` at `/World/Grids/GRD_<uuid>` in the authored model; moves to the `Grids` layer with U19; GridId and AxisId are UUIDv7 and never change | 5 |
| Relation to levels | None stored. Grids are vertical, levels are horizontal; a grid point has no height | 6 |
| Use: snap, placement, views | Snap kinds `grid_line` and `grid_intersection`; placement helper that bakes coordinates; plane of an axis for "view by grid line"; visibility is view state | 7 |
| Rights | Grids and levels share one shared scope; an ordinary section edit changes neither | 9 |
| API, limits, codes | Engine API first, then CLI; limits and reuse of the F04 codes with `GRID_*` reason tokens | 8, 10 |
| Interchange | No grid in IFC, BOM or numbering in the MVP | 12 |

## 2. What a grid is

A **grid** is a named, plan-only construction object: a set of **axes** in the horizontal plane,
each of which stands for a vertical plane at every height. It has no thickness, no mass, no
material, no geometry in the engineering sense, and no effect on any quantity. Units are
millimetres and radians, right-handed, Z up, as everywhere (F04).

A grid has a **frame**: an origin `(x, y)` and a rotation about +Z. Local coordinates `(u, v)`
are measured in that frame; the grid lives at `z = 0` and has no elevation of its own.

| Kind | Families | Axes |
|------|----------|------|
| `rectangular` | `x`: straight lines at `u = position`, parallel to local Y. `y`: straight lines at `v = position`, parallel to local X | 0 to 200 in each family, at least one axis in total |
| `radial` | `ring`: circles about the origin, `position` = radius. `ray`: half-lines from the origin, `position` = angle in radians, counter-clockwise from local X | 0 to 200 in each family, at least one axis in total |
| `line` | `line`: exactly one straight axis, the segment from the origin along local X for `length_mm` | exactly 1 |

* An **intersection** is a point where an axis of one family of the same grid crosses an axis of the
  other: for a rectangular grid `(u, v) = (x_i, y_j)`; for a radial grid `(u, v) = r_i (cos θ_j,
  sin θ_j)`. Axes of the same family never intersect (rings are concentric, parallel lines never
  meet, rays meet only at the origin, which is not an intersection). A `line` grid has none.
  Intersections are computed, never stored.
* **Extent.** Each axis is drawn and snapped only over a finite extent, which grows with the
  grid's `extension_mm` (default 2000, 0 to 1,000,000). A rectangular `x` axis runs over
  `v` from `min(y positions) - extension` to `max(y positions) + extension` (the other family
  empty: `-extension` to `+extension`); a `y` axis likewise over `u`. A ray runs from the origin to
  `max(ring radii) + extension` (no rings: to `extension`). A ring is a full circle. A `line`
  axis is exactly its segment. Per-axis extents and partial ring arcs are open (O-G3).
* The kind is fixed at creation. Changing it is `UNSUPPORTED_PROPERTY`.
* Not supported in this version (`UNSUPPORTED_GEOMETRY` unless noted): grids inclined from the
  vertical or rotated about any axis but Z, arcs and splines other than the full ring, skewed
  rectangular grids (a family not at a right angle to the other), a grid with its own elevation,
  and three-dimensional grids. A rectangular family at an arbitrary angle is made as a second
  rectangular grid with its own frame; a single skewed axis is a `line` grid.

## 3. Axis labels

Every axis has one **label**, the name the user sees and types.

| Rule | Value |
|------|-------|
| `GR-LBL-001` Characters | 1 to 16 Unicode characters and at most 64 UTF-8 bytes; Unicode letters and digits, `-`, `_`, `.`, `+`. No spaces and none of `/ : , #`, so a label can appear in the point syntax of section 7. Letters of any script are allowed |
| `GR-LBL-002` Uniqueness | Unique **in the grid**, across both families, so one label names one axis and the label of an intersection needs no family tag. Compared exactly (code point by code point, no case folding, no normalisation). A clash is `INVALID_ARGUMENT`, reason `GRID_LABEL_DUPLICATE`. A different grid may reuse a label |
| `GR-LBL-003` Stored | Labels are stored on the axes. Moving an axis, or adding or removing another, never renumbers or relabels. A gap in the sequence is allowed |
| `GR-LBL-004` Not an identity | A label can change; the identity is the AxisId (section 5). Nothing in the model may depend on the text of a label |

**Label schemes.** A scheme expands to an explicit list of labels when a grid is created or a
family is relabelled. The expansion is a pure function of the model crate, used by the CLI and the
UI; a command carries the expanded list, so replay does not depend on the scheme code.

| Scheme | Rule |
|--------|------|
| `numeric` | Decimal integers, `start` (default 1, at least 0) and `step` (default 1, at least 1): `start`, `start + step`, ... No padding |
| `alpha` | Letters in the order A to Z, AA, AB, ..., AZ, BA, ... (bijective base 26: after `Z` comes `AA`). `start` is a label of this sequence (default `A`). An optional `skip` set of letters (default none) is left out of every label, so with `skip = {I, O}` the sequence is A to H, J to N, P to Z, then AA, AB, ... |
| `custom` | An explicit list with exactly one label for each axis |

**Direction.** `order` is `ascending` (default: the first label goes to the axis with the lowest
position) or `descending` (the first label goes to the axis with the highest position). For
`ray` the position is the angle. Defaults when a command does not say: `x` numeric, `y` alpha,
`ring` alpha, `ray` numeric; the user can change every one (O-G9).

**Labels per view.** Which labels a view shows, at which end of the axis and in which size, and
whether a grid is shown at all, is **view state** (task W01, V12 and the drawing views of D04), not
model data. Until the views layer exists the UI keeps it in the session. A hidden grid or a hidden
label never changes the model, any revision, any signature or any export (`GR-USE-005`).

## 4. How spacing is given

A family is created from a starting position and either of two inputs, as for bolt groups
([A14 draft](f04-bolt-fastener-amendment-draft-v0.1.md) section 4):

| Input | Rule |
|-------|------|
| `start` | Position of the first axis. Default 0 for `x`, `y`; the radius of the first ring (greater than zero); the angle of the first ray, 0 to less than 2π (degrees at the CLI) |
| `spacings` | The distances between neighbouring axes, a list. The number of axes is `len(spacings) + 1`. The CLI writes repetition as `N*D`: `3*6000,7500` is `6000, 6000, 6000, 7500` |
| `count` + `pitch` | `count` axes at an equal distance: the same as `count - 1` spacings equal to `pitch`. `count` 1 takes no `pitch` |

Positions follow by summation: `p[0] = start`, `p[i] = p[i-1] + spacings[i-1]`. For rays the
spacings are angles.

**Positions are authoritative.** `spacings` and `count` + `pitch` are generators used when a grid or
an axis is created; they are expanded at once and **not stored**, so editing one axis never has to
guess what an equal pitch meant. (The bolt amendment keeps spacings authoritative because a group is
a pattern; a grid is a set of named lines, so the position is the value users edit. O-G2.)

**Editing afterwards** is by axis commands (section 8): add an axis at a position, move one axis
alone, move an axis together with every later axis of the family (stretching or shrinking a bay),
remove an axis, relabel. A request to change `count` or `pitch` of an existing family does not
exist.

## 5. Storage and identity

### 5.1 One element, axes as records

A grid is **one element** of kind `grid`. Its axes are **records inside it**, not elements. The
reasons, against one element per axis:

* Label uniqueness, ordering and the minimum distance are properties of the whole set. One element
  keeps them inside one footprint and one validator rule. Separate axis elements would let two
  concurrent edits each pass their own footprint and together break uniqueness.
* A family of up to 200 axes stays one prim, not 200 elements with their own identity, lifecycle,
  relationship and copy rules.
* Deleting, copying and permission checks act on one object.

The price is that an axis cannot be the target of a USD relationship. A reference to an axis is a
pair `(GridId, AxisId)`: a relationship to the grid plus the AxisId attribute (section 5.4). Axes as
child elements is the alternative, listed as O-G1.

### 5.2 Where it is stored

| Item | Rule |
|------|------|
| Path | `/World/Grids/GRD_<uuid>`, the category folder `Grids` (USD `kind = group`) beside `/World/Levels`; prefix `GRD`; the full 32 hex digits of the UUIDv7, as every element (F04 `A-3`, [architecture](../architecture.md#prim-hierarchy-and-naming-rule)). A grid is shared by every discipline and sits outside `/Steel` |
| Layer | Today the authored model, as levels are. When U19 lands, grid **and** level elements move to the `Grids` layer (`<code>[_<zone>]_Grids.usda`) by task T01.5; their paths and identities do not change, and no migration of earlier stores is promised while the project is pre-alpha |
| Kind | `bim:elementKind = grid`. USD `kind = component`. No `Geometry` child, no material, no mass |
| Placement | `xformOp:translate` = `(x, y, 0)` and `xformOp:orient` = a rotation about +Z only. `z != 0`, or an orientation with an X or Y component, is `UNSUPPORTED_GEOMETRY`, reason `GRID_PLACEMENT_UNSUPPORTED` |
| Common | `bim:id`, `bim:name` (label rule of F04, 1 to 256 bytes), `bim:schemaVersion`, `bim:createdBy` |

Attributes (explicit, readable without a schema, as all others):

| Attribute | Meaning |
|-----------|---------|
| `bim:grid:kind` | token `rectangular`, `radial` or `line` |
| `bim:grid:extensionMm` | extension of section 2 |
| `bim:grid:lengthMm` | `line` only: the length of the axis |
| `bim:grid:<family>:ids` | string array, the AxisIds, in order |
| `bim:grid:<family>:labels` | string array, the labels, same order |
| `bim:grid:<family>:positionsMm` | double array: `x`, `y`, `ring`, `line` (always `[0]`) |
| `bim:grid:ray:positionsRad` | double array: the ray angles |

`<family>` is `x`, `y`, `ring`, `ray` or `line`. The three arrays of a family have the same
length, positions are strictly increasing, and a kind carries only its own families. A stored grid
that breaks these rules is refused by the structural validator with `INVALID_ARGUMENT` (rule
`T01-V01`), array lengths above the limits are refused before allocation with `LIMIT_EXCEEDED`.

### 5.3 Identity

* **GridId** is a UUIDv7 and never changes: not on rename, move, rotation, relabelling or edit of
  any axis. The prim path is derived from it.
* **AxisId** is a UUIDv7 unique **in the project**, never reused (also not after the axis is
  removed), so `(GridId, AxisId)` and the AxisId alone each name one axis for ever. Moving an axis,
  changing its label or its position keeps the AxisId. Removing an axis ends it; adding an axis with
  the same label creates a new AxisId.
* A command carries the ids of everything it creates (`new_id`, `axis_ids`), so replay under the
  same `op_id` is exact (F04 section 6). The CLI generates them when they are omitted. A duplicate
  GridId or AxisId, inside the command or against the project, is `DUPLICATE_ID`.

### 5.4 Edits, forks, copies and references

| Event | Behaviour |
|-------|-----------|
| Edit | Any grid command keeps GridId and every unaffected AxisId (section 8) |
| Workspace fork, publication, rebase | A fork is the same project history, so all ids are the same. A later merge matches grids and axes by id, never by label or position |
| `CopyElement` of a grid | A new independent grid: a fresh GridId and **fresh AxisIds**, all supplied by the command. Nothing is shared, so a reference to the copy's axis can never mean the source's |
| Reference to an axis | `(GridId, AxisId)`. Saved views (W01) and, later, drawing dimensions (D03) store the pair, plus a relationship to the grid so a dangling grid is found. An axis that disappears leaves the reference **unresolved** and visible, never silently dropped |
| Delete or remove | Refused with `DEPENDENCY_CONFLICT` (reason `GRID_IN_USE`) while a stored reference names the grid or the axis; the diagnostic names the referrers. A project with no such referrer deletes freely |

No element of the model refers to a grid or an axis: parts keep coordinates only (section 7).

## 6. Grids, levels and zones

* **Levels stay as D2 made them:** elements of kind `level` with `bim:elevationMm`, shared by every
  discipline. A level has no relation to a grid. A grid has no height, a level has no plan position;
  a point on a level and an intersection give a three-dimensional point only when a caller pairs
  them (section 7.2). The elevation of a level against grid axes is therefore the elevation value
  itself, in the same project coordinates the grid origin uses.
* **No "one common grid" is required.** A project may hold any number of grids (at most 64), which
  may overlap. A "common" grid is simply one in the unzoned `Grids` layer. Once U19 lands, a zone's
  grids and levels live in that zone's `Grids` layer, with the zone's own frame, so two zones may
  hold two rotated rectangular grids side by side. Which zone a grid belongs to is the layer it is
  stored in, never an attribute of the grid.
* **Levels and grids are shared data of one kind** (section 9): both are written only by the shared
  scope, and sections only read them.
* A "view by grid line" and a "view at a level" combine freely in V12: the first fixes a vertical
  plane, the second a horizontal one.

## 7. How grids are used

### 7.1 Snapping

`snap` (V04) gains two kinds beside `vertex`, `axis_end` and `axis`:

| Kind | Candidate |
|------|-----------|
| `grid_intersection` | An intersection of section 2 |
| `grid_line` | The nearest point of an axis over its extent (straight: perpendicular foot clamped to the extent; ring: the nearest point of the circle) |

* **Plan distance.** The distance is measured in the XY plane. A hit keeps the query point's `z`:
  a grid has no height, and the caller's work plane or level supplies it.
* **Ordering and bands** are the V04 rules: quantized distance, kind, then ids. The new kinds rank
  after the existing ones, `grid_intersection` before `grid_line`, then GridId, AxisId (the pair of
  AxisIds for an intersection, in family order `x`, `y` and `ring`, `ray`). A hit carries `(GridId,
  AxisId[, AxisId])` and the label(s), never a FeatureId. A UI may prefer intersections by its own
  rule; the engine returns the whole ordered list.
* **Work bounds.** At most 64 grids and 400 axes per grid are read. Intersections are not
  enumerated: for each family the candidates within the radius are found in the sorted positions,
  so a `200 x 200` rectangular grid costs the work of the nearby axes, not 40,000 points.
* **Visibility is the caller's.** `snap` takes an optional grid filter (`--grids ID,ID`, default all).
  A grid a view hides is simply not passed.
* The query is read-only and never changes the head.

### 7.2 Placing an element on an intersection

A grid is a **placement helper, not a link** (`GR-USE-003`). The engine API
`grid_point(grid, label_a, label_b)` returns the world point `(x, y)` of the intersection of two
labels, one from each family, in either order (`B/3` and `3/B` are the same point); two labels of
one family, a label of another grid or an unknown label are refused (section 10). Commands that
take points accept the syntax `GRID:LABEL/LABEL` (`GRID` is a GridId, or a grid name when exactly
one live grid has it) and an explicit `--z`. The CLI resolves them **before** it builds the
command, so the stored command and the stored element hold plain coordinates:

* moving a grid or an axis afterwards moves no element and changes no signature, and replaying an
  old `op_id` gives the same result;
* the part has no relationship to the grid, so an ordinary section edit never reads or writes it;
* an element that follows grid changes (associative placement) is out of scope (O-G5).

T01.4 adds the syntax to `create-member` (`--start-at-grid`, `--end-at-grid`) and to the
placement origin of `create-plate`. Taking `z` from a level is O-G6.

### 7.3 View by grid line

`grid_axis_plane(grid, axis)` returns the vertical plane of a straight axis as a world point and a
unit normal (horizontal, perpendicular to the axis direction) for a `rectangular` axis, a `ray`
or the `line` axis. A `ring` has no plane: `UNSUPPORTED_GEOMETRY`, reason `GRID_AXIS_NOT_PLANAR`.
A saved view (V12) stores `(GridId, AxisId)`, the side and the depth, and **re-resolves** the plane
each time it opens, so moving the axis moves the view; a missing axis leaves the view unresolved
(`GR-USE-004`). Plane views by level are unchanged.

### 7.4 Visibility and selection

Showing or hiding grids, labels and ends is view state (section 3). Selecting a grid is a V11
selection switch (`grids`, already in V11) that changes picking only. Hiding a grid never changes
the model, the revision, the snapshot token or any export (`GR-USE-005`).

## 8. Commands (engine API first, then CLI)

Each command is one atomic edit and one undo unit, takes `op_id`, validates before it writes and, on
any refusal, changes nothing. CLI option names are indicative; T01.1 fixes them.

| Command (CLI) | Arguments | Behaviour |
|---------------|-----------|-----------|
| `CreateGrid` (`create-grid`) | `new_id`, `name`, `kind`, `origin`, `rotation`, `extension`; per family `start` and the spacings or `count` + `pitch`, the scheme (`numeric`, `alpha`, `custom`, `order`), and `axis_ids`; for `line`: `start`, `end` points instead of origin and rotation, and one `label` | Validates, expands generators and schemes, authors the grid. The CLI takes degrees for angles |
| `UpdateGrid` (`update-grid`) | `target_id`; any of `name`, `origin`, `rotation`, `extension`, `length` (`line`) | Moves or renames the grid as a whole; axes and ids are unchanged |
| `GridAxisAdd` (`grid-axis-add`) | `target_id`, `family`, `new_axis_id`, `position`, `label` | Inserts one axis in order; the minimum distance and label rules hold |
| `GridAxisMove` (`grid-axis-move`) | `target_id`, `axis_id`, `position`, `mode` = `single` (default) or `shift` | `single`: moves only this axis, which must stay strictly between its neighbours. `shift`: moves this axis and every later axis of the family by the same delta (a bay becomes longer or shorter) |
| `GridAxisRemove` (`grid-axis-remove`) | `target_id`, `axis_id` | Removes the axis; other labels and ids are untouched. The last axis of a `line` grid, or of a grid whose removal leaves no axis, is refused: delete the grid |
| `GridRelabel` (`grid-relabel`) | `target_id`, then `axis_id` + `label`, or `family` + the expanded `labels` list | Changes labels only; ids and positions are untouched |
| `DeleteElement` (`delete-element`) | `target_id` | Deletes the grid with its axes; refused while a stored reference exists (section 5.4) |
| `CopyElement` (`copy-element`) | `source_id`, `new_id`, `name`, `new_axis_ids` | An independent copy (section 5.4) |

**Queries** (read-only, no revision): `grids` (every grid with kind, frame and each axis: id, label,
position, and the world coordinates of the axis ends), `grid-point` (section 7.2), `grid-plane`
(section 7.3) and `snap` with the new kinds (section 7.1). `plate-inspect` lists grids beside
levels.

**Footprints.** Create: read the absence of the GridId and of every AxisId, write the new
lifecycle and the complete authored fields. Any other grid command: read the lifecycle and the
complete `shape_inputs` group of the grid, write that group (and `label` for a rename or
`placement` for a move). A grid command reads and writes no other element. Axis ids are checked
against an index of all AxisIds in the project, which a stored-graph validator also rebuilds.

## 9. Rights

Grids and levels are **shared data** with their own shared scope (F12 `PC-OWN-002`, F10 0.2
`DV-SCOPE-004`).

| Operation | Section user (ordinary edit) | Shared `Grids` scope |
|-----------|------------------------------|----------------------|
| Read grids and levels, `snap`, `grid-point`, `grid-plane`, `--at-grid` placement | allowed | allowed |
| Create, update, copy or delete a grid; add, move, remove or relabel an axis | `PERMISSION_DENIED` | allowed |
| Create or change a level (D2) | `PERMISSION_DENIED` | allowed |
| Ordinary `UpdateElement`, create or copy of a part | may not touch a grid or a level and cannot: a part holds only coordinates and a `bim:level` link | n/a |

* An ordinary section edit therefore **never changes a grid or a level**. A KMD edit preserves the
  grids exactly (`PC-OWN-001`).
* A grid reached through a read-only reference or a pinned source project is `FOREIGN_READ_ONLY` for
  every write; whether a section project may *use* a pinned grid for snaps and placement is open
  (O-G4).
* **Transition.** Before U19 a workspace has one scope and grid commands run without an
  authorization check, as `create-level` does today. T01.5 puts grids and levels under the shared
  scope together with U19; the command set and the engine API do not change, only the refusal.
* Authority comes from the trusted execution context, never from a file name, a layer name or a
  client-provided scope (`DV-SCOPE-001`, `DV-SCOPE-002`).

## 10. Limits, codes and what is unsupported

Limits are original operating limits, not promises of anything else. They are checked before
allocation and never widened.

| Item | Limit | Above or outside |
|------|-------|------------------|
| Grids in a project | 64 | `LIMIT_EXCEEDED`, `GRID_COUNT_LIMIT` |
| Axes in one family | 200 (`line`: 1) | `LIMIT_EXCEEDED`, `GRID_COUNT_LIMIT` |
| Entries of a `spacings` list, a repetition `N`, a `count` | 200 | `LIMIT_EXCEEDED`, `GRID_COUNT_LIMIT` |
| One spacing, `abs(position)`, `length_mm` | at most 1,000,000 mm | `LIMIT_EXCEEDED` |
| Origin | the coordinate cap of F04, 1,000,000,000 mm | `LIMIT_EXCEEDED` |
| Minimum distance between neighbouring axes | 1 mm (rings and positions); 1e-6 rad (rays). Positions strictly increase | `INVALID_ARGUMENT`, `GRID_POSITION_ORDER` |
| Spacing, pitch, extension | spacing and pitch greater than zero; extension 0 to 1,000,000 | `INVALID_ARGUMENT`, `GRID_SPACING_INVALID` |
| Ring radius, ray angle, rotation | radius at least 1 mm; angle in `[0, 2π)`; `abs(rotation)` at most `2π` | `INVALID_ARGUMENT` |
| Label, name | section 3; F04 label rule | `INVALID_ARGUMENT`, `GRID_LABEL_INVALID` |
| Non-finite number | any numeric input | `NON_FINITE_NUMBER` |

**Codes.** No new top-level code: F04's codes are used with a diagnostic `reason` token.

| Case | Code | Reason |
|------|------|--------|
| Two axes with one label in a grid | `INVALID_ARGUMENT` | `GRID_LABEL_DUPLICATE` |
| Unknown grid, unknown axis or label | `NOT_FOUND` | `GRID_AXIS_UNKNOWN` |
| Two labels of one family, or a `line` grid, for an intersection | `INVALID_ARGUMENT` | `GRID_INTERSECTION_UNDEFINED` |
| A grid name that more than one live grid carries, used as a grid reference | `INVALID_ARGUMENT` | `GRID_NAME_AMBIGUOUS` |
| A family the kind does not have; a change of kind; an axis added to a `line` grid | `UNSUPPORTED_PROPERTY` | `GRID_KIND_FIXED` |
| Placement or shape outside section 2 | `UNSUPPORTED_GEOMETRY` | `GRID_PLACEMENT_UNSUPPORTED`, `GRID_AXIS_NOT_PLANAR` |
| Delete or remove while a reference exists | `DEPENDENCY_CONFLICT` | `GRID_IN_USE` |
| Used GridId or AxisId | `DUPLICATE_ID` | `GRID_ID_USED` |
| No authority; foreign grid | `PERMISSION_DENIED`; `FOREIGN_READ_ONLY` | |
| Same `op_id`, other content | `OP_ID_REUSE` | |

A malformed stored grid is `INVALID_ARGUMENT`, rule `T01-V01`; the reader never indexes with a
stored position and caps every array before reading it.

## 11. Open questions for the owner

| # | Question | Draft position | Blocks |
|---|----------|----------------|--------|
| O-G1 | Axes as records of one grid element, or as child elements | Records (section 5.1) | T01.1 |
| O-G2 | Positions authoritative, generators not stored, or keep the generator for later "change the pitch" edits | Positions authoritative | T01.1 |
| O-G3 | Per-axis extents, partial ring arcs, skewed and arc axes, an axis style | After the MVP; one grid-wide extension now | T01.2 |
| O-G4 | May a section project use a grid from a pinned coordination source for snaps and placement | Yes, read-only, by `(project, GridId)` | T01.5 |
| O-G5 | Associative placement: parts that follow a moved axis | No link in the MVP (section 7.2) | after MVP |
| O-G6 | Take `z` of a placed element from a level | Not in T01.4; `--z` only | T01.4 |
| O-G7 | Write the axes as USD `guide` curves for other USD viewers (U12 axis representation) | No geometry now | U12 |
| O-G8 | Limits of section 10 (64 grids, 200 axes per family) and the label character set | As drafted | T01.1 |
| O-G9 | Default schemes and an optional `skip` set of letters | `x` numeric, `y` alpha, no skipped letters | T01.1 |
| O-G10 | Must grid names be unique | No; a duplicate name is refused only as a grid reference (`GRID_NAME_AMBIGUOUS`) | T01.1 |

## 12. Interchange

In the MVP **no export carries grids**.

* **IFC** (E01, deferred): an element of kind `grid` is not exported and is not counted as an
  element, as a `level` is a storey and not an exported element. The subset of E01 section 2.3
  already lists grids as not exported; limitation L-12 now reads "the export records no grids".
* **BOM, numbering, schedules, quantities:** a grid has no part number, mass or volume and is
  skipped by every one; adding a grid never makes numbering stale and never changes a signature.
* **DXF, DSTV, STEP, FBX, DWG:** not part of the MVP. A drawing view (D04) may draw the grid lines
  its view shows as view content; that is drawing data, not an exchange of grids.
* **USD:** the grid is an ordinary `Xform` with attributes in the project's layers, readable by any
  USD tool, and carries no geometry (O-G7).

## 13. Acceptance cases (to be bound to fixtures)

Tolerance for coordinates is 1e-6 mm unless stated. Each negative case changes nothing.

Positive:

| # | Case | Expected |
|---|------|----------|
| GR-P01 | Rectangular grid, origin `(0, 0)`, rotation 0; `x`: start 0, spacings `3*6000`, numeric; `y`: start 0, spacings `7500,4500`, alpha | `x` positions `[0, 6000, 12000, 18000]` labelled `1` to `4`; `y` positions `[0, 7500, 12000]` labelled `A` to `C`; 12 intersections; `B/3` and `3/B` are `(12000, 7500)` |
| GR-P02 | The same `x` family from `count 4`, `pitch 6000`; `count 1` without a pitch | Identical positions; a single axis at `start` |
| GR-P03 | The grid of GR-P01 with origin `(1000, 2000)` and rotation 90° | `B/3` is `(-6500, 14000)` (`(-7500 + 1000, 12000 + 2000)`) |
| GR-P04 | Schemes: `alpha` from `A`, 28 axes; `alpha` with `skip = {I, O}`, 26 axes; `numeric` descending, 5 axes at 0 to 4000 pitch 1000; `custom` list | A to Z, `AA`, `AB`; A to H, J to N, P to Z, `AA`, `AB`; labels `5,4,3,2,1` at positions `0,1000,2000,3000,4000`; the list as given |
| GR-P05 | Radial grid: `ring` start 5000, spacings `5000` (radii 5000 and 10000); `ray` start 0°, spacings `2*45°` (0°, 45°, 90°) | Intersection of ring 10000 and ray 45° is `(7071.067811865476, 7071.067811865476)`; 6 intersections |
| GR-P06 | `line` grid from `(0, 0)` to `(3000, 4000)` | origin `(0, 0)`, rotation `0.9272952180016122` rad, length 5000, one axis |
| GR-P07 | Each applied alone to the grid of GR-P01: `single` move of axis `2` to 6500; `shift` move of axis `2` to 6500; add axis `2A` at 9000; remove axis `4` | `[0, 6500, 12000, 18000]`; `[0, 6500, 12500, 18500]`; `[0, 6000, 9000, 12000, 18000]`; labels `1, 2, 3` remain. Every AxisId unchanged except the new one; GridId unchanged; each command is one undo unit |
| GR-P08 | `snap` at `(12003, 7502, 3000)`, radius 10, kinds `grid_intersection,grid_line` on GR-P01 | Hits in order: line `B` at `(12003, 7500, 3000)` (distance 2), line `3` at `(12000, 7502, 3000)` (3), intersection `B/3` at `(12000, 7500, 3000)` (3.6056). The same query with `--grids` omitting the grid returns none, and the head is unchanged |
| GR-P09 | `create-member` from `--start-at-grid G:B/3` to `--end-at-grid G:B/4`, `--z 3000` on GR-P01; then move axis `3` | Member axis `(12000, 7500, 3000)` to `(18000, 7500, 3000)`; after the move the member, its shape key and its mass signature are unchanged; its stored command holds coordinates |
| GR-P10 | `CopyElement` of a grid | A new GridId and new AxisIds, same labels and positions; both grids snap independently; deleting one leaves the other |
| GR-P11 | Create a grid on a project with a plate, a member and a level; then export the BOM and run numbering | Element list, quantities and part numbers equal those of the same project without the grid; numbering is not stale; the grid is not in the IFC element list |
| GR-P12 | A KMD edit (rename a part, move a plate) under the shared-scope rules of T01.5 | Every grid and level is byte-equal before and after; with no `Grids` authority a grid command is `PERMISSION_DENIED` |
| GR-P13 | A rectangular axis and a ray: `grid-plane` | Plane through the axis, horizontal unit normal; a ring is `UNSUPPORTED_GEOMETRY`. Restart and reopen give the same result |

Negative:

| # | Case | Expected |
|---|------|----------|
| GR-N01 | `x` label `1` and `y` label `1` in one grid; relabel to an existing label | `INVALID_ARGUMENT`, `GRID_LABEL_DUPLICATE` |
| GR-N02 | A label with a space, `/`, `:` or `,`; an empty label; 17 characters | `INVALID_ARGUMENT`, `GRID_LABEL_INVALID` |
| GR-N03 | Spacing 0 or negative; neighbours 0.5 mm apart; `single` move past a neighbour; a ray at 360° | `INVALID_ARGUMENT` (`GRID_SPACING_INVALID` or `GRID_POSITION_ORDER`) |
| GR-N04 | 201 axes in a family; the 65th grid; spacing 1,000,001 mm; origin 1,000,000,001 mm | `LIMIT_EXCEEDED` |
| GR-N05 | NaN or infinite spacing | `NON_FINITE_NUMBER` |
| GR-N06 | `grid-point` with two labels of one family, an unknown label, or on a `line` grid | `INVALID_ARGUMENT` (`GRID_INTERSECTION_UNDEFINED`) or `NOT_FOUND` (`GRID_AXIS_UNKNOWN`) |
| GR-N07 | Add an axis to a `line` grid; change the kind; give `ring` axes to a rectangular grid | `UNSUPPORTED_PROPERTY` |
| GR-N08 | Placement rotated about X, or `z != 0` | `UNSUPPORTED_GEOMETRY`, `GRID_PLACEMENT_UNSUPPORTED` |
| GR-N09 | Duplicate GridId; an AxisId repeated in the command or used by another grid | `DUPLICATE_ID` |
| GR-N10 | The same `op_id` with other content | `OP_ID_REUSE` |
| GR-N11 | Remove an axis, or delete a grid, named by a saved view (after W01) | `DEPENDENCY_CONFLICT`, `GRID_IN_USE`; the view stays |
| GR-N12 | A stored grid whose arrays differ in length, whose positions are not increasing, whose AxisId repeats, or whose array exceeds the limit | `INVALID_ARGUMENT` or `LIMIT_EXCEEDED`; no panic, no allocation sized by the file |
| GR-N13 | A write to a grid of a foreign reference | `FOREIGN_READ_ONLY` |

## 14. Slices and dependencies

Recorded in [`tasks.md`](../plan/tasks.md): T01.1 rectangular grids and single lines (model,
layer engine, engine, CLI); T01.2 radial grids; T01.3 axis commands; T01.4 queries, snaps and
placement; T01.5 shared-scope rights and the move to the `Grids` layer with U19; T01.6 the UI.
Every slice has its own end-to-end test (`AGENTS.md` section 3).

## 15. Provenance

The request and the decision to begin with grids are the owner's (issue #192). The model, the
rules, the limits and the label sequence are our own and use only what this repository already
defines: the element and prim rules of F04, the level of D2, the snap of V04, the bolt-group
generators and the F10 0.2 and F12 scope rules. The alphabetic sequence is the ordinary
bijective base-26 numbering. The behaviour is described in our own words; nothing was taken from
Tekla Structures or another commercial product, and no screenshot or asset is part of this
document (`AGENTS.md` sections 1 and 2).
