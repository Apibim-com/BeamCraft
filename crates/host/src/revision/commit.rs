// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The commit procedure: verify everything, then switch the root in one filesystem step.

use std::io;
use std::path::Path;

use beamcraft_model::{ErrorCode, digest::ContentDigest};

use crate::ledger::{AbortReason, CommittedRecord, Intent, OpStatus, Record, now_secs};

use super::manifest::{BaseRef, Content, IndexRecord, LayerRole, Manifest, RevisionDraft};
use super::read::RevisionStore;
use super::root::{self, RevisionInfo, RootRecord, TempRecord};
use super::{MAX_CONTENT_VERIFY_BYTES, MAX_REVISIONS, REPLAY_LOOKBACK, RevisionError, Uuid7};

/// How the root switch ended: this call installed the record, or another committer already
/// installed this very operation.
enum Switched {
    Created(RevisionInfo),
    Replayed(RevisionInfo),
}

/// The base the caller believes is the current root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectedBase {
    /// Nothing has been published yet.
    Initial,
    /// The revision that is the current root. Any other root is
    /// [`RevisionError::BaseMismatch`].
    Revision(Uuid7),
}

/// How a successful commit ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitOutcome {
    /// This call switched the root to the new revision.
    Created,
    /// The same operation with identical content was already committed; the recorded revision
    /// is returned and nothing was written to the root.
    Replayed,
}

/// A successful commit: the revision that is, or was at its time, the root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Committed {
    /// Whether this call created the revision or found it already recorded.
    pub outcome: CommitOutcome,
    /// The committed record and manifest.
    pub info: RevisionInfo,
}

/// A defined point of the commit between the first read and the end. A [`CommitHook`] is called
/// when the commit reaches it. U07 uses these points to inject faults; they are the contract
/// between U06 and U07, in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitPoint {
    /// Draft, base, replay and project checks passed. Nothing has been written.
    Checked,
    /// The write-ahead intent is recorded in the ledger and synced (U07). The root is unchanged.
    IntentRecorded,
    /// Every referenced blob (and the content verifier) passed. The root is unchanged.
    BlobsVerified,
    /// The manifest blob is stored. It is unreferenced until the switch.
    ManifestStored,
    /// The complete record is written and synced under a temporary name; the root is unchanged.
    RecordStaged,
    /// The root switched: the record is installed, the ledger has no outcome yet. A failure here
    /// is `OUTCOME_UNKNOWN`.
    Switched,
    /// The `Committed` record is in the ledger: the commit is complete and its outcome proven.
    /// The hook is only told; an error here is ignored.
    Recorded,
}

/// A fault a [`CommitHook`] injects. It carries no detail on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Injected;

/// Observer and fault-injection seam called at each [`CommitPoint`] in order. Returning
/// `Err(Injected)` before [`CommitPoint::Switched`] aborts the commit with the root unchanged
/// ([`RevisionError::Aborted`]) and the attempt closed in the ledger as cancelled; returning it
/// at `Switched` yields [`RevisionError::OutcomeUnknown`], because the new root is already
/// visible and the ledger is left for recovery or a retry to resolve. A hook that panics
/// simulates a crash at that point: no error handling runs and the ledger keeps exactly what
/// was synced so far.
pub trait CommitHook {
    /// Called when the commit reaches `point`.
    fn reached(&mut self, point: CommitPoint) -> Result<(), Injected>;
}

/// The hook that never fails.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoFault;

impl CommitHook for NoFault {
    fn reached(&mut self, _point: CommitPoint) -> Result<(), Injected> {
        Ok(())
    }
}

impl<F: FnMut(CommitPoint) -> Result<(), Injected>> CommitHook for F {
    fn reached(&mut self, point: CommitPoint) -> Result<(), Injected> {
        self(point)
    }
}

/// A verified layer handed to a [`ContentVerifier`]: the manifest claim and the exact bytes
/// whose digest and size the host has already checked.
#[derive(Debug, Clone, Copy)]
pub struct VerifiedLayer<'a> {
    /// Virtual layer name.
    pub name: &'a str,
    /// Layer role.
    pub role: LayerRole,
    /// Verified full digest.
    pub digest: ContentDigest,
    /// Verified bytes.
    pub bytes: &'a [u8],
}

/// Why a verifier refused the layers: a stable F04 code and a fixed safe message, mirroring
/// the layer engine's `Diagnostic` so its codes (`DUPLICATE_ID`, `INVALID_ID`,
/// `DANGLING_RELATIONSHIP`, `FOREIGN_READ_ONLY`, ...) pass through unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentRejection {
    /// Stable F04 code.
    pub code: ErrorCode,
    /// Fixed ASCII explanation.
    pub message: &'static str,
}

/// Plain evidence of a U03 index the verifier derived from the layers it was given. It carries
/// no authority by itself: the host compares it with the draft before recording it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexEvidence {
    /// Project of the context the index was built in.
    pub project: String,
    /// Workspace of that context.
    pub workspace: String,
    /// Workspace generation of that context; must equal the draft's.
    pub generation: u64,
    /// Revision digest pinned by that context: the base manifest digest of the draft.
    pub pinned_revision: ContentDigest,
    /// `Completeness::Complete` of the index.
    pub complete: bool,
    /// `IndexToken::structure_digest`.
    pub structure_digest: ContentDigest,
    /// `IndexToken::prim_count`.
    pub prim_count: u64,
}

/// Content-level check run after the blobs verify and before the root switch. The host never
/// parses USD: an implementation (the layer engine, in its own process or crate) opens the
/// layers, applies the U03 guards (`verify_index`: live, complete, untampered) and the
/// duplicate/short-ID/dangling checks, and returns the evidence or a [`ContentRejection`].
pub trait ContentVerifier {
    /// Verifies the complete layer set of one revision.
    fn verify(&self, layers: &[VerifiedLayer<'_>]) -> Result<IndexEvidence, ContentRejection>;
}

fn io_error(error: io::Error) -> RevisionError {
    RevisionError::Io(error.kind())
}

fn point(hook: &mut dyn CommitHook, at: CommitPoint) -> Result<(), RevisionError> {
    hook.reached(at).map_err(|_| RevisionError::Aborted(at))
}

impl RevisionStore {
    /// Commits a draft with no content verifier and no fault hook. See [`Self::commit_with`].
    pub fn commit(
        &self,
        draft: &RevisionDraft,
        expected: &ExpectedBase,
    ) -> Result<Committed, RevisionError> {
        self.commit_with(draft, expected, None, &mut NoFault)
    }

    /// Commits one complete revision.
    ///
    /// Order: normalize and validate the draft; read and verify the current root; return the
    /// recorded revision if this op id was already committed with identical content
    /// ([`RevisionError::OpIdReuse`] if with different content, [`RevisionError::RevisionIdReuse`]
    /// if the revision id belongs to something else); compare `expected` with the root
    /// ([`RevisionError::BaseMismatch`]) and the project; verify every referenced blob and run
    /// the optional `verifier`; store the manifest blob; write and sync the record; install it
    /// with a non-replacing link. Everything before the link leaves the root unchanged, so any
    /// error before [`CommitPoint::Switched`] is all-or-nothing. A concurrent committer that
    /// takes the same sequence first makes the link fail, and this call re-resolves to a replay
    /// or a [`RevisionError::BaseMismatch`].
    ///
    /// With a verifier the manifest records its verified index; without one it records none.
    ///
    /// U07: after the base, project and replay checks the commit appends a synced intent to the
    /// outcome ledger ([`crate::ledger`]) before anything of the attempt can reach the root (if
    /// that fails the commit fails with nothing changed), and after the switch appends a
    /// `Committed` record. A failure before the switch closes the attempt as aborted; a failure
    /// to record after the switch is [`RevisionError::OutcomeUnknown`], resolved by
    /// [`RevisionStore::recover`] or an identical retry. An op id stays bound to its content
    /// while an attempt is unresolved or committed ([`RevisionError::OpIdReuse`]), also beyond
    /// the head log's look-back.
    pub fn commit_with(
        &self,
        draft: &RevisionDraft,
        expected: &ExpectedBase,
        verifier: Option<&dyn ContentVerifier>,
        hook: &mut dyn CommitHook,
    ) -> Result<Committed, RevisionError> {
        let content = Content::from_draft(draft)?;
        let fingerprint = content.fingerprint()?;
        let directory = self.head_dir()?;
        let head = match &directory {
            Some(directory) => self.load_head_in(directory)?,
            None => None,
        };
        let ledger = self.ledger();
        let scan = ledger.scan()?;
        if let (Some(directory), Some(head)) = (&directory, &head)
            && let Some(info) = self.find_replay(directory, head, &content)?
        {
            self.record_committed(&info);
            return Ok(Committed {
                outcome: CommitOutcome::Replayed,
                info,
            });
        }
        // An op id stays bound to its content while an attempt is unresolved or committed. An
        // attempt closed as not accepted had no effect and leaves the id free.
        if scan
            .open_intents_of(content.op())
            .iter()
            .any(|open| open.fingerprint != fingerprint)
        {
            return Err(RevisionError::OpIdReuse);
        }
        if let OpStatus::Committed(record) = scan.status(content.op()) {
            let info = self.load_committed(directory.as_deref(), &record)?;
            return if info.manifest().content() == &content {
                Ok(Committed {
                    outcome: CommitOutcome::Replayed,
                    info,
                })
            } else {
                Err(RevisionError::OpIdReuse)
            };
        }
        match (&head, expected) {
            (None, ExpectedBase::Initial) => {}
            (Some(head), ExpectedBase::Revision(id)) if head.revision() == id => {}
            _ => return Err(RevisionError::BaseMismatch),
        }
        if head
            .as_ref()
            .is_some_and(|h| h.manifest().content().project() != content.project())
        {
            return Err(RevisionError::ProjectMismatch);
        }
        let sequence = head
            .as_ref()
            .map_or(Some(1), |h| h.sequence().checked_add(1))
            .filter(|s| *s <= MAX_REVISIONS)
            .ok_or(RevisionError::LimitExceeded)?;
        let base = head.as_ref().map(|h| BaseRef {
            revision: h.revision().clone(),
            manifest: h.manifest_digest(),
        });
        point(hook, CommitPoint::Checked)?;

        // Write-ahead: the intent is on stable storage before anything of this attempt can
        // reach the root. If it cannot be recorded, the commit fails here with nothing changed.
        let intent = Intent {
            op: content.op().clone(),
            project: content.project().clone(),
            revision: content.revision().clone(),
            sequence,
            base: head.as_ref().map(|h| h.revision().clone()),
            fingerprint,
            created: now_secs(),
            blobs: content.entries().iter().map(|e| e.digest()).collect(),
        };
        let record = Record::Intent(intent);
        let intent_id = record.encode().1;
        let genesis = scan
            .genesis_head()
            .is_none()
            .then(|| head.as_ref().map_or(0, RevisionInfo::sequence));
        ledger.append(&[record], genesis)?;

        match self.switch_root(&content, sequence, base, verifier, hook, &intent_id) {
            Err(error) => {
                // Best effort: a failure to write this leaves the intent for recovery to close.
                let reason = match error {
                    RevisionError::Aborted(_) => AbortReason::Cancelled,
                    RevisionError::BaseMismatch => AbortReason::LostRace,
                    _ => AbortReason::Rejected,
                };
                let _ = ledger.append(
                    &[Record::Aborted {
                        op: content.op().clone(),
                        intent: intent_id,
                        reason,
                    }],
                    None,
                );
                Err(error)
            }
            Ok(Switched::Replayed(info)) => {
                self.record_committed(&info);
                Ok(Committed {
                    outcome: CommitOutcome::Replayed,
                    info,
                })
            }
            Ok(Switched::Created(info)) => {
                if hook.reached(CommitPoint::Switched).is_err() {
                    return Err(RevisionError::OutcomeUnknown);
                }
                let committed = CommittedRecord {
                    op: content.op().clone(),
                    revision: info.revision().clone(),
                    sequence: info.sequence(),
                    manifest: info.manifest_digest(),
                };
                // The root already switched: a failed write is an unknown outcome, never a
                // success and never a rollback. Recovery or a retry resolves it.
                ledger
                    .append(&[Record::Committed(committed)], None)
                    .map_err(|_| RevisionError::OutcomeUnknown)?;
                let _ = hook.reached(CommitPoint::Recorded);
                Ok(Committed {
                    outcome: CommitOutcome::Created,
                    info,
                })
            }
        }
    }

    /// Everything between the recorded intent and the end of the root switch. Any error leaves
    /// the root unchanged; success means the record is installed (or another committer
    /// installed this very operation first).
    fn switch_root(
        &self,
        content: &Content,
        sequence: u64,
        base: Option<BaseRef>,
        verifier: Option<&dyn ContentVerifier>,
        hook: &mut dyn CommitHook,
        intent_id: &ContentDigest,
    ) -> Result<Switched, RevisionError> {
        point(hook, CommitPoint::IntentRecorded)?;

        let index = self.verify_content(content, base.as_ref(), verifier)?;
        point(hook, CommitPoint::BlobsVerified)?;

        let manifest = Manifest::assemble(content.clone(), sequence, base, index)?;
        let (manifest_digest, _) = self.blobs.publish(&manifest.encode())?;
        point(hook, CommitPoint::ManifestStored)?;

        let record = RootRecord::new(sequence, content.revision().clone(), manifest_digest);
        let directory = self.blobs.namespace_dir(root::DIR, true)?;
        let temp = TempRecord::create(&directory, &record.encode()).map_err(io_error)?;
        point(hook, CommitPoint::RecordStaged)?;

        // Recovery may have closed this intent while it looked abandoned. Re-check right before
        // the link; this narrows that race, it cannot close it (see the ledger module docs).
        let again = self.ledger().scan()?;
        if !again.is_open(intent_id) {
            drop(temp);
            return if matches!(again.status(content.op()), OpStatus::Committed(_)) {
                self.resolve_lost_race(content).map(Switched::Replayed)
            } else {
                Err(RevisionError::Aborted(CommitPoint::RecordStaged))
            };
        }

        match temp.install(&directory, sequence) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                drop(temp);
                return self.resolve_lost_race(content).map(Switched::Replayed);
            }
            Err(e) => return Err(io_error(e)),
        }
        Ok(Switched::Created(RevisionInfo { record, manifest }))
    }

    /// Appends a `Committed` record for a revision the root proves, unless the ledger already
    /// holds one. Best effort: the root is the authority, the ledger only restates it.
    fn record_committed(&self, info: &RevisionInfo) {
        let ledger = self.ledger();
        let Ok(scan) = ledger.scan() else { return };
        let op = info.manifest().content().op();
        if matches!(scan.status(op), OpStatus::Committed(_)) {
            return;
        }
        let genesis = scan
            .genesis_head()
            .is_none()
            .then(|| self.head().ok().flatten().map_or(0, |h| h.sequence()));
        let record = Record::Committed(CommittedRecord {
            op: op.clone(),
            revision: info.revision().clone(),
            sequence: info.sequence(),
            manifest: info.manifest_digest(),
        });
        let _ = ledger.append(&[record], genesis);
    }

    /// Looks for the draft's op id (and revision id) in the head and at most
    /// [`REPLAY_LOOKBACK`] ancestors. The walk stops quietly at an unreadable ancestor: an
    /// older replay is then answered by the base check, never by a second commit.
    fn find_replay(
        &self,
        directory: &Path,
        head: &RevisionInfo,
        content: &Content,
    ) -> Result<Option<RevisionInfo>, RevisionError> {
        let mut current = head.clone();
        for _ in 0..REPLAY_LOOKBACK {
            let seen = current.manifest().content();
            if seen.op() == content.op() {
                return if seen == content {
                    Ok(Some(current))
                } else {
                    Err(RevisionError::OpIdReuse)
                };
            }
            if seen.revision() == content.revision() {
                return Err(RevisionError::RevisionIdReuse);
            }
            let Some(previous) = current.sequence().checked_sub(1).filter(|s| *s > 0) else {
                break;
            };
            match self.load_at(directory, previous) {
                Ok(info) => current = info,
                Err(_) => break,
            }
        }
        Ok(None)
    }

    fn resolve_lost_race(&self, content: &Content) -> Result<RevisionInfo, RevisionError> {
        let directory = self.head_dir()?.ok_or(RevisionError::TornRoot)?;
        let head = self
            .load_head_in(&directory)?
            .ok_or(RevisionError::TornRoot)?;
        self.find_replay(&directory, &head, content)?
            .ok_or(RevisionError::BaseMismatch)
    }

    /// Verifies every referenced blob and, with a verifier, the content-level evidence.
    fn verify_content(
        &self,
        content: &Content,
        base: Option<&BaseRef>,
        verifier: Option<&dyn ContentVerifier>,
    ) -> Result<Option<IndexRecord>, RevisionError> {
        let Some(verifier) = verifier else {
            for (index, entry) in content.entries().iter().enumerate() {
                self.read_entry(index, entry)?;
            }
            return Ok(None);
        };
        // Bound the memory before reading anything: the declared sizes are the claims the
        // verified reads below must then match exactly.
        let total = content
            .entries()
            .iter()
            .try_fold(0u64, |sum, e| sum.checked_add(e.size()))
            .filter(|t| *t <= MAX_CONTENT_VERIFY_BYTES);
        if total.is_none() {
            return Err(RevisionError::LimitExceeded);
        }
        let mut loaded = Vec::with_capacity(content.entries().len());
        for (index, entry) in content.entries().iter().enumerate() {
            loaded.push(self.read_entry(index, entry)?);
        }
        let layers: Vec<VerifiedLayer<'_>> = content
            .entries()
            .iter()
            .zip(&loaded)
            .map(|(entry, bytes)| VerifiedLayer {
                name: entry.name(),
                role: entry.role(),
                digest: entry.digest(),
                bytes,
            })
            .collect();
        let evidence = verifier.verify(&layers).map_err(RevisionError::Content)?;
        // A different project, workspace, generation or pinned base is a stale or foreign
        // context before it is a coverage question, as in the U03 `verify_index` order.
        if evidence.project != content.project().as_str()
            || evidence.workspace != content.workspace().as_str()
            || evidence.generation != content.workspace_generation()
            || base.is_some_and(|b| b.manifest != evidence.pinned_revision)
        {
            return Err(RevisionError::IndexMismatch);
        }
        if !evidence.complete {
            return Err(RevisionError::IndexIncomplete);
        }
        Ok(Some(IndexRecord {
            structure: evidence.structure_digest,
            prims: evidence.prim_count,
        }))
    }
}
