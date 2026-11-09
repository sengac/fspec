//! RPC-415 — Live streaming dies permanently after first auto-reconnect.
//!
//! Feature: spec/features/reconnect-resubscribe-broadcast-streams.feature
//!
//! These integration tests pin the correctness behaviour that
//! `Action::Reconnected` must respawn the five broadcast subscriber tasks
//! (work_units / chunks / logs / status_changes / session_created) bound to
//! the NEW RPC client's receivers, after the old subscriber loops have all
//! exited on `RecvError::Closed`.
//!
//! The `MockBackend::disconnect_all()` / `reconnect_all()` helpers model
//! the transport supervisor dropping the old client (Closed) and swapping
//! in a brand-new client whose broadcast Senders are DISTINCT from the old
//! (dropped) ones — so a subscriber that re-subscribes after
//! `reconnect_all()` is genuinely bound to the new client's receivers.

use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};

mod common;
use common::MockBackend;

#[path = "reconnect_resubscribe_rpc415/support.rs"]
mod support;
use support::{collect_post_reconnect_events, sid, wait_for_action, wu};

/// The full set of broadcast subscriber streams (BUG-351: one shared total).
const SUBSCRIBER_STREAM_COUNT: usize = common::BOOTSTRAP_SUBSCRIBER_TASKS;

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Old subscriber loops are dead before reconnect
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn old_subscriber_loops_are_dead_before_reconnect() -> TestResult {
    // @step Given an App bootstrapped against a backend whose broadcast senders are then closed
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    app.dispatch(Action::SessionCreated(sid("s-1")));
    assert_eq!(
        app.subscriber_task_count(),
        SUBSCRIBER_STREAM_COUNT,
        "precondition: bootstrap spawns the full set of subscriber tasks"
    );

    // @step When the backend's broadcast senders are closed so every subscriber receiver observes RecvError::Closed
    mock.disconnect_all();

    // @step Then the original subscriber tasks have all exited and no live stream reaches the App
    // The tasks self-exit on Closed; the App's stored JoinHandles are now
    // finished. Pushing events after disconnect must NOT reach the bus:
    // there are no live senders and no live subscriber loops.
    // TEST-042: no waiting window. The old loops hold receivers of the
    // dropped senders (each ends on its next recv with Closed); nothing is
    // bound to the new sender, so the push reaches no receiver, and the bus
    // is read after it.
    mock.reconnect_all();
    assert_eq!(
        mock.work_units_receiver_count(),
        0,
        "no subscriber may be bound to the new client's work_units sender"
    );
    mock.push_work_units(vec![wu("AUTH-DEAD", "backlog")]);
    let reached = std::iter::from_fn(|| app.try_recv_action()).find(|a| matches!(a, Action::WorkUnitsLoaded(units) if units.iter().any(|u| u.id == "AUTH-DEAD")));
    assert!(
        reached.is_none(),
        "with the old loops dead and no respawn, no live work_units stream may reach the App"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Each broadcast stream delivers a post-reconnect event to the App
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn each_broadcast_stream_delivers_a_post_reconnect_event_to_the_app() -> TestResult {
    // @step Given an App bootstrapped against a backend whose subscriber tasks have exited after a simulated disconnect
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    // Seed the mock's in-memory work units so the RPC-011 re-bootstrap
    // refetch (list_work_units on Reconnected) carries a live snapshot.
    mock.seed_work_units(vec![wu("SEED", "backlog")]);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    mock.disconnect_all();

    // @step When the App dispatches Action::Reconnected and the backend then emits one event on each of the work_units, chunks, logs, status_changes and session_created streams
    // BUG-353: capture the refetch baseline BEFORE dispatching Reconnected.
    // On the 4-worker runtime the spawned refetch can call list_work_units
    // before the next statement runs, so a baseline read after dispatch
    // already includes it and the wait below never sees a rise.
    let baseline_list = mock.list_work_units_calls();
    mock.reconnect_all();
    app.dispatch(Action::Reconnected);
    // RPC-415 determinism: `Action::Reconnected` also spawns a one-shot
    // RPC-011 re-bootstrap (list_work_units refetch) whose WorkUnitsLoaded
    // lands on the same bus as the broadcast events below. Wait for the
    // refetch backend call to have run (polling the mock's call counter)
    // so the refetch snapshot is already queued — and drained by the next
    // bus sweep — BEFORE the broadcast events are pushed.
    //
    // TUI-113 R5: the create_session(None) half of the re-bootstrap is
    // superseded — the reconnect now re-ATTACHES the focused session via
    // the RPC-049 path; create only fires as the empty-daemon fallback.
    // The gate below therefore waits for the list_work_units refetch only.
    codelet_test_helpers::wait_until("the list_work_units refetch", || {
        std::future::ready(mock.list_work_units_calls() > baseline_list)
    })
    .await?;
    assert!(
        mock.list_work_units_calls() > baseline_list,
        "Reconnected must trigger the RPC-011 list_work_units refetch"
    );
    let events = collect_post_reconnect_events(&mut app, &mock).await;

    // @step Then the App receives a WorkUnitsLoaded action carrying the post-reconnect work_units update
    assert!(
        events.work_units.is_some(),
        "post-reconnect work_units update must reach the App"
    );

    // @step And the App receives a ChunkReceived action for the post-reconnect chunk
    assert!(
        events.chunk.is_some(),
        "post-reconnect chunk must reach the App"
    );

    // @step And the App receives a SessionStatusChanged action for the post-reconnect status change
    assert!(
        events.status.is_some(),
        "post-reconnect status change must reach the App"
    );

    // @step And the App receives a SessionCreated action for the post-reconnect session_created event
    assert!(
        events.created.is_some(),
        "post-reconnect session_created event must reach the App"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Respawn binds subscribers to the new client receivers
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn respawn_binds_subscribers_to_the_new_client_receivers() -> TestResult {
    // @step Given an App bootstrapped against a backend whose original subscriber tasks have exited after a simulated disconnect
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    mock.disconnect_all();
    // Swap in the NEW client's senders. Any receiver still bound to the OLD
    // (dropped) senders can never observe events pushed after this point.
    mock.reconnect_all();

    // @step When the App dispatches Action::Reconnected and the backend emits a work_units update from its current senders
    app.dispatch(Action::Reconnected);
    work_units_subscriber_bound(&mock).await?;
    mock.push_work_units(vec![wu("AUTH-NEWCLIENT", "backlog")]);

    // @step Then the respawned subscriber tasks are bound to the current receivers and deliver the update as a WorkUnitsLoaded action
    let action = wait_for_action(&mut app, |a| {
        matches!(a, Action::WorkUnitsLoaded(units) if units.iter().any(|u| u.id == "AUTH-NEWCLIENT"))
    })
    .await;
    assert!(
        action.is_some(),
        "only subscribers rebound to the NEW client's receivers can deliver this update"
    );

    // @step And the live subscriber task count returns to the full set of broadcast streams
    assert_eq!(
        app.subscriber_task_count(),
        SUBSCRIBER_STREAM_COUNT,
        "after respawn the live subscriber task count must equal the full stream set"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Flapping reconnects do not accumulate duplicate subscriber tasks
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn flapping_reconnects_do_not_accumulate_duplicate_subscriber_tasks() -> TestResult {
    // @step Given an App bootstrapped against a backend
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;

    // @step When the App dispatches Action::Reconnected twice in succession and the backend then emits a single work_units update
    mock.disconnect_all();
    mock.reconnect_all();
    app.dispatch(Action::Reconnected);
    work_units_subscriber_bound(&mock).await?;
    mock.disconnect_all();
    mock.reconnect_all();
    app.dispatch(Action::Reconnected);
    work_units_subscriber_bound(&mock).await?;
    mock.push_work_units(vec![wu("AUTH-ONCE", "backlog")]);

    // @step Then the live subscriber task count equals the full set of broadcast streams and does not grow with each reconnect
    assert_eq!(
        app.subscriber_task_count(),
        SUBSCRIBER_STREAM_COUNT,
        "flapping must not accumulate N x {SUBSCRIBER_STREAM_COUNT} subscriber tasks"
    );

    // @step And the App receives exactly one WorkUnitsLoaded action for that update with no duplicate delivery
    // TEST-042: wait for the delivery and for every bound receiver to have
    // taken the push; one bound receiver means no duplicate can follow.
    let mut once_count = 0usize;
    codelet_test_helpers::deadlock_guard("the AUTH-ONCE delivery", async {
        while once_count == 0 || mock.work_units_unreceived() > 0 {
            while let Some(action) = app.try_recv_action() {
                if let Action::WorkUnitsLoaded(units) = &action {
                    if units.iter().any(|u| u.id == "AUTH-ONCE") {
                        once_count += 1;
                    }
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    assert_eq!(
        mock.work_units_receiver_count(),
        1,
        "exactly one subscriber may be bound to the work_units stream"
    );
    once_count += std::iter::from_fn(|| app.try_recv_action())
        .filter(|action| matches!(action, Action::WorkUnitsLoaded(units) if units.iter().any(|u| u.id == "AUTH-ONCE")))
        .count();
    assert_eq!(
        once_count, 1,
        "each broadcast event must be delivered exactly once (no leaked duplicate subscribers)"
    );
    Ok(())
}

/// TEST-042: wait until the respawned work_units subscriber is bound to the
/// current sender, so a push cannot be dropped for want of a receiver.
async fn work_units_subscriber_bound(mock: &MockBackend) -> TestResult {
    codelet_test_helpers::wait_until("the work_units subscriber to bind", || {
        std::future::ready(mock.work_units_receiver_count() > 0)
    })
    .await?;
    Ok(())
}
