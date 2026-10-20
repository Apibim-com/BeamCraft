# Tasks

Every task by ID. Size: **S** is about 2–4 hours of focused work, **M** about 4–8; a guide, not
a promise. A large task is split into numbered slices (A07.1, V05.2, ...) before it starts, and
the parent ID stays the acceptance gate. Status is the state on `master`:

- **merged**: slices are on `master`, experimental, acceptance not claimed (see the design note);
- **accepted**: the acceptance criteria hold on the integrated result;
- **done**: a research, audit or contract task that delivered its report or document;
- **open**, **deferred** (after the MVP), **later** (needs an owner decision).

Status as of 2026-10-20, `master` at `d456670`. New tasks are added here in a docs-only PR.

## F: foundations and contracts

| ID | Task | Depends on | Result | Acceptance | Status |
|----|------|------------|--------|------------|--------|
| F01 / S | Integration baseline | — | Audit of the base commit, licence and rules; list of roadmap discrepancies | Exact base SHA, clean state, discrepancy matrix | done |
| F02 / M | Requirements map | — | Index requirement → source → gate → test; owner decisions above the specification | Every source classified; unknowns marked; IFC import not in the MVP | done, kept outside the repo |
| F03 / M | OpenUSD distribution audit | — | Audit of the SDK version, TOST, dependencies, packaging and platforms | Reproducible delivery option and its conditions; no exception added unasked | done ([audit](../audits/openusd-26.8.md)) |
| F04 / M | [Model and command contract](../contracts/model-and-commands-v0.1.md) | F01, F02, F09 | UUID and path, parameter types, schema version, errors, base revision, read/write dependencies, create/update/delete | Contract and negative cases agreed; no hidden JSON model; units and limits set | frozen 0.1.0 |
| F05 / M | [Private USD-worker protocol](../contracts/worker-protocol-v0.1.md) | F04 | Versioned requests and responses, capabilities, timeouts, cancellation, limits, revision token | Same contract for CLI and host; private IPC without a listening socket | draft (SDK fallback path) |
| F06 / M | [Geometry and viewport contract](../contracts/geometry-contract-v0.1.md) | F04 | Kernel inputs, results and diagnostics, FeatureId, mesh buffers, transforms, query API | Shape, world measurement and presentation have separate keys; topology and bounds bounded | frozen 0.1.0, amendment A-2 |
| F07 / M | Original synthetic fixtures | — | Generator and manifest of our own test bodies, provenance, analytical references | No vendor asset; references separate from unverified cases; stable fixture IDs | done (`tests/fixtures/f07`) |
| F08 / M | [Module boundaries](../contracts/module-boundaries-v0.1.md) | F04, F05, F06, F07 | Crate layout and allowed dependencies; CI targets | Pure crates pass the WASM check; native host isolated; checker in CI | draft; checker in CI |
| F09 / M | Clean-room separation for the kernel | F01, F02 | Admission rules, requirement provenance, a package without source-derived material | Independent review of the specification passed | done (`docs/clean-room/`) |
| F10 / M | [Domain vocabulary and discipline modules](../contracts/domain-vocabulary-v0.1.md) | F04, F08 | Common element types and attributes, namespaces, ownership, Opening/hosted-in relations, rule catalogue, discipline-module contract | Steel is the only implemented module; every rule has a positive and a negative case | draft |
| F11 / M | [USD layer-engine decision](../design/layer-engine-decision-v0.1.md) | F03, F05, F08 | Comparison of SDK worker, own subset, `openusd` crate and hybrid | Recommendation marked as a proposal to the owner | done: variant C chosen |

## N: spikes and evaluations

Time-boxed research with a report; never a production gate.

| ID | Task | Depends on | Result | Status |
|----|------|------------|--------|--------|
| N01 / M | Layer-engine spike on `openusd` | F11 | Prototype on synthetic fixtures, measurements, differential check | done ([report](../design/layer-engine-spike-n01-report-v0.1.md)) |
| N02 / M | IFC-lite evaluation | — | Evaluation as an IFC path | done ([report](../design/n02-ifc-lite-evaluation-v0.1.md)); not adopted |
| N03 / M | Linux wrapper build and SDK-oracle verification | N01 | Build and differential checks against the official SDK where available | open |
| N04 / M | `truck` kernel evaluation | — | Evaluation as a kernel | done ([report](../design/n04-truck-evaluation-v0.1.md)); not adopted |
| N05 / M | SolidCraft kernel evaluation | — | Evaluation for contour extrusion and plane cuts | done, report not in the repo; rejected by decision OD-2 |
| N07 / S | Variant sets for viewing packages only | N01, O02, U12 | variantSets generated only in an export package for USD viewers; authored model keeps none | open |
| N08 / S | Local compatibility run on a third-party USD sample | U01, R01 | Black-box run of the reader and R01 outside the repository; report without the sample's bytes | open, needs a licence check |

## U: USD storage, layers and revisions

| ID | Task | Depends on | Result | Acceptance | Status |
|----|------|------------|--------|------------|--------|
| U01 / M | Open and read USD | F08, F03 | Bounded open/read with verified roots and traversal limits | API and CLI open our fixture; a damaged file is an error | merged (partial) |
| U02 / M | Codeless schemas and authored layers | U01, F04 | Element and plate attributes, model/derived composition | OpenUSD reads the attributes without schema registration; own validator separate | merged |
| U03 / M | UUID and relationship index | U02 | Full UUIDv7, ID → prim path, relationship validation | Same-millisecond IDs not shortened; duplicate and dangling targets refused; rename keeps the ID | merged |
| U04 / M | CLI and native host | U01, F05 | Command dispatcher, machine-readable results | One command contract, correct exit codes, timeout and cancel | merged (partial) |
| U05 / M | Immutable assets | F08, F04 | Full SHA-256, publish-if-absent, byte verification, path containment | A repeated publication never overwrites a blob; a bad digest is diagnosed | merged |
| U06 / M | Commit of a complete revision | U02, U03, U05 | Immutable generations and an atomic root switch | The root reads only complete generations; expected base checked; no partial save | merged |
| U07 / M | Journal and recovery | U06 | Consistent intent/commit record and recovery of an interrupted save | Fault injection before and after the switch opens the old or the new snapshot; recovery idempotent | merged |
| U08 / M | Portable appearance | P02, U09 | Visual material: `UsdShade` Material + `UsdPreviewSurface`, `material:binding` on the element, `primvars:displayColor` as fallback; colour comes from presentation, not from the engineering material | A colour change keeps shape and mass keys; usdview and Omniverse show the colour; no MDL or external textures | open |
| U09 / S | Standard metadata for other programs | P01, P03 | Model `kind`, `displayName` from `bim:name`, authored `extent`, normals for flat faces | `extent` matches the computed bounds; a rename changes `displayName`, not path, ID or shape key | merged |
| U10 / M | Payload geometry and selective loading | U09, N01, V02 | Element or zone geometry as payloads, `extentsHint`, a bounds-only snapshot | Opening with unloaded payloads gives IDs, properties and bounds; measured on 10 000 elements | open |
| U11 / M | Instancing of shared geometry | P05, U09 | `instanceable` reference to an immutable geometry asset | 1000 copies give one prototype; editing one copy publishes a new blob only for it | open |
| U12 / M | Representation levels | U09, V02, U10 | Axis (`guide`), Simplified (`proxy`), Reduced (cuts without holes), Full (`render`) | Viewers switch `purpose`; quantities, BOM and snaps read Full only; the others are marked inexact | open |
| U13 / S | Lighting as a presentation layer | U08, U09 | Optional weakest sublayer with a dome and a distant light outside `defaultPrim` | Light is in no shape or mass key; a reference to our model carries no lamps | open |
| U14 / M | History compaction and garbage collection | U06, U07, P06, P07 | `history-stats`, `history-compact` with `--dry-run`, `--keep-head`, `--keep N`, backup, journal rotation | Dry run changes nothing; compaction atomic and fault-tolerant; only unreachable blobs removed; refused with a live workspace base or a running coordinator | merged |
| U15 / M | Per-element derived layers | U06, P04 | Unchanged elements are not rewritten on every edit | Editing one element of 1000 adds about one element's size to the store; measured before and after | open |

## G: geometry kernel

| ID | Task | Depends on | Result | Acceptance | Status |
|----|------|------------|--------|------------|--------|
| G01 / M | Kernel math | F08, F06 | Checked vectors, planes, transforms, finite f64 policy | Zero axes, NaN/Inf and bad transforms diagnosed; Rust and WASM tests | merged |
| G02 / M | Rectangular plate | G01 | Standard mesh, analytical volume, local FeatureIds | Synthetic sizes and negative inputs checked; topology and winding | merged |
| G03 / M | Original kernel fixtures | G02, F07 | About 50 own descriptions with an independent harness | Supported and unsupported marked; references with provenance | merged |
| G04 / M | [Kernel design](../design/kernel-design-v0.1.md) | F06, F09 | Public mathematical sources, representation, predicates, tolerances, failure contract | Algorithms justified independently of other kernels | draft |
| G05 / M | Profiles, extrusion and triangulation | G03, G04 | Profiles with cavities and straight sweeps | Own fixtures check topology and volume; bad contours refused | merged |
| G06 / M | Topology and measurements | G03, G04 | Shell, orientation and volume checks, bounded spatial index | Open or unsupported meshes never get an exact status | merged |
| G07 / S | [Acceptance of the first kernel scope](../design/g07-kernel-first-scope-acceptance.md) | G05, G06, G09 | Evidence of correctness and speed, list of supported operations | Readiness shown by fixtures; unknown explicitly unsupported | accepted with conditions |
| G08 / M | Kernel integration for the accepted scope | G07 | Bounded API, cancellation and revision checks, engine commands | Real API and CLI end-to-end use our kernel; a failure never damages USD | open; contract amendment A-2 done |
| G09 / M | Pilot plane cuts and booleans | G05, G06 | Bounded own operations on agreed bodies | Independent fixtures check volume, bounds, topology and failure | merged |
| G10 / M | Faster kernel certification | G08 | Spatial index instead of a face-pair search; tens of holes per plate or flange | Measured before and after; `OpLimits` revised by measurement | open |

## P: core model

| ID | Task | Depends on | Result | Acceptance | Status |
|----|------|------------|--------|------------|--------|
| P01 / M | First end-to-end plate | U03, U04, U07, G02 | CreatePlate → USD authoring → derived → commit → open | Two independent IDs saved; ordinary OpenUSD reads geometry and attributes; volume checked | merged |
| P02 / M | Profile and material as USD assets | P01, U05 | Parametric profile, engineering material, immutable references (P02.2 section properties) | Material separate from shading; digest and units pinned; unknown density stays unknown | merged |
| P03 / M | Straight beam and column | P02, G01 | Straight member from a supported profile, placement, FeatureId | Length, local and world transforms and saved shape checked end to end | merged |
| P04 / M | Quantities and freshness | P01, P02 | Volume, mass, status and input signature; shape and mass invalidated separately | A material change updates mass without copying the shape; a rename rebuilds nothing; stale never shown as current | merged |
| P05 / M | Copy-on-write | P04, U05 | CopyElement with a new UUID, remapped internal links, shared geometry asset | Editing the copy creates a new blob; the original is unchanged | merged |
| P06 / M | Personal workspace | P01, U07 | Workspace and user IDs, base revision, draft save and reopen | Two workspaces independent; drafts never in the published root; unsent changes survive a restart | merged |
| P07 / M | Local undo | P06, P05 | Undo and redo of the draft with links and quantities | A copy or change is undone whole; errors and restarts leave no half operation | merged |
| P08 / M | Move and delete with dependencies | P03, P06 | Placement update and deletion with relationship preconditions | Moves never copy the shape; deletion never leaves a dangling link | merged |
| P09 / M | Validation layer | F10, U03 | One validator (dangling links, duplicates, module rules) for client and post-merge | Each rule has a positive and a negative case; bounded and deterministic | open |
| P-Mass / M | Mass from the section area | A07 | Mass = area × length × density; area from the catalogue or computed exactly | Source of the area visible; see decision 2026-10-18 | merged |
| P-Degenerate / M | Degenerate element state | G08, F04 amendment | A wrongly built element is kept without derived data and recomputed once fixed (decision A-2 D-7) | — | open |

## R: references

| ID | Task | Depends on | Result | Acceptance | Status |
|----|------|------------|--------|------------|--------|
| R01 / M | Read-only USD reference | P01 | Attach wrapper, pinned source, no edits or overrides, explicit units correction on the wrapper | Writing the source or a foreign prim is refused; bytes preserved; network paths refused | merged |
| R02 / M | Measurements of a foreign model | R01, P04, G08 | Supported mesh measurements as our own result objects (W03) | Units and transform respected; open or self-intersecting meshes never exact; mass never invented | deferred |
| R03 / M | Explicit reference update | R02, U05 | Source versions and comparison of supported properties and geometry | No automatic update; nothing written to foreign prims | deferred |
| R05 / M | BIM properties of foreign models | R01 | Read-only "declared properties" from common USD BIM conventions; external IDs for R03 | Own synthetic fixtures; strings with units never become exact values | open |
| R06 / M | "Built from" link | R01, R05, R03 | Relation from our element to a reference element with source revision | Never writes foreign prims; a changed source gives an explicit status | open |

## C: collaboration

| ID | Task | Depends on | Result | Acceptance | Status |
|----|------|------------|--------|------------|--------|
| C01 / M | Changeset and idempotency | P06, P08, F05 | Base, op_id, actor, dependencies, USD delta, assets; bounded validation | The same op_id with the same payload is safe, with another refused; an author string grants nothing | merged |
| C02 / M | Local coordinator | C01, U07 | Explicit loopback server, token auth, one publish queue, expected head | Off by default; refused without a token; no parallel overwrite of the head | merged |
| C03 / M | Rebase of independent edits | C02, P04 | Footprint comparison with intermediate revisions | Two independent edits kept; unsent draft never lost | deferred |
| C04 / M | Semantic conflicts | C03, R01 | Same property, atomic groups, read dependencies, delete/link | A conflict keeps both sides; no silent server-wins | deferred |
| C05 / M | Two offline clients | C04, P05, P07 | Harness for 1000 + 1000 IDs, retries, crash and two workspaces | No path collisions; accepted snapshot readable by OpenUSD | deferred |
| C06 / M | Undo of a published operation | C05 | Compensating command with current preconditions | A later edit by someone else is never rolled back | deferred |
| C07 / M | Live delta channel | C05 | Optional loopback channel over the same changesets | Off by default; same conflict semantics | later |
| C08 / M | Permissions and layer storage | C02 | Permission model and remote storage | Separate owner decision on hosting | later |

## V: native UI

| ID | Task | Depends on | Result | Acceptance | Status |
|----|------|------------|--------|------------|--------|
| V01 / M | Viewport spike | F06, F07 | `wgpu` + `egui` spike on 10 000 own objects; UI stack audit | Measurements and font and asset audit | done |
| V02 / M | View snapshot and query contract | P01, F06 | Incremental buffers, revision token, bounds, ID mapping, selection API | Available through API and CLI; the renderer computes no engineering geometry | merged |
| V03 / M | Window and model display | V01, V02 | Viewport, camera, loading the accepted snapshot | Real engine model, not a mock; instancing; baseline GPU metrics | merged |
| V04 / M | Picking and snaps | V02, R01, G01 | Picking, axis and vertex snaps, stable IDs | Headless end to end with independent expected positions | merged |
| V05 / M | Create and edit tools | V03, V04, P07 | V05.1 create plate and member; V05.2 select, move, copy, delete, undo | The UI calls the same commands; cancel publishes nothing; saved result equals the CLI's | merged |
| V06 / M | Properties and publication | V05, C05 | Property editor, local/accepted/stale states, publish (V06.1 without conflicts) | Unknown mass and unsupported states visible | merged (V06.1) |
| V07 / M | Discipline view presets | V03, R01, W01, U12 | Presets: reference visibility and underlay, kind filter, representation, colours, enabled options | Switching never changes the model or the snapshot token | open |

## A: fabrication operations

| ID | Task | Depends on | Result | Acceptance | Status |
|----|------|------------|--------|------------|--------|
| A01 / M | Assemblies | P03, P08, U03 | Minimal assembly hierarchy, cycle and ownership checks | Create, attach and detach end to end | merged |
| A02 / M | Contour plate | G08, P04 | Closed planar contour, triangulation, stable features | Bad contour refused; authored geometry kept | merged (contour in z = 0) |
| A03 / M | Plane cut | G08, P03, P05 | Cut as an owned `CutByPlane` operation record (amendment A-2) | Volume and freshness updated; stable features or an explicit loss | merged as element attributes; operation record open |
| A04 / M | Plate hole | G08, A02, P05 | Through hole as an `AddHole` operation record, parameters and tolerance | Geometry, volume and undo end to end; other hole types explicitly unsupported | merged pilot; acceptance and operation record open |
| A05 / M | Minimal bolt group | A01, A04 | Bolt group, links to the parts, generated holes | Read/write dependencies touch the linked parts; deleting a parent conflicts | merged without generated holes |
| A06 / M | Minimal weld | A01, P03 | Weld links and properties without full solid geometry | Workshop/site and side explicitly authored | merged |
| A07 / M | Section families and libraries | P02, G08 | A07.1 nine families and custom outlines; A07.2 names and libraries; A07.3 environments | Analytical references; unknown designations never substituted | merged |
| A08 / M | Numbering preview | A01, P04, P05 | Equivalence of supported parts, series and overlap, stale tracking | Preview approves nothing; material and operations counted | merged |
| A09 / M | Numbering approval | A08, A03–A06 | Serialisable approval over the current revision; single-user for the MVP | A fabrication change makes it stale; a camera change does not | merged |

## E, D: exchange, reports and drawings

| ID | Task | Depends on | Result | Acceptance | Status |
|----|------|------------|--------|------------|--------|
| E01 / M | [Own IFC export contract](../contracts/ifc-export-v0.1.md) | F02, P03, P04 | IFC version and subset, stable GlobalId mapping, own Rust writer | Mapping to the public specification and a list of losses | draft |
| E02 / M | Minimal IFC exporter | E01, P03, P04 | Beams, columns and plates with placement, material, shape and a loss report | End to end; independent schema and geometry validation | merged; independent validation open |
| E03 / M | BOM and reports | A01, P04, A09 | BOM of the accepted revision for supported parts | No double count; unknown and stale shown; reproducible | merged |
| E04 / S | IFC import study | F02, E01 | Import options and the boundary with read-only references | No conversion without an owner decision | open |
| D01 / M | Drawing contract and FeatureIds | F02, F06, P03, A01 | Drawing IDs, model revision, views, dimensions, marks, stale states | A dimension never addresses a triangle index | deferred |
| D02 / M | First projection and dimension | D01, A03, A04, V04 | One view, one dimension with a stable reference, headless output | A lost feature is explicitly unresolved | deferred |
| D03 / M | Drawing manager and publication | D02, A09 | Document version, freshness, minimal print/export | Publication bound to a snapshot | deferred |
| D1, D2 / S | Project ID and levels | P06 | ProjectId in the root layer; levels (storeys) in the model | Needed by E02 | merged |

## Q: integration and delivery

| ID | Task | Depends on | Result | Acceptance | Status |
|----|------|------------|--------|------------|--------|
| Q01 / M | End-to-end alpha | V06, A09, E02, E03 (for the MVP without C06 and R03) | Frame → numbering → IFC and BOM, in the UI and the CLI | Invalid files and crash regressions; OS and GPU matrix not replaced by mocks | open |
| Q02 / M | Packaging and release rehearsal | Q01 | Reproducible native packages of CLI, UI and licences | A clean machine opens a complete USD project; attribution checked | merged; rehearsal not run |

## W, O: views, review and design options

| ID | Task | Depends on | Result | Acceptance | Status |
|----|------|------------|--------|------------|--------|
| W01 / M | Saved views | V03 | Camera prim with author, time, comment and model revision in a views layer | Restored through API and CLI; no effect on model keys or numbering | open |
| W02 / M | Review notes | W01, U03 | Note bound to `bim:id`, revision, 3D point and camera; open/approved/rejected | A deleted element gives an explicit unresolved state; BCF later | open |
| W03 / S | Saved measurements | R02, V04 | Measurement object with references, units, precision, value, input signature, status | A shape change makes it stale | open |
| W04 / S | Compatibility with edits made in other USD tools | U06, U02 | Policy for foreign content in our root and layers | Publication keeps such data or moves it with a diagnostic; nothing lost silently | open |
| O01 / M | Design-option contract | F04, F10, C01 | Identity, lifecycle, layer layout, composition, footprint exclusion, effect on numbering, BOM, IFC and drawings | Acceptance follows the changeset path | open |
| O02 / M | Local design options | O01, P06, P07, P08 | Create, switch, compare, accept, reject in one workspace | A rejected option and its blobs are removed; the base is unchanged until acceptance | open |
| O03 / M | Comparing an option with its base | O02, V03 | Diff of added, changed and removed elements | Built on `bim:id` and footprints, not paths | open |
| O04 / M | Shared design options | O02, C04, W02 | Proposals for review, notes, acceptance through the coordinator | No silent server-wins | open |
