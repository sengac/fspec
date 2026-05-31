//! RPC-050 — Work-unit binding (BoardView attach + SessionHeader chip)
//! integration tests.
//!
//! Feature: spec/features/slash-command-detach-and-work-unit-binding.feature
//!
//! Drives the App::dispatch routing for the new
//! `Action::AttachWorkUnitToSession(work_unit_id)` action so it reaches
//! the backend's `set_work_unit_context(session_id, Some(ctx))` RPC and
//! folds the resulting `Action::WorkUnitAttached(SessionId,
//! WorkUnitContext)` into `AgentViewStore.work_unit_context_by_session`.
//!
//! Also pins the SessionHeader chip render path: when
//! `store.work_unit_context_for(sid)` returns Some(ctx), the SessionHeader
//! paints `(<id>: <status>)` between the session prefix and the model name.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{SessionId, WorkUnitContext, WorkUnitInfo};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tokio::time::timeout;

mod common;
use common::{render_one_frame, test_app, MockBackend};

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

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

async fn wait_until<F: FnMut() -> bool>(mut predicate: F, label: &str) {
    timeout(Duration::from_secs(1), async {
        loop {
            if predicate() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for: {label}"));
}

/// Await every spawned tokio task AND fold any queued action_tx messages
/// back into the App.
async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: AttachWorkUnitToSession with a current session calls the
// backend and folds the context into AgentViewStore
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn attach_work_unit_to_session_with_current_session_calls_backend_and_folds_into_store() {
    // @step Given an App wired to a MockBackend with open session s-1 as the current session
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    assert_eq!(app.agent_view_store().current_session(), Some(&sid("s-1")));

    // @step And the BoardStore contains work unit AUTH-001 in the "implementing" column
    app.board_store_mut()
        .replace_work_units(vec![wu("AUTH-001", "implementing")]);

    // @step When Action::AttachWorkUnitToSession("AUTH-001") is dispatched
    app.dispatch(Action::AttachWorkUnitToSession("AUTH-001".to_string()));
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.set_work_unit_context is called exactly once with (s-1, Some(WorkUnitContext{id:"AUTH-001", title:"AUTH-001", status:"implementing"}))
    wait_until(
        || mock.set_work_unit_context_calls() == 1,
        "backend.set_work_unit_context call count to reach 1",
    )
    .await;
    let last = mock.last_set_work_unit_context().expect("last set call");
    assert_eq!(last.0, sid("s-1"));
    assert_eq!(
        last.1,
        Some(WorkUnitContext {
            id: "AUTH-001".to_string(),
            title: "AUTH-001".to_string(),
            status: "implementing".to_string(),
        })
    );

    // @step And within 1 second AgentViewStore.work_unit_context_for(s-1) returns Some(ctx) with id "AUTH-001" and status "implementing"
    let stored = app
        .agent_view_store()
        .work_unit_context_for(&sid("s-1"))
        .expect("work-unit ctx stored for s-1");
    assert_eq!(stored.id, "AUTH-001");
    assert_eq!(stored.status, "implementing");

    // @step And the Navigator's active_view equals ViewMode::Agent
    assert_eq!(app.active_view(), ViewMode::Agent);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: AttachWorkUnitToSession with NO current session is a silent
// no-op
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn attach_work_unit_to_session_with_no_current_session_is_silent_no_op() {
    // @step Given an App wired to a MockBackend with NO open session
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    assert!(app.agent_view_store().current_session().is_none());

    // @step And the BoardStore contains work unit AUTH-001
    app.board_store_mut()
        .replace_work_units(vec![wu("AUTH-001", "backlog")]);

    // @step When Action::AttachWorkUnitToSession("AUTH-001") is dispatched
    app.dispatch(Action::AttachWorkUnitToSession("AUTH-001".to_string()));
    drain_pending(&mut app).await;

    // @step Then backend.set_work_unit_context is NEVER called
    assert_eq!(
        mock.set_work_unit_context_calls(),
        0,
        "set_work_unit_context must not run when there is no current session",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: SessionHeader renders the work-unit chip from per-session
// context
// ─────────────────────────────────────────────────────────────────────────

fn top_row_text(terminal: &Terminal<TestBackend>) -> String {
    let buf = terminal.backend().buffer();
    let mut row = String::with_capacity(buf.area.width as usize);
    for x in 0..buf.area.width {
        row.push_str(buf[(x, 0)].symbol());
    }
    row
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_header_renders_work_unit_chip_from_per_session_context() {
    // @step Given an App with open session s-1 bound to work unit AUTH-001 with status "implementing"
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let (mut app, mut terminal) = {
        let backend = backend.clone();
        let (a, t) = test_app(backend);
        (a, t)
    };
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;
    // Drop legacy slots so the test asserts the PER-SESSION read path is
    // the source of truth.
    app.agent_view_store_mut()
        .set_current_work_unit(None, None);
    app.dispatch(Action::WorkUnitAttached(
        sid("s-1"),
        WorkUnitContext {
            id: "AUTH-001".to_string(),
            title: "AUTH-001".to_string(),
            status: "implementing".to_string(),
        },
    ));
    // Switch to AgentView so the SessionHeader paints.
    app.dispatch(Action::OpenAgentView(Some(sid("s-1"))));
    assert_eq!(app.active_view(), ViewMode::Agent);

    // @step When the AgentView is rendered against an 80x10 TestBackend
    let buf = render_one_frame(&mut terminal, &mut app);
    let _ = buf;

    // @step Then the rendered top row contains the substring "(AUTH-001: implementing)"
    let row = top_row_text(&terminal);
    assert!(
        row.contains("(AUTH-001: implementing)"),
        "expected SessionHeader chip in top row, got: {row:?}",
    );
}
