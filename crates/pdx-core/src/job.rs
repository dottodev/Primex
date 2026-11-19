//! Cancellable background job model shared by Rust workers and the JNI bridge.

use crate::error::{PdxError, Result};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId(pub u64);

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "job-{}", self.0)
    }
}

#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    pub fn check(&self) -> Result<()> {
        if self.is_cancelled() {
            Err(PdxError::cancelled("job cancelled"))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
pub struct Progress {
    pub job: JobId,
    pub done: u64,
    pub total: u64,
    pub message: String,
}

impl Progress {
    pub fn fraction(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.done as f32 / self.total as f32).clamp(0.0, 1.0)
        }
    }
}

#[derive(Debug)]
pub struct JobSequencer {
    next: AtomicU64,
}

impl Default for JobSequencer {
    fn default() -> Self {
        Self {
            next: AtomicU64::new(1),
        }
    }
}

impl JobSequencer {
    pub fn next(&self) -> JobId {
        JobId(self.next.fetch_add(1, Ordering::SeqCst))
    }
}

pub trait Job: Send {
    fn id(&self) -> JobId;
    fn description(&self) -> &str;
    fn run(&self, cancel: &CancelToken, report: &dyn Fn(Progress)) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_token_stops_job() {
        let token = CancelToken::new();
        assert!(token.check().is_ok());
        token.cancel();
        assert_eq!(
            token.check().unwrap_err().kind,
            crate::error::ErrorKind::Cancelled
        );
    }

    #[test]
    fn sequencer_is_monotonic() {
        let seq = JobSequencer::default();
        assert_eq!(seq.next(), JobId(1));
        assert_eq!(seq.next(), JobId(2));
    }

    #[test]
    fn progress_fraction_is_clamped() {
        let p = Progress {
            job: JobId(1),
            done: 5,
            total: 10,
            message: String::new(),
        };
        assert!((p.fraction() - 0.5).abs() < f32::EPSILON);
    }
}
