# Implementation task template

For kernel and engineering tasks issued under the [admission policy](admission-policy.md).
Fill every field. The task is the implementer's **entire** brief: do not attach chat history,
reviewer inputs or private files, and do not widen the inputs afterwards.

```text
Task:               <ID and title>
Issued by / date:   <orchestrator; date>
Base:               <repository commit SHA>   Branch/worktree: <codex/<id>-<slug>>
Fresh context:      required; start with no inherited history (a sub-agent gets no parent turns)
Exposure record:    <filed privately: eligible for <area> | owner-accepted risk reference>

Goal (one result):
  <what must exist when the task is done, in terms of observable behaviour>

Approved inputs (this list is exhaustive):
  Kind               | Identifier (document + section / package ID + version) | Commit | Notes
  -------------------|-------------------------------------------------------|--------|------
  Admitted package   | <e.g. first-scope package 0.1.0>                      | <sha>  |
  Public reference   | <e.g. RFC 9562 section 5.7>                           | n/a    |
  Owner decision     | <repo path and section>                               | <sha>  |
  Project code       | <crate/module paths>                                  | <sha>  |
  Acceptance tests   | <path to independent cases, version>                  | <sha>  |

Not inputs (state plainly):
  Anything not listed above. No commercial-product material, no third-party geometry kernel
  source or tests, no vendor numerical results, no private working files. Do not search for
  existing implementations of the algorithms involved.

Allowed paths:      <explicit allowlist; shared files such as manifests and lock files excluded>
Interfaces:         <engine API and CLI entry points that must exist; error and unknown behaviour>
Units and limits:   <mm, rad, right-handed, Z up; input limits; allocation and recursion bounds>
Failure semantics:  <what is a rejection, what is unknown, what is unsupported; stable codes>
Out of scope:       <copy the relevant exclusions from the admitted package>

Acceptance:
  - Independent cases <IDs> pass, compared under the package's comparison rule.
  - Repository checks pass (format, Clippy with warnings denied, tests, documentation, WASM
    check of library crates, dependency checks) or each unavailable tool is reported.
  - Meaningful unit tests of your own code are included. Acceptance cases are not edited;
    suspected errors in them are reported with evidence.

Report back:        base and commit IDs; changed paths; commands and exact results; limitations;
                    any material consulted outside the input list; any exposure incurred.
Stop and report if: you meet third-party kernel code or restricted material; a behaviour you need
                    is not in the inputs; an input conflicts with another; a check cannot run.
Commits and PR:     imperative English messages; one focused PR; no merge by the implementer.
```
