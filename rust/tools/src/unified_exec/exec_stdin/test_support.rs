//! Shared fixtures for the exec_stdin test modules (tests.rs,
//! clock_and_emit_tests.rs, fault_reporting_tests.rs). TEST_LOCK serialises
//! every test that touches the process-global callback and cooldown maps.

use super::*;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub(super) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(super) type TestResult = Result<(), String>;

pub(super) fn lock_tests() -> Result<std::sync::MutexGuard<'static, ()>, String> {
    TEST_LOCK.lock().map_err(|error| error.to_string())
}

pub(super) fn captured_len(
    captured: &Mutex<Vec<Option<ExecStdinRequest>>>,
) -> Result<usize, String> {
    Ok(captured.lock().map_err(|error| error.to_string())?.len())
}

/// A capturing callback for `session_id`; returns what it captures.
pub(super) fn capture(session_id: Uuid) -> Arc<Mutex<Vec<Option<ExecStdinRequest>>>> {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&captured);
    set_exec_stdin_request_callback(
        session_id,
        Some(Arc::new(move |req: Option<ExecStdinRequest>| {
            if let Ok(mut events) = sink.lock() {
                events.push(req);
            }
        })),
    );
    captured
}

pub(super) fn request(exec: &str, command: &str) -> ExecStdinRequest {
    ExecStdinRequest {
        exec_session_id: exec.to_string(),
        command: command.to_string(),
        quiet_seconds: 3,
        ts_ms: 1,
    }
}

/// `Instant::now()` minus `secs` (the monotonic clock may be too young).
pub(super) fn ago(secs: u64) -> Result<Instant, String> {
    Instant::now()
        .checked_sub(Duration::from_secs(secs))
        .ok_or_else(|| format!("the monotonic clock is younger than {secs}s"))
}
