// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Strict bounded reading of a whole ledger file into the per-operation state.

use std::collections::{BTreeMap, HashMap};

use beamcraft_model::digest::ContentDigest;

use crate::revision::Uuid7;

use super::record::{AbortReason, CommittedRecord, Intent, Record};
use super::{
    LEDGER_HEADROOM_INTENTS, LEDGER_HEADROOM_RECORDS, LedgerError, MAX_LEDGER_RECORDS,
    MAX_UNRESOLVED_INTENTS,
};

/// What the reader found that was not a valid record. None of it is ever trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LedgerDiagnostics {
    /// Size of the ledger file in bytes.
    pub bytes: u64,
    /// Valid records read.
    pub records: usize,
    /// Complete lines read, valid or not (blank lines included).
    pub lines: usize,
    /// Complete lines that are not a valid record (garbage, flipped bytes, a torn append that
    /// later appends moved past). Ignored.
    pub damaged_lines: usize,
    /// The file does not end in a newline: a torn final append. Ignored.
    pub torn_tail: bool,
}

/// Where one operation stands according to the ledger alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpStatus {
    /// An accepted commit is recorded.
    Committed(CommittedRecord),
    /// At least one attempt has no resolution.
    Unresolved,
    /// Every recorded attempt was closed as not accepted; the last reason.
    Aborted(AbortReason),
    /// No record of the operation.
    Unseen,
}

#[derive(Debug, Default)]
struct OpState {
    fingerprint: Option<ContentDigest>,
    committed: Option<CommittedRecord>,
    open: Vec<ContentDigest>,
    aborted: Option<AbortReason>,
}

/// The parsed ledger: per-operation state, the open (unresolved) intents and diagnostics.
#[derive(Debug, Default)]
pub struct Scan {
    genesis: Option<u64>,
    ops: HashMap<Uuid7, OpState>,
    open: BTreeMap<ContentDigest, Intent>,
    diagnostics: LedgerDiagnostics,
}

impl Scan {
    /// Parses ledger bytes. Records are applied in file order. An unterminated final fragment is
    /// a torn append and is ignored; a complete line that is not a valid record is counted and
    /// ignored; two accepted records for one operation that disagree are
    /// [`LedgerError::Inconsistent`]; more than [`MAX_LEDGER_RECORDS`] plus
    /// [`LEDGER_HEADROOM_RECORDS`] lines or [`MAX_UNRESOLVED_INTENTS`] plus
    /// [`LEDGER_HEADROOM_INTENTS`] open intents are [`LedgerError::LimitExceeded`]. New intents
    /// are admitted only below the base limits; the headroom is for racing appenders and for
    /// closing records.
    pub fn parse(bytes: &[u8]) -> Result<Self, LedgerError> {
        let mut scan = Self {
            diagnostics: LedgerDiagnostics {
                bytes: u64::try_from(bytes.len()).map_err(|_| LedgerError::LimitExceeded)?,
                ..LedgerDiagnostics::default()
            },
            ..Self::default()
        };
        let mut lines = 0usize;
        let mut pieces = bytes.split(|b| *b == b'\n').peekable();
        while let Some(piece) = pieces.next() {
            if pieces.peek().is_none() {
                scan.diagnostics.torn_tail = !piece.is_empty();
                break;
            }
            lines = lines
                .checked_add(1)
                .filter(|n| *n <= MAX_LEDGER_RECORDS + LEDGER_HEADROOM_RECORDS)
                .ok_or(LedgerError::LimitExceeded)?;
            scan.diagnostics.lines = lines;
            if piece.is_empty() {
                continue;
            }
            match std::str::from_utf8(piece).ok().and_then(Record::parse) {
                Some((record, id)) => {
                    scan.diagnostics.records += 1;
                    scan.apply(record, id)?;
                }
                None => scan.diagnostics.damaged_lines += 1,
            }
        }
        Ok(scan)
    }

    fn apply(&mut self, record: Record, id: ContentDigest) -> Result<(), LedgerError> {
        match record {
            Record::Genesis { head } => {
                // The first valid genesis wins: it describes the oldest state.
                self.genesis.get_or_insert(head);
            }
            Record::Intent(intent) => {
                let state = self.ops.entry(intent.op.clone()).or_default();
                state.fingerprint.get_or_insert(intent.fingerprint);
                if state.committed.is_none() {
                    state.open.push(id);
                    self.open.insert(id, intent);
                    if self.open.len() > MAX_UNRESOLVED_INTENTS + LEDGER_HEADROOM_INTENTS {
                        return Err(LedgerError::LimitExceeded);
                    }
                }
            }
            Record::Committed(committed) => {
                let state = self.ops.entry(committed.op.clone()).or_default();
                match &state.committed {
                    Some(first) if *first != committed => return Err(LedgerError::Inconsistent),
                    Some(_) => {}
                    None => {
                        for open in state.open.drain(..) {
                            self.open.remove(&open);
                        }
                        state.committed = Some(committed);
                    }
                }
            }
            Record::Aborted { op, intent, reason } if self.open.contains_key(&intent) => {
                self.open.remove(&intent);
                if let Some(state) = self.ops.get_mut(&op) {
                    state.open.retain(|open| *open != intent);
                    state.aborted = Some(reason);
                }
            }
            // An abort of an unknown or already closed intent changes nothing.
            Record::Aborted { .. } => {}
        }
        Ok(())
    }

    /// Head-log sequence recorded by the first valid genesis, if the ledger has one.
    pub const fn genesis_head(&self) -> Option<u64> {
        self.genesis
    }

    /// Fingerprint the operation was first bound to by an intent.
    pub fn fingerprint(&self, op: &Uuid7) -> Option<ContentDigest> {
        self.ops.get(op).and_then(|state| state.fingerprint)
    }

    /// The operation's standing according to this ledger.
    pub fn status(&self, op: &Uuid7) -> OpStatus {
        match self.ops.get(op) {
            None => OpStatus::Unseen,
            Some(state) => match (&state.committed, state.open.is_empty(), state.aborted) {
                (Some(committed), _, _) => OpStatus::Committed(committed.clone()),
                (None, false, _) => OpStatus::Unresolved,
                (None, true, Some(reason)) => OpStatus::Aborted(reason),
                (None, true, None) => OpStatus::Unseen,
            },
        }
    }

    /// True while the intent with this identity has no resolution.
    pub fn is_open(&self, intent: &ContentDigest) -> bool {
        self.open.contains_key(intent)
    }

    /// Intents without resolution, in a stable (identity) order.
    pub fn open_intents(&self) -> impl Iterator<Item = (&ContentDigest, &Intent)> {
        self.open.iter()
    }

    /// The unresolved intents of one operation.
    pub fn open_intents_of(&self, op: &Uuid7) -> Vec<&Intent> {
        self.ops
            .get(op)
            .into_iter()
            .flat_map(|state| state.open.iter())
            .filter_map(|id| self.open.get(id))
            .collect()
    }

    /// Every accepted operation, in no particular order.
    pub fn committed(&self) -> impl Iterator<Item = &CommittedRecord> {
        self.ops
            .values()
            .filter_map(|state| state.committed.as_ref())
    }

    /// How many head-log sequences in `after+1 ..= upto` the ledger accounts for, by a
    /// `committed` record or an unresolved intent. Equal to `upto - after` when nothing in that
    /// range is missing from the ledger (forged records are cross-checked against the root
    /// separately).
    pub fn covered(&self, after: u64, upto: u64) -> u64 {
        let range = after.saturating_add(1)..=upto;
        let sequences: std::collections::BTreeSet<u64> = self
            .committed()
            .map(|record| record.sequence)
            .chain(self.open.values().map(|intent| intent.sequence))
            .filter(|sequence| range.contains(sequence))
            .collect();
        u64::try_from(sequences.len()).unwrap_or(u64::MAX)
    }

    /// What the reader ignored.
    pub const fn diagnostics(&self) -> LedgerDiagnostics {
        self.diagnostics
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use beamcraft_model::digest::Sha256;

    fn id(n: u32) -> Uuid7 {
        Uuid7::parse(&format!("0199e7a0-0000-7000-8000-{n:012x}")).unwrap()
    }
    fn digest(text: &str) -> ContentDigest {
        Sha256::digest(text.as_bytes()).unwrap()
    }
    fn intent(op: u32, content: &str) -> Intent {
        Intent {
            op: id(op),
            project: id(1),
            revision: id(100 + op),
            sequence: u64::from(op),
            base: None,
            fingerprint: digest(content),
            created: 0,
            attempt: digest("attempt"),
            blobs: vec![digest("blob")],
        }
    }
    fn committed(op: u32) -> CommittedRecord {
        CommittedRecord {
            op: id(op),
            revision: id(100 + op),
            sequence: u64::from(op),
            manifest: digest("manifest"),
        }
    }
    fn join(records: &[Record]) -> Vec<u8> {
        records.iter().flat_map(|r| r.encode().0).collect()
    }

    #[test]
    fn states_follow_file_order() {
        let first = Record::Intent(intent(1, "a"));
        let first_id = first.encode().1;
        let scan = Scan::parse(&join(&[
            Record::Genesis { head: 0 },
            first,
            Record::Intent(intent(2, "b")),
        ]))
        .unwrap();
        assert_eq!(scan.genesis_head(), Some(0));
        assert_eq!(scan.status(&id(1)), OpStatus::Unresolved);
        assert_eq!(scan.status(&id(3)), OpStatus::Unseen);
        assert!(scan.is_open(&first_id));
        assert_eq!(scan.open_intents().count(), 2);

        let closed = Scan::parse(&join(&[
            Record::Intent(intent(1, "a")),
            Record::Aborted {
                op: id(1),
                intent: Record::Intent(intent(1, "a")).encode().1,
                reason: AbortReason::Cancelled,
            },
            Record::Intent(intent(2, "b")),
            Record::Committed(committed(2)),
        ]))
        .unwrap();
        assert_eq!(
            closed.status(&id(1)),
            OpStatus::Aborted(AbortReason::Cancelled)
        );
        assert_eq!(closed.status(&id(2)), OpStatus::Committed(committed(2)));
        assert_eq!(closed.open_intents().count(), 0);
        assert_eq!(closed.fingerprint(&id(2)), Some(digest("b")));
        assert_eq!(closed.committed().count(), 1);
    }

    #[test]
    fn a_retry_after_an_abort_is_open_again_and_a_stale_abort_does_not_close_it() {
        let a1 = Record::Intent(intent(1, "a"));
        let mut retry = intent(1, "a");
        retry.created = 9;
        let a2 = Record::Intent(retry);
        let (id1, id2) = (a1.encode().1, a2.encode().1);
        let scan = Scan::parse(&join(&[
            a1,
            Record::Aborted {
                op: id(1),
                intent: id1,
                reason: AbortReason::Recovered,
            },
            a2,
            // A second, late abort of the first attempt must not close the retry.
            Record::Aborted {
                op: id(1),
                intent: id1,
                reason: AbortReason::Recovered,
            },
        ]))
        .unwrap();
        assert_eq!(scan.status(&id(1)), OpStatus::Unresolved);
        assert!(scan.is_open(&id2) && !scan.is_open(&id1));
    }

    #[test]
    fn a_committed_record_wins_over_a_late_abort_and_resolves_every_attempt() {
        let a = Record::Intent(intent(1, "a"));
        let aid = a.encode().1;
        let scan = Scan::parse(&join(&[
            a,
            Record::Committed(committed(1)),
            Record::Aborted {
                op: id(1),
                intent: aid,
                reason: AbortReason::Recovered,
            },
            Record::Committed(committed(1)),
            Record::Intent(intent(1, "a")),
        ]))
        .unwrap();
        assert_eq!(scan.status(&id(1)), OpStatus::Committed(committed(1)));
        assert_eq!(scan.open_intents().count(), 0);
    }

    #[test]
    fn two_different_accepted_records_for_one_op_are_inconsistent() {
        let mut other = committed(1);
        other.manifest = digest("other manifest");
        let bytes = join(&[Record::Committed(committed(1)), Record::Committed(other)]);
        assert_eq!(Scan::parse(&bytes).unwrap_err(), LedgerError::Inconsistent);
    }

    #[test]
    fn a_torn_tail_at_every_byte_is_ignored_and_never_trusted() {
        let head = join(&[Record::Genesis { head: 0 }, Record::Intent(intent(1, "a"))]);
        let last = Record::Committed(committed(1)).encode().0;
        for cut in 0..last.len() {
            let mut bytes = head.clone();
            bytes.extend_from_slice(&last[..cut]);
            let scan = Scan::parse(&bytes).unwrap();
            assert_eq!(scan.status(&id(1)), OpStatus::Unresolved, "cut {cut}");
            assert_eq!(scan.diagnostics().torn_tail, cut > 0, "cut {cut}");
            assert_eq!(scan.diagnostics().records, 2);
        }
        let mut whole = head;
        whole.extend_from_slice(&last);
        let scan = Scan::parse(&whole).unwrap();
        assert_eq!(scan.status(&id(1)), OpStatus::Committed(committed(1)));
        assert!(!scan.diagnostics().torn_tail);
    }

    #[test]
    fn garbage_lines_are_counted_and_skipped_while_valid_neighbours_still_count() {
        let mut bytes = join(&[Record::Intent(intent(1, "a"))]);
        bytes.extend_from_slice(b"not a record\n\n\xff\xfe\n");
        bytes.extend_from_slice(&Record::Committed(committed(1)).encode().0);
        let scan = Scan::parse(&bytes).unwrap();
        assert_eq!(scan.diagnostics().damaged_lines, 2);
        assert_eq!(scan.diagnostics().records, 2);
        assert_eq!(scan.status(&id(1)), OpStatus::Committed(committed(1)));
        assert_eq!(Scan::parse(b"").unwrap().genesis_head(), None);
        assert_eq!(Scan::parse(b"\n\n").unwrap().diagnostics().records, 0);
    }

    #[test]
    fn the_first_valid_genesis_wins() {
        let scan = Scan::parse(&join(&[
            Record::Genesis { head: 4 },
            Record::Genesis { head: 9 },
        ]))
        .unwrap();
        assert_eq!(scan.genesis_head(), Some(4));
    }

    #[test]
    fn line_and_open_intent_caps_are_enforced() {
        let blanks = vec![b'\n'; MAX_LEDGER_RECORDS + LEDGER_HEADROOM_RECORDS + 1];
        assert_eq!(
            Scan::parse(&blanks).unwrap_err(),
            LedgerError::LimitExceeded
        );
        let mut bytes = Vec::new();
        for n in 0..=u32::try_from(MAX_UNRESOLVED_INTENTS + LEDGER_HEADROOM_INTENTS).unwrap() {
            bytes.extend(Record::Intent(intent(n + 1, "a")).encode().0);
        }
        assert_eq!(Scan::parse(&bytes).unwrap_err(), LedgerError::LimitExceeded);
    }
}
