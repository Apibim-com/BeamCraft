# N04 truck reproducible harness

Contributor-original probes, BeamCraft contributors, 2026-10-16, MPL-2.0.
Isolated workspace; the pinned default-feature stack is a rejected route for
BeamCraft holes/cuts. See [the report](../../docs/design/n04-truck-evaluation-v0.1.md).

From the repository root in PowerShell, with the locked Cargo packages already
cached, Rust 1.96, native MSVC tools and Python 3.11 (standard library only):

```powershell
Set-Location spikes/n04-truck
$env:CARGO_NET_OFFLINE = 'true'
$env:CARGO_TARGET_DIR = Join-Path (Get-Location) 'target'
cargo build --offline --locked
cargo build --offline --locked --release
cargo fmt --all -- --check
cargo clippy --offline --workspace --all-targets -- -D warnings
cargo test --offline --locked
cargo test --offline --locked --release
python test_harness.py
```

Optional full reproduction (not run during N04T):

```powershell
python run.py
python summarize.py
```

`run.py` writes separate `target/evidence/results-debug.json` and
`results-release.json`, including for the default two-profile schedule. Each of
44 cases has three caught one-thread calls, one caught four-thread call and one
direct one-thread call per profile (440 planned calls), each with a 30-second
process deadline. `--debug-only` or `--release-only` selects one profile.
`python run.py --quick plate` is a one-call debug check; add `--release` for release.
Each selected profile file is cleared before starting; interrupted schedules stay
partial. `src/main.rs` also accepts `<case> <caught|direct> <output-prefix>`.

Raw mesh/triangle bytes stay in ignored `target/probes`; all generated JSON stays
in ignored `target/evidence`. Do not commit outputs or logs. Re-execution reproduces
measurements and classifications, not elapsed times or nondeterministic byte hashes.
The historical run was partial: 366 calls, with no supplementary result file.

The small Python files are needed for report numbers: `run.py` supplies deadlines,
hashes and an independent scalar mesh audit; `oracle.py` supplies analytic references
and six G03 checks; `summarize.py` prints observed case counts and measured-only
byte-comparison denominators. To summarize archived records without executing geometry:
`python summarize.py --input-dir <archive-evidence-directory>`.
`test_harness.py` checks corruption handling and review regressions with synthetic data.
Audit, environment capture, supplementary-run and document-generation scripts were
removed under the evidence policy. The report records removed-file sizes and hashes.

PASS is the analytic-volume/B-rep probe result, not F06 mesh conformance. Missing
runs/hashes mean unmeasured; only observed byte mismatches mean comparison failure.
No healing, production API, control server or dependency admission is delivered.
