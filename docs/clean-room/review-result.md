# F09 review result (public), version 0.1.0

Date: 2026-10-14. Reviewer: an independent chat started without the history of F02, the
architecture work or integration. Contributor-original, MPL-2.0.

This is the public-only result. The admission matrix, source trace, exposure record and full
notes are private and are not part of this repository. Nothing here is a legal clearance, an
independence declaration or a provenance proof; it records what was checked and what is open.

## Decision and scope

- **Requirements:** a limited package is admitted for one scope: a rectangular plate, a straight
  member with a synthetic rectangular section, identities, units, rigid placement and declared
  failures. See the [package](handoff/first-scope-v0.1.md). Broader requirements stay pending.
- **Not decided here:** the F04 contract is still a draft and is **not** declared frozen. K0 is
  not passed. No kernel code was written and no future kernel test was run.
- **Source-exposure admission** is separate from the above. This review does not declare any
  analyst or reviewer eligible to implement kernel algorithms (see "Exposure" below).

## Base

The task card recorded base `b34205281ab8a1f95052c4dc7d84f0995633e179`. The remote default
branch moved twice during the work. It first gained policy PR #7 (the scoped OpenUSD licence
exception), and the branch was fast-forwarded to `d297d796a99323ae3105b3d384e6a8515d22faf6`
before any change. It then gained the F04 draft (PR #6, merge `d190e7c`) and the branch-naming
policy (PR #8). This change is rebased onto `ebb5b928dda3d36d6b6570549c3f745bb1883ff7`; the
only conflicts were the shared attribution and roadmap entries, and all existing entries were kept.

## What was checked

| Check | Result |
|-------|--------|
| The private requirements analysis was recounted by an independent script (new code, own pattern, no reuse of the analyst's tools) | 35 sources, 160 occurrences, 156 unique IDs; every occurrence matches exactly one analysis row; all recorded source digests equal freshly computed digests; 0 failures |
| Admission matrix covers all 156 unique IDs once | 156 rows: 5 accepted for a limited scope, 63 pending, 15 rejected as superseded, 2 unsupported, 71 not applicable to the kernel handoff |
| Evidence is not promoted | No accepted row rests on documentation-only or unverified evidence; no row treats vendor numerical results as expected output |
| Package entries | 12 entries, each with approved source, status and expected-result basis; open gaps are excluded explicitly |
| Expected results | Re-derived with exact rational arithmetic; every case value matches; the quaternion literals used as inputs match their ideal rotations within 1e-14 and satisfy the 1e-12 norm limit |
| Public files | Scanned for private paths, filenames, requirement IDs, digests, product names, catalogue designations and numbered source names: none found |
| Links | All relative links and heading anchors resolve; the two external references returned HTTP 200 |
| Coincidence check | One round-number overlap between a chosen test input and private observations was judged non-derivative; no combination of values matched; details stay private |

## Findings on the F02 analysis boundary

1. The analysis is accepted as a local analytical artifact. Its counts, hashes and row mapping
   reproduce. It is **not** an approved kernel input.
2. Of the 156 IDs, most carry documentation-only or unverified evidence (75 documentation-only,
   7 unverified, 27 documentation plus observation). These cannot be treated as confirmed
   behaviour, and the matrix keeps them pending.
3. Several rows mix a statement with product-specific defaults and numbers. Those numbers are
   oracle-only. They are not copied to this package.
4. 19 IDs have no planned package and one needs an owner decision. None is admitted; each
   carries an open owner question in the private matrix.

## Findings on the F04 draft handoff (reviewed at PR #6 head `b378d7b339dc5319b77892c5f95ed9ae168ae943`)

The reviewed [contract](../contracts/model-and-commands-v0.1.md) and
[cases](../contracts/model-command-cases-v0.1.json) are byte-identical to the versions now on the
default branch (compared by digest). The text and cases contain no private identifiers, product wording or catalogue
values. Its worked numbers (1000 x 500 x 10 mm gives 0.005 m3; width 750 gives 0.00375 m3; mass
5 kg at 1000 kg/m3) were recomputed and are correct.

Concrete issues, none of which blocks the limited package but all of which should be resolved
before freezing the contract:

- **Rebase (resolved).** At review time PR #6 predated PR #7 and, as submitted, would have
  reverted the scoped OpenUSD exception in `AGENTS.md`. The merged result keeps that exception
  (checked in the current `AGENTS.md`), so no action remains.
- **G-01, G-02:** the plate's local frame and the member section's orientation are undefined.
- **G-03:** the code for a zero-length axis is ambiguous.
- **G-04:** the quaternion convention and composition order are not stated.
- **G-05:** the cases give expected values with no comparison rule and omit degenerate, boundary,
  invalid-quaternion and translation-limit cases. The package adds original cases for these and
  does not duplicate the existing ones; it refers to them by ID.
- **G-06:** no minimum dimension or geometric tolerance policy exists (owned by F06).

## Exposure

- The reviewer viewed no third-party geometry kernel source or tests in this chat.
- The reviewer read excerpts of restricted-class analysis notes for a small set of rows
  (identity, project data, coordinates, plates, members, quantities). For those areas the
  category is E4, so this reviewer is **not eligible to implement them**.
- Earlier training exposure is unknown (EU) and cannot be certified.
- The orchestrator of earlier work has recorded that it viewed individual kernel sources, and
  the author of the F02 analysis studied the restricted material. Neither is declared eligible.
  Eligible implementers must file their own record and start a fresh context.

## Verification of the repository

Run from the branch before opening the PR (Windows x64, Rust 1.96.0, `--offline --locked`):

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| `cargo test --workspace` | passed: 3 tests (the scaffold's) |
| `cargo doc --workspace --no-deps` with warnings denied | passed |
| `cargo run -p beamcraft-cli -- --version` | `beamcraft 0.0.1` |
| WASM check of the domain crates | **unverified**: the target is not installed (E0463) |
| `cargo deny check` | **unverified**: the tool is not installed |

The change is documentation only. Remote CI results for the pull request are recorded in the PR.

## Limitations

- Statuses are the reviewer's judgement from the evidence available, not a mechanical result.
- The source text was read only for the five accepted rows and a few boundary rows (position
  presets, diagnostics, project data). Every other status was assigned by area rules from the
  analysis's recorded evidence, coverage and owner-decision fields, with the "pending" default
  for kernel-relevant rows. It was not a re-reading of all 160 occurrences, and the 71
  "not applicable" rows were not individually re-read for hidden kernel requirements.
- No oracle tool was run; no commercial product was operated or opened.
- The package does not validate F07 fixtures. The checklist in the
  [policy](admission-policy.md#fixture-provenance-checklist) lists what they must pass.

## Remaining owner decisions

1. Confirm the limited package scope, or amend it, and record it as the first admitted input.
2. Decide the plate local frame and member section orientation (G-01, G-02), or delegate them
   to F06 with a deadline.
3. Decide which candidates are eligible implementers, from filed exposure records.
4. Decide whether, and how, vendor numerical results may serve as oracle-only comparisons.
5. Decide scope for requirement rows with no package.
6. Adopt or replace the amendments G-03 to G-05 in the F04 contract before freezing it.
