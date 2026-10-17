// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Production source of fresh UUIDv7 identities: the system clock plus per-process random bytes
//! feeding the pure [`beamcraft_model::id::UuidV7Generator`].
//!
//! The model generator never shortens or repeats an identity inside one process, even within one
//! millisecond (a 12-bit counter plus a logical clock). Across processes uniqueness rests on 62
//! random bits per identity and on the actual existence check that F04 requires at creation
//! (`DUPLICATE_ID`, and the U03 index at commit).
//!
//! # Entropy limits
//!
//! The bytes come from `std::hash::RandomState`, whose keys the standard library seeds from the
//! operating system's random source for each process, mixed with the clock and a counter. This
//! is adequate to avoid collisions; it is **not** documented by `std` as a cryptographic random
//! generator, and no identity here is a secret or a capability (UUID clock order gives neither
//! authority nor conflict priority). An injected source ([`IdSource::with_sources`]) replaces
//! both inputs, for tests and for a future audited entropy API; no dependency is added.

use beamcraft_model::id::{IdError, Uuid7, UuidV7Generator};
use std::hash::{BuildHasher, RandomState};
use std::time::{SystemTime, UNIX_EPOCH};

type Clock = Box<dyn FnMut() -> Option<u64> + Send>;
type Entropy = Box<dyn FnMut() -> [u8; 10] + Send>;

/// A stateful maker of fresh UUIDv7 identities. Identities from one source are strictly
/// increasing and complete (36 characters in canonical form).
pub struct IdSource {
    generator: UuidV7Generator,
    clock: Clock,
    entropy: Entropy,
}

impl IdSource {
    /// The system clock and per-process OS-seeded entropy.
    pub fn system() -> Self {
        let mut counter = 0u64;
        Self::with_sources(
            || {
                let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
                u64::try_from(elapsed.as_millis()).ok()
            },
            move || {
                counter = counter.wrapping_add(1);
                let nanos = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| d.subsec_nanos());
                let first = RandomState::new().hash_one((counter, nanos));
                let second = RandomState::new().hash_one(first ^ counter);
                let [a, b, c, d, e, f, g, h] = first.to_be_bytes();
                let [i, j, ..] = second.to_be_bytes();
                [a, b, c, d, e, f, g, h, i, j]
            },
        )
    }

    /// A source with an injected clock (Unix milliseconds, `None` if unavailable) and entropy.
    pub fn with_sources(
        clock: impl FnMut() -> Option<u64> + Send + 'static,
        entropy: impl FnMut() -> [u8; 10] + Send + 'static,
    ) -> Self {
        Self {
            generator: UuidV7Generator::new(),
            clock: Box::new(clock),
            entropy: Box::new(entropy),
        }
    }

    /// The next identity. Fails with [`IdError`] only if the clock is unavailable or exhausted.
    pub fn next_id(&mut self) -> Result<Uuid7, IdError> {
        let now = (self.clock)().ok_or(IdError)?;
        let entropy = (self.entropy)();
        self.generator.next(now, entropy)
    }
}

impl std::fmt::Debug for IdSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("IdSource")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn system_identities_are_complete_distinct_and_increasing() {
        let mut source = IdSource::system();
        let mut seen = BTreeSet::new();
        let mut last = None;
        for _ in 0..5000 {
            let id = source.next_id().unwrap();
            assert_eq!(id.to_string().len(), 36);
            assert!(Uuid7::parse(&id.to_string()).is_ok());
            assert!(seen.insert(id));
            if let Some(l) = last {
                assert!(l < id);
            }
            last = Some(id);
        }
    }

    #[test]
    fn two_sources_in_the_same_millisecond_do_not_collide() {
        let mut a = IdSource::system();
        let mut b = IdSource::system();
        let mut seen = BTreeSet::new();
        for _ in 0..2000 {
            assert!(seen.insert(a.next_id().unwrap()));
            assert!(seen.insert(b.next_id().unwrap()));
        }
    }

    #[test]
    fn an_unavailable_clock_is_an_error_not_a_guess() {
        let mut source = IdSource::with_sources(|| None, || [0; 10]);
        assert!(source.next_id().is_err());
    }

    #[test]
    fn injected_sources_make_a_reproducible_identity() {
        let mut source =
            IdSource::with_sources(|| Some(0x0123_4567_89ab), || [0, 5, 1, 2, 3, 4, 5, 6, 7, 8]);
        let id = source.next_id().unwrap();
        assert!(id.to_string().starts_with("01234567-89ab-7005-"));
    }
}
