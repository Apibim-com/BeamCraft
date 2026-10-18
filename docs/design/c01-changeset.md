# C01 changeset and idempotency

Date: 2026-10-18. Status: **experimental vertical slice; C01 acceptance is limited as listed below.**
Branch `feat/c01-changeset`, based on `origin/master` `3f9f0f5`.

Plan row: C01 / M, depends on P06, P08 and F05. Result: base, op_id, actor, dependencies, USD delta
and assets; bounded validation. Acceptance: a repeated op_id with the same payload is safe; with a
different payload it is rejected; the author string grants no rights.

## What is implemented

```text
beamcraft changeset-create  --store PUBLISHED --workspace-dir DIR [--changeset-id UUID] [--revision-id UUID] [--actor-id UUID]
beamcraft changeset-inspect --workspace-dir DIR --changeset SHA256
beamcraft changeset-apply   --store PUBLISHED --workspace-dir DIR --changeset SHA256 --actor-id UUID
```

| Area | What exists |
|---|---|
| `beamcraft_model::changeset` | `Changeset { changeset_id, project, workspace, actor, base_revision, base_manifest, revision, operations, dependencies, layers, root_layer, assets }`; canonical binary `encode`/`decode` (versioned, length-prefixed, sorted, bounded at 1 MiB, round trip enforced); `digest`; `validate`; `admit(trusted_actor, recorded) -> Apply | Replay`. `ActorId` joins the typed ids. Pure and WASM-compatible. |
| `beamcraft_host::changeset` | `RevisionStore::claim_changeset` / `changeset_claim`: an append-only, checksummed `changesets/claims.log`; the first valid line of an id is its binding. |
| `beamcraft_cli::changeset` | `create_changeset` (from a workspace: base, unsent operations, complete layer set with base digests, changed layers as assets, conservative footprint from the USD difference), `read_changeset`, `apply_changeset` (decode, actor, replay, claim, base, blob copy and verification, commit with the strict verifier, workspace base advanced). |

## Decisions

- **What a changeset is.** A message about a USD state, never a second model: the layers stay USD
  blobs addressed by full SHA-256, and the changeset names the complete resulting layer set with
  each layer's base digest. A receiver can rebuild the revision exactly and can see which layers
  changed (the delta) without a diff format of its own.
- **Operations.** Every unsent draft revision contributes its operation id and its manifest digest
  (the content identity of its accepted result), in draft order. The order is part of the payload.
- **Dependencies.** The per-command engine footprints of draft operations are not journaled.
  Writes come from the difference between the base model and the draft head, in the F04 section 8
  groups: a changed field writes its group (an unknown field: every group), a placement change
  also writes `derived`, a changed descendant writes `derived`, created and removed elements write
  every group, new definitions are written; provenance changes alone are not changes. Reads cannot
  come from a difference (a copy reads its unchanged source, a move reads the shape), so the
  changeset reads every group of every element and every definition of the base and the head, a
  superset of any operation's reads; over 4096 entries it is `LIMIT_EXCEEDED`. Recording
  per-command footprints would make reads precise.
- **Idempotency (project-wide).** A changeset id names one payload forever. The receiver binds the
  id to the payload digest durably *before* it publishes anything, so a crash after the binding
  resumes on retry, a retry of the same payload is a replay (the ledger proves the commit), and
  any other payload under the id is `OP_ID_REUSE` with nothing written — also when the first
  receipt was refused (for example `BASE_REVISION_MISMATCH`). This is stricter than F04's
  draft rule (an aborted op_id is free again) on purpose: a refused changeset is rebuilt under a
  new id, and an id that once named a payload can never be confused with another. The published
  revision's operation id is the changeset id, so the U06 commit binds it to the exact content
  as a second line of defence. A replay is reported only when the revision committed under the id
  is exactly this changeset's publication (revision id, base, root layer, every layer's name,
  role, digest and size); an id already committed for another operation is `OP_ID_REUSE` before
  anything is claimed. After the claim, a moved head is re-checked against the ledger, so two
  identical concurrent receipts both return the published result.
- **Workspace update after publication.** Metadata is validated before publishing; if the update
  of `workspace.json` still fails afterwards, the result is a successful publication with
  `workspace_synced: false` (never a rejection of published work), and a retry repairs it.
- **Authority.** `admit` checks the claimed actor against the trusted caller *before* the replay
  decision (F04 section 8 order), so a mismatched caller learns nothing about an id. The CLI's
  `--actor-id` stands for the trusted context until C02 adds tokens.
- **Concurrency of claims.** One synced append per claim and no lock file, as in the U07 ledger;
  racing receivers re-read and the earliest line decides (tested with eight threads). Records are
  decoded line by line, so a damaged line hides nothing after it. Size admission is not atomic
  across writers, so readers accept a 1 MiB headroom beyond the 16 MiB admission cap.
- **No merge.** The base must be the published head (revision id and manifest digest), otherwise
  `BASE_REVISION_MISMATCH`; rebase is C03, conflicts C04.

## Evidence

`apps/beamcraft-cli/tests/changeset.rs` (4 end-to-end tests through the binary): a changeset of two
renames carries the base, both operations, the label/lifecycle footprint, one changed layer and one
asset; the digest is deterministic; inspect shows the same; apply publishes revision and op ids as
declared and moves the workspace base; a retry replays with nothing written. The caller must be the
claimed actor (also for a forged author and after binding); another payload under the id is
`OP_ID_REUSE`. A moved head is `BASE_REVISION_MISMATCH` on every retry, the id stays bound, and the
draft stays unsent. Garbage, a non-canonical trailing byte, a future version and an unknown digest
are refused with nothing published. `crates/host/tests/changeset_claims.rs` covers reopen and an
eight-thread race. `beamcraft_model::changeset` unit tests cover the round trip, every prefix,
single-byte flips, caps before allocation, ordering, duplicates and the admission matrix.

## Not done

- No coordinator, server, token or queue (C02); no rebase (C03) or semantic conflicts (C04).
- The footprint is derived from the USD difference, not recorded per command; it is conservative
  but coarser than the engine's command footprints for some edits.
- Changesets travel inside the workspace store (the receiver reads the workspace's blobs); there is
  no transport or bundle format yet (F05 frames would carry them).
- Claims are never garbage-collected; the claims file is capped at 16 MiB.

## Proposed Roadmap entry

`2026-10-18 C01: changesets (model::changeset canonical bytes and admission, host changeset claims, CLI changeset-create/inspect/apply; same payload replays, another payload under the id is OP_ID_REUSE, the claimed actor must be the caller); experimental slice.`
