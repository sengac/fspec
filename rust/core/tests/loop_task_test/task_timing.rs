//! Feature: spec/features/per-entry-spawned-task-for-sub-minute-loop-intervals.feature
use super::*;

// =============================================================================
// Scenario: Loop fires at exact sub-minute interval
// =============================================================================
//
// The NEW behavior: when a loop is registered, LoopStore spawns a tokio task
// that sleeps for exactly the configured interval and fires the callback.
// The prompt should fire after ~2 seconds, NOT after 30 seconds.

#[tokio::test(start_paused = true)] // TEST-042: virtual time
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
        "Prompt should have fired at least once after 2.5s (got {count_after_first} fires)"
    );

    // @step And the prompt continues firing every 5 seconds thereafter
    // Sleep another 2.5 seconds (total ~5s) — should have fired at least twice.
    tokio::time::sleep(tokio::time::Duration::from_millis(2500)).await;
    let count_after_second = fire_count.load(Ordering::SeqCst);
    assert!(
        count_after_second >= 2,
        "Prompt should have fired at least twice after 5s (got {count_after_second} fires)"
    );

    // Cleanup
    store.cancel("fire-test").await;
}
// =============================================================================
// Scenario: Minimum interval is 1 second
// =============================================================================

#[tokio::test(start_paused = true)] // TEST-042: virtual time
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
        "1-second loop should fire at least 2 times in 2.5s (got {count} fires)"
    );

    // Also verify: sub-1-second intervals should be rejected.
    // An entry with interval_seconds=0 should be rejected by try_register_with_task.
    let bad_entry = make_entry("sub-second", session_id, 0);
    let noop: OnFireFn = Arc::new(|_prompt: String| {});
    let result: Result<(), String> = store.try_register_with_task(bad_entry, noop).await;
    assert!(result.is_err(), "Interval of 0 seconds should be rejected");

    store.cancel("min-interval").await;
}
// =============================================================================
// Scenario: Cancel aborts the spawned task immediately
// =============================================================================

#[tokio::test(start_paused = true)] // TEST-042: virtual time
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
        "No additional prompts should fire after cancel (before={count_at_cancel}, after={count_after_cancel})"
    );
}
