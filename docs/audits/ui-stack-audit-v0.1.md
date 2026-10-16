# V01.0: UI stack dependency and asset audit

Date: 2026-10-16. Base: `adbc407` (parent of this branch; the branch tip at the start of the audit was `810dd745ce63b2732758df324264d3783162bb86`).
Disposition: **conditional admission of a narrowed stack. V01 viewport work is not started. No UI crate is added to the workspace.**

This audit is the V01.0 prerequisite from issue #37. Implementation-plan section 7.7 is not in this checkout (`docs/architecture.md` keeps the owner's plan outside the repository). Scope is the issue plus the PdfCraft stack note in `docs/architecture.md`. Nothing here is a Tekla, Trimble, or other commercial-product asset. No PdfCraft application icon, screenshot, or sample document was copied.

## Method

Evidence is from the original publishers, not from recollection.

- crates.io packages were resolved in an isolated scratch workspace under `/tmp`, never in the BeamCraft workspace, and were not added to `Cargo.lock`.
- `storytold/craft-libs` was cloned and inspected at `83bf4cb25c01ed46e7645c4172974239868aaf06`. PdfCraft's three older pins were compared with `git show` and `git merge-base --is-ancestor`. They are ancestors of that commit. Cargo cannot depend on one git URL at three revisions, so those pins are recorded, not proposed as a combined dependency.
- Vendored patches were read from PdfCraft `f0c9901c398372782a4eb001c95638289a5edab0` (`vendor/winit`, `vendor/egui-wgpu`, `vendor/vello_common`) and used only as path patches in the scratch workspace.
- Transitive licences were checked with `cargo deny` 0.20.2 against a copy of this repository's `deny.toml` (`confidence-threshold = 0.9`). `cargo tree` / `cargo metadata` supplied the package list. Individual licence files were opened only for the crates called out below. A passing `cargo deny` result is not a claim that every transitive file was read.
- Host for the checks that did run: `rustc 1.99.0` / `cargo 1.99.0`, `x86_64-unknown-linux-gnu`. Repository MSRV is Rust 1.96 (`Cargo.toml`). A 1.96 build was not run. `wasm32-unknown-unknown` is not installed here. Windows and macOS were not run.

`cargo deny` 0.20.2 `allow-git` is a list of URLs. It does not store a revision. `required-git-spec = "rev"` is the tool's pin control: a git dependency must name a commit, but the tool cannot bind one URL to one commit. The audited revision is therefore a written condition, not a deny.toml key.

## Verdicts

| Item | Pin | SPDX from the original source | Verdict |
|------|-----|-------------------------------|---------|
| eframe | crates.io 0.36.2 | `MIT OR Apache-2.0` | ADMIT WITH CONDITIONS |
| egui | crates.io 0.36.2 | `MIT OR Apache-2.0` | ADMIT WITH CONDITIONS |
| epaint_default_fonts / `default_fonts` | crates.io 0.36.2 | `(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0`, plus Bitstream Vera in the font files | REJECT |
| wgpu | crates.io 30.0.1 | `MIT OR Apache-2.0` | ADMIT WITH CONDITIONS |
| winit, crates.io | 0.30.13 | `Apache-2.0` | ADMIT WITH CONDITIONS |
| winit, PdfCraft vendor patch | 0.30.13 at PdfCraft `f0c9901c` | upstream `Apache-2.0`; patch comments are in that tree | ADMIT WITH CONDITIONS |
| egui-wgpu, PdfCraft vendor patch | 0.36.2 at PdfCraft `f0c9901c` | `MIT OR Apache-2.0` | ADMIT WITH CONDITIONS |
| vello_common, PdfCraft vendor patch | 0.0.8 at PdfCraft `f0c9901c` | `Apache-2.0 OR MIT` | ADMIT WITH CONDITIONS |
| rfd | crates.io 0.17.2 | `MIT` | ADMIT WITH CONDITIONS |
| egui_extras | crates.io 0.36.2, `svg` only | `MIT OR Apache-2.0` | ADMIT WITH CONDITIONS |
| egui_kittest | crates.io 0.36.2 | `MIT OR Apache-2.0` | ADMIT WITH CONDITIONS, tests only |
| craft-ui | git `83bf4cb25c01ed46e7645c4172974239868aaf06` | `MIT OR Apache-2.0` | ADMIT WITH CONDITIONS |
| craft-quit | same revision | `MIT OR Apache-2.0` | ADMIT WITH CONDITIONS |
| craft-print | same revision, and PdfCraft pin `666c3580` | crate text is `MIT OR Apache-2.0`; closure is not | REJECT |
| craft-pdf, craft-cms, and the repo's vendored PDF trees | present at the same revision, not depended on | not fully audited | REJECT for this admission |
| storytold/winprint.rs | `417ac3c4` and `5f80b9ba` | not admitted; second git source | REJECT for this admission |

No item is an unconditional ADMIT. The conditions are mandatory for any later integration PR.

## Conditions that apply to every admitted item

1. Do not enable `default_fonts` on eframe, egui, or epaint. eframe's default features include it. A later dependency must set `default-features = false` and opt in only to reviewed features.
2. Do not copy PdfCraft product assets, and do not vendor the whole PdfCraft or craft-libs tree.
3. Do not add a UI crate to this workspace in a docs PR. The `deny.toml` git allow is permission to depend on the admitted crates later, not a dependency.
4. Keep upstream licence files and PdfCraft `PdfCraft patch:` notices if a patched crate is vendored. Do not relicense those files to MPL-2.0.
5. Choose the permissive alternative where a crate is `MIT OR Apache-2.0 OR LGPL-2.1-or-later` (`r-efi`) or `Apache-2.0 OR GPL-2.0-only` (`self_cell`). Do not select the copyleft alternative. `cargo deny` accepted those expressions because an allowed alternative exists. That is not a GPL/LGPL exception.
6. Replace the window icon with a contributor-original icon before a BeamCraft product window is shipped. Do not adopt the egui logo as the BeamCraft brand (`docs/brand/` is a separate trademark and is not modified here).
7. Windows, macOS, and wasm32 builds of this stack were not run. A later integration PR must run them, or record them as not run, before claiming those targets.

## eframe 0.36.2 and egui 0.36.2

Published from emilk/egui. Annotated tag `0.36.2` is `2ec7a836879b958f05ef542fab5a12a86250ef62`; the peeled commit, which `.cargo_vcs_info.json` records for the crates.io package, is `49682f8baa058bf49e011035cfbd6e825f88a5ef`.

| Crate | crates.io checksum | Manifest licence | `rust-version` |
|-------|--------------------|------------------|----------------|
| eframe 0.36.2 | `55ad41fac6e3149abb4866e93df37dbd33baa95db525c3368c944584e95a5dc2` | `MIT OR Apache-2.0` | 1.95 |
| egui 0.36.2 | `dc938cc27cd911415e1e4d151bc56ff9df063d8db081374e87aeb805ea74b2ba` | `MIT OR Apache-2.0` | 1.95 |
| epaint 0.36.2 | `a0ec0308dc23130fc1a7d4623a8534c2ed069c6dc84e258ee4f3de99615aa1a3` | `MIT OR Apache-2.0` | 1.95 |

Licence files at tag `0.36.2`:

- `LICENSE-MIT`, 1092 bytes, SHA-256 `95ca92f5f8ea5231f1580b3a2a799e8260af3114b900e1def5355a7f44bcf60c`, copyright Emil Ernerfeldt.
- `LICENSE-APACHE`, 10850 bytes, SHA-256 `8173d5c29b4f956d532781d2b86e4e30f83e6b7878dce18c919451d6ba707c90`.

The published eframe, egui, and epaint crates on disk do not contain those licence files. The manifest field is the crates.io declaration. A later vendor or redistribution must copy the tag's licence files; the crates.io tarball alone is not a complete notice.

eframe default features are `accesskit`, `default_fonts`, `links`, `wayland`, `web_screen_reader`, `wgpu`, `winit/default`, and `x11`. `default_fonts` is `egui/default_fonts`. The scratch that passed `cargo deny` disabled default features and enabled `wgpu`, `accesskit`, `persistence`, `links`, `wayland`, and `x11` only.

`crates/eframe/src/native/epi_integration.rs` always calls `include_bytes!("../../data/icon.png")` on the native path. The published file is `data/icon.png`, SHA-256 `9e076c34e32a0b0ef35c62ebf6be8b5594a4601e7bcd478b681e9eff1f327973`. No separate licence file sits beside it, and no separate pixel author was named in the crate. The package licence field covers the file; no exception was found, unlike the fonts crate below. Pixel author: unknown. Licence: the package declaration, not a separate grant. Do not copy the PNG into this repository. Do not ship it as the BeamCraft mark.

egui 1.95 is below the repository MSRV of 1.96, so a 1.96 compiler can accept the manifest. That was not compiled on 1.96.

## epaint_default_fonts 0.36.2 — REJECT

Manifest licence, from the published `Cargo.toml`:

```text
(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0
```

`cargo deny check licenses` on a scratch that enabled only `default_fonts` exited 4. It rejected `OFL-1.1` and `Ubuntu-font-1.0`. OFL-1.1 is an allowed asset licence in `AGENTS.md` section 1.2, but it is not in `deny.toml`, and the expression is an AND. Adding OFL-1.1 would not make the crate pass. This audit does not widen the allow list.

Files in the published crate `fonts/` directory:

| File | What the file itself says | Section 1.2 |
|------|---------------------------|-------------|
| `Ubuntu-Light.ttf` with `UFL.txt` | Ubuntu Font Licence Version 1.0 | not listed |
| `Hack-Regular.ttf` with `Hack-Regular.txt` | MIT for the Hack work, and the Bitstream Vera License for Bitstream Vera Sans Mono, with reserved names | Bitstream Vera License is not listed |
| `NotoEmoji-Regular.ttf` with `OFL.txt` | SIL Open Font License 1.1 | OFL-1.1 is listed for assets, but it is not separable from the AND expression |
| `emoji-icon-font.ttf` with `emoji-icon-font-mit-license.txt` | MIT, Copyright 2014 John Slegers | MIT is listed, but it is not separable from the AND expression |

The font bytes are embedded when the feature is on. craft-ui's own dev-dependency enables `default_fonts`. Cargo does not compile a dependency's dev-dependencies, and the admitted craft-ui library dependency uses workspace `egui` with `default-features = false`. Do not copy that dev-dependency into BeamCraft.

## wgpu 30.0.1

crates.io checksum `527ccdf43dd5b2e8676eed9984ce00e2bbb0a1b85b70c1969dcb6cd2eb55ab9e`. Manifest `license = "MIT OR Apache-2.0"`, `rust-version = "1.87.0"`, repository `https://github.com/gfx-rs/wgpu`. Published VCS commit from `.cargo_vcs_info.json`: `40f4a34ebaf56f9a046231f54125ad046239d3f3`.

The published crate contains `LICENSE.MIT` (SHA-256 `dc0d97139e8205818c703741c7be7cb3b96888bd5917b8d6fc6133731e403c21`) and `LICENSE.APACHE` (SHA-256 `a6cba85bc92e0cff7a450b1d873c0eaa2e9fc96bf472df0247a26bec77bf3ff9`).

`src/color_gamuts.svg` and `src/sdr_hdr_range.svg` are in the crate. They were not `include_bytes`'d by a search of the UI library sources used here, and they are not copied into this repository. Their separate authorship was not proven beyond the crate licence. Do not copy them into docs or the product.

System graphics libraries (Vulkan, Metal, Direct3D, OpenGL) are not Cargo crates. Their presence on a machine was not tested. wasm32 dependencies appear in the lockfile for the wasm target; that target was not compiled.

## winit 0.30.13 and the PdfCraft patch

Published crate: checksum not used, because the proposed source is the vendor patch, not crates.io. The published tree was still downloaded for comparison. Manifest `license = "Apache-2.0"`, `rust-version = "1.70.0"`. `.cargo_vcs_info.json` git sha `e9809ef54b18499bb4f2cac945719ecc2a61061b`. `LICENSE` SHA-256 `6dc0e068dcf3a5bc8e054205b85b7720e1d49265bbc64bf515d2cf79197df69a`, identical in the PdfCraft vendor tree.

The vendor tree at PdfCraft `f0c9901c` differs from the published crate in Linux Wayland pointer/scroll handling, X11 event and RandR code, macOS IME (`ime_startup.rs` and view/window-delegate edits), and Windows DPI/IME/dark-mode files. Changed files carry `PdfCraft patch:` comments. The vendor `LICENSE` file is unchanged. The vendor snapshot omits `examples/data/` (`icon.png`, `gradient.png`, `cross.png`, `cross2.png`), which the published crate has. Those example images were not given a separate licence in this audit: unknown, and not admitted. The library sources do not `include_bytes` them. Do not build the examples or copy those PNGs.

`src/platform_impl/web/web_sys/worker.min.js` is 127 bytes and is `include_str`'d for the web target. It is a one-line scheduler shim with no third-party notice. It is treated as part of the Apache-2.0 crate, not as a font or icon. wasm32 was not compiled.

## egui-wgpu 0.36.2 vendor patch

Path patch from PdfCraft `vendor/egui-wgpu` at `f0c9901c`. Manifest `license = "MIT OR Apache-2.0"`, `rust-version = "1.95"`. `LICENSE-MIT` and `LICENSE-APACHE` match the egui tag hashes above (`95ca92f5…` and `8173d5c29b…`). Patches are marked `PdfCraft patch:`. No font or icon file was found in that vendor directory. The crates.io tarball was not the one resolved, because the path patch replaced it, so a crates.io checksum is intentionally not cited for the patched crate.

## vello_common 0.0.8 vendor patch

Path patch from PdfCraft `vendor/vello_common` at `f0c9901c`. Manifest `license = "Apache-2.0 OR MIT"`, `rust-version = "1.88"`, repository `https://github.com/linebender/vello`. `LICENSE-APACHE` SHA-256 `a6cba85bc92e0cff7a450b1d873c0eaa2e9fc96bf472df0247a26bec77bf3ff9`. `LICENSE-MIT` SHA-256 `adf157330f722fe7cfd5964a2a6974eff851b718228c6735724a11706e6f4dec`.

`src/lib.rs` includes `probe` only under `#[cfg(feature = "probe")]`. `probe.rs` then `include_bytes!`s `assets/probe.rgba`. Default features are `std` and `png`, not `probe`. Do not enable `probe`. `assets/probe.png` is a test reference in the tree. No separate licence file was found for it. The crate says how to regenerate it and the module copyright is "Copyright 2026 the Vello Authors" with `SPDX-License-Identifier: Apache-2.0 OR MIT`. That is the proof used here. Do not copy the PNG into BeamCraft. An upstream `vello_common` 0.1.0 also appeared in the eframe scratch through `glifo` / `vello_cpu` (checksum `b2e9aed918117e8152c9eddfd8362d73c465c23f26c44786aa331707b8a64fa2`). It is a second version, not the patched 0.0.8. `deny.toml` allows multiple versions. It was not patched and was not separately asset-audited beyond the crate licence field that `cargo deny` accepted.

## rfd 0.17.2, egui_extras 0.36.2, egui_kittest 0.36.2

These are the related crates named by the architecture note. They were in the passing scratch graph.

| Crate | Checksum | Licence | `rust-version` | VCS sha |
|-------|----------|---------|----------------|---------|
| rfd 0.17.2 | `20dafead71c16a34e1ff357ddefc8afc11e7d51d6d2b9fbd07eaa48e3e540220` | `MIT` (`LICENSE` SHA-256 `65fc947f4bac882a2cd104ef73c246b1b037059df9f2b3a58b7f77f0b9e56071`) | not declared (unknown) | `cca0cfa5fc38c2752c9c98f9f5168904a515fd55` |
| egui_extras 0.36.2 | `05cbdc785020861553f230f23d330ba1fb72613b9d8e0e457f3bdc8ba6db6dcf` | `MIT OR Apache-2.0` | 1.95, from the egui workspace | same egui release commit as eframe |
| egui_kittest 0.36.2 | `5eb3f3effb2726fee23449293e67acbf6cc6ce35525b1722bd5743bb348114d5` | `MIT OR Apache-2.0` | 1.95 | same egui release commit |

rfd's MSRV is unknown because the manifest has no `rust-version`. Do not assume 1.96. egui_kittest is a test harness, not a product dependency. `resvg` 0.45.1, pulled by `egui_extras`'s `svg` feature, contains `examples/ferris.png` and example SVGs. Those are examples, not library bytes. Their separate licence was not proven. Do not copy them. The Rust logo is not a BeamCraft asset.

## craft-libs

Repository: `https://github.com/storytold/craft-libs`. Audited revision: `83bf4cb25c01ed46e7645c4172974239868aaf06` (2026-10-16, merge of pull request 148). Workspace `license = "MIT OR Apache-2.0"`, `edition = "2024"`, `rust-version = "1.95"`.

- `LICENSE-MIT` SHA-256 `2f42c0cafc8ad394d91373fe1f342aee5453850b151a51405c68cae9ada86d26` (1102 bytes).
- `LICENSE-APACHE` SHA-256 `0d542e0c8804e39aa7f37eb00da5a762149dc682d7829451287e11b938e94594` (10174 bytes).

PdfCraft `f0c9901c` pins three revisions of the same URL:

- craft-ui `5c874d51eeb3de70e735c8ce87a59a7dedd47675`
- craft-quit `f58bc1a3df490992d633393d0898af0588ce882c`
- craft-print `666c358065261a2816b376fa3bb10538dbf53040`

All three are ancestors of `83bf4cb`. Cargo will not resolve one git URL at three revisions. The proposed pin is the single audited revision. A later bump requires a new audit. `required-git-spec = "rev"` does not enforce this exact hash.

### craft-ui and craft-quit — ADMIT WITH CONDITIONS

`crates/ui` and `crates/quit` at the audited revision declare the workspace licence and `rust-version`. craft-ui depends on workspace `egui` with `default-features = false`. Optional feature `serde` was enabled in the scratch. craft-quit's `egui` feature was enabled. No PNG, SVG, font, or ICC file was found under those two crates. `include_bytes` appears only in their tests, as egui context calls, not as embedded image files. The "icon catalogs" in the crate description are code, not shipped artwork.

A scratch depending only on those two crates, with the proposed `allow-git` entry, passed `cargo deny check` (advisories, bans, licences, sources). `cargo check --locked --offline` of that scratch finished on this Linux host in the dev profile. macOS `objc2` dependencies are `cfg(target_os = "macos")` and were not compiled. The `menu` feature (optional `muda`) was not enabled and was not separately audited.

### craft-print — REJECT

The crate's own manifest licence is the workspace `MIT OR Apache-2.0`. That does not admit the closure.

At `83bf4cb`, `craft-winprint-gdi` is an unconditional git dependency:

`https://github.com/storytold/winprint.rs`, rev `417ac3c4d14d3da5485a99c7d5ce8f05664ec312`.

Windows-only, the same crate also depends on `winprint` from that repository at rev `5f80b9baf6b58937c401656634e42d33f00ddf49`. The older PdfCraft pin `666c3580` does not have `craft-winprint-gdi`, but it still has the Windows `winprint` git dependency. This task authorizes one `craft-libs` allow-git entry, not a `winprint.rs` entry.

A scratch that depended on craft-ui, craft-quit, and craft-print together failed `cargo deny check`:

- licences: `webpki-root-certs` 1.0.9, `CDLA-Permissive-2.0`, reached through `winprint`'s build dependency (`rustls-platform-verifier` / `reqwest` / `sigstore-*`). CDLA-Permissive-2.0 is not in the allow list. It was not added.
- sources: both git URLs are unknown under the current policy.
- advisories: `RUSTSEC-2024-0370`, `proc-macro-error` unmaintained, via `winprint`.

The winprint.rs licence text was not accepted as proof of the closure, because the closure already fails the policy this task is allowed to change. Do not add the second git URL in a follow-up without a new audit.

### Other crates in the same git repository — REJECT for this admission

`allow-git` is URL-wide. It cannot say "craft-ui only". The following are in the repository at the audited revision and are not admitted by this URL entry:

- `crates/cms/profiles/craft-coated-fogra39.icc` and `photocraft-coated-cmyk.icc`. Their licence was not proven from a licence file next to the profiles in this audit. Unknown, therefore not admitted.
- `vendor/krilla/icc/*.icc` with `vendor/krilla/icc/LICENSE` beginning "CC0 1.0 Universal". CC0-1.0 is an allowed asset licence, but these files belong to a PDF vendor tree that this task does not admit. Not copied.
- `vendor/hayro-interpret/assets/LICENSE_FOXIT` begins with a PDFium Authors copyright and a BSD-style grant. The font files that notice belongs to were not admitted and were not copied. A notice in a repo we do not depend on is not a BeamCraft asset.
- `vendor/hayro*` and `vendor/krilla` are patched PDF crates. Out of scope. Not audited as a dependency closure.

A later PR must not add `craft-pdf`, `craft-cms`, `craft-print`, or those vendor paths just because the URL is allowed. `cargo deny` will not catch that mistake.

## Transitive closure that passed

Scratch `v01-ui-stack-scratch`: eframe/egui/egui_extras/egui_kittest pinned to 0.36.2 without `default_fonts`, rfd 0.17.2, wgpu 30.0.1, and path patches for winit, egui-wgpu, and vello_common 0.0.8. `cargo metadata --locked` reported 429 resolved packages, including the scratch package. No package had an empty licence field. `epaint_default_fonts` was not in the graph.

`cargo deny check` with an unmodified copy of `deny.toml` reported `advisories ok, bans ok, licenses ok, sources ok`. Warnings were only unmatched BeamCraft MPL exceptions and unused allow-list entries, because the scratch is not the BeamCraft workspace.

Expressions that are not a single allow-list identifier, and that still passed because an allowed alternative or an accepted spelling was present:

- `MIT/Apache-2.0`, `Apache-2.0/MIT`, `MIT / Apache-2.0`, `Unlicense/MIT` (slash forms; accepted by cargo-deny 0.20.2, not re-read crate by crate)
- `Unlicense OR MIT`, `0BSD OR MIT OR Apache-2.0`
- `MIT OR Apache-2.0 OR LGPL-2.1-or-later` (`r-efi` 5.3.0 and 6.0.0)
- `Apache-2.0 OR GPL-2.0-only` (`self_cell` 1.3.0)
- `(MIT OR Apache-2.0) AND Unicode-3.0`
- `Apache-2.0 AND MIT`
- `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT`

The scratch package's own `MPL-2.0` was ignored as a private package (`[licenses.private] ignore = true`). It is not a third-party exception.

Image and font scan of the resolved crate directories: no `.ttf` or `.otf` in the graph that excludes `default_fonts`. Runtime-relevant embedded image: eframe `data/icon.png` only, as above. Other PNGs and SVGs were examples or tests (`tiny-skia` tests, `resvg` examples, `simd_cesu8` docs, `base64` docs). They are not copied and must not be copied.

The craft-ui/craft-quit scratch resolved 81 packages and passed the same four `cargo deny` checks once `allow-git` contained `https://github.com/storytold/craft-libs` and `required-git-spec = "rev"`.

## Platforms

| Target | What was run | Result |
|--------|----------------|--------|
| Linux x86_64, rustc 1.99.0 | resolve, `cargo deny`, `cargo check` of craft-ui and craft-quit | check finished |
| Linux x86_64, rustc 1.99.0 | `cargo check --locked --offline` of the eframe scratch with the three path patches, without `default_fonts` | finished in the dev profile |
| Rust 1.96 MSRV | not run | unknown whether the stack builds on 1.96; manifests that declare a rust-version declare 1.95 or lower, except rfd, which declares none |
| Windows | not run | unknown |
| macOS | not run | unknown; craft-quit's AppKit path was not compiled |
| wasm32-unknown-unknown | target not installed; not run | unknown |

Native GPU libraries were not probed. "Resolves on Linux" is not "renders on Windows".

## deny.toml change

Added, and nothing else:

- `allow-git = ["https://github.com/storytold/craft-libs"]`
- `required-git-spec = "rev"`
- a comment naming rev `83bf4cb25c01ed46e7645c4172974239868aaf06` and the crate limits

No licence was added to `[licenses] allow`. No package exception was added. `unknown-git` stays `deny`. The allow entry is unused until a later PR adds a git dependency, so `cargo deny` warns `unmatched allow source`. That warning is expected. It is not a licence widening. The entry is present because the craft-ui/craft-quit audit passed and the next task must not rediscover the URL policy.

## Checks

Run, in isolated scratch workspaces unless noted:

- `cargo deny` 0.20.2 `check` on the eframe graph without `default_fonts`, using the previous `deny.toml`: advisories, bans, licences, sources ok.
- `cargo deny check licenses` on eframe with only `default_fonts`: failed, as required for the REJECT verdict.
- `cargo deny check` on craft-ui + craft-quit + craft-print at `83bf4cb`, previous `deny.toml`: licences, sources, and advisories failed.
- `cargo deny check` on craft-ui + craft-quit with the proposed sources config: advisories, bans, licences, sources ok.
- `cargo check --locked --offline` of that craft-ui/craft-quit scratch: finished.
- `cargo check --locked --offline` of the eframe scratch with the three path patches and without `default_fonts`: finished on this Linux host. That is a compile, not a GPU or window test.
- `cargo metadata` / `cargo tree` on the eframe scratch: 429 packages, licence inventory above.

Not run:

- Rust 1.96, Windows, macOS, and wasm32 builds of any candidate crate.
- A full `cargo test` of egui, wgpu, winit, or craft-libs.
- `cargo deny` of PdfCraft's three older revisions as separate lockfiles. The older print manifest was read with `git show` only.
- Separate licence-file review of every transitive crate. `cargo deny` at confidence 0.9 is the closure check.

BeamCraft workspace checks on this change, rustc 1.99.0, Linux x86_64:

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test --workspace`: passed.
- `cargo deny check` 0.20.2: advisories, bans, licences, and sources ok. Expected warning: `unmatched-source` for the unused `craft-libs` allow-git entry. Pre-existing `license-not-encountered` warnings remain for allow-list identifiers that this workspace does not yet use. No licence was added to clear them.
