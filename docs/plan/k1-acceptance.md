# K1 acceptance verdict — 2026-10-21

**Verdict: acceptance remains open.** The basic two-plate CLI/save/reopen scenario passes on the
recorded Windows environment, including the independent OpenUSD reader. This is not full F04/F06
contract acceptance: deviations, partial assertions and unexecuted fixture adapters remain.
No K2 acceptance is claimed.

This verdict depends on [test PR #172](https://github.com/Apibim-com/BeamCraft/pull/172), based on
integration commit `09403ce8fabf973a975e523734af3dab8dcd90fa`. U16 and U17 are integrated there.
The [detailed evidence and complete case inventory](../design/k1-contract-acceptance.md) names
every F04/F06 case, its test, the checked fields and the reason for every omission. The
verification work is tracked by [issue #125](https://github.com/Apibim-com/BeamCraft/issues/125);
the reproduced contract gaps are in [issue #171](https://github.com/Apibim-com/BeamCraft/issues/171).

## Observed results

| Evidence | Result | Meaning |
|---|---|---|
| Two independent plate IDs, separate complete revisions, fresh-process reopen | Passed locally | Real CLI and persisted authored/derived layers; this is not a mock-only result |
| Independent reader | Passed locally | Four `pxr_oracle` tests with `BEAMCRAFT_REQUIRE_PXR=1`, Python 3.11.9, OpenUSD 0.25.11; compliance errors, failed checks and warnings all empty |
| Required reader negative control | Passed | A nonexistent interpreter makes the selected test fail with `K1 independent reader NOT RUN`; absence cannot be accepted as a pass |
| F04 fixture admission | 33 matching projections, 4 deviations, 5 skipped plate cases | Typed admission does not verify all response fields or the F04 envelope/workspace context |
| Other F04 cases | 27 logical and 7 raw cases not run by this adapter | All IDs and omission reasons are recorded; member/copy cases remain assigned to K2 |
| F06 fixture driver | 21 plate cases executed | One full pass, 16 partial projections, 4 rejected-input deviations; the face-table format discrepancy is recorded within POS-01's partial result |
| Other F06 cases | 91 explicitly accounted for | 30 member/mixed cases assigned to K2, 52 protocol-adapter omissions, 7 injection-adapter omissions, 2 superseded cases; none counted as passed |

The retained runtime tests also exercise killed-process commit checkpoints, old-or-new complete
revisions, concurrent writers, operation replay, recovery, corrupt journal/blob refusal and
compaction. Their exact test names are linked in the evidence note. Broader developer/CI checks
are recorded in the prerequisite PR; an unrun platform check is not a pass.

## Remaining acceptance work

1. Implement ordered 3D plate contour validation and the supported rectangular body frame.
   F04-POS-18 / F06-POS-48 currently return `UNSUPPORTED_GEOMETRY`. The non-planar negative
   cases return that code instead of `INVALID_ARGUMENT` (F04-NEG-29, F04-NEG-38,
   F06-NEG-38, F06-NEG-49). F04-POS-28 also needs the authored-acceptance versus unsupported
   evaluation distinction preserved. Each fix needs API, CLI, saved/reopened USD and independent
   reader evidence where applicable.
2. Complete and verify rigid plate placements in the existing P10 scope before claiming the
   world geometry and key assertions of F06-POS-11. Do not treat local unrotated mesh evidence as
   a passing world-space case.
3. Resolve the face-table descriptor interpretation explicitly: the fixture has `x0`, the
   current public mesh table has `plate:f.x0`, and the written contract describes `f.x0` within
   its body/element identity grammar. Role agreement is established; exact identifier-format
   acceptance is not. Preserve the frozen fixture unless the documented amendment process
   resolves it.
4. Supply the remaining K1-relevant fixture adapters and missing output assertions, including
   envelope/context handling, feature/key/query protocols and quantity statuses, or make an
   explicit owner-reviewed scope decision. Existing hand-copied unit cases alone do not close
   that verification gap. Member/copy/workspace/undo acceptance is assessed separately for K2.
5. Rerun the requirement matrix and independent reader after the fixes, collect required CI and
   platform evidence, then update this verdict and the roadmap in a docs-only PR. Do not close
   the umbrella issue while K2 remains outstanding.

This records observed evidence and the unchanged open status. It does not introduce a new plan
task, waive a contract requirement, restore the removed worker requirement or require a different
SDK version. The first implementation priority is the plate contour/body-frame gap in #171.
