# F03: OpenUSD 26.8 distribution audit

Date: 2026-10-14. Base: `9ee590ec75abdcfd0174dfa1b8288153b1637c9f`.
Disposition: **conditional candidate; SDK integration and redistribution gates are not passed**.
This audit completes the package investigation with explicit conditions. It does not implement U01.

## Pinned candidate

Investigated `usd-core==26.8`, CPython 3.12 wheels from the publisher's
[PyPI release](https://pypi.org/project/usd-core/26.8/).
The [candidate manifest](openusd-26.8-candidate.json) records exact URLs, SHA-256 digests,
sizes, tags, native members and licence evidence. `production_approved` remains `false`.
All three downloaded wheels matched their PyPI SHA-256 values; none were installed or imported.

Upstream reference: OpenUSD tag `v26.08`, annotated tag object
`cb5613f6da7c61b56fe86dbe8cc1cbe9f0d84ef1`, source commit
`ee47c679abde5b467a7b6a41f3b2285564a4222e`.
This is the matching release reference, not a verified source attestation for the PyPI binaries.
Artifact pinning reproduces acquired bytes; it does not prove a reproducible upstream build.

| CPython 3.12 target | Compressed bytes | Uncompressed bytes | ZIP entries | Native files |
|--------------------|------------------|--------------------|-------------|--------------|
| Windows x64, `win_amd64` | 13,887,898 | 51,060,968 | 179 | 41 |
| Linux x64, `manylinux_2_27` / `manylinux_2_28` | 29,530,957 | 217,298,656 | 280 | 32 |
| macOS, `macosx_10_15_universal2` | 40,698,020 | 238,937,566 | 280 | 32 |

Tags advertise platform compatibility; these are not BeamCraft runtime test results.
Linux needs an appropriate glibc-based x64 environment; no musl or Linux ARM wheel was selected.
No Windows ARM64 wheel was selected. macOS universal2 is a candidate for x64/ARM64, not proof of
operation on either architecture or every tagged OS version. Native load tests were not performed.

The inspected metadata declares Python `>=3.9, <3.15` and no `Requires-Dist` entries.
The wheel tag still selects CPython 3.12: `none` does not make these native modules portable
across Python minor versions. No Python 3.15 candidate is approved.

## Actual package contents

Each wheel contains 45 `plugInfo.json` / `generatedSchema.usda` resources, `pxr` Python/native
modules, a monolithic USD library and TBB. Module names such as `UsdHydra` do not establish the
presence of an imaging renderer. No inspected wheel contained font/image files with the audited
extensions, `usdview`, a `usdchecker` executable or distribution entry points.
`pxr/Usd/usdGenSchema.py` is present; this does not prove that schema generation and its optional
tooling dependencies work in the proposed runtime.

The official [wheel workflow](https://github.com/PixarAnimationStudios/OpenUSD/blob/ee47c679abde5b467a7b6a41f3b2285564a4222e/.github/workflows/pypi.yml)
disables imaging, MaterialX, examples, tutorials, USD tools and Exec in its build commands.
The [packaging recipe](https://github.com/PixarAnimationStudios/OpenUSD/blob/ee47c679abde5b467a7b6a41f3b2285564a4222e/build_scripts/pypi/package_files/setup.py)
copies native libraries and plugin resources into the wheel. Empty Python dependency metadata
therefore does not imply an empty native dependency graph.

U02's independent reader and `usdchecker` gate require a separately audited tool distribution
or a pinned SDK build with the needed validation tools. Wheel import alone cannot close that gate.

## Licences and notices

The actual wheel metadata identifies `LicenseRef-TOST-1.0`.
The upstream [licence at the pinned source commit](https://raw.githubusercontent.com/PixarAnimationStudios/OpenUSD/ee47c679abde5b467a7b6a41f3b2285564a4222e/LICENSE.txt)
describes TOST as an Apache-derived licence with different trademark terms. Keep its identity;
do not label it Apache-2.0 or MPL-2.0. Redistribution requires the licence and applicable notices;
modified files require change notices. Trademark permissions are limited.

MPL covers our own code. Separating the SDK into a worker does not waive its obligations or
approve its resources under the asset policy. At audit time TOST was not in AGENTS.md's explicit
asset list, so the schema resources required an owner decision before product integration.
No licence exceptions or policies were changed by this audit. PR #7 later recorded that decision
as a scoped exception for official OpenUSD code, binaries and SDK resources (`AGENTS.md`
section 1.2); the remaining gates below still apply to this candidate.

Each wheel has one aggregate `LICENSE.txt`, but no separate upstream `NOTICE` or TBB licence.
The three aggregate texts match the pinned source licence after line-ending normalization.
Windows uses different line endings, with a different raw digest; the manifest preserves both.
The licence contains notices for optional and bundled source components. A notice section alone
does not establish that component's presence or absence in the compiled core.

| Component / evidence | Licence finding | Delivery condition |
|----------------------|-----------------|--------------------|
| OpenUSD core and resources | TOST-1.0 | Owner records permitted SDK/resource use; retain exact licence and applicable notices |
| TBB Windows DLLs | Exact match to official 2020.3 archive; Apache-2.0 licence retrieved | Include original TBB licence; validate the selected release-only native closure |
| TBB Linux/macOS | Build recipe selects 2020.3.1 / 2020.3; native provenance not independently attested | Confirm native versions and original notices on those platforms |
| Aggregate core notices | Includes MIT/BSD/BSL and other component-specific notices | Map compiled components to licences; do not approve all optional components from a notice file |
| RapidJSON notice | Identifies separately licensed JSON_checker material | Prove that excluded material is not shipped; no catalogue or fixture copying |
| Apple-specific source notice | Separate custom terms in upstream aggregate | Resolve applicability for the selected macOS build; no blanket asset exception |
| CPython runtime | PSF-2.0 plus incorporated components' terms | Audit the actual interpreter build and notices before redistribution |

Primary evidence for TBB: [2020.3 licence](https://raw.githubusercontent.com/oneapi-src/oneTBB/v2020.3/LICENSE),
[2020.3.1 licence](https://raw.githubusercontent.com/oneapi-src/oneTBB/v2020.3.1/LICENSE),
and the [OpenUSD dependency recipe](https://github.com/PixarAnimationStudios/OpenUSD/blob/ee47c679abde5b467a7b6a41f3b2285564a4222e/build_scripts/build_usd.py).
The matching [upstream NOTICE](https://raw.githubusercontent.com/PixarAnimationStudios/OpenUSD/ee47c679abde5b467a7b6a41f3b2285564a4222e/NOTICE.txt)
must be assessed and retained in an eventual SDK notice bundle. Downloaded third-party materials
remain in a temporary audit cache; no library, schema, fixture or third-party asset is vendored.

## Windows native closure

Static `dumpbin /DEPENDENTS` inspection covered all 41 Windows native members.
`usd_ms.dll` and `_usd.pyd` directly require `python312.dll`, `tbb.dll`, release MSVC/UCRT
libraries and Windows system DLLs. The wheel does not include the interpreter or those system
runtimes. Use explicit prerequisites or an audited redistribution plan; do not silently copy
libraries from a developer installation.

The Windows wheel also contains ten TBB DLLs: release, debug, preview, binding, allocation and
proxy variants. Every one matched the official
[TBB 2020.3 Windows archive](https://github.com/oneapi-src/oneTBB/releases/download/v2020.3/tbb-2020.3-win.zip)
byte for byte. The downloaded archive digest was
`cda37eed5209746a79c88a658f8c1bf3782f58bd9f9f6ba0da3a16624a9bfaa1`;
an independent publisher digest/signature was not verified.

`tbbbind.dll` and its debug variant directly import `libhwloc-5.dll`, absent from both inspected
archives. Debug TBB members import `MSVCP140D.dll`, `VCRUNTIME140D.dll` and `ucrtbased.dll`.
Microsoft states that [debug Visual C++ runtime DLLs are not redistributable](https://learn.microsoft.com/en-us/cpp/windows/preparing-a-test-machine-to-run-a-debug-executable?view=msvc-170).
Do not add them to satisfy a wholesale wheel dependency scan.

These findings do not prove failure of ordinary `pxr` import: the core's inspected direct imports
use release `tbb.dll`, not the optional binding/debug members. Dynamic loads and transitive
dependencies remain untested. Product packaging needs a verified release-only file set or a
controlled core SDK build. Stripping or repacking changes the artifact: record a new digest,
updated metadata and all required notices, then test that exact distribution.

## Python runtime and delivery recipe

Candidate ABI: CPython 3.12. Proposed patched interpreter: 3.12.15, supplied as an independently
audited prerequisite. The [official release](https://www.python.org/downloads/release/python-31215/)
is source-only; there is no current official Windows installer in that series.
The manifest records the official source URL and published SHA-256, but the source archive
was not downloaded or built. A runtime binary, toolchain and native components are not yet pinned.

The local inventory tool ran under Codex's CPython 3.12.14. That interpreter was used only for
static audit, is not a BeamCraft dependency and is not the proposed product runtime.
CPython's [original licence](https://raw.githubusercontent.com/python/cpython/v3.12.15/LICENSE)
requires preserving its terms and notices; do not relicense the interpreter under MPL.

After the SDK/resource policy decision, native closure and runtime prerequisites are accepted:

1. Acquire the exact platform wheel from the candidate manifest and verify its complete SHA-256.
   Keep downloads separate from the source checkout. Do not use an unpinned latest package.
2. Supply an audited CPython 3.12.15 interpreter; record its binary digest and platform details.
3. Create an isolated environment and install the pinned wheel offline with no dependency resolution.
   Example commands below describe a future authorized bootstrap; they were not run in F03.

```sh
python -m venv /absolute/path/to/usd-environment
# Use the new environment's interpreter for every subsequent command.
python -m pip install --no-index --no-deps /absolute/path/to/verified-platform-wheel.whl
python -I -c "from pxr import Usd; print(Usd.GetVersion())"
```

4. Exercise original synthetic stage creation, USDC save/reopen, local composition and malformed
   input diagnostics. Verify plugin search paths and actual loaded libraries on a clean machine.
5. Package the accepted runtime/SDK file set and original notices; record its own digest.
   Run a separate ordinary OpenUSD reader and the selected validation tool against saved outputs.

This is a reproducible acquisition recipe for the pinned wheels, with a conditional installation
recipe. It is not a completed runtime bundle or evidence of a clean-machine load. A custom source
build is an alternative when the wheel file set is unsuitable, but compiler, flags, dependency
sources and output hashes must be pinned and audited separately. Neither route is authorized by
the mere presence of this document.

The future host selects the interpreter explicitly, isolates Python and native search paths,
allows only approved USD plugins and uses bounded private-pipe IPC. The worker opens no server
or network resolver by default. Importing a file must not grant permission to fetch remote assets.
F05/U01 own those implementation and negative-test contracts.

## Reproducing this audit

The original [inventory tool](../../tools/audit/openusd_inventory.py) uses only Python's standard
library. It has no download, extraction, installation or package-import operation.
The archive must already exist locally; substitute one filename and digest from the manifest:

```sh
python -B tools/audit/openusd_inventory.py /absolute/path/to/wheel.whl --sha256 FULL_SHA256 --output /absolute/path/to/inventory.json
python -B -m unittest discover -s tools/audit -p "test_*.py" -v
```

It hashes into a private capped snapshot and preflights central-directory bytes and actual records
before the ZIP parser creates member objects. Classic single-disk ZIP containers are supported;
ZIP64 containers are rejected. This layout check follows
[PKWARE APPNOTE sections 4.3.12 and 4.3.16](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT).
Text reads accept only stored/deflated members with an explicit byte limit; licence hashes reuse
the same bytes. Output paths cannot alias the input wheel, including symlinks and hard links.
Reports publish through a temporary file without following an output symlink on write.

It also enforces compressed/member/total size limits, rejects unsafe/duplicate paths, checks the
exact package version and reports licence/native/schema members. It does not classify licences,
verify every RECORD entry or resolve native imports. Its twelve synthetic tests cover real CLI
subprocess cases, pre-allocation directory rejection, understated text sizes, unsupported
compression, input/output aliases, digest/version mismatch and unsafe paths.
All twelve passed; CI runs these tests on Windows and Linux.
The CLI also successfully inventoried all three real, hash-verified wheels.
Raw inventories, Windows direct-import evidence and the TBB byte comparison are local under
`Desktop/BeamCraft/work/F03`; the source checkout contains original audit text and the pinned manifest.

## Gate conditions

Status at audit time (2026-10-14). Later changes are noted in the table without rewriting the
audit result.

| Condition | F03 result / remaining owner |
|-----------|------------------------------|
| Exact package, platform artifacts and hashes | Recorded and statically verified |
| TOST and SDK schema/resource policy | At audit time: owner decision still required; no exception added. Later, PR #7 added the scoped official-OpenUSD TOST exception (`AGENTS.md` section 1.2); this candidate's provenance, attribution, notices and runtime checks remain open |
| Complete native component licences/notices | TBB/NOTICE omissions identified; compiled inclusion map still required |
| Runtime binary and release-only native closure | Not yet delivered; platform packaging work required |
| Real import, synthetic save/reopen and independent validation | Not run; clean-machine platform checks required |
| Python/native security maintenance | Patched interpreter proposed; native advisory review not completed |
| Rust/WASM/cargo-deny | No Rust graph changes; local WASM target and cargo-deny remain absent |

F03 research can be accepted with this conditional outcome. U01 and the product distribution gate
remain waiting for packaging and runtime evidence (and, at audit time, the policy decision). F02 and
original fixture work can continue independently. A code-review acceptance of this report does not
approve the SDK package.
