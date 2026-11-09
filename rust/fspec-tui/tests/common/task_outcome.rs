//! `handle.await.completed_or_cancelled()?` — a finished tokio task as a
//! test outcome (BUG-366). A task that completed, or that the code under
//! test cancelled (the App aborts superseded tasks such as debounce timers),
//! is fine; a task that panicked fails the test.
//!
//! BUG-487: this lives with the fspec-tui tests, its only users, so
//! codelet-test-helpers keeps no tokio dependency (RPC-067).

use codelet_test_helpers::{Failure, OrFail};

pub trait TaskOutcome {
    fn completed_or_cancelled(self) -> Result<(), Failure>;
}

impl<T> TaskOutcome for Result<T, tokio::task::JoinError> {
    fn completed_or_cancelled(self) -> Result<(), Failure> {
        match self {
            Ok(_) => Ok(()),
            Err(error) if error.is_cancelled() => Ok(()),
            Err(error) => Err::<(), _>(error).or_fail("task panicked"),
        }
    }
}
