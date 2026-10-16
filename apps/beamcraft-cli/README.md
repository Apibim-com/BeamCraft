# BeamCraft CLI and Rust USD worker

U04 adds two native binaries in this existing package: `beamcraft` and
`beamcraft-usd-worker`. The CLI calls `beamcraft_host::dispatch::Dispatcher`;
only the worker constructs a `beamcraft_layer_engine::Reader`. Neither starts
a socket or a service. The SDK fallback, mutations, saves and publication are unavailable.
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

The worker accepts only `--store ROOT`; unknown plugin/runtime options fail before reading
model data. It has no plugin/script loader. Two explicit unquoted USDA directive names,
`beamcraft:plugin` and `beamcraft:script`, are reserved and rejected before parser entry;
comments and inert strings mentioning these names remain inert. No execution semantics
are assigned to unknown binary metadata. Environment clearing removes Python, SDK, plugin
and library discovery overrides. See [U04 review](../../docs/design/u04-cli-host.md) for
the precise profile, production gates and tests.
