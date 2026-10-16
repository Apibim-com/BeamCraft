# BeamCraft layer engine

Native, experimental U01/U02 layer adapter for exactly `openusd =0.8.0`, without optional
features. This library performs no file or network I/O, starts no process, and exposes
no upstream handles. This is a desktop-native library, not a domain/WASM crate.

`Reader::open` accepts an explicit manifest of local bytes, exact digests, owned layer
names, full project/workspace identities, revision/generation, backend identity, budgets
and a cancellation/deadline control. Manifest integrity precedes parsing. Only canonical
relative asset identifiers are accepted; they never name files on the host.

The static profile supports USDA sublayers and external references with identity layer
offsets and explicit reference targets, plus a restricted USDC scalar/vector profile.
Unsupported composition and binary value encodings fail explicitly. Snapshots expose
bounded typed static attributes and contributing prim sites. They are read results,
not engineering validation or authorization. `Reader` remains read-only; the separate
`authored::Author` API accepts closed engine-produced change items. Static relationships,
including unknown names and list-edit composition, are returned explicitly.

An open is built privately and installed only after complete validation and a final
cancellation check. Failed opens leave the previous context intact. Reopen/close invalidate
all earlier contexts/cursors, including across reader instances. Pages read a frozen result
over one retained stage; they never reopen or recompose it.

Cancellation is cooperative before/after upstream synchronous calls and during wrapper
loops. Upstream allocation, panic-abort, stack overflow, and hard deadlines cannot be
contained in-process. The declared/decompressed preflight and output caps are not an
aggregate heap/work proof. A separate Rust process was selected by the owner on 2026-10-16.
U04 owns its executable, F05 transport and host client; this crate contains library logic
only. This slice must not be integrated as an untrusted production parser until
LEW-03 containment is reviewed and proved. See [the U01 placement/containment review](../../docs/design/u01-placement-containment.md).

Tests use contributor-original synthetic USDA and binary output of those fixtures.
Same-crate binary generation is not independent-reader evidence; U04 and N03 remain open.

## Codeless authored and derived layers

`authored::SCHEMA_DESCRIPTOR` fixes the field/type mappings, units, paths and closed item
profile. `schema_identity()` returns its full SHA-256 and model version; the backend
archive/features pin remains independently checked. No schema generation, `openusd-build`,
Python, C++ plugin or new dependency is used. Elements are standard `Xform` prims with
explicit custom attributes and profile/material relationships. Transferred engine results
are standard `Mesh` children with standard points, topology, orientation and subdivision
properties. Ordinary USD readers need no BeamCraft plugin. This compatibility intention
is tested with the same Rust backend; SDK interchange has **not run**. No solid evaluation
or exact quantity computation runs here.

`Author::open` consumes a verified U01 manifest plus distinct, existing owned USDA edit
targets participating directly in the root's sublayer stack. The root must also be owned.
`prepare` consumes a pinned context, full op/actor identities, exact schema pin, bounded
immutable resources and typed changes. Host authorization, canonical F04 replay checks,
engineering validation and serialized decisions are prerequisites. It returns owned layer
bytes/digests, exact local authored opinion items, conservative semantic footprints and a
structural validation receipt while preserving the live view and generation. There is one
candidate; neither upstream handles nor raw USD setters are exposed.

`install` requires the trusted host's proven `Accepted` decision and rotates the context
once. `discard` permits only `Undecided` or proven `Rejected`. Uncertainty or cancellation
after proven acceptance retains the candidate and reports `OUTCOME_UNKNOWN`. Host claims
are not authenticated by this library. Dropping the process or library is not evidence
of rollback. Durable outcome queries, recovery, op_id replay and publication remain host
duties. Saved bytes are not an atomic U06 publication.

Changes cover create, label, contour, thickness, axis, roll, profile, material, placement
and transferred meshes. Optional roll distinguishes value, declaration without default,
block and property removal. A material distinguishes one pinned target, an explicit empty
list and removal; omitting a change leaves it untouched. Every contributing source and
ancestor must be owned, including sources contributed through references. Unsupported
arcs retain U01's explicit refusal. Shape edits remove exclusively owned derived meshes
before replacement; mixed-layer geometry fails rather than revealing a weaker stale result.
Unknown admitted metadata, attributes, relationships, specs and sublayer order survive
editing; untouched layers retain their exact bytes. No flatten operation is used.

Resources are contributor/engine-supplied rectangular profiles or optional-density
materials. Their digest commits the canonical isolated USD definition layer produced by
`Resource::bytes`, excluding the eventual embedding path/digest label. Existing known
definition values are rehashed before use. Unknown fields remain inert, outside that
closed resource identity; this is not a catalogue or a hash of arbitrary embedded specs.

U01 ceilings only tighten. Additional input accounting caps a batch at 1 MiB, 4096 items
and 4096 total semantic footprint entries, with the smaller stage caps applying. Points,
mesh arrays and labels are counted before scanning/copying. Retained candidates, exported
layers and typed opinions are bounded by these source/snapshot/value caps. These are
wrapper bounds, not a proof of aggregate upstream allocation or preemptive cancellation.

`validate_layers` is the offline `u02-structural-v1` checker. Its six fixed rules cover
U01 admission; schema markers/known types; UUID/path/required inputs; pinned definition
links; scalar caps, scaled length minima and rigid pose; and standard mesh buffer structure.
Newer element schemas/kinds/modes remain inert and cannot be edited. This checker is separate
from external `usdchecker` (**not run**, no offline executable found), the F10 module-rule
catalogue and the F06 engineering validator. It does not certify contour topology,
planarity, support, mesh solidness, exact quantities, F06 result freshness or signatures.
Those remain engine prerequisites. See [the U02 validation report](U02-VALIDATION.md).
