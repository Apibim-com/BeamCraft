# N04 truck B-rep spike

Contributor-original code and synthetic numeric probes, BeamCraft contributors, 2026-10-16,
MPL-2.0. Isolated workspace; no production dependency or feature is adopted.

The pinned default-feature graph is the prepared graph, unchanged. Run offline from the repository:

```powershell
cargo build --offline --manifest-path spikes/n04-truck/Cargo.toml
cargo build --offline --release --manifest-path spikes/n04-truck/Cargo.toml
python spikes/n04-truck/audit.py
python spikes/n04-truck/run.py --debug-only
python spikes/n04-truck/run.py --release-only
python spikes/n04-truck/supplement.py
python spikes/n04-truck/summarize.py
python spikes/n04-truck/checks.py
```

Run the two supervisors sequentially for isolated timing; concurrent execution is supported but
timings are observational, not benchmarks. Each case runs caught three times with
`RAYON_NUM_THREADS=1`, once caught with 4, and once directly with 1, in **each** build profile.
Every process has a 30-second deadline. A killed process is FAIL/timeout, never inferred WRONG or
PANIC. A caught panic exits zero with PANIC; a direct panic exits nonzero. No library object crosses
the catch boundary. Repeated timeouts have no triangle bytes, so their determinism is unmeasured.
Two separate 120-second supplementary probes (release pattern and debug hole) test whether the
matrix's shorter budget hid a completed result. They are stored separately and do not alter the
440-call matrix or its determinism comparisons.
`target/probes` contains raw f64 mesh/triangle bytes and exploratory runs; only compact final evidence
is intended for review. `run.py --quick [case ...]` is an exploratory debug pass; add `--release` for
release. The case list is fixed and bounded. No random input generator or third-party fixture is used.

`src/lib.rs` builds through truck public APIs. `src/main.rs` is a one-case CLI, not an engine control
server. `oracle.py` independently derives expectations and checks six G03 rational references.
`run.py` measures the actual emitted triangles again using Python scalar arithmetic and directed
incidence. Exact-position alias enumeration is diagnostic only: no welding, snapping or healed mesh
is returned. `tests/probes.rs` exercises actual extrusion, G03 quantities, minimum/translated
dimensions and panic recovery. All authored Rust forbids unsafe.

PASS in case tables means the analytic-volume/B-rep probe passed. It **does not mean F06 mesh
conformance**: raw indexed closure, canonical ordering, face mapping, caps and stable bytes are
reported separately. Missing meshed surfaces make a returned mesh WRONG. F06 topology equality is
exact, never relaxed by a numeric epsilon. Invalid-axis and singular-touch cases are separate from
ordinary supported geometry. See [the evaluation](../../docs/design/n04-truck-evaluation-v0.1.md)
for formulas, provenance, results and adoption gates.

`audit.py` stores the complete cargo-metadata package inventory, Cargo.lock checksums verified against
cached archives, feature/resolve trees and hashes of available licence files. It does not read
upstream source or tests. Declared-SPDX screening is not legal admission or cargo-deny. Metadata
contains target-specific packages not built on the host. `doc_text.py` reads generated public API
documentation under ignored `target/doc`; it does not follow source links. No upstream code, licence
text, artwork, catalogue or test fixture is copied into deliverables. The N04 report includes the
proposed attribution row because the task forbids editing ATTRIBUTION.md.
