# BeamCraft CLI and Rust USD worker

U04 adds two native binaries in this existing package: `beamcraft` and
`beamcraft-usd-worker`. The CLI calls `beamcraft_host::dispatch::Dispatcher`;
only the worker constructs a `beamcraft_layer_engine::Reader`. Neither starts
a socket or a service. The SDK fallback and mutations are unavailable; `revision-commit` and
`revision-open` (below) are the only publication path, and they do not parse USD.
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
selected identity and `--op-id UUIDV7`, works without a worker, and returns OUTCOME_UNKNOWN
until U07 can provide durable proof. It does not report an assumed rejection on replay.

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
(`BASE_REVISION_MISMATCH`, `OP_ID_REUSE`, `INTEGRITY_ERROR`, ... exit 2). No deadline is
checked after the root switch, because a timeout would hide a committed revision.

These commands check blob integrity, not USD content: the layer engine runs in its own process
and the frozen U04 profile has no verify/index message, so duplicate-ID, dangling-link and
U03-index checks run only through the host API's injected `ContentVerifier` (exercised in the
tests with the real layer engine). A CLI commit records `index: null`. U07 recovery is not
implemented. See the [U06 record](../../docs/design/u06-revision-commit.md).

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
P01 does not exist, so the scene is **synthetic**, built with the real kernel
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
