//! RPC-060 — CreateSessionDialog component + isolated-session dispatch
//! end-to-end tests.
//!
//! Feature: spec/features/rpc060-isolated-session-dialog.feature
//!
//! Drives the new CreateSessionDialog through its public Component
//! surface plus the App::dispatch routing for
//! `Action::OpenCreateSessionDialog` / `CreateSessionSubmitted` /
//! `CreateSessionCancelled`. Mirrors the thinking_level_dialog_rpc022 +
//! slash_resume_rpc049 layouts.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::too_many_lines)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{
    Accent, Action, App, Compositor, CreateSessionDialog, CreateSessionOption, EventResult,
    FspecBackend, Priority, CREATE_SESSION_DIALOG_ID,
};
use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_rpc_types::{IsolatedSessionInfo, SessionId, WorkUnitContext};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn render_to_string(dialog: &mut CreateSessionDialog) -> String {
    let backend = TestBackend::new(80, 24);
    let mut term = Terminal::new(backend).expect("Terminal::new");
    term.draw(|frame| {
        use codelet_fspec_tui::Component;
        Component::render(dialog, frame.area(), frame.buffer_mut());
    })
    .expect("draw");
    let buf: Buffer = term.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

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

fn session_scrollback_text(app: &App, id: &SessionId) -> String {
    let chunks = app
        .agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default();
    chunks
        .iter()
        .flat_map(|c| {
            c.lines
                .iter()
                .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        })
        .collect::<Vec<String>>()
        .join("\n")
}

// ─────────────────────────────────────────────────────────────────────
// Dialog component scenarios
// ─────────────────────────────────────────────────────────────────────

/// Scenario: CreateSessionDialog defaults selection to "Yes" when opened without a preselection
#[test]
fn create_session_dialog_defaults_to_yes_with_unattached_title() {
    use codelet_fspec_tui::Component;
    // @step Given the CreateSessionDialog is constructed with preselect=None and no work_unit binding
    let dialog = CreateSessionDialog::new(None, None);
    // @step Then the selected option is "Yes"
    assert_eq!(dialog.selected_option(), CreateSessionOption::Yes);
    // @step And the dialog title is "Start New Agent?"
    assert_eq!(dialog.title(), "Start New Agent?");
    // @step And the dialog accent is cyan
    assert_eq!(dialog.accent(), Accent::Cyan);
    assert_eq!(dialog.priority(), Priority::Foreground);
    assert_eq!(dialog.id(), CREATE_SESSION_DIALOG_ID);
}

/// Scenario: CreateSessionDialog renders work-unit-aware title when a work_unit is bound
#[test]
fn create_session_dialog_renders_work_unit_aware_title() {
    // @step Given the CreateSessionDialog is constructed with preselect=None and work_unit_id "AUTH-001"
    let ctx = WorkUnitContext {
        id: "AUTH-001".to_string(),
        title: "User Login".to_string(),
        status: "backlog".to_string(),
    };
    let dialog = CreateSessionDialog::new(None, Some(ctx));
    // @step Then the dialog title is "Work on AUTH-001?"
    assert_eq!(dialog.title(), "Work on AUTH-001?");
}

/// Scenario: CreateSessionDialog can be preselected to "Yes - Isolated" via /isolation shortcut
#[test]
fn create_session_dialog_accepts_isolated_preselection() {
    // @step Given the CreateSessionDialog is constructed with preselect=Some(Isolated)
    let dialog = CreateSessionDialog::new(Some(CreateSessionOption::Isolated), None);
    // @step Then the selected option is "Yes - Isolated"
    assert_eq!(dialog.selected_option(), CreateSessionOption::Isolated);
}

/// Scenario: CreateSessionDialog Right arrow cycles forward with wrap-around
#[test]
fn create_session_dialog_right_arrow_cycles_forward() {
    use codelet_fspec_tui::Component;
    // @step Given a CreateSessionDialog with selection "Yes"
    let mut dialog = CreateSessionDialog::new(None, None);
    assert_eq!(dialog.selected_option(), CreateSessionOption::Yes);
    // @step When the user presses Right arrow
    let _ = dialog.handle_event(&key(KeyCode::Right));
    // @step Then the selection becomes "Yes - Isolated"
    assert_eq!(dialog.selected_option(), CreateSessionOption::Isolated);
    // @step When the user presses Right arrow
    let _ = dialog.handle_event(&key(KeyCode::Right));
    // @step Then the selection becomes "Cancel"
    assert_eq!(dialog.selected_option(), CreateSessionOption::Cancel);
    // @step When the user presses Right arrow
    let _ = dialog.handle_event(&key(KeyCode::Right));
    // @step Then the selection becomes "Yes"
    assert_eq!(dialog.selected_option(), CreateSessionOption::Yes);
}

/// Scenario: CreateSessionDialog Left arrow cycles backward with wrap-around
#[test]
fn create_session_dialog_left_arrow_cycles_backward() {
    use codelet_fspec_tui::Component;
    // @step Given a CreateSessionDialog with selection "Yes"
    let mut dialog = CreateSessionDialog::new(None, None);
    assert_eq!(dialog.selected_option(), CreateSessionOption::Yes);
    // @step When the user presses Left arrow
    let _ = dialog.handle_event(&key(KeyCode::Left));
    // @step Then the selection becomes "Cancel"
    assert_eq!(dialog.selected_option(), CreateSessionOption::Cancel);
    // @step When the user presses Left arrow
    let _ = dialog.handle_event(&key(KeyCode::Left));
    // @step Then the selection becomes "Yes - Isolated"
    assert_eq!(dialog.selected_option(), CreateSessionOption::Isolated);
}

/// Scenario: CreateSessionDialog Enter on "Yes" emits CreateSessionSubmitted{isolated:false}
#[test]
fn create_session_dialog_enter_on_yes_emits_submitted_non_isolated() {
    use codelet_fspec_tui::Component;
    // @step Given a CreateSessionDialog with selection "Yes"
    let mut dialog = CreateSessionDialog::new(None, None);
    assert_eq!(dialog.selected_option(), CreateSessionOption::Yes);
    // @step When the user presses Enter
    let result = dialog.handle_event(&key(KeyCode::Enter));
    // @step Then Action::CreateSessionSubmitted { isolated: false } is emitted
    let callback = match result {
        EventResult::Consumed(Some(cb)) => cb,
        _ => panic!("expected Consumed(Some(callback))"),
    };
    let action = dialog
        .take_pending_action()
        .expect("pending action must be set");
    match action {
        Action::CreateSessionSubmitted { isolated } => assert!(!isolated),
        other => panic!("expected CreateSessionSubmitted, got {other:?}"),
    }
    // @step And the dialog requests removal from the compositor
    let mut compositor = Compositor::new();
    compositor.push(Box::new(CreateSessionDialog::new(None, None)));
    callback(&mut compositor);
    assert!(!compositor.contains(CREATE_SESSION_DIALOG_ID));
}

/// Scenario: CreateSessionDialog Enter on "Yes - Isolated" emits CreateSessionSubmitted{isolated:true}
#[test]
fn create_session_dialog_enter_on_isolated_emits_submitted_isolated() {
    use codelet_fspec_tui::Component;
    // @step Given a CreateSessionDialog with selection "Yes - Isolated"
    let mut dialog =
        CreateSessionDialog::new(Some(CreateSessionOption::Isolated), None);
    assert_eq!(dialog.selected_option(), CreateSessionOption::Isolated);
    // @step When the user presses Enter
    let result = dialog.handle_event(&key(KeyCode::Enter));
    // @step Then Action::CreateSessionSubmitted { isolated: true } is emitted
    let callback = match result {
        EventResult::Consumed(Some(cb)) => cb,
        _ => panic!("expected Consumed(Some(callback))"),
    };
    let action = dialog
        .take_pending_action()
        .expect("pending action must be set");
    match action {
        Action::CreateSessionSubmitted { isolated } => assert!(isolated),
        other => panic!("expected CreateSessionSubmitted, got {other:?}"),
    }
    // @step And the dialog requests removal from the compositor
    let mut compositor = Compositor::new();
    compositor.push(Box::new(CreateSessionDialog::new(None, None)));
    callback(&mut compositor);
    assert!(!compositor.contains(CREATE_SESSION_DIALOG_ID));
}

/// Scenario: CreateSessionDialog Enter on "Cancel" emits CreateSessionCancelled
#[test]
fn create_session_dialog_enter_on_cancel_emits_cancelled() {
    use codelet_fspec_tui::Component;
    // @step Given a CreateSessionDialog with selection "Cancel"
    let mut dialog =
        CreateSessionDialog::new(Some(CreateSessionOption::Cancel), None);
    assert_eq!(dialog.selected_option(), CreateSessionOption::Cancel);
    // @step When the user presses Enter
    let result = dialog.handle_event(&key(KeyCode::Enter));
    // @step Then Action::CreateSessionCancelled is emitted
    let callback = match result {
        EventResult::Consumed(Some(cb)) => cb,
        _ => panic!("expected Consumed(Some(callback))"),
    };
    let action = dialog
        .take_pending_action()
        .expect("pending action must be set");
    match action {
        Action::CreateSessionCancelled => {}
        other => panic!("expected CreateSessionCancelled, got {other:?}"),
    }
    // @step And the dialog requests removal from the compositor
    let mut compositor = Compositor::new();
    compositor.push(Box::new(CreateSessionDialog::new(None, None)));
    callback(&mut compositor);
    assert!(!compositor.contains(CREATE_SESSION_DIALOG_ID));
}

/// Scenario: CreateSessionDialog Esc emits CreateSessionCancelled
#[test]
fn create_session_dialog_esc_emits_cancelled() {
    use codelet_fspec_tui::Component;
    // @step Given a CreateSessionDialog with any selection
    let mut dialog = CreateSessionDialog::new(None, None);
    // @step When the user presses Esc
    let result = dialog.handle_event(&key(KeyCode::Esc));
    // @step Then Action::CreateSessionCancelled is emitted
    let callback = match result {
        EventResult::Consumed(Some(cb)) => cb,
        _ => panic!("expected Consumed(Some(callback))"),
    };
    let action = dialog
        .take_pending_action()
        .expect("pending action must be set");
    match action {
        Action::CreateSessionCancelled => {}
        other => panic!("expected CreateSessionCancelled, got {other:?}"),
    }
    // @step And the dialog requests removal from the compositor
    let mut compositor = Compositor::new();
    compositor.push(Box::new(CreateSessionDialog::new(None, None)));
    callback(&mut compositor);
    assert!(!compositor.contains(CREATE_SESSION_DIALOG_ID));
}

/// Smoke: dialog renders the three canonical labels
#[test]
fn create_session_dialog_renders_three_canonical_labels() {
    let mut dialog = CreateSessionDialog::new(None, None);
    let painted = render_to_string(&mut dialog);
    assert!(painted.contains("Start New Agent?"));
    assert!(painted.contains("Yes"));
    assert!(painted.contains("Isolated"));
    assert!(painted.contains("Cancel"));
}

// ─────────────────────────────────────────────────────────────────────
// Slash command + dispatch scenarios
// ─────────────────────────────────────────────────────────────────────

/// Scenario: /isolation slash command opens the CreateSessionDialog with "Yes - Isolated" preselected
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn isolation_slash_command_opens_dialog_with_isolated_preselected() {
    // @step Given an App with open session s-1
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.agent_view_store_mut()
        .append_session(codelet_fspec_tui::SessionContext::new(sid("s-1")));

    // @step When SlashCommandSelected(SlashCommandAction::Isolation) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Isolation));
    drain_pending(&mut app).await;

    // @step Then a CreateSessionDialog is pushed onto the compositor at Priority::Foreground with preselect=Some(Isolated)
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "CreateSessionDialog should be mounted on the compositor"
    );

    // @step And no backend method is called
    assert_eq!(mock.create_isolated_session_calls(), 0);
    assert_eq!(mock.create_session_calls(), 0);
}

/// Scenario: CreateSessionSubmitted{isolated:true} spawns backend.create_isolated_session
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_session_submitted_isolated_spawns_create_isolated_session() {
    // @step Given an App with open session s-1 wired to a MockBackend whose create_isolated_session returns Ok(IsolatedSessionInfo)
    let mock = Arc::new(MockBackend::new());
    mock.seed_create_isolated_session_result(Ok(IsolatedSessionInfo {
        session_id: sid("iso-1"),
        worktree_path: "/tmp/.fspec/worktrees/iso-1".to_string(),
        base_commit: "abc123".to_string(),
    }));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.agent_view_store_mut()
        .append_session(codelet_fspec_tui::SessionContext::new(sid("s-1")));

    // @step When Action::CreateSessionSubmitted { isolated: true } is dispatched
    app.dispatch(Action::CreateSessionSubmitted { isolated: true });

    // @step Then within 1 second backend.create_isolated_session is called exactly once
    wait_until(
        || mock.create_isolated_session_calls() == 1,
        "create_isolated_session called once",
    )
    .await;
    drain_pending(&mut app).await;

    // @step And within 1 second Action::SessionCreated for SessionId "iso-1" is observed
    let opened: Vec<SessionId> = app
        .agent_view_store()
        .open_sessions()
        .iter()
        .map(|c| c.id.clone())
        .collect();
    assert!(
        opened.iter().any(|s| s == &sid("iso-1")),
        "expected iso-1 session to be opened, got {opened:?}"
    );

    // @step And backend.create_session is NOT called
    assert_eq!(mock.create_session_calls(), 0);
}

/// Scenario: CreateSessionSubmitted{isolated:false} spawns backend.create_session
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_session_submitted_non_isolated_spawns_create_session() {
    // @step Given an App with open session s-1 wired to a MockBackend whose create_session returns Ok(SessionId("plain-1"))
    let mock = Arc::new(MockBackend::new());
    mock.script_create_session(sid("plain-1"));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.agent_view_store_mut()
        .append_session(codelet_fspec_tui::SessionContext::new(sid("s-1")));
    let baseline_create_session_calls = mock.create_session_calls();

    // @step When Action::CreateSessionSubmitted { isolated: false } is dispatched
    app.dispatch(Action::CreateSessionSubmitted { isolated: false });

    // @step Then within 1 second backend.create_session is called exactly once
    wait_until(
        || mock.create_session_calls() == baseline_create_session_calls + 1,
        "create_session called once",
    )
    .await;
    drain_pending(&mut app).await;

    // @step And within 1 second Action::SessionCreated for SessionId "plain-1" is observed
    let opened: Vec<SessionId> = app
        .agent_view_store()
        .open_sessions()
        .iter()
        .map(|c| c.id.clone())
        .collect();
    assert!(
        opened.iter().any(|s| s == &sid("plain-1")),
        "expected plain-1 session to be opened, got {opened:?}"
    );

    // @step And backend.create_isolated_session is NOT called
    assert_eq!(mock.create_isolated_session_calls(), 0);
}

/// Scenario: CreateSessionCancelled is a silent no-op
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_session_cancelled_is_silent_no_op() {
    // @step Given an App with open session s-1
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.agent_view_store_mut()
        .append_session(codelet_fspec_tui::SessionContext::new(sid("s-1")));
    let baseline_create_session_calls = mock.create_session_calls();

    // @step When Action::CreateSessionCancelled is dispatched
    app.dispatch(Action::CreateSessionCancelled);
    drain_pending(&mut app).await;

    // @step Then no backend method is called
    assert_eq!(mock.create_isolated_session_calls(), 0);
    assert_eq!(mock.create_session_calls(), baseline_create_session_calls);
}

/// Scenario: create_isolated_session error emits an error notice into the focused session scrollback
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_isolated_session_error_emits_error_notice() {
    // @step Given an App with open session s-1 wired to a MockBackend whose create_isolated_session returns Err("not a git repository")
    let mock = Arc::new(MockBackend::new());
    mock.seed_create_isolated_session_result(Err("not a git repository".to_string()));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.agent_view_store_mut()
        .append_session(codelet_fspec_tui::SessionContext::new(sid("s-1")));

    // @step When Action::CreateSessionSubmitted { isolated: true } is dispatched
    app.dispatch(Action::CreateSessionSubmitted { isolated: true });
    drain_pending(&mut app).await;

    // @step Then within 1 second Action::EmitSessionNotice for s-1 with text containing the error is observed
    wait_until(
        || {
            session_scrollback_text(&app, &sid("s-1"))
                .contains("[error] create isolated session: not a git repository")
        },
        "error notice in s-1 scrollback",
    )
    .await;
}

/// Scenario: create_isolated_session error with no open session is a silent no-op
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_isolated_session_error_no_open_session_is_silent() {
    // @step Given an App with NO open AgentView session wired to a MockBackend whose create_isolated_session returns Err("e")
    let mock = Arc::new(MockBackend::new());
    mock.seed_create_isolated_session_result(Err("e".to_string()));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When Action::CreateSessionSubmitted { isolated: true } is dispatched
    app.dispatch(Action::CreateSessionSubmitted { isolated: true });

    // @step Then within 1 second backend.create_isolated_session is called exactly once
    wait_until(
        || mock.create_isolated_session_calls() == 1,
        "create_isolated_session called once",
    )
    .await;
    drain_pending(&mut app).await;

    // @step And no Action::EmitSessionNotice is observed on the action bus
    // (No open session, so nothing in any scrollback. Verify open_sessions is still empty.)
    assert_eq!(app.agent_view_store().open_sessions().len(), 0);
}
