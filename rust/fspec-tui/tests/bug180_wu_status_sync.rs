//! BUG-180 — SessionHeader work-unit status goes stale when the work
//! unit's status changes externally.
//!
//! Feature: spec/features/bug-180-sessionheader-work-unit-status-sync.feature
//!
//! The `WorkUnitsWatcher` push already reaches the TUI on both transports
//! (embedded `work_units_rx` broadcast + websocket `Envelope::WorkUnitsUpdate`),
//! and the App's subscriber task converts it into
//! `Action::WorkUnitsLoaded(Vec<WorkUnitInfo>)`. The bug is that the
//! `App::dispatch` handler only re-seeds `BoardStore` and never syncs
//! `AgentViewStore.work_unit_context_by_session` (the per-session
//! `WorkUnitContext` the SessionHeader chip paints) nor the legacy
//! `current_work_unit_id/status` fallback slots — so the chip stays
//! frozen at the attach-time status until the session is detached and
//! re-attached.
//!
//! RED phase: every scenario below fails today because
//! `sync_work_unit_contexts` does not exist / is not called from the
//! `Action::WorkUnitsLoaded` arm.
//!
//! Each test maps 1:1 to a Scenario in the feature file and drives the
//! REAL `App::dispatch` path (same pattern as
//! `work_unit_binding_rpc050.rs` / `bug178_isolated_badge_chrome_paint.rs`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, WorkUnitContext, WorkUnitInfo};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

mod common;
use common::{render_one_frame, test_app, MockBackend};

// ─────────────────────────── helpers ─────────────────────────────────────

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

fn wu_custom(id: &str, title: &str, status: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: title.to_string(),
        work_type: "story".to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

fn bind(app: &mut App, session: &SessionId, unit_id: &str, status: &str) {
    app.dispatch(Action::WorkUnitAttached(
        session.clone(),
        WorkUnitContext {
            id: unit_id.to_string(),
            title: unit_id.to_string(),
            status: status.to_string(),
        },
    ));
}

fn top_row_text(terminal: &Terminal<TestBackend>) -> String {
    let buf = terminal.backend().buffer();
    let mut row = String::with_capacity(buf.area.width as usize);
    for x in 0..buf.area.width {
        row.push_str(buf[(x, 0)].symbol());
    }
    row
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 1: WorkUnitsLoaded syncs per-session WorkUnitContext status
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_work_units_loaded_syncs_per_session_context_status() {
    // @step Given an App wired to a MockBackend with open session s-1 bound to work unit AUTH-001 whose current status is "backlog"
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    bind(&mut app, &sid("s-1"), "AUTH-001", "backlog");
    assert_eq!(
        app.agent_view_store()
            .work_unit_context_for(&sid("s-1"))
            .map(|c| c.status.clone()),
        Some("backlog".to_string())
    );

    // @step When Action::WorkUnitsLoaded carrying a snapshot where AUTH-001 now has status "implementing" is dispatched
    app.dispatch(Action::WorkUnitsLoaded(vec![wu(
        "AUTH-001",
        "implementing",
    )]));

    // @step Then AgentViewStore.work_unit_context_for(s-1) returns Some with status "implementing"
    assert_eq!(
        app.agent_view_store()
            .work_unit_context_for(&sid("s-1"))
            .map(|c| c.status.clone()),
        Some("implementing".to_string()),
        "BUG-180: per-session WorkUnitContext must track the live snapshot"
    );

    // @step And BoardStore still groups AUTH-001 in the "implementing" column
    assert!(
        app.board_store()
            .column_units("implementing")
            .iter()
            .any(|u| u.id == "AUTH-001"),
        "BoardStore re-seed must still happen on WorkUnitsLoaded"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 2: WorkUnitsLoaded syncs legacy fallback slots for the focused
// session
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_work_units_loaded_syncs_legacy_fallback_slots() {
    // @step Given an App with legacy fallback slots current_work_unit_id="AUTH-001" current_work_unit_status="backlog"
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let mut app = App::new(backend);
    app.agent_view_store_mut()
        .set_current_work_unit(Some("AUTH-001".to_string()), Some("backlog".to_string()));

    // @step When Action::WorkUnitsLoaded carrying AUTH-001 status "testing" is dispatched
    app.dispatch(Action::WorkUnitsLoaded(vec![wu("AUTH-001", "testing")]));

    // @step Then AgentViewStore.current_work_unit_status equals Some("testing")
    assert_eq!(
        app.agent_view_store().current_work_unit_status(),
        Some("testing"),
        "BUG-180: legacy fallback status must track the live snapshot"
    );
    assert_eq!(
        app.agent_view_store().current_work_unit_id(),
        Some("AUTH-001"),
        "legacy fallback id is preserved"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 3: WorkUnitsLoaded does not destroy a binding whose unit was
// deleted
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_work_units_loaded_preserves_binding_when_unit_deleted() {
    // @step Given an App with open session s-1 bound to work unit AUTH-001 with status "backlog"
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    bind(&mut app, &sid("s-1"), "AUTH-001", "backlog");

    // @step When Action::WorkUnitsLoaded carrying a snapshot that no longer contains AUTH-001 is dispatched
    app.dispatch(Action::WorkUnitsLoaded(vec![wu("AUTH-002", "backlog")]));

    // @step Then AgentViewStore.work_unit_context_for(s-1) still returns Some with status "backlog"
    assert_eq!(
        app.agent_view_store()
            .work_unit_context_for(&sid("s-1"))
            .map(|c| (c.id.clone(), c.status.clone())),
        Some(("AUTH-001".to_string(), "backlog".to_string())),
        "BUG-180: a deleted unit must not silently detach the session — the \
         last-known context survives until /detach (board shows the unit as gone)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 4: WorkUnitsLoaded leaves detached sessions untouched
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_work_units_loaded_leaves_detached_sessions_untouched() {
    // @step Given an App with open session s-1 bound to AUTH-001 and open session s-2 with NO work unit bound
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::SessionCreated(sid("s-2")));
    bind(&mut app, &sid("s-1"), "AUTH-001", "backlog");
    assert!(app
        .agent_view_store()
        .work_unit_context_for(&sid("s-2"))
        .is_none());

    // @step When Action::WorkUnitsLoaded carrying AUTH-001 status "validating" is dispatched
    app.dispatch(Action::WorkUnitsLoaded(vec![wu("AUTH-001", "validating")]));

    // @step Then AgentViewStore.work_unit_context_for(s-1) returns Some with status "validating"
    assert_eq!(
        app.agent_view_store()
            .work_unit_context_for(&sid("s-1"))
            .map(|c| c.status.clone()),
        Some("validating".to_string())
    );
    // @step And AgentViewStore.work_unit_context_for(s-2) returns None
    assert!(
        app.agent_view_store()
            .work_unit_context_for(&sid("s-2"))
            .is_none(),
        "detached sessions must gain NO binding from a snapshot"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 5: SessionHeader repaints with the fresh status after
// WorkUnitsLoaded
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_session_header_repaints_with_fresh_status_after_work_units_loaded() {
    // @step Given an App with open session s-1 bound to AUTH-001 with status "backlog" rendered in AgentView
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let (mut app, mut terminal) = test_app(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    bind(&mut app, &sid("s-1"), "AUTH-001", "backlog");
    // Drop legacy slots so the test asserts the PER-SESSION read path.
    app.agent_view_store_mut().set_current_work_unit(None, None);
    app.dispatch(Action::OpenAgentView(Some(sid("s-1"))));
    assert_eq!(app.active_view(), ViewMode::Agent);

    // @step When Action::WorkUnitsLoaded carrying AUTH-001 status "implementing" is dispatched
    app.dispatch(Action::WorkUnitsLoaded(vec![wu(
        "AUTH-001",
        "implementing",
    )]));

    // @step And the AgentView is rendered against an 80x10 TestBackend
    let _ = render_one_frame(&mut terminal, &mut app);

    // @step Then the rendered top row contains the substring "(AUTH-001: implementing)"
    let row = top_row_text(&terminal);
    assert!(
        row.contains("(AUTH-001: implementing)"),
        "BUG-180: SessionHeader must paint the FRESH status, got: {row:?}"
    );
    // @step And the rendered top row does NOT contain the substring "(AUTH-001: backlog)"
    assert!(
        !row.contains("(AUTH-001: backlog)"),
        "BUG-180: stale attach-time status must no longer paint, got: {row:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Extra regression: sync also refreshes the title (not just the status),
// so a title rename via update-work-unit propagates to the chip too.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_work_units_loaded_syncs_title_along_with_status() {
    // @step Given an App with open session s-1 bound to AUTH-001 (title "AUTH-001", status "backlog")
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    bind(&mut app, &sid("s-1"), "AUTH-001", "backlog");

    // @step When Action::WorkUnitsLoaded carrying AUTH-001 with title "User Login" and status "specifying" is dispatched
    app.dispatch(Action::WorkUnitsLoaded(vec![wu_custom(
        "AUTH-001",
        "User Login",
        "specifying",
    )]));

    // @step Then AgentViewStore.work_unit_context_for(s-1) returns Some with title "User Login" and status "specifying"
    let stored = app
        .agent_view_store()
        .work_unit_context_for(&sid("s-1"))
        .expect("binding preserved");
    assert_eq!(stored.title, "User Login");
    assert_eq!(stored.status, "specifying");
}
