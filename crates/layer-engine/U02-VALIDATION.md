# U02 validation report

Date: 2026-10-16. Base: `bca89e23671470ba0633497b82d38a83fb5cc11d`.
Branch: `feat/u02-authored-layers`. Status: experimental library slice; full U02 acceptance
remains open. Runtime evidence is Windows x64, Rust 1.96.0, cached dependencies, offline.

Verification: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --
-D warnings` and `cargo test --workspace` all passed offline (236 workspace tests, 31 in
layer-engine, including 11 new U02 tests). Git staging failed with permission denied when
creating the shared worktree index lock. Changes are **uncommitted**; the orchestrator
must commit/push and open a draft using [the full PR body](U02-PR.md).

## Provenance and scope

Contributor-original code, synthetic test-generated fixtures and documentation, MPL-2.0,
covered by the `crates/layer-engine/**` attribution row. No external asset, catalogue,
schema resource, upstream implementation, generated plugin or new dependency is included.
The mapping follows frozen F04 sections 2-4 and 8, and F06 sections 2, 4 and 7. The only
numeric operations here implement their explicit norm, scale and quaternion-norm checks;
there is no geometry builder. Public math provenance is IEEE binary64 arithmetic and
Euclidean norms as recorded by F06. USD authoring uses the existing pinned public crate
API, inspected offline; no upstream source or tests are copied.

The owner selected a separate process on 2026-10-16. This is its library boundary only.
There is no process binary, CLI, host client, framing, JSON decoder, durable ledger, UUID
index, storage, network operation or atomic publication implementation in U02.

## Structural checker

`authored::validate_layers` returns one fixed receipt or one bounded F04 diagnostic.
It is a separately callable checker, not a call to `usdchecker`, and it shares U01's
bounded real-backend open path. It is not an independent parser or engineering validator.
Its rule profile is `u02-structural-v1`:

| Rule | Checked | Limit |
|---|---|---|
| U02-V01 | Full source/manifest digests, static profile, explicit mm/Z metadata, numeric admission, budgets and contributing provenance | U01 upstream allocator/abort/deadline containment remains unproved |
| U02-V02 | Exact requested schema digest/model version, present root schema marker, supported known property types | Unknown/newer element kinds, modes and versions remain inert; they cannot be edited |
| U02-V03 | Full matching UUID/path, actor, label, required supported inputs and Xform type; element `kind` component and `displayName` equal to `bim:name`; `/World` and `/World/Steel` kinds when authored | No persistent/reconstructible UUID index is added |
| U02-V04 | Required/optional definition targets, definition kind, units/version and recomputed canonical definition digest | Digests identify the closed isolated definition representation, not arbitrary embedded unknown metadata |
| U02-V05 | Scalar caps, positive/scaled length minima, local size, pose norm/order and optional density | No exact contour topology, planarity, rectangle support, engineering result certification or full F06 ordered multi-finding validator |
| U02-V06 | Known owned standard Mesh point/topology buffers, counts/indices, subdivision/orientation, full shape-key format, `subcomponent` kind, `extent` equal to the point bounds and one unit face normal per face (1e-5 per component) | No closed-solid, manifold, freshness, signed-volume or signature verification |

Prepare additionally checks edit-layer ownership/participation/present schema markers,
normalized change keys, immutable resource bytes, field/kind compatibility and every
contributing ancestor/target/descendant source. It refuses foreign opinions before editing.
Result validation runs after constructing private per-layer data and composing a new stage.
The host must run the full F04/F06 validators before calling this trusted boundary and
validate the candidate result before recording acceptance. A structural receipt alone
cannot admit an engineering command. Checker/schema-profile review remains open.

## Reproducible evidence

Run `cargo test --offline -p beamcraft-layer-engine` using the existing lockfile/cache.
The 11 U02 integration tests exercise actual codeless USD authoring and composition:

- Supported plate/member create, local binary64 input preservation, explicit actor/units,
  immutable profile/material relationships, standard Mesh transfer and fresh reopen.
- Twenty fixed-ID saves with deterministic insertion permutations; compare every owned
  byte/digest and reopen each result. This establishes Windows/build-local stability only.
- Unknown metadata/relationship/attribute, declaration without default and block preservation;
  roll value/absence/block/removal with a weaker authored default and unchanged weaker bytes.
- Explicit empty/remove material relationships, required profile changes and newer-schema
  inert preservation with edit refusal.
- Foreign-reference and foreign-ancestor write refusal, and valid/invalid mixed-patch atomicity.
- Wrong schema/backend pins, corrupt bytes, forged/changed definition identities, non-finite,
  zero/negative/subminimum/above-cap dimensions, invalid mesh buffers and input/label bounds.
- One candidate, stale/wrong-owner tokens, generation exhaustion, cancellation, uncertain
  decisions, rejection-preserving discard, and unchanged live bytes before installation.
- One hundred serialized label changes, context rotation/generation checks, conservative
  semantic groups and derived invalidation after a shape-input change.

U01's unchanged read regressions also exercise the raw-crate foreign-override control,
unsupported composition safe failures, malformed USDC/USDA, manifests, budgets and cancellation.
No variants, payloads, inherits or specializes become available in U02. Unsupported input
is safe refusal, not a claim of required-arc capability.

The 100-install sequence is not host op_id replay evidence. Canonical command comparison,
durable exactly-once replay, semantic conflict acceptance, complete incoming-link checks,
loss recovery and event-gap/resnapshot delivery remain the respective engine/host/U03/U04/
U06/U07 responsibilities. This library exposes no raw upstream diff replay path.

## External checks and acceptance limits

External `usdchecker`: **not run**. No `usdchecker`, `usdchecker.exe` or `usdview` executable
was found on PATH. No downloads, SDK admission, Python imports or N03 runs were attempted.
SDK-to-wrapper and wrapper-to-SDK: **not run**. Direct parsing and fresh composition in
these tests use the same `openusd` 0.8.0 backend; they are not independent-reader evidence.
Linux/macOS/cross-platform digests: **not run**. WASM: **not run**, desktop-native library
under the 2026-10-16 F08 portability decision; no new crate was created and the target is absent.

LEW-01/02/05/06/07 have scoped library evidence above, not full acceptance. Required
independent-reader/validator admission, platform evidence, full engineering validation,
canonical replay/host decisions, CLI E2E and process failure isolation are not closed here.
No save result or provisional candidate establishes U06 publication or durable acceptance.
