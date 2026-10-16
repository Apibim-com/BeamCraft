# BeamCraft layer engine

Native, experimental U01 read adapter for exactly `openusd =0.8.0`, without optional
features. This library performs no file or network I/O, starts no process, and exposes
no upstream handles. The backend remains a native dependency under the proposed F08
amendment; this is not a domain/WASM crate.

`Reader::open` accepts an explicit manifest of local bytes, exact digests, owned layer
names, full project/workspace identities, revision/generation, backend identity, budgets
and a cancellation/deadline control. Manifest integrity precedes parsing. Only canonical
relative asset identifiers are accepted; they never name files on the host.

The static profile supports USDA sublayers and external references with identity layer
offsets and explicit reference targets, plus a restricted USDC scalar/vector profile.
Unsupported composition and binary value encodings fail explicitly. Snapshots expose
bounded typed static attributes and contributing prim sites. They are read results,
not engineering validation or authorization. Mutations are unavailable; the write guard
refuses foreign prims and never grants a write capability for owned ones.

An open is built privately and installed only after complete validation and a final
cancellation check. Failed opens leave the previous context intact. Reopen/close invalidate
all earlier contexts/cursors, including across reader instances. Pages read a frozen result
over one retained stage; they never reopen or recompose it.

Cancellation is cooperative before/after upstream synchronous calls and during wrapper
loops. Upstream allocation, panic-abort, stack overflow, and hard deadlines cannot be
contained in-process. The declared/decompressed preflight and output caps are not an
aggregate heap/work proof. A separate Rust process is proposed for review, not selected
or implemented. This slice must not be integrated as an untrusted production parser until
LEW-03 containment is reviewed and proved. See [the U01 placement/containment review](../../docs/design/u01-placement-containment.md).

Tests use contributor-original synthetic USDA and binary output of those fixtures.
Same-crate binary generation is not independent-reader evidence; U04 and N03 remain open.
