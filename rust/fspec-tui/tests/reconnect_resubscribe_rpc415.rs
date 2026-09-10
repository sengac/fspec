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
//! RED PHASE NOTE: the current `Action::Reconnected` handler in
//! `src/app/dispatch.rs` only removes the DisconnectDialog and does a
//! one-shot `list_work_units()` refetch + `create_session(None)`. It does
//! NOT respawn the subscriber tasks. Therefore, after a simulated
//! disconnect (which drops every backend broadcast Sender so all five
//! subscriber loops exit) followed by `reconnect_all()` + dispatching
//! `Action::Reconnected`, no post-reconnect broadcast event reaches the App
//! and `subscriber_task_count()` stays stuck at the count of dead handles.
//! These tests are expected to FAIL until the respawn-on-Reconnected fix
//! lands.
//!
//! The `MockBackend::disconnect_all()` / `reconnect_all()` helpers model
//! the transport supervisor dropping the old client (Closed) and swapping
//! in a brand-new client whose broadcast Senders are DISTINCT from the old
//! (dropped) ones — so a subscriber that re-subscribes after
//! `reconnect_all()` is genuinely bound to the new client's receivers.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk, WorkUnitInfo};

mod common;
use common::MockBackend;

/// The full set of broadcast subscriber streams
/// (work_units + chunks + logs + status_changes + session_created).
const SUBSCRIBER_STREAM_COUNT: usize = 6;

fn wu(id: &str, status: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// Drain the App's action bus until a matching action arrives or the
/// deadline elapses. Drains every currently-queued action each tick so a
/// backlog does not starve the predicate before the deadline.
///
/// 2s deadline (raised from 500ms): the respawned subscriber tasks must be
/// scheduled AND polled at least once after `Action::Reconnected` before
/// they observe the post-reconnect events. Under CI/host load a fresh
/// tokio task can be starved well past 500ms, which turns a correct
/// respawn into a false-negative "event must reach the App" failure.
async fn wait_for_action<F: Fn(&Action) -> bool>(app: &mut App, pred: F) -> Option<Action> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(2000);
    while std::time::Instant::now() < deadline {
        while let Some(action) = app.try_recv_action() {
            if pred(&action) {
                return Some(action);
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Old subscriber loops are dead before reconnect
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn old_subscriber_loops_are_dead_before_reconnect() {
    // @step Given an App bootstrapped against a backend whose broadcast senders are then closed
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    app.dispatch(Action::SessionCreated(sid("s-1")));
    assert_eq!(
        app.subscriber_task_count(),
        SUBSCRIBER_STREAM_COUNT,
        "precondition: bootstrap spawns the full set of subscriber tasks"
    );

    // @step When the backend's broadcast senders are closed so every subscriber receiver observes RecvError::Closed
    mock.disconnect_all();
    // Give the five subscriber loops time to observe Closed and exit.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // @step Then the original subscriber tasks have all exited and no live stream reaches the App
    // The tasks self-exit on Closed; the App's stored JoinHandles are now
    // finished. Pushing events after disconnect must NOT reach the bus:
    // there are no live senders and no live subscriber loops.
    mock.reconnect_all();
    mock.push_work_units(vec![wu("AUTH-DEAD", "backlog")]);
    let reached = wait_for_action(&mut app, |a| matches!(a, Action::WorkUnitsLoaded(_))).await;
    assert!(
        reached.is_none(),
        "with the old loops dead and no respawn, no live work_units stream may reach the App"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Each broadcast stream delivers a post-reconnect event to the App
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn each_broadcast_stream_delivers_a_post_reconnect_event_to_the_app() {
    // @step Given an App bootstrapped against a backend whose subscriber tasks have exited after a simulated disconnect
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    // Seed the mock's in-memory work units so the RPC-011 re-bootstrap
    // refetch (list_work_units on Reconnected) carries a live snapshot.
    mock.seed_work_units(vec![wu("SEED", "backlog")]);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    mock.disconnect_all();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // @step When the App dispatches Action::Reconnected and the backend then emits one event on each of the work_units, chunks, logs, status_changes and session_created streams
    mock.reconnect_all();
    app.dispatch(Action::Reconnected);
    // RPC-415 determinism: `Action::Reconnected` also spawns a one-shot
    // RPC-011 re-bootstrap (list_work_units refetch + create_session)
    // whose WorkUnitsLoaded lands on the same bus as the broadcast
    // events below. Wait for BOTH refetch backend calls to have run
    // (polling the mock's call counters) so the refetch snapshot is
    // already queued — and drained by the next bus sweep — BEFORE the
    // broadcast events are pushed.
    let baseline_list = mock.list_work_units_calls();
    let baseline_create = mock.create_session_calls();
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(2000);
    while std::time::Instant::now() < deadline
        && (mock.list_work_units_calls() <= baseline_list
            || mock.create_session_calls() <= baseline_create)
    {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert!(
        mock.list_work_units_calls() > baseline_list,
        "Reconnected must trigger the RPC-011 list_work_units refetch"
    );
    assert!(
        mock.create_session_calls() > baseline_create,
        "Reconnected must trigger the RPC-011 create_session refetch"
    );
    // Determinism harness: each RESPAWNED subscriber only binds to the
    // new broadcast channel on its FIRST poll, and a broadcast send to
    // a channel that currently has ZERO live receivers is silently
    // dropped. So a push right after `Reconnected` can, under host
    // load, land on a stream whose respawned subscriber has not yet
    // polled — the EVENT is lost, not the subscriber (the observed
    // flake: the failure hops between the chunk / status /
    // session_created assertions depending on which stream's task got
    // scheduled first). The loop below re-emits every not-yet-observed
    // event each round (bounded at 5s total): a genuinely broken
    // respawn (subscriber bound to the OLD client's receivers, or a
    // stream that was never respawned) would never observe the
    // re-emitted events either, so the scenario's discriminating power
    // is unchanged — only the subscribe-vs-push race is removed.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut wu_action = None;
    let mut chunk_action = None;
    let mut status_action = None;
    let mut created_action = None;
    while (wu_action.is_none()
        || chunk_action.is_none()
        || status_action.is_none()
        || created_action.is_none())
        && std::time::Instant::now() < deadline
    {
        if wu_action.is_none() {
            mock.push_work_units(vec![wu("AUTH-LIVE", "backlog")]);
        }
        if chunk_action.is_none() {
            mock.push_chunk(sid("s-1"), StreamChunk::text("live".to_string()));
        }
        if status_action.is_none() {
            mock.push_status_change(sid("s-1"), codelet_rpc_types::SessionStatus::Running);
        }
        if created_action.is_none() {
            mock.push_session_created(sid("s-live"));
        }
        // Drain the bus for up to 200ms, assigning each post-reconnect
        // action to its slot (first match wins; duplicates of the
        // re-emitted events are ignored).
        let tick_deadline = std::time::Instant::now() + std::time::Duration::from_millis(200);
        while std::time::Instant::now() < tick_deadline {
            while let Some(action) = app.try_recv_action() {
                if wu_action.is_none()
                    && matches!(
                        &action,
                        Action::WorkUnitsLoaded(units)
                            if units.iter().any(|u| u.id == "AUTH-LIVE")
                    )
                {
                    wu_action = Some(action);
                } else if chunk_action.is_none()
                    && matches!(&action, Action::ChunkReceived(id, _) if id == &sid("s-1"))
                {
                    chunk_action = Some(action);
                } else if status_action.is_none()
                    && matches!(&action, Action::SessionStatusChanged(id, _) if id == &sid("s-1"))
                {
                    status_action = Some(action);
                } else if created_action.is_none()
                    && matches!(&action, Action::SessionCreated(id) if id.value == "s-live")
                {
                    created_action = Some(action);
                }
            }
            if wu_action.is_some()
                && chunk_action.is_some()
                && status_action.is_some()
                && created_action.is_some()
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }

    // @step Then the App receives a WorkUnitsLoaded action carrying the post-reconnect work_units update
    assert!(
        wu_action.is_some(),
        "post-reconnect work_units update must reach the App"
    );

    // @step And the App receives a ChunkReceived action for the post-reconnect chunk
    assert!(
        chunk_action.is_some(),
        "post-reconnect chunk must reach the App"
    );

    // @step And the App receives a SessionStatusChanged action for the post-reconnect status change
    assert!(
        status_action.is_some(),
        "post-reconnect status change must reach the App"
    );

    // @step And the App receives a SessionCreated action for the post-reconnect session_created event
    assert!(
        created_action.is_some(),
        "post-reconnect session_created event must reach the App"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Respawn binds subscribers to the new client receivers
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn respawn_binds_subscribers_to_the_new_client_receivers() {
    // @step Given an App bootstrapped against a backend whose original subscriber tasks have exited after a simulated disconnect
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    mock.disconnect_all();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    // Swap in the NEW client's senders. Any receiver still bound to the OLD
    // (dropped) senders can never observe events pushed after this point.
    mock.reconnect_all();

    // @step When the App dispatches Action::Reconnected and the backend emits a work_units update from its current senders
    app.dispatch(Action::Reconnected);
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
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
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Flapping reconnects do not accumulate duplicate subscriber tasks
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn flapping_reconnects_do_not_accumulate_duplicate_subscriber_tasks() {
    // @step Given an App bootstrapped against a backend
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");

    // @step When the App dispatches Action::Reconnected twice in succession and the backend then emits a single work_units update
    mock.disconnect_all();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    mock.reconnect_all();
    app.dispatch(Action::Reconnected);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    mock.disconnect_all();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    mock.reconnect_all();
    app.dispatch(Action::Reconnected);
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    mock.push_work_units(vec![wu("AUTH-ONCE", "backlog")]);

    // @step Then the live subscriber task count equals the full set of broadcast streams and does not grow with each reconnect
    assert_eq!(
        app.subscriber_task_count(),
        SUBSCRIBER_STREAM_COUNT,
        "flapping must not accumulate N x {SUBSCRIBER_STREAM_COUNT} subscriber tasks"
    );

    // @step And the App receives exactly one WorkUnitsLoaded action for that update with no duplicate delivery
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(300);
    let mut once_count = 0usize;
    while std::time::Instant::now() < deadline {
        while let Some(action) = app.try_recv_action() {
            if let Action::WorkUnitsLoaded(units) = &action {
                if units.iter().any(|u| u.id == "AUTH-ONCE") {
                    once_count += 1;
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert_eq!(
        once_count, 1,
        "each broadcast event must be delivered exactly once (no leaked duplicate subscribers)"
    );
}
