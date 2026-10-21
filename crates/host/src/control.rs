// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The error, deadline and cancellation of a host operation started from the CLI.

use beamcraft_model::ErrorCode;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

/// A bounded, fixed safe host error; it never carries a file path or a submitted value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostError {
    /// The F04 code.
    pub code: ErrorCode,
    /// Whether the operation's deadline expired, which the CLI reports with its own exit code.
    pub deadline: bool,
}
impl HostError {
    /// Creates a failure with an F04 code.
    pub const fn logical(code: ErrorCode) -> Self {
        Self {
            code,
            deadline: false,
        }
    }
    /// The failure of an expired deadline: `BACKEND_UNAVAILABLE`, exit code 124.
    pub const fn deadline() -> Self {
        Self {
            code: ErrorCode::BackendUnavailable,
            deadline: true,
        }
    }
    /// Fixed safe text; does not include file paths or submitted values.
    pub const fn message(self) -> &'static str {
        "operation unavailable or rejected"
    }
    /// Stable CLI exit mapping.
    pub const fn exit_code(self) -> u8 {
        match self.code {
            ErrorCode::Cancelled => 130,
            ErrorCode::BackendUnavailable => {
                if self.deadline {
                    124
                } else {
                    3
                }
            }
            ErrorCode::OutcomeUnknown => 4,
            _ => 2,
        }
    }
}
impl From<beamcraft_json::Error> for HostError {
    fn from(e: beamcraft_json::Error) -> Self {
        Self::logical(e.code())
    }
}
impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message())
    }
}
impl std::error::Error for HostError {}

/// Explicit cooperative cancellation of a host operation.
#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    /// Requests cancellation. No acceptance or rollback is inferred from this flag.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    /// Whether the caller has requested cancellation.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
/// One deadline and cancellation token across a whole host operation.
pub struct Control {
    /// Monotonic absolute deadline.
    pub deadline: Instant,
    /// Cancellation token.
    pub cancellation: Cancellation,
}
impl Control {
    /// A 30 second default; an explicit timeout must lie within 1..=120000 ms.
    pub fn new(timeout_ms: Option<u64>, cancellation: Cancellation) -> Result<Self, HostError> {
        let timeout = timeout_ms.unwrap_or(30_000);
        if !(1..=120_000).contains(&timeout) {
            return Err(HostError::logical(ErrorCode::InvalidArgument));
        }
        Ok(Self {
            deadline: Instant::now() + Duration::from_millis(timeout),
            cancellation,
        })
    }
    /// Remaining whole milliseconds; never resets an enclosing deadline.
    pub fn remaining(&self) -> Result<u64, HostError> {
        if self.cancellation.is_cancelled() {
            return Err(HostError::logical(ErrorCode::Cancelled));
        }
        let ms = self
            .deadline
            .saturating_duration_since(Instant::now())
            .as_millis();
        if ms == 0 {
            return Err(HostError::deadline());
        }
        u64::try_from(ms.min(120_000)).map_err(|_| HostError::deadline())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_bounds_cancellation_and_exit_codes() {
        assert!(Control::new(Some(0), Cancellation::default()).is_err());
        assert!(Control::new(Some(120_001), Cancellation::default()).is_err());
        let cancellation = Cancellation::default();
        let control = Control::new(Some(120_000), cancellation.clone()).unwrap();
        assert!(control.remaining().is_ok_and(|ms| ms > 0));
        cancellation.cancel();
        assert_eq!(
            control.remaining(),
            Err(HostError::logical(ErrorCode::Cancelled))
        );
        let expired = Control {
            deadline: Instant::now(),
            cancellation: Cancellation::default(),
        };
        assert_eq!(expired.remaining(), Err(HostError::deadline()));
        assert_eq!(HostError::deadline().exit_code(), 124);
        assert_eq!(
            HostError::logical(ErrorCode::BackendUnavailable).exit_code(),
            3
        );
        assert_eq!(HostError::logical(ErrorCode::Cancelled).exit_code(), 130);
        assert_eq!(HostError::logical(ErrorCode::OutcomeUnknown).exit_code(), 4);
        assert_eq!(
            HostError::from(beamcraft_json::Error::Limit).code,
            ErrorCode::LimitExceeded
        );
    }
}
