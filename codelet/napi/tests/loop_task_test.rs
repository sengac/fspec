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

use chrono::{Duration, Utc};
use codelet_napi::scheduler::loop_store::{LoopEntry, LoopStore};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

/// Callback type for when a loop fires its prompt.
type OnFireFn = Arc<dyn Fn(String) + Send + Sync + 'static>;

/// Callback type for checking if a session is idle.
/// Returns a pinned future that resolves to bool.
type IdleCheckFn = Arc<
    dyn Fn(Uuid) -> Pin<Box<dyn Future<Output = bool> + Send>> + Send + Sync + 'static,
>;

/// Helper: create a LoopEntry with sensible defaults for testing.
fn make_entry(id: &str, session: Uuid, interval_sec: u32) -> LoopEntry {
    let now = Utc::now();
    LoopEntry {
        id: id.to_string(),
        session_id: session,
        prompt: format!("check {}", id),
        interval_seconds: interval_sec,
        created_at: now,
        expires_at: now + Duration::hours(1),
        last_run_at: None,
    }
}

// =============================================================================
// Scenario: Loop fires at exact sub-minute interval
// =============================================================================
//
// The NEW behavior: when a loop is registered, LoopStore spawns a tokio task
// that sleeps for exactly the configured interval and fires the callback.
// The prompt should fire after ~2 seconds, NOT after 30 seconds.

#[tokio::test]
async fn test_loop_fires_at_exact_sub_minute_interval() {
    // @step Given a session is active and idle
    let session_id = Uuid::new_v4();
    let fire_count = Arc::new(AtomicU32::new(0));

    // @step When the user registers a loop with a 5-second interval
    // (Using 2-second interval for faster test execution.)
    let entry = make_entry("fire-test", session_id, 2);

    // NEW API: new_local() creates a non-singleton LoopStore for testing.
    // register_with_task() spawns a tokio task that fires `on_fire` every interval.
    let store = LoopStore::new_local();
    let fc = fire_count.clone();
    let on_fire: OnFireFn = Arc::new(move |_prompt: String| {
        fc.fetch_add(1, Ordering::SeqCst);
    });

    store.register_with_task(entry, on_fire).await;

    // @step Then the loop task spawns immediately
    assert!(
        !store.is_empty().await,
        "Store should have the entry after registration"
    );

    // @step And the prompt fires after exactly 5 seconds
    // (2-second interval: sleep 2.5s to give it time to fire once.)
    tokio::time::sleep(tokio::time::Duration::from_millis(2500)).await;
    let count_after_first = fire_count.load(Ordering::SeqCst);
    assert!(
        count_after_first >= 1,
        "Prompt should have fired at least once after 2.5s (got {} fires)",
        count_after_first
    );

    // @step And the prompt continues firing every 5 seconds thereafter
    // Sleep another 2.5 seconds (total ~5s) — should have fired at least twice.
    tokio::time::sleep(tokio::time::Duration::from_millis(2500)).await;
    let count_after_second = fire_count.load(Ordering::SeqCst);
    assert!(
        count_after_second >= 2,
        "Prompt should have fired at least twice after 5s (got {} fires)",
        count_after_second
    );

    // Cleanup
    store.cancel("fire-test").await;
}

// =============================================================================
// Scenario: Minimum interval is 1 second
// =============================================================================

#[tokio::test]
async fn test_minimum_interval_is_1_second() {
    // @step Given a session is active and idle
    let session_id = Uuid::new_v4();
    let fire_count = Arc::new(AtomicU32::new(0));

    // @step When the user registers a loop with a 1-second interval
    let entry = make_entry("min-interval", session_id, 1);

    let store = LoopStore::new_local();
    let fc = fire_count.clone();
    let on_fire: OnFireFn = Arc::new(move |_prompt: String| {
        fc.fetch_add(1, Ordering::SeqCst);
    });

    store.register_with_task(entry, on_fire).await;

    // @step Then the prompt fires every 1 second
    // Wait 2.5 seconds — should have fired at least 2 times.
    tokio::time::sleep(tokio::time::Duration::from_millis(2500)).await;
    let count = fire_count.load(Ordering::SeqCst);
    assert!(
        count >= 2,
        "1-second loop should fire at least 2 times in 2.5s (got {} fires)",
        count
    );

    // Also verify: sub-1-second intervals should be rejected.
    // An entry with interval_seconds=0 should be rejected by try_register_with_task.
    let bad_entry = make_entry("sub-second", session_id, 0);
    let noop: OnFireFn = Arc::new(|_prompt: String| {});
    let result: Result<(), String> = store.try_register_with_task(bad_entry, noop).await;
    assert!(
        result.is_err(),
        "Interval of 0 seconds should be rejected"
    );

    store.cancel("min-interval").await;
}

// =============================================================================
// Scenario: Cancel aborts the spawned task immediately
// =============================================================================

#[tokio::test]
async fn test_cancel_aborts_spawned_task() {
    // @step Given a session has an active loop
    let session_id = Uuid::new_v4();
    let fire_count = Arc::new(AtomicU32::new(0));
    let entry = make_entry("cancel-test", session_id, 1);

    let store = LoopStore::new_local();
    let fc = fire_count.clone();
    let on_fire: OnFireFn = Arc::new(move |_prompt: String| {
        fc.fetch_add(1, Ordering::SeqCst);
    });

    store.register_with_task(entry, on_fire).await;

    // Wait for at least one firing
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    let count_before = fire_count.load(Ordering::SeqCst);
    assert!(
        count_before >= 1,
        "Should have fired at least once before cancel"
    );

    // @step When the user cancels the loop
    let cancelled = store.cancel("cancel-test").await;

    // @step Then the loop task is aborted via JoinHandle
    assert!(cancelled, "cancel() should return true for existing loop");
    assert!(store.is_empty().await, "Store should be empty after cancel");

    // @step And no further prompts fire for that loop
    let count_at_cancel = fire_count.load(Ordering::SeqCst);
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
    let count_after_cancel = fire_count.load(Ordering::SeqCst);
    assert_eq!(
        count_at_cancel, count_after_cancel,
        "No additional prompts should fire after cancel (before={}, after={})",
        count_at_cancel, count_after_cancel
    );
}

// =============================================================================
// Scenario: Skip firing when session is busy
// =============================================================================

#[tokio::test]
async fn test_skip_firing_when_session_is_busy() {
    // @step Given a session has an active loop with a 5-second interval
    let session_id = Uuid::new_v4();
    let fire_count = Arc::new(AtomicU32::new(0));
    // Use 1-second interval for faster test
    let entry = make_entry("busy-test", session_id, 1);

    // @step And the session is currently busy
    // The idle flag starts as false (busy). The task should check this before firing.
    let is_idle = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let store = LoopStore::new_local();
    let fc = fire_count.clone();
    let on_fire: OnFireFn = Arc::new(move |_prompt: String| {
        fc.fetch_add(1, Ordering::SeqCst);
    });
    let idle_flag = is_idle.clone();
    let idle_check: IdleCheckFn = Arc::new(move |_session_id: Uuid| {
        let flag = idle_flag.clone();
        Box::pin(async move { flag.load(Ordering::SeqCst) })
    });

    store
        .register_with_task_and_idle_check(entry, on_fire, idle_check)
        .await;

    // @step When the loop interval elapses
    tokio::time::sleep(tokio::time::Duration::from_millis(2500)).await;

    // @step Then the prompt is not sent
    let count_while_busy = fire_count.load(Ordering::SeqCst);
    assert_eq!(
        count_while_busy, 0,
        "Prompt should not fire while session is busy (got {} fires)",
        count_while_busy
    );

    // @step And the loop retries after the next interval
    // Now mark session as idle — prompt should start firing.
    is_idle.store(true, Ordering::SeqCst);
    tokio::time::sleep(tokio::time::Duration::from_millis(2500)).await;
    let count_after_idle = fire_count.load(Ordering::SeqCst);
    assert!(
        count_after_idle >= 1,
        "Prompt should fire after session becomes idle (got {} fires)",
        count_after_idle
    );

    store.cancel("busy-test").await;
}

// =============================================================================
// Scenario: Auto-terminate when session is destroyed
// =============================================================================

#[tokio::test]
async fn test_auto_terminate_when_session_destroyed() {
    // @step Given a session has an active loop
    let session_id = Uuid::new_v4();
    let fire_count = Arc::new(AtomicU32::new(0));
    let entry = make_entry("destroy-test", session_id, 1);

    let store = LoopStore::new_local();
    let fc = fire_count.clone();
    let on_fire: OnFireFn = Arc::new(move |_prompt: String| {
        fc.fetch_add(1, Ordering::SeqCst);
    });

    store.register_with_task(entry, on_fire).await;

    // Verify it's firing
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    assert!(
        fire_count.load(Ordering::SeqCst) >= 1,
        "Should fire at least once before session destroy"
    );

    // @step When the session is destroyed
    let removed = store.remove_for_session(session_id).await;

    // @step Then the loop task self-terminates
    assert_eq!(removed, 1, "Should remove exactly 1 loop");

    // @step And no orphaned tasks remain in the LoopStore
    assert!(
        store.is_empty().await,
        "Store should be empty after session destroy"
    );

    // Verify no more firings (JoinHandle was aborted)
    let count_at_destroy = fire_count.load(Ordering::SeqCst);
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
    let count_after = fire_count.load(Ordering::SeqCst);
    assert_eq!(
        count_at_destroy, count_after,
        "No prompts should fire after session destroy (before={}, after={})",
        count_at_destroy, count_after
    );
}

// =============================================================================
// Scenario: Cron engine tick is unaffected
// =============================================================================
//
// After SCHED-013, evaluate_and_run() should NOT call evaluate_and_fire_loops().
// Loops are fully self-managed via per-entry spawned tasks. The cron 30s tick
// continues unchanged for cron schedules only.

#[tokio::test]
async fn test_cron_engine_tick_does_not_evaluate_loops() {
    // @step Given the cron scheduler is running with a 30-second tick
    // @step And loop entries exist with sub-minute intervals
    let session_id = Uuid::new_v4();
    let fire_count = Arc::new(AtomicU32::new(0));

    // Register a loop that is immediately due (created in the past)
    let mut entry = make_entry("cron-decoupled", session_id, 1);
    entry.created_at = Utc::now() - Duration::seconds(10);

    let store = LoopStore::new_local();
    let fc = fire_count.clone();
    let on_fire: OnFireFn = Arc::new(move |_prompt: String| {
        fc.fetch_add(1, Ordering::SeqCst);
    });

    store.register_with_task(entry, on_fire).await;

    // @step When the cron tick fires
    // The key structural contract: LoopStore manages its own tasks.
    // evaluate_and_fire_loops() should be removed from engine.rs.
    // We verify this by checking that the store tracks JoinHandles —
    // i.e., has_active_task() returns true for registered loops.

    // @step Then cron schedules are evaluated as before
    // (Covered by existing scheduler_engine_test.rs — no regression.)

    // @step And loop entries are not evaluated by the cron tick
    // The store should have an active spawned task for this entry,
    // proving it's self-managed rather than polled by the engine.
    let has_task = store.has_active_task("cron-decoupled").await;
    assert!(
        has_task,
        "Loop entry should have an active spawned task (not driven by engine tick)"
    );

    store.cancel("cron-decoupled").await;
}

// =============================================================================
// Scenario: Expired loop auto-terminates
// =============================================================================

#[tokio::test]
async fn test_expired_loop_auto_terminates() {
    // @step Given a session has an active loop that has reached its expiry time
    let session_id = Uuid::new_v4();
    let fire_count = Arc::new(AtomicU32::new(0));

    // Create an entry that expires very soon (in 2 seconds)
    let now = Utc::now();
    let mut entry = make_entry("expiry-test", session_id, 1);
    entry.expires_at = now + Duration::seconds(2);

    let store = LoopStore::new_local();
    let fc = fire_count.clone();
    let on_fire: OnFireFn = Arc::new(move |_prompt: String| {
        fc.fetch_add(1, Ordering::SeqCst);
    });

    store.register_with_task(entry, on_fire).await;

    // It should fire at least once (interval=1s, expiry in 2s)
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    let count_before_expiry = fire_count.load(Ordering::SeqCst);
    assert!(
        count_before_expiry >= 1,
        "Should fire at least once before expiry"
    );

    // @step When the loop task checks expiry before sleeping
    // Wait past the expiry time (total ~3.5s from start, expiry at 2s)
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    // @step Then the loop task terminates
    // @step And the entry is removed from the LoopStore
    // The spawned task should detect expiry and self-remove from the store.
    let still_exists = store.has_active_task("expiry-test").await;
    assert!(
        !still_exists,
        "Expired loop task should have self-terminated and been removed from store"
    );

    let entries: Vec<LoopEntry> = store.list_for_session(session_id).await;
    assert!(
        entries.is_empty(),
        "Expired entry should be removed from the store"
    );

    // No more firings after expiry
    let count_at_expiry = fire_count.load(Ordering::SeqCst);
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    let count_after = fire_count.load(Ordering::SeqCst);
    assert_eq!(
        count_at_expiry, count_after,
        "No prompts should fire after expiry"
    );
}
