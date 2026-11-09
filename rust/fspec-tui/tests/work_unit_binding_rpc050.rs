//! RPC-050 — Work-unit binding: the SessionHeader chip.
//!
//! Feature: spec/features/work-unit-attach-binding.feature
//!
//! Pins the SessionHeader chip render path: when
//! `store.work_unit_context_for(sid)` returns Some(ctx) (from the session's
//! work-unit focus, BUG-230), the SessionHeader paints `(<id>: <status>)`
//! between the session prefix and the model name. TUI-132 removed the
//! AttachWorkUnitToSession / WorkUnitAttached actions nothing emitted.

use crate::common::TaskOutcome;
use codelet_test_helpers::TestResult;
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{SessionId, WorkUnitContext};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

mod common;
use common::{render_one_frame, test_app, MockBackend};

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// Await every spawned tokio task AND fold any queued action_tx messages
/// back into the App.
async fn drain_pending(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
    }
    Ok(())
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
async fn session_header_renders_work_unit_chip_from_per_session_context() -> TestResult {
    // @step Given an App with open session s-1 bound to work unit AUTH-001 with status "implementing"
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let (mut app, mut terminal) = {
        let backend = backend.clone();
        let (a, t) = test_app(backend)?;
        (a, t)
    };
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await?;
    // Drop legacy slots so the test asserts the PER-SESSION read path is
    // the source of truth.
    app.agent_view_store_mut().set_current_work_unit(None, None);
    app.dispatch(Action::WorkUnitFocusChanged(
        sid("s-1"),
        codelet_rpc_types::WorkUnitFocus {
            revision: 1,
            context: Some(WorkUnitContext {
                id: "AUTH-001".to_string(),
                title: "AUTH-001".to_string(),
                status: "implementing".to_string(),
            }),
            source: "test".to_string(),
        },
    ));
    // Switch to AgentView so the SessionHeader paints.
    app.dispatch(Action::OpenAgentView(Some(sid("s-1"))));
    assert_eq!(app.active_view(), ViewMode::Agent);

    // @step When the AgentView is rendered against an 80x10 TestBackend
    let buf = render_one_frame(&mut terminal, &mut app)?;
    let _ = buf;

    // @step Then the rendered top row contains the substring "(AUTH-001: implementing)"
    let row = top_row_text(&terminal);
    assert!(
        row.contains("(AUTH-001: implementing)"),
        "expected SessionHeader chip in top row, got: {row:?}",
    );
    Ok(())
}
