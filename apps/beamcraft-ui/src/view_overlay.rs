// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The display of a view's filter overrides: which elements each filter matches on the store
//! head, found by the CLI's read-only `query-elements` (the V14 evaluator, the same one V17's
//! colour sets use), and the overlay a view draws with: the first override whose filter matches
//! an element colours it, makes it transparent or hides it.
//!
//! Display state only. The matches are kept per store head and dropped when the head changes;
//! nothing here writes the model, the snapshot or the view file.

use crate::colour_sets::{Overlay, WHITE};
use crate::stats::Stats;
use beamcraft_engine::ElementId;
use beamcraft_engine::views::{FilterOverride, ModelView, ViewBook, ViewFilter};
use beamcraft_json::{Json, Limits};
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

/// Most rules one evaluation runs, so a long list still answers in parts.
const BATCH: usize = 8;

/// What a filter matched on the head: its elements, or why it was refused.
pub type Matched = Result<Arc<BTreeSet<ElementId>>, String>;

/// The V14 rule of `filter` and the text it is keyed by.
pub fn rule_of(filter: &ViewFilter) -> (String, Json) {
    let rule = beamcraft_host::views::filter_rule(filter);
    let key = rule
        .encode(&Limits::DEFAULT)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_else(|_| filter.name.clone());
    (key, rule)
}

/// The tint an override gives what it matches; `None` hides it. A visible override without a
/// colour keeps the model colours, at its transparency.
pub fn tint_of(o: &FilterOverride) -> Option<[f32; 4]> {
    if !o.visible {
        return None;
    }
    let alpha = (1.0 - o.transparency.clamp(0.0, 1.0)) as f32;
    Some(match o.color {
        Some([r, g, b]) => [r as f32, g as f32, b as f32, alpha],
        None => [WHITE[0], WHITE[1], WHITE[2], alpha],
    })
}

struct Job {
    rx: Receiver<Vec<(String, Matched)>>,
    cancel: Arc<AtomicBool>,
}

/// The filters' matches on one store head, evaluated on a worker.
#[derive(Default)]
pub struct FilterMatches {
    token: String,
    found: BTreeMap<String, Matched>,
    job: Option<Job>,
    /// Bumped whenever `found` changes, so a view's overlay knows to rebuild.
    generation: u64,
}

impl FilterMatches {
    /// Forgets every match when the head on screen is no longer `token`.
    pub fn follow(&mut self, token: &str) {
        if self.token != token {
            if let Some(job) = self.job.take() {
                job.cancel.store(true, Ordering::Relaxed);
            }
            self.found.clear();
            token.clone_into(&mut self.token);
            self.generation = self.generation.wrapping_add(1);
        }
    }

    /// Forgets every match, so the next frame asks again (after a filter changed).
    pub fn clear(&mut self) {
        if let Some(job) = self.job.take() {
            job.cancel.store(true, Ordering::Relaxed);
        }
        self.found.clear();
        self.generation = self.generation.wrapping_add(1);
    }

    /// Records what the rule keyed `key` matched, evaluated elsewhere (the self-test's own
    /// model, which has no store to query).
    pub fn insert(&mut self, key: String, matched: Matched) {
        self.found.insert(key, matched);
        self.generation = self.generation.wrapping_add(1);
    }

    /// What the rule keyed `key` matched, once evaluated.
    pub fn get(&self, key: &str) -> Option<&Matched> {
        self.found.get(key)
    }

    /// The generation of the matches.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Whether an evaluation runs.
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    /// Starts evaluating the rules in `wanted` that have no match yet, unless one runs.
    pub fn request(
        &mut self,
        cli: &Path,
        store: &Path,
        stats: &Arc<Stats>,
        wanted: Vec<(String, Json)>,
    ) {
        if self.job.is_some() {
            return;
        }
        let mut missing: Vec<(String, Json)> = Vec::new();
        for (key, rule) in wanted {
            if !self.found.contains_key(&key) && !missing.iter().any(|(k, _)| *k == key) {
                missing.push((key, rule));
            }
        }
        missing.truncate(BATCH);
        if missing.is_empty() {
            return;
        }
        let (cli, store): (PathBuf, PathBuf) = (cli.to_owned(), store.to_owned());
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancel);
        let stats = Arc::clone(stats);
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("beamcraft-view-filters".into())
            .spawn(move || {
                let mut out = Vec::new();
                for (key, rule) in missing {
                    if flag.load(Ordering::Relaxed) {
                        return;
                    }
                    let matched =
                        crate::colour_sets::matching_ids(&cli, &store, &rule, &flag, &stats)
                            .map(|ids| Arc::new(ids.into_iter().collect::<BTreeSet<_>>()));
                    out.push((key, matched));
                }
                let _ = tx.send(out);
            });
        if spawned.is_ok() {
            self.job = Some(Job { rx, cancel });
        }
    }

    /// Takes a finished evaluation in; true when the matches changed.
    pub fn poll(&mut self) -> bool {
        let Some(job) = &self.job else {
            return false;
        };
        match job.rx.try_recv() {
            Ok(results) => {
                self.job = None;
                self.found.extend(results);
                self.generation = self.generation.wrapping_add(1);
                true
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                self.job = None;
                false
            }
        }
    }
}

/// What one view's overlay was built from, to know when to build it again.
pub fn overlay_key(
    view: &ModelView,
    book: &ViewBook,
    matches: &FilterMatches,
    base: Option<&Arc<Overlay>>,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    matches.generation().hash(&mut hasher);
    base.map(|b| Arc::as_ptr(b) as usize).hash(&mut hasher);
    for o in &view.overrides {
        o.to_spec().hash(&mut hasher);
        if let Some(filter) = book.filter(&o.filter) {
            rule_of(filter).0.hash(&mut hasher);
        }
    }
    hasher.finish()
}

/// A view's overlay, the rules it still waits for (key and rule), and the refused filters
/// (name and reason).
pub type Built = (Overlay, Vec<(String, Json)>, Vec<(String, String)>);

/// The overlay of `view`: its overrides in order over `base`, and the rules it still waits for
/// and the filters that were refused (by name, with the reason).
pub fn overlay_of(
    view: &ModelView,
    book: &ViewBook,
    matches: &FilterMatches,
    base: Option<&Overlay>,
) -> Built {
    let mut entries = Vec::new();
    let mut wanted = Vec::new();
    let mut refused = Vec::new();
    for o in &view.overrides {
        let Some(filter) = book.filter(&o.filter) else {
            continue;
        };
        let (key, rule) = rule_of(filter);
        match matches.get(&key) {
            Some(Ok(ids)) => entries.push((Arc::clone(ids), tint_of(o))),
            Some(Err(reason)) => refused.push((filter.name.clone(), reason.clone())),
            None => wanted.push((key, rule)),
        }
    }
    (Overlay::from_overrides(entries, base), wanted, refused)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use beamcraft_engine::filter::Operator;
    use beamcraft_engine::views::{FilterCondition, FilterField};
    use beamcraft_engine::work_plane::WorkPlane;
    use beamcraft_model::id::Uuid7;

    fn eid(n: u64) -> ElementId {
        ElementId::parse(&format!("019a7bf0-0000-7000-8000-{n:012}")).unwrap()
    }

    fn filter(name: &str, kind: &str) -> ViewFilter {
        ViewFilter::new(
            name,
            vec![FilterCondition::new(FilterField::Type, Operator::Eq, kind)],
        )
    }

    #[test]
    fn the_first_override_decides_and_hidden_elements_leave() {
        let mut book = ViewBook::new();
        book.save_filter(filter("Beams", "beam"), None).unwrap();
        book.save_filter(filter("Columns", "column"), None).unwrap();
        let id = Uuid7::parse("019a7bf0-0000-7000-8000-000000000001").unwrap();
        let mut view = ModelView::new(id, "Plan", WorkPlane::default(), 5000.0);
        view.overrides = vec![
            FilterOverride::parse_spec("Beams|show|1,0,0|0.5").unwrap(),
            FilterOverride::parse_spec("Columns|hide").unwrap(),
        ];
        book.insert(view.clone()).unwrap();
        let mut matches = FilterMatches::default();
        matches.follow("head-1");
        let (empty, wanted, _) = overlay_of(&view, &book, &matches, None);
        assert!(empty.is_empty());
        assert_eq!(wanted.len(), 2, "both rules are asked for");
        // Element 1 is a beam, 2 a column, 3 both (an overlapping rule): the beam rule wins.
        let beams: BTreeSet<_> = [eid(1), eid(3)].into_iter().collect();
        let columns: BTreeSet<_> = [eid(2), eid(3)].into_iter().collect();
        matches
            .found
            .insert(wanted[0].0.clone(), Ok(Arc::new(beams)));
        matches
            .found
            .insert(wanted[1].0.clone(), Err("refused".into()));
        let (partial, _, refused) = overlay_of(&view, &book, &matches, None);
        assert_eq!(refused, [("Columns".to_owned(), "refused".to_owned())]);
        assert_eq!(partial.tint_of(eid(1)), [1.0, 0.0, 0.0, 0.5]);
        matches
            .found
            .insert(wanted[1].0.clone(), Ok(Arc::new(columns)));
        let base = Overlay::from_rules(vec![(
            [eid(4), eid(2)].into_iter().collect(),
            [0.0, 1.0, 0.0, 1.0],
        )]);
        let (overlay, wanted, refused) = overlay_of(&view, &book, &matches, Some(&base));
        assert!(wanted.is_empty() && refused.is_empty());
        assert!(overlay.hides(eid(2)) && !overlay.hides(eid(3)));
        assert_eq!(overlay.tint_of(eid(3)), [1.0, 0.0, 0.0, 0.5]);
        assert_eq!(
            overlay.tint_of(eid(4)),
            [0.0, 1.0, 0.0, 1.0],
            "the base colours the rest"
        );
        // A new head forgets the matches.
        let before = matches.generation();
        matches.follow("head-2");
        assert!(matches.get(&rule_of(&filter("Beams", "beam")).0).is_none());
        assert_ne!(matches.generation(), before);
    }

    #[test]
    fn a_visible_override_without_colour_keeps_the_model_colours() {
        let o = FilterOverride::parse_spec("Beams|show|-|0.25").unwrap();
        assert_eq!(tint_of(&o), Some([1.0, 1.0, 1.0, 0.75]));
        assert_eq!(
            tint_of(&FilterOverride::parse_spec("Beams|hide").unwrap()),
            None
        );
    }
}
