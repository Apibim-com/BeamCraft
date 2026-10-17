# BeamCraft CLI and Rust USD worker

U04 adds two native binaries in this existing package: `beamcraft` and
`beamcraft-usd-worker`. The CLI calls `beamcraft_host::dispatch::Dispatcher`;
only the worker constructs a `beamcraft_layer_engine::Reader`. Neither starts
a socket or a service. The SDK fallback and the worker mutation protocol are unavailable;
`revision-commit` and `revision-open` (below) publish already stored blobs and do not parse USD.
`create-plate` (P01, below) and the P02 definition commands are the ones that author USD and publish it, in this process.
This is an experimental read path, not complete LEW-03/F05 admission.

Build offline with `cargo build --offline -p beamcraft-cli --bins`.
`beamcraft --help` describes the bounded arguments. Example argument shape:

```text
beamcraft read --backend rust --store ABSOLUTE_PROTECTED_ROOT
  --worker ABSOLUTE_PACKAGED_WORKER --worker-digest FULL_SHA256
  --project-id FULL_UUIDV7 --workspace-id FULL_UUIDV7
  --revision FULL_SHA256 --generation 42
  --file EXPLICIT_LOCAL_LAYER --digest EXPECTED_LAYER_SHA256
```

Put these arguments on one line. The package manifest supplies the independently
verified worker digest. This example includes no fabricated identity/digest or model.
The file route names the virtual root `root.usda` and supports only a self-contained layer.
The CLI verifies the supplied source digest. For multiple assets, use `--manifest SHA256`;
the U05 blob contains `dispatch::read_manifest`'s closed format. Its identity must match
the independently selected tuple. `--owned NAME,...` explicitly grants owned virtual layers;
without it every contributing layer is foreign. USD metadata cannot grant ownership.

`open` returns stage metadata/capabilities/context. `read` opens and returns the first
bounded snapshot page (up to 64 prims). The native API supports continuation over a retained
context. A new CLI invocation opens a fresh incarnation; its stage token/cursor cannot
resume a previous CLI process. Results are JSON, with strings for generation and lossless
signed/unsigned attribute integers. Static values use closed `kind`/`value` records.

`--timeout-ms` accepts 1..120000 (default 30000); `--cancel-after-ms` accepts 0..120000
and provides explicit headless cancellation. Exit codes: 0 success, 2 input/validation or
unavailable command, 3 backend unavailable, 4 outcome unknown, 124 deadline, 130 cancelled.
An explicitly selected unavailable backend returns 3; there is no automatic SDK fallback.

`blob-put --store ROOT --file FILE` and `blob-verify --store ROOT --digest SHA256`
use U05 byte storage. They do not publish a project revision. `query-outcome` requires the
selected identity and `--op-id UUIDV7`, works without a worker, and still returns
OUTCOME_UNKNOWN: it is the frozen U04 stage-profile command and is not wired to the U07
ledger (the profile has no recorded-result message). The durable host-ledger answer is
`revision-outcome` below. Neither reports an assumed rejection on replay.

## U06 revision commands

```text
beamcraft revision-commit --store ROOT --project-id UUIDV7 --workspace-id UUIDV7
  --revision-id UUIDV7 --op-id UUIDV7 --generation U64 --base none|REVISION_UUIDV7
  --schema SHA256 --root-layer NAME --layers ROLE:NAME:SHA256,...
beamcraft revision-open --store ROOT
```

`ROLE` is `authored`, `derived` or `external`; every layer blob must already be in the store
(`blob-put`), and each size is taken from the verified stored bytes. The commit verifies the
expected base, every blob and the op id, then switches the root in one step; a retry of the
same op returns `revision_replayed`. `revision-open` verifies the root, its manifest and every
layer blob and prints the revision (`revision_verified`). Output is JSON with decimal strings
for `sequence`, `workspace_generation` and sizes. Errors use the existing exit codes
(`BASE_REVISION_MISMATCH`, `OP_ID_REUSE`, `INTEGRITY_ERROR`, ... exit 2). `--timeout-ms` and
`--cancel-after-ms` are checked while preparing and at every commit point before the root
switch (the root is then unchanged and the cancellation/deadline error is reported); they are
never checked after the switch, because a timeout would hide a committed revision. A retry of
a commit must repeat the same `--base`: another base under the same op id is `OP_ID_REUSE`.

These commands check blob integrity, not USD content: the layer engine runs in its own process
and the frozen U04 profile has no verify/index message, so duplicate-ID, dangling-link and
U03-index checks run only through the host API's injected `ContentVerifier` (exercised in the
tests with the real layer engine). A CLI commit records `index: null`. See the
[U06 record](../../docs/design/u06-revision-commit.md).

## U07 recovery commands

```text
beamcraft revision-recover --store ROOT --min-age-secs U64
beamcraft revision-outcome --store ROOT --op-id UUIDV7
```

Every `revision-commit` now writes a synced intent to `ledger/outcomes.log` before the root
switch and a committed record after it. `revision-recover` closes unresolved intents from the
root, removes abandoned temporary files, reports unreferenced blobs (it deletes none), verifies
the root and prints one JSON report (`recovery_complete`, or `recovery_incomplete` with exit 0
when an intent is in flight, unprovable, or the ledger contradicts the root; check `outcome`).
`--min-age-secs` is required and has no default: `0` asserts that no commit is running on the
store, a larger value leaves younger intents and temporary files alone. Running it twice changes
nothing. `revision-outcome` is read-only and prints `outcome_committed` (with `source`
`ledger` or `root`), `outcome_aborted` (with `reason`), `outcome_not_recorded` (only when the
ledger covers the whole history) or the standard `outcome_unknown` (exit 4); a ledger that the
root contradicts is `INTEGRITY_ERROR`. Not implemented: a CLI fault-injection switch (the
tests simulate the crash through the host commit hook and then run the real binary), ledger
rotation, garbage collection. See the [U07 record](../../docs/design/u07-journal-recovery.md).

The worker accepts only `--store ROOT`; unknown plugin/runtime options fail before reading
model data. It has no plugin/script loader. Two explicit unquoted USDA directive names,
`beamcraft:plugin` and `beamcraft:script`, are reserved and rejected before parser entry;
comments and inert strings mentioning these names remain inert. No execution semantics
are assigned to unknown binary metadata. Environment clearing removes Python, SDK, plugin
and library discovery overrides. See [U04 review](../../docs/design/u04-cli-host.md) for
the precise profile, production gates and tests.

## V02 view snapshot commands

`view-snapshot` and `view-changes` need no store, worker or backend. They call
`beamcraft-engine` and print one JSON object (same shape for errors as above, plus a
`diagnostics` entry with the view code such as `GEO_STALE_TOKEN`; exit 2 on rejection).
The scene is **synthetic** (see `plate-inspect` for committed plates), built with the real kernel
(`--plates 1..500`): every fifth element is labelled `support` and, with
`--stand-in-simplified true`, every third carries a distinct stand-in simplified mesh. The
output marks both under `stand_ins`.

```text
beamcraft view-snapshot --plates 20 [--hide plate,support] [--representation full|simplified]
  [--stand-in-simplified true|false] [--select ID,ID] [--revision SHA256] [--generation U64]
beamcraft view-changes --plates 20 --edit move-first,resize-first,add-one,remove-last
  [--since TOKEN] [--stand-in-simplified true|false] [--revision SHA256] [--generation U64]
```

`view-snapshot` reports the pin, the snapshot token (unchanged by `--hide` and
`--representation`), the projected elements with instance slot, triangle range, mesh digest,
exactness and world bounds, the display and engineering bounds, the meshes the view must load
(digest, bytes, counts) and, for `--select`, the ids and their instance slots. `view-changes`
snapshots the scene, applies the scripted edits as the next generation and prints the
`GetChanges` delta, with `new_meshes` empty for a placement-only edit. An unknown `--since`
token is `GEO_STALE_TOKEN`. No mesh bytes are exported; a consumer reads blobs by digest through
the engine API. Defaults: revision is the SHA-256 of `beamcraft-synthetic-scene`, generation 1.

## P01 first plate commands

```text
beamcraft create-plate --store ROOT --width MM --length MM --thickness MM
  [--name TEXT] [--translation X,Y,Z] [--material-digest SHA256]
  [--hole-rect "X,Y,W,H;..."] [--hole-round "X,Y,D;..."] [--hole-segments 8..256]
  [--new-id UUID] [--op-id UUID]
  [--revision-id UUID] [--actor-id UUID] [--project-id UUID] [--workspace-id UUID]
beamcraft plate-inspect --store ROOT
```

`--hole-rect` and `--hole-round` (A04 pilot, see the [design note](../../docs/design/a04-plate-holes.md))
author through holes together with the plate: axis-aligned rectangles from the corner `X,Y`, and
regular polygons (default 32 sides) inscribed in the circle of centre `X,Y` and diameter `D`.
Only the clockwise polygons are stored, so a "round" hole is that polygon and its volume is exact for
it; the kernel cuts them (strictly inside the outline, clear of each other, otherwise
`INVALID_ARGUMENT`) and the volume is the certified enclosure's midpoint. At most 64 holes.

`create-plate` builds the origin rectangle contour `(0,0,0),(w,0,0),(w,l,0),(0,l,0)`, runs the
engine's `CreatePlate` (F04 admission, kernel mesh and volume, declared footprint), authors the
model layer and the derived layer as standard USDA with the U02 layer engine, checks that the layer
engine's writes are inside the engine's footprint, and commits through U06/U07 with a verifier that
runs the U02 structural check, the U03 index and a kernel re-evaluation of every plate in the
layers. Output is one JSON object: element id, prim path, dimensions, volume (mm3 and m3, status,
algorithm), mass (kg, density and status: `current` only when `--material-digest` pins a material
that has a density, otherwise `unknown` with no value), the pinned material digest, mesh digest,
revision id, sequence, manifest digest,
generation, layer digests and the index record, and the footprint. Ids that are not given are
generated (UUIDv7, strictly increasing, safe inside one millisecond). A retry of the same
`--op-id` with the same `--new-id` and arguments is `plate_replayed` (nothing written); other
content (including another material) under that op id is `OP_ID_REUSE`; an owned `--new-id` is
`DUPLICATE_ID`. A lost race is
`BASE_REVISION_MISMATCH`. `--store` must be an existing absolute directory; the first call
creates project and workspace ids (or takes them from the flags), later calls reuse the head's.

`plate-inspect` opens the head revision with full verification in a fresh process, re-reads the
layers with a fresh reader, re-evaluates every plate and checks its committed mesh against the
kernel's, and prints the plates, the pinned snapshot token, the element table and the meshes a
viewer must load (`synthetic: false`).

Limits: rectangular origin plate only (other contours are `UNSUPPORTED_GEOMETRY`);
the layer engine runs in-process (no process isolation, LEW-03 open); no workspace, undo or
envelope/JSON command; quantities are printed, not stored in USD (the pinned schema has none).
A library target (`beamcraft_cli::plate`) exposes the same path with a commit hook for fault
injection in tests. `tests/pxr_oracle.rs` (opt-in; skips without a Python that can `import pxr`, honours `BEAMCRAFT_PXR_PYTHON`) reads the saved layers with the OpenUSD reference implementation and runs `UsdUtils.ComplianceChecker`; see the design note for the version and result. See the [design note](../../docs/design/p01-first-plate.md).

## P02 profile and material definitions

```text
beamcraft define-material --store ROOT [--density KG_M3] [--revision-id UUID] [--actor-id UUID]
  [--project-id UUID] [--workspace-id UUID]
beamcraft define-profile --store ROOT --width MM --height MM [same id options]
beamcraft definitions --store ROOT
```

`define-material` and `define-profile` publish one immutable definition as the next revision: a
`/World/Definitions/D_<sha256>` prim in the model layer, with units `mm` and schema version
`0.1.0`. The digest is the full SHA-256 of the canonical isolated definition layer that the layer
engine writes, so it pins the values, the units and the version together; the output is
`definition_created` with that digest, or `definition_exists` (no new revision, nothing written)
when the head already holds the same content. A material without `--density` keeps an unknown
density: no default is selected, no value is stored, and plates that pin it report mass `unknown`.
Density (greater than zero, at most 1,000,000 kg/m3) and rectangle dimensions (0.001 to 1,000,000
mm) are admitted with the F04 codes (`NON_FINITE_NUMBER`, `LIMIT_EXCEEDED`, `INVALID_ARGUMENT`)
before anything is written. `definitions` opens the verified head in a fresh process, re-admits
every definition, recomputes its digest from the stored values (a layer edited after publication is
`INTEGRITY_ERROR`) and lists digest, kind, values and the plates that pin it. A pinned digest that
is not in the model is `NOT_FOUND`; one that names a profile is `INVALID_ARGUMENT`.

Limits: no catalogue, designation lookup or environment default; the physical material carries no display appearance (a separate reference and signature
later); definitions are not an F04 command, so there is no `op_id` replay for them and publication
is idempotent by content digest; in-process layer engine as above. See the
[design note](../../docs/design/p02-definitions.md).

## P03 straight member

```text
beamcraft create-member --store ROOT --start X,Y,Z --end X,Y,Z --profile-digest SHA256
  [--kind beam|column] [--roll RAD] [--material-digest SHA256] [--name TEXT]
  [--translation X,Y,Z] [--new-id UUID] [--op-id UUID] [--revision-id UUID] [--actor-id UUID]
  [--project-id UUID] [--workspace-id UUID]
```

`create-member` publishes one beam or column whose rectangle section comes from a profile
definition published earlier (`define-profile`); the optional material works as for plates. The
engine resolves both from the head (`NOT_FOUND` for an absent digest, `INVALID_ARGUMENT` for the
wrong kind), the kernel builds the box in the F06 member frame (section centred on the axis, the
default frame of F06 section 4 plus the explicit roll), and the commit runs the same verifier as
plates, which now re-evaluates members too. Output: element id, kind, profile and material digests,
section, axis points, roll, placement, volume and mass (as for plates), mesh digest, revision and
footprint (the profile and material are reads). `plate-inspect` lists `plates` and `members` and
puts both in the one pinned snapshot. A placement rotation is `UNSUPPORTED_GEOMETRY`: lay a member
out with its axis points (an axis along `+X` gives width along `Y` and height along `Z`).
See the [design note](../../docs/design/p03-straight-member.md).