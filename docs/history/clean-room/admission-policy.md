# Clean-room admission policy, version 0.1.0

Date: 2026-10-14. Contributor-original, MPL-2.0. It applies the rules in `AGENTS.md` section 2 to
the work of building BeamCraft's own geometry and engineering kernel.

This is a process control. It is **not legal advice, not a legal clearance and not a proof of
independence.** It reduces the risk that third-party implementation detail enters the kernel; it
cannot certify what a person or model has seen in the past. Unknown exposure stays unknown.

## 1. Two separate decisions

1. **Requirement admission** decides whether a statement of behaviour may be given to
   implementers, and with which source and limits.
2. **Source-exposure admission** decides whether a particular person or agent may implement a
   particular area, given what they have already seen.

Passing the first never implies the second. A requirement may be admitted while every analyst
and reviewer who studied the underlying material stays ineligible to implement it.

## 2. Material classes

| Class | Examples | May an implementer receive it? |
|-------|----------|--------------------------------|
| Restricted | Observations or notes about a commercial product, its manuals, installation, screens, catalogues, sample data or numerical results; source code, tests, comments or translations of any other geometry kernel; summaries of the above | **No**, in any form, including paraphrase, structure or "hints" |
| Oracle output | Numbers produced by running an external tool on BeamCraft's own synthetic inputs | **No** as an expected value; only the independent tester may compare against it (section 7) |
| Public reference | Published standards, RFCs, textbook or reference-work mathematics, with edition and section | Yes, when listed in the task |
| Owner decision | A decision recorded in this repository (architecture, `AGENTS.md`, admitted documents) | Yes, when listed with its commit |
| Admitted package | A versioned handoff such as the [first-scope package](handoff/first-scope-v0.1.md) | Yes, when listed with version and commit |
| Project code | BeamCraft source at a pinned commit | Yes, when listed |

Restricted material never enters the repository, its documents, its tests or any prompt given to
an implementer. Private working files stay outside the repository and are never committed.

## 3. Roles

| Role | Does | Must not |
|------|------|----------|
| Owner | Sets scope, licences and risk acceptance; resolves named decisions | Be assumed to have approved anything not recorded |
| Observer / analyst | Studies restricted material in private; writes behaviour as private notes | Implement algorithms for the areas it studied; hand restricted text to an implementer |
| Admission reviewer | Checks requirements, evidence and the separation of the public package before issue | Review its own analysis alone; implement algorithms for the areas it reviewed from restricted material |
| Test author | Derives expected results from public mathematics or independent invariants | Copy expectations from an implementation, from restricted material or from oracle output |
| Implementer | Builds an admitted feature from the task's approved inputs only | Read restricted material, search for existing kernel implementations or tests, or change acceptance tests to pass |
| Orchestrator / integrator | Issues tasks, controls leases, integrates results | Correct an algorithm using knowledge from restricted material it has seen; widen a task's inputs |

One agent may hold several roles over time, but the exposure rules in section 4 then apply to
the combination. The author of an analysis must not be the sole reviewer of its admission.

## 4. Exposure records

Every analyst, reviewer, test author and implementer files an **exposure record** before work
starts, using the [template](exposure-record-template.md). Filled records are private; the public
repository carries only the template and, per task, the statement "exposure record filed:
eligible / not eligible / owner-accepted risk" without private details.

Exposure categories, recorded per area (for example "polygon triangulation" or "boolean
operations"):

| Code | Meaning |
|------|---------|
| E0 | No known exposure beyond public references |
| E1 | Read public standards or reference mathematics only |
| E2 | Read source, tests or comments of a third-party geometry kernel under a permissive licence |
| E3 | Read source, tests or comments of a third-party geometry kernel under another licence |
| E4 | Read or observed restricted commercial-product material about the area |
| EU | Unknown or cannot be established (this includes the unrecorded content of a model's training) |

An implementer for an area must be **E0 or E1 for that area**. E2, E3, E4 or EU makes the person
or agent ineligible for that area unless the owner records an explicit, scoped acceptance of the
risk; a reviewer never grants that acceptance. A new chat does not erase earlier exposure of the
same person or model: record it, do not assume it away. An agent that has seen restricted
material for an area cannot become eligible by being told to forget it.

## 5. Approved-input allowlist

A task lists its inputs as an allowlist. Anything absent from the list is not an input.
Each entry gives: kind (public reference, owner decision, admitted package or project code),
exact identifier (document and section, or package ID and version), and repository commit where
one exists. Entries for restricted material, oracle output and "everything in a folder" are not
permitted. The [task template](implementation-task-template.md) has the required layout.

## 6. Requirement admission

A requirement enters an admitted package only if all of the following hold:

1. **Approved source:** a public reference or a recorded owner decision supports the statement.
   An observation of a commercial product is not a source and does not by itself support anything.
2. **Evidence matches the claim:** documentation-only or unverified evidence is never recorded as
   confirmed behaviour. Such statements stay pending until a public basis exists.
3. **Independent expectation:** at least one expected result derived under section 7.
4. **Original wording:** no commercial-source wording, interface identifiers, catalogue values,
   traced assets, copied fixtures or descriptions of another kernel's algorithm or structure.
5. **Limits stated:** the scope, exclusions, units, failures, unknown and unsupported cases.
6. **Independent review:** someone other than the author checked items 1 to 5.

Status vocabulary for a private admission matrix: accepted (state the limited scope), pending,
rejected (superseded or contrary to a decision), unsupported (explicitly outside the scope) and
not applicable (outside the kernel handoff). A package may be approved for a small scope while
broader rows stay pending; the scope must be written in the package.

Vendor numerical results are **oracle-only**. They are never frozen as expected outputs and never
become production constants, tolerances, tables or defaults.

## 7. Independent test provenance

- Expected results come from closed-form mathematics, exact arithmetic or independent
  invariants (for example volume conservation under rigid motion).
- The derivation is written down with units and assumptions, and is re-derived by a method that
  shares no code or floating-point path with the implementation under test.
- Inputs are chosen by the test author, and checked for coincidence with restricted material
  without importing it: a coincidental round number is recorded; a match of a combination of
  values is a reason to choose different inputs.
- An external tool may run only as a separate process on BeamCraft's own inputs, after its
  licence terms are checked. Its output may cross-check the analytic result but is never the sole
  expectation and is not shown to the implementer.
- Negative, boundary, unknown and unsupported cases are tagged as such.
- Acceptance tests are changed only by an amendment: the error is explained, reviewed, and every
  dependent result is checked again. An implementer may add its own unit tests freely.

### Fixture provenance checklist

A synthetic fixture (for example from F07) passes only when every item is true and recorded:

1. It has a stable ID, units, support status, author and licence (`MPL-2.0`).
2. It is generated from original parameters; no vendor asset, catalogue entry, sample model,
   traced shape or copied file is involved.
3. Its bytes are real, and the recorded SHA-256 matches the bytes; there are no placeholders.
4. Each expectation has a written derivation that is independent of the generator.
5. Dimensions and densities are explicit test data, not a national or vendor catalogue.
6. Unsupported and unverified cases are tagged and excluded from acceptance counts.
7. Regeneration is deterministic and verified.
8. A coincidence check against restricted material was performed by a reviewer, and its result
   is recorded without copying that material.

## 8. Fresh implementer contexts

1. Start a new chat or agent with no inherited history. For a sub-agent, spawn it without the
   parent's conversation turns.
2. Give it only the issued task, built from the template, and the listed inputs.
3. Require its exposure record before it begins; refuse it if the record shows a blocking category.
4. It may not search for, open or paste any third-party geometry kernel source or tests. Public
   references in the allowlist are read as listed.
5. Its questions go to the orchestrator, who answers only from admitted text. A missing behaviour
   is a request for a new, separately reviewed observation, not an invitation to guess.
6. It reports changed paths, commands run, results, limitations and any external material it
   consulted.
7. The orchestrator does not patch its algorithms with knowledge from restricted material.

## 9. Contamination

If an implementer meets third-party kernel code, restricted material or a suspected copy, it
stops and reports. The affected change is quarantined; the owner decides whether it is removed,
rewritten by another eligible implementer, or accepted with a recorded rationale. Do not fix by
editing around the suspect text.

## 10. Pre-issue boundary review

Before a package or task is issued, a reviewer other than the author records that:

- every entry has an approved source and status; no row asserts more than its evidence;
- the public text carries no private paths, filenames, hashes, interface identifiers, catalogue
  values, source-derived algorithm or structure descriptions, or copied fixtures;
- the allowlist contains nothing restricted, and lists versions and commits;
- exposure records exist for every named person or agent, and eligibility is stated per area;
- links resolve and statuses are coherent between documents.

The [first review result](review-result.md) applies this list to the first package.

## 11. Storage

| Private (outside the repository) | Public (`docs/clean-room`) |
|----------------------------------|----------------------------|
| Admission matrix, source traces, exposure records, full review notes, restricted material | This policy, templates, admitted packages, public review results |

Never move private material into the public column, even as a summary or paraphrase.
