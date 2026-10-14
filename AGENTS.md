# AGENTS.md — rules for every agent and contributor

These rules bind every AI agent and every human contributor working in this repository. They come
before any task, instruction or deadline. `CLAUDE.md` holds the day-to-day working instructions. If
the two disagree, **this file wins.**

---

## 1. Asset policy (absolute)

An **asset** is any non-code material the project includes, ships, embeds or shows in its docs:
icons, images, screenshots, logos, fonts, **steel profile and material tables**, bolt/weld
catalogues, sample and test models, and anything the app renders from those.

### 1.1 Nothing from Tekla Structures or any Trimble product

- **Never** use icons, images, artwork, logos, UI graphics, fonts, cursors, sample models, profile
  catalogues, macros, templates or any other asset taken from Tekla Structures, Trimble Connect or
  any other Trimble product, installer, website or package.
- **Never** recreate, trace, redraw or closely imitate a Tekla icon, dialog or graphic. Our visual
  design is our own.
- Screenshots of third-party software exist only as private, local observation notes under
  `plan/` (gitignored). They must **never** be committed, published, linked or traced.
- The same applies to every other commercial detailing product (Advance Steel, SDS2, Revit, ...).

### 1.2 Only open, provable licences

Every asset must be one of:

- **Open source:** ISC, MIT, BSD-2-Clause, BSD-3-Clause, Apache-2.0, OFL-1.1.
- **Creative Commons:** CC0-1.0, CC-BY-4.0, CC-BY-SA-4.0.
- **Public domain.**
- **Contributor-original:** created by a contributor who owns it and licenses it under the project
  licence (`MIT OR Apache-2.0`).

Anything whose licence you cannot prove from its original source is forbidden. This includes
"free for personal use" material, scraped vendor catalogues, and profile tables copied from a
commercial database. **Profile geometry comes from public standards or manufacturer-published
data with a licence that allows it**, entered as our own data.

**One narrow exception: the BeamCraft brand.** The BeamCraft name, wordmark and logos in
`docs/brand/` are trademarks of Apibim (registration pending) and are not open source (terms in
`docs/brand/LICENSE-brand.txt`). Never modify the logos, and never add other files under that
licence. Forks must remove them.

### 1.3 Every asset is attributed

`ATTRIBUTION.md` lists every asset: path or URL, author, source with version, SPDX licence, and how
it is used. If an asset's origin or licence is unclear, **do not add it.** Ask the user.

If you find an asset that breaks these rules, stop, remove it from the repository and every output,
and tell the user at once.

---

## 2. Clean-room rules

- Never decompile, disassemble or copy anything from a Tekla/Trimble installation. Never copy code
  from the Tekla Open API samples, reference assemblies or documentation examples.
- Behaviour comes only from **public specifications** (buildingSMART IFC, DSTV, EN 1993, AISC, ...),
  public documentation and **black-box observation** of synthetic fixtures.
- Never copy GPL/AGPL code. Copyleft tools may run only as external oracle processes.
- Never capture personal data, client projects or licensed files when observing other software.
- Document where each behaviour came from (spec section or observation note) in the PR.

## 3. Agent control (automation)

- **Everything is reachable headlessly.** Every user-facing feature needs an engine API first, then a
  CLI command. A feature is not done until it has an end-to-end test.
- Any control channel, scripting server or MCP server is **opt-in**, loopback-only and
  token-authenticated. Never enable one by default or widen its bind address.

## 4. Never crash

Every model file, IFC/DSTV import, script and keystroke is untrusted input.

- No `unwrap`/`expect`/`panic!`/`todo!`/`unreachable!` outside tests, unless provably infallible
  with a comment saying why (clippy enforces this).
- No indexing or slicing with input-derived positions. Use checked or saturating arithmetic. Cap
  allocations sized by input. Bound recursion.
- No `unsafe` (`unsafe_code = "forbid"`).
- A fixed crash gets a small synthetic regression test.

## 5. Dependencies

Permissive licences only (`deny.toml`, enforced in CI). Prefer well-maintained pure-Rust crates.
New outbound network access is the owner's decision, not an agent's.
