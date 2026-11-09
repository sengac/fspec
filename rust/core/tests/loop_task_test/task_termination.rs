//! Feature: spec/features/per-entry-spawned-task-for-sub-minute-loop-intervals.feature
use super::*;

// =============================================================================
// Scenario: Skip firing when session is busy
// =============================================================================

#[tokio::test(start_paused = true)] // TEST-042: virtual time
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
        "Prompt should not fire while session is busy (got {count_while_busy} fires)"
    );

    // @step And the loop retries after the next interval
    // Now mark session as idle — prompt should start firing.
    is_idle.store(true, Ordering::SeqCst);
    tokio::time::sleep(tokio::time::Duration::from_millis(2500)).await;
    let count_after_idle = fire_count.load(Ordering::SeqCst);
    assert!(
        count_after_idle >= 1,
        "Prompt should fire after session becomes idle (got {count_after_idle} fires)"
    );

    store.cancel("busy-test").await;
}
// =============================================================================
// Scenario: Auto-terminate when session is destroyed
// =============================================================================

#[tokio::test(start_paused = true)] // TEST-042: virtual time
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
        "No prompts should fire after session destroy (before={count_at_destroy}, after={count_after})"
    );
}
// =============================================================================
// Scenario: Cron engine tick is unaffected
// =============================================================================
//
// After SCHED-013, evaluate_and_run() should NOT call evaluate_and_fire_loops().
// Loops are fully self-managed via per-entry spawned tasks. The cron 30s tick
// continues unchanged for cron schedules only.

#[tokio::test(start_paused = true)] // TEST-042: virtual time
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

#[tokio::test(start_paused = true)] // TEST-042: virtual time
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
