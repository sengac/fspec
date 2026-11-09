//! Feature: spec/features/per-entry-spawned-task-for-sub-minute-loop-intervals.feature
//!
//! Tests for Per-Entry Spawned Task for Sub-Minute Loop Intervals (SCHED-013).
//! Validates that each /loop entry spawns its own tokio task with exact interval
//! timing, cancellation via JoinHandle::abort, busy-session skip policy,
//! auto-termination on session destroy, cron engine decoupling, and expiry cleanup.
//!
//! These tests exercise the NEW LoopStore API where:
//!   - `new_local()` creates a non-singleton instance for testing
//!   - `register_with_task()` spawns a per-entry tokio task (replaces passive `register()`)
//!   - `try_register_with_task()` validates interval >= 1s before spawning
//!   - `register_with_task_and_idle_check()` adds a session-idle gate to the task loop
//!   - `has_active_task()` checks if a JoinHandle is alive for a given loop_id
//!   - `cancel()` aborts the JoinHandle (existing method, upgraded behavior)
//!   - `remove_for_session()` aborts all JoinHandles for a session (existing, upgraded)
//!
//! All tests MUST FAIL in the red phase because these methods do not exist yet.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{Duration, Utc};
use codelet_core::loops::{LoopEntry, LoopStore};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use uuid::Uuid;

#[path = "loop_task_test/task_termination.rs"]
mod task_termination;
#[path = "loop_task_test/task_timing.rs"]
mod task_timing;

/// Callback type for when a loop fires its prompt.
type OnFireFn = Arc<dyn Fn(String) + Send + Sync + 'static>;

/// Callback type for checking if a session is idle.
/// Returns a pinned future that resolves to bool.
type IdleCheckFn =
    Arc<dyn Fn(Uuid) -> Pin<Box<dyn Future<Output = bool> + Send>> + Send + Sync + 'static>;

/// Helper: create a LoopEntry with sensible defaults for testing.
fn make_entry(id: &str, session: Uuid, interval_sec: u32) -> LoopEntry {
    let now = Utc::now();
    LoopEntry {
        id: id.to_string(),
        session_id: session,
        prompt: format!("check {id}"),
        interval_seconds: interval_sec,
        created_at: now,
        expires_at: now + Duration::hours(1),
        last_run_at: None,
    }
}
