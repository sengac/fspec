//! RPC-385 — Spawned subordinate agents are not registered/visible in the
//! Rust TUI (TUI-facing half).
//!
//! Feature: spec/features/agentview-spawned-subordinate-session-registration.feature
//!
//! This file owns the four TUI-facing scenarios of the feature:
//!
//!   #2 A spawned subordinate appears as a new tab in the TUI
//!   #3 A duplicate session-created event does not create a second tab
//!   #4 TUI-initiated session creation produces exactly one tab
//!   #5 The subscriber recovers from a lagged broadcast receiver
//!
//! Scenarios #2-#4 drive `Action::SessionCreated` through `App::dispatch`
//! (every creation path — spawned subordinate, duplicate event, and the
//! create-session dialog — funnels into this one action). The idempotency
//! deliverable (`AgentViewStore::append_session` / `handle_session_created`
//! becomes a no-op when a tab for the id already exists) is what these pins.
//!
//! Scenario #5 mirrors the existing `work_units_rx` lag-recovery test
//! (`app_bootstrap_rpc009.rs::subscriber_tasks_honour_recverror_lagged_*`):
//! it overflows the session-created broadcast to force `RecvError::Lagged`
//! and asserts the new subscriber task keeps processing subsequent events
//! rather than terminating.
//!
//! RED PHASE NOTE: scenarios #3-#5 reference API that does not yet exist —
//! the idempotent `append_session` no-op behaviour, the
//! `MockBackend::push_session_created` test helper, and the
//! `FspecBackend::session_created_rx()` subscriber wiring in
//! `spawn_subscriber_tasks` (which raises `subscriber_task_count` to 6).
//! These tests are therefore expected to FAIL until Approach A lands.

use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, SessionContext};
use codelet_rpc_types::SessionId;

mod common;
use common::MockBackend;

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// Count the open tabs whose SessionContext id matches `id`.
fn tabs_for(app: &App, id: &SessionId) -> usize {
    app.agent_view_store()
        .open_sessions()
        .iter()
        .filter(|c| &c.id == id)
        .count()
}

/// Drain the App's action bus until a matching action arrives or 2s
/// elapses. Mirrors the helper in `app_bootstrap_rpc009.rs`. Drains every
/// currently-queued action each tick (not just one) so a backlog of
/// preceding actions — e.g. the flood of `lagged-NNN` SessionCreated events
/// emitted before the post-lag probe in the lag-recovery scenario — does not
/// starve the predicate before the deadline.
async fn wait_for_action<F: Fn(&Action) -> bool>(app: &mut App, pred: F) -> Option<Action> {
    // TEST-042: wait for the action itself under the deadlock guard (None
    // only when the guard expires), never for a fixed window.
    codelet_test_helpers::deadlock_guard("the awaited action", async {
        loop {
            while let Some(action) = app.try_recv_action() {
                if pred(&action) {
                    return action;
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .ok()
}

// =============================================================================
// Scenario: A spawned subordinate appears as a new tab in the TUI
// =============================================================================
//
// RPC-385 STRENGTHENED: this scenario now drives the FULL end-to-end wiring
// rather than a bare `app.dispatch(SessionCreated)`. It boots the App (which
// spawns the new fifth `session_created_rx` subscriber task), pushes a
// session-created broadcast frame onto the backend — modelling exactly what
// `SessionManager::create_session_with_id` does for a spawned subordinate —
// and asserts the subscriber folds it into `Action::SessionCreated`, which
// the App's dispatch loop then turns into a new tab. This proves the path
//   broadcast → subscriber task → Action::SessionCreated → append_session
// is genuinely connected, not just the store append in isolation.
#[tokio::test]
async fn a_spawned_subordinate_appears_as_a_new_tab_in_the_tui() -> TestResult {
    // @step Given a running TUI with no sessions open
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    assert!(
        app.agent_view_store().open_sessions().is_empty(),
        "precondition: the TUI starts with no open sessions"
    );

    // @step When a subordinate session is spawned via AgentManager and its session-created event is delivered
    // The backend broadcast (what create_session_with_id fires) is delivered
    // through the real fifth subscriber task, NOT a hand-rolled dispatch.
    let subordinate = sid("subordinate-1");
    mock.push_session_created(subordinate.clone());

    // Drive the action through the subscriber task → action bus → dispatch.
    let action = wait_for_action(
        &mut app,
        |a| matches!(a, Action::SessionObserved(info) if info.id == "subordinate-1"),
    )
    .await
    .or_fail("the session-created broadcast must reach the action bus as SessionObserved")?;
    app.dispatch(action);

    // @step Then the TUI appends a new agent tab for the subordinate session id
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        1,
        "exactly one tab must exist after the subordinate is delivered"
    );
    assert_eq!(tabs_for(&app, &subordinate), 1);
    assert_eq!(
        app.agent_view_store().open_sessions()[0].id,
        subordinate,
        "the new tab must carry the subordinate's session id"
    );
    Ok(())
}

// =============================================================================
// Scenario: A duplicate session-created event does not create a second tab
// =============================================================================
#[test]
fn a_duplicate_session_created_event_does_not_create_a_second_tab() -> TestResult {
    // @step Given a TUI that already has a tab for a session id
    let (mut app, _mock) = fresh_app();
    let id = sid("dup-1");
    app.dispatch(Action::SessionCreated(id.clone()));
    assert_eq!(tabs_for(&app, &id), 1, "precondition: one tab exists");

    // @step When a second session-created event arrives for the same session id
    app.dispatch(Action::SessionCreated(id.clone()));

    // @step Then the TUI still shows exactly one tab for that session id
    assert_eq!(
        tabs_for(&app, &id),
        1,
        "the duplicate session-created event must be a no-op (idempotent append)"
    );
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        1,
        "no duplicate tab may be appended for an already-open id"
    );
    Ok(())
}

// =============================================================================
// Scenario: TUI-initiated session creation produces exactly one tab
// =============================================================================
#[test]
fn tui_initiated_session_creation_produces_exactly_one_tab() -> TestResult {
    // @step Given a user opens the create-session dialog and confirms a new session
    // The dialog's confirm path emits Action::SessionCreated(id) (see
    // dispatch_create_session_dialog.rs); model that first delivery here.
    let (mut app, _mock) = fresh_app();
    let id = sid("dialog-1");
    app.dispatch(Action::SessionCreated(id.clone()));
    assert_eq!(
        tabs_for(&app, &id),
        1,
        "precondition: the dialog confirm appends one tab"
    );

    // @step When both the dialog and the session-created broadcast feed Action::SessionCreated for the same id
    // The broadcast subscriber folds the SAME id into a second SessionCreated.
    app.dispatch(Action::SessionCreated(id.clone()));

    // @step Then exactly one tab appears for the new session
    assert_eq!(
        tabs_for(&app, &id),
        1,
        "the broadcast for a TUI-initiated id must not double-append a tab"
    );
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);
    Ok(())
}

// =============================================================================
// Scenario: The subscriber recovers from a lagged broadcast receiver
// =============================================================================
#[tokio::test]
async fn the_subscriber_recovers_from_a_lagged_broadcast_receiver() -> TestResult {
    // @step Given the session-created subscriber whose receiver has lagged
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    // The bootstrap spawns the full subscriber set, session_created
    // (RPC-385) among them (BUG-351: one shared total).
    assert_eq!(
        app.subscriber_task_count(),
        common::BOOTSTRAP_SUBSCRIBER_TASKS,
        "bootstrap must spawn the new session-created subscriber task"
    );
    // Overflow the session-created broadcast to force RecvError::Lagged.
    for i in 0..200 {
        mock.push_session_created(sid(&format!("lagged-{i:03}")));
    }

    // @step When the subscriber observes the lagged receiver error
    // TEST-042: no delay: the lag is observed on the subscriber's next
    // recv, and the event pushed below is still delivered after it.

    // @step Then it continues processing subsequent session-created events instead of terminating
    let fresh = sid("post-lag-session");
    mock.push_session_created(fresh.clone());
    let action = wait_for_action(
        &mut app,
        |a| matches!(a, Action::SessionObserved(info) if info.id == "post-lag-session"),
    )
    .await
    .or_fail("a post-lag SessionObserved must still reach the action bus")?;
    assert!(matches!(action, Action::SessionObserved(_)));
    Ok(())
}

// =============================================================================
// Store-level unit: append_session is idempotent (rule [2]/[3] guard-rail).
// =============================================================================
#[test]
fn append_session_is_idempotent_at_the_store_level() -> TestResult {
    let mut store = codelet_fspec_tui::AgentViewStore::default();
    store.append_session(SessionContext::new(sid("only-1")));
    store.append_session(SessionContext::new(sid("only-1")));
    assert_eq!(
        store.open_sessions().len(),
        1,
        "appending the same SessionId twice must yield exactly one tab"
    );
    Ok(())
}
