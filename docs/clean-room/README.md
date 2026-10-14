# Clean-room admission

Contributor-original documents under MPL-2.0 that control which requirements and which people
or agents may feed BeamCraft's own geometry and engineering kernel. They apply `AGENTS.md`
section 2. Nothing here is legal advice or a proof of independence.

| Document | Purpose | Status |
|----------|---------|--------|
| [admission-policy.md](admission-policy.md) | Roles, exposure categories, approved-input allowlists, independent test provenance, fresh contexts, contamination | Version 0.1.0 |
| [exposure-record-template.md](exposure-record-template.md) | Private record each analyst, reviewer, tester and implementer files | Template |
| [implementation-task-template.md](implementation-task-template.md) | Task layout that lists only approved inputs | Template |
| [handoff/first-scope-v0.1.md](handoff/first-scope-v0.1.md) | Admitted requirements: rectangular plate, straight member, identities, units, placement, declared failures | Admitted for that scope only |
| [handoff/first-scope-cases-v0.1.json](handoff/first-scope-cases-v0.1.json) | Original analytic expectations for the package | Admitted for that scope only |
| [review-result.md](review-result.md) | Public result of the independent review, exact checks, limits and owner decisions | Version 0.1.0 |

Key rules in one place:

- Requirement admission and source-exposure admission are separate decisions.
- Implementers receive only what an issued task lists. Restricted material, summaries of it
  and vendor numerical results are never inputs; vendor numbers are oracle-only.
- A new chat does not erase earlier exposure. Unknown exposure stays unknown.
- The package refers to the [F04 contract](../contracts/model-and-commands-v0.1.md) by case ID.
  That contract is a review draft, not frozen.
- Admitted today: the first-scope package only. K0 is not passed and no kernel exists yet.
