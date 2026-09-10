//! WT-009 — `/isolation` is a real state toggle and `/merge-worktree` closes
//! the session on success (UX contract broken in the Rust TUI).
//!
//! Feature: spec/features/isolation-only-opens-the-create-session-dialog-and-merge-worktree-never-closes-the-session-ux-contract-broken-in-the-rust-tui.feature
//!
//! RED phase: written BEFORE the implementation. Until WT-009 lands,
//! `backend.detach_session_worktree`, `Action::MergeSuccessTeardown`, the
//! `App::handle_isolation_toggle` / `App::handle_merge_success_teardown`
//! routing, and the `MergeOutcome.worktree_path` field do not yet exist —
//! this file fails to compile, which is the canonical Rust red phase.
//!
//! Each feature scenario is exercised by exactly one `#[test]`
//! (`#[tokio::test]` for the async paths), and every Gherkin step has a
//! matching `// @step ...` comment placed immediately before the code that
//! exercises it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::components::create_session_dialog::CREATE_SESSION_DIALOG_ID;
use codelet_fspec_tui::views::agent::merge_confirm_dialog::MERGE_CONFIRM_DIALOG_ID;
use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend, IsolationState};
use codelet_rpc_types::{
    MergeOutcome, MergeStatus, SessionChangesSummary, SessionId, SessionWorktreeInfo,
};
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
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

/// Await every spawned backend task WITHOUT dispatching queued actions
/// (used to observe in-flight bus actions like the pre-teardown
/// `[merge] success` notice).
async fn drain_once(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
}

/// Await spawned tasks AND dispatch every queued bus action (up to 1s,
/// stopping after two consecutive quiet iterations). Use after a
/// dispatch that spawns backend work so the resulting
/// `EmitSessionNotice` / dialog actions reach the stores.
async fn settle(app: &mut App) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
    let mut quiet = 0u32;
    while quiet < 2 && tokio::time::Instant::now() < deadline {
        drain_once(app).await;
        let mut acted = false;
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
            acted = true;
        }
        if acted {
            drain_once(app).await;
        } else {
            quiet += 1;
            tokio::time::sleep(Duration::from_millis(5)).await;
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

fn fresh_app(mock: Arc<MockBackend>) -> App {
    let backend: Arc<dyn FspecBackend> = mock;
    App::new(backend)
}

/// Scrollback text for `id`, or `None` when the session context is no longer
/// open (e.g. after a merge-success teardown removed it).
fn session_scrollback_text_opt(app: &App, id: &SessionId) -> Option<String> {
    let ctx = app.agent_view_store().session_context_for(id)?;
    let chunks = ctx.scrollback.visible_window(1024);
    Some(
        chunks
            .iter()
            .flat_map(|c| {
                c.lines.iter().map(|l| {
                    l.spans
                        .iter()
                        .map(|s| s.content.as_ref())
                        .collect::<String>()
                })
            })
            .collect::<Vec<String>>()
            .join("\n"),
    )
}

fn open_session(app: &mut App, id: &str) {
    app.dispatch(Action::SessionCreated(sid(id)));
    app.navigator_mut().active_view = ViewMode::Agent;
}

/// Set the store's isolation state for `id` (mirrors what the
/// `IsolationStateChange` chunk pipeline would have recorded).
fn set_isolated(app: &mut App, id: &str, path: &str) {
    app.agent_view_store_mut()
        .set_isolation_state(
            sid(id),
            IsolationState {
                is_isolated: true,
                worktree_path: Some(path.to_string()),
                base_commit: None,
            },
        );
}

fn seed_worktree_row(mock: &MockBackend, id: &str) {
    mock.seed_session_worktrees(vec![SessionWorktreeInfo {
        session_id: sid(id),
        worktree_path: format!("/repo/.fspec/worktrees/{id}"),
        base_commit: "base123".to_string(),
        head_commit: "head456".to_string(),
        dirty: false,
    }]);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /isolation in a non-isolated session opens the create-session
// dialog preselecting Isolated
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn isolation_in_non_isolated_session_opens_create_dialog_preselecting_isolated() {
    // @step Given an App with open session s-1 whose isolation state is non-isolated
    let mock = Arc::new(MockBackend::new());
    let mut app = fresh_app(mock.clone());
    open_session(&mut app, "s-1");
    drain_pending(&mut app).await;
    app.agent_view_store_mut()
        .set_isolation_state(
            sid("s-1"),
            IsolationState {
                is_isolated: false,
                worktree_path: None,
                base_commit: None,
            },
        );

    // @step And the session worktree listing has no row for session s-1
    mock.seed_session_worktrees(Vec::new());

    // @step When SlashCommandSelected(SlashCommandAction::Isolation) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Isolation));
    drain_pending(&mut app).await;

    // @step Then a CreateSessionDialog is pushed onto the compositor with preselect "Yes - Isolated"
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "CreateSessionDialog should be on the compositor"
    );

    // @step And no backend worktree method is called
    assert_eq!(
        mock.detach_session_worktree_calls(),
        0,
        "non-isolated session must NOT call detach_session_worktree"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /isolation in an isolated session detaches the session from its
// worktree
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn isolation_in_isolated_session_detaches_from_worktree() {
    // @step Given an App with open session s-1 whose isolation state is isolated with worktree path "/repo/.fspec/worktrees/s-1"
    let mock = Arc::new(MockBackend::new());
    let mut app = fresh_app(mock.clone());
    open_session(&mut app, "s-1");
    drain_pending(&mut app).await;
    set_isolated(&mut app, "s-1", "/repo/.fspec/worktrees/s-1");

    // @step When SlashCommandSelected(SlashCommandAction::Isolation) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Isolation));

    // @step Then within 1 second backend.detach_session_worktree is called exactly once with session_id "s-1"
    wait_until(
        || mock.detach_session_worktree_calls() == 1,
        "detach_session_worktree called once",
    )
    .await;

    // @step And within 1 second Action::EmitSessionNotice for s-1 with text "[isolation] detached from worktree" is observed on the action bus
    settle(&mut app).await;
    assert!(
        session_scrollback_text_opt(&app, &sid("s-1"))
            .is_some_and(|t| t.contains("[isolation] detached from worktree")),
        "detached notice"
    );

    // @step And the CreateSessionDialog is NOT pushed onto the compositor
    assert!(
        !app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "detach path must not open the create-session dialog"
    );

    // @step And the session stays open in the AgentViewStore
    assert!(
        app.agent_view_store()
            .open_sessions()
            .iter()
            .any(|c| c.id == sid("s-1")),
        "detached session must remain open"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /isolation in an isolated session surfaces the detach outcome
// as an error when the backend fails
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn isolation_in_isolated_session_surfaces_detach_error_when_backend_fails() {
    // @step Given an App with open session s-1 whose isolation state is isolated with worktree path "/repo/.fspec/worktrees/s-1"
    let mock = Arc::new(MockBackend::new());
    let mut app = fresh_app(mock.clone());
    open_session(&mut app, "s-1");
    drain_pending(&mut app).await;
    set_isolated(&mut app, "s-1", "/repo/.fspec/worktrees/s-1");

    // @step And the backend's detach_session_worktree returns Err("worktree not found")
    mock.set_detach_session_worktree_error("worktree not found".to_string());

    // @step When SlashCommandSelected(SlashCommandAction::Isolation) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Isolation));

    // @step Then within 1 second Action::EmitSessionNotice for s-1 with text "[error] /isolation: worktree not found" is observed on the action bus
    settle(&mut app).await;
    assert!(
        session_scrollback_text_opt(&app, &sid("s-1"))
            .is_some_and(|t| t.contains("[error] /isolation: worktree not found")),
        "detach error notice"
    );

    // @step And the session's isolation state remains isolated
    let st = app
        .agent_view_store()
        .isolation_state_for(&sid("s-1"))
        .expect("isolation state present");
    assert!(
        st.is_isolated,
        "a failed detach must leave the session isolated"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /isolation with a live worktree the store does not track warns
// without a backend call
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn isolation_with_untracked_live_worktree_warns_without_backend_call() {
    // @step Given an App with open session s-1 whose isolation state is non-isolated
    let mock = Arc::new(MockBackend::new());
    let mut app = fresh_app(mock.clone());
    open_session(&mut app, "s-1");
    drain_pending(&mut app).await;
    app.agent_view_store_mut()
        .set_isolation_state(
            sid("s-1"),
            IsolationState {
                is_isolated: false,
                worktree_path: None,
                base_commit: None,
            },
        );

    // @step And the backend's session worktree listing contains a row for session s-1
    seed_worktree_row(&mock, "s-1");

    // @step When SlashCommandSelected(SlashCommandAction::Isolation) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Isolation));

    // @step Then within 1 second Action::EmitSessionNotice for s-1 with text containing "session still isolated" and "/merge-worktree" is observed on the action bus
    settle(&mut app).await;
    assert!(
        session_scrollback_text_opt(&app, &sid("s-1"))
            .is_some_and(|t| t.contains("session still isolated") && t.contains("/merge-worktree")),
        "still-isolated notice"
    );

    // @step And no backend detach_session_worktree call is made
    assert_eq!(
        mock.detach_session_worktree_calls(),
        0,
        "untracked live worktree must NOT call detach_session_worktree"
    );

    // @step And the CreateSessionDialog is NOT pushed onto the compositor
    assert!(
        !app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "still-isolated path must not open the create-session dialog"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /isolation with no active session is a silent no-op
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn isolation_with_no_active_session_is_silent_noop() {
    // @step Given an App with NO open AgentView session
    let mock = Arc::new(MockBackend::new());
    let mut app = fresh_app(mock.clone());

    let initial_detach = mock.detach_session_worktree_calls();
    let initial_list = mock.list_session_worktrees_calls();

    // @step When SlashCommandSelected(SlashCommandAction::Isolation) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Isolation));
    drain_pending(&mut app).await;

    // @step Then no backend method is called
    assert_eq!(
        mock.detach_session_worktree_calls(),
        initial_detach,
        "no-session isolation must not call detach"
    );
    assert_eq!(
        mock.list_session_worktrees_calls(),
        initial_list,
        "no-session isolation must not call list_session_worktrees"
    );

    // @step And the compositor contains no create-session-dialog
    assert!(
        !app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "no-session isolation must not open the create-session dialog"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Successful merge closes the session and returns to the board
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn successful_merge_closes_session_and_returns_to_board() {
    // @step Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    let mock = Arc::new(MockBackend::new());
    // @step And the backend's merge_session_worktree returns Ok(MergeOutcome { status: Success, conflicts: [], merge_commit: Some("abc1234"), worktree_path: Some("/repo/.fspec/worktrees/s-1") })
    mock.seed_merge_outcome(MergeOutcome {
        status: MergeStatus::Success,
        conflicts: Vec::new(),
        merge_commit: Some("abc1234".to_string()),
        worktree_path: Some("/repo/.fspec/worktrees/s-1".to_string()),
    });
    let mut app = fresh_app(mock.clone());
    open_session(&mut app, "s-1");
    drain_pending(&mut app).await;
    app.dispatch(Action::OpenMergeConfirmDialog {
        session_id: sid("s-1"),
        summary: SessionChangesSummary {
            files_changed: 1,
            insertions: 1,
            deletions: 0,
            commits: vec!["abc1234".to_string()],
            files_ignored: 0,
        },
    });
    drain_pending(&mut app).await;
    assert!(app.compositor().contains(MERGE_CONFIRM_DIALOG_ID));
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
    let initial_destroy = mock.destroy_session_calls();

    // @step When Action::MergeConfirmed { session_id: "s-1" } is dispatched
    app.dispatch(Action::MergeConfirmed {
        session_id: sid("s-1"),
    });

    // @step Then within 1 second Action::EmitSessionNotice for s-1 with text starting with "[merge] success" is observed on the action bus
    // (Captured on the bus BEFORE dispatching the follow-up
    // `MergeSuccess` teardown action — the teardown removes the session
    // context, so the notice is only observable in flight.)
    let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
    let mut saw_success = false;
    while !saw_success && tokio::time::Instant::now() < deadline {
        drain_once(&mut app).await;
        while let Some(action) = app.try_recv_action() {
            if let Action::EmitSessionNotice(id, text) = &action {
                if *id == sid("s-1") && text.starts_with("[merge] success") {
                    saw_success = true;
                }
            }
            app.dispatch(action);
            drain_once(&mut app).await;
        }
        if !saw_success {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    assert!(saw_success, "expected a [merge] success notice for s-1");

    // Drain the remaining actions (MergeSuccess teardown) to completion.
    drain_pending(&mut app).await;

    // @step And within 1 second backend.destroy_session is called exactly once with session_id "s-1"
    wait_until(
        || mock.destroy_session_calls() - initial_destroy == 1,
        "destroy_session called once",
    )
    .await;
    assert_eq!(
        mock.last_destroyed_session(),
        Some(sid("s-1")),
        "the destroyed session must be s-1"
    );

    // @step And session s-1 is removed from the AgentViewStore open sessions
    assert!(
        app.agent_view_store()
            .open_sessions()
            .iter()
            .all(|c| c.id != sid("s-1")),
        "merge success must remove s-1 from open_sessions"
    );

    // @step And the active view flips back to the board
    assert_eq!(
        app.navigator().active_view,
        ViewMode::Board,
        "merge success must return the user to the board"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: NoChanges merge keeps the session open without any teardown
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_changes_merge_keeps_session_open_without_teardown() {
    // @step Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    let mock = Arc::new(MockBackend::new());
    // @step And the backend's merge_session_worktree returns Ok(MergeOutcome { status: NoChanges, conflicts: [], merge_commit: None, worktree_path: None })
    mock.seed_merge_outcome(MergeOutcome {
        status: MergeStatus::NoChanges,
        conflicts: Vec::new(),
        merge_commit: None,
        worktree_path: None,
    });
    let mut app = fresh_app(mock.clone());
    open_session(&mut app, "s-1");
    drain_pending(&mut app).await;
    app.dispatch(Action::OpenMergeConfirmDialog {
        session_id: sid("s-1"),
        summary: SessionChangesSummary {
            files_changed: 0,
            insertions: 0,
            deletions: 0,
            commits: vec![],
            files_ignored: 0,
        },
    });
    drain_pending(&mut app).await;
    let initial_destroy = mock.destroy_session_calls();

    // @step When Action::MergeConfirmed { session_id: "s-1" } is dispatched
    app.dispatch(Action::MergeConfirmed {
        session_id: sid("s-1"),
    });
    drain_pending(&mut app).await;

    // @step Then within 1 second Action::EmitSessionNotice for s-1 with text "[merge] nothing to merge" is observed on the action bus
    wait_until(
        || session_scrollback_text_opt(&app, &sid("s-1"))
            .is_some_and(|t| t.contains("[merge] nothing to merge")),
        "nothing-to-merge notice",
    )
    .await;

    // @step And no backend destroy_session call is made
    assert_eq!(
        mock.destroy_session_calls(),
        initial_destroy,
        "NoChanges must not destroy the session"
    );

    // @step And session s-1 remains open in the AgentViewStore
    assert!(
        app.agent_view_store()
            .open_sessions()
            .iter()
            .any(|c| c.id == sid("s-1")),
        "NoChanges must keep s-1 open"
    );

    // @step And the active view does not change
    assert_eq!(
        app.navigator().active_view,
        ViewMode::Agent,
        "NoChanges must not flip the view to the board"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Conflict merge seeds the input with the worktree path instead
// of the session id
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conflict_merge_seeds_input_with_worktree_path() {
    // @step Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    let mock = Arc::new(MockBackend::new());
    // @step And the backend's merge_session_worktree returns Ok(MergeOutcome { status: Conflict, conflicts: ["src/a.rs"], merge_commit: None, worktree_path: Some("/repo/.fspec/worktrees/s-1") })
    mock.seed_merge_outcome(MergeOutcome {
        status: MergeStatus::Conflict,
        conflicts: vec!["src/a.rs".to_string()],
        merge_commit: None,
        worktree_path: Some("/repo/.fspec/worktrees/s-1".to_string()),
    });
    let mut app = fresh_app(mock.clone());
    open_session(&mut app, "s-1");
    drain_pending(&mut app).await;
    app.dispatch(Action::OpenMergeConfirmDialog {
        session_id: sid("s-1"),
        summary: SessionChangesSummary {
            files_changed: 1,
            insertions: 0,
            deletions: 0,
            commits: vec![],
            files_ignored: 0,
        },
    });
    drain_pending(&mut app).await;
    let initial_destroy = mock.destroy_session_calls();

    // @step When Action::MergeConfirmed { session_id: "s-1" } is dispatched
    app.dispatch(Action::MergeConfirmed {
        session_id: sid("s-1"),
    });
    drain_pending(&mut app).await;

    // @step Then within 1 second the seeded input draft contains "Effective worktree: /repo/.fspec/worktrees/s-1"
    wait_until(
        || app
            .agent_view_store()
            .session_context_for(&sid("s-1"))
            .map(|ctx| ctx.input_draft.contains("Effective worktree: /repo/.fspec/worktrees/s-1"))
            .unwrap_or(false),
        "seeded conflict context with worktree path",
    )
    .await;

    // @step And no backend destroy_session call is made
    assert_eq!(
        mock.destroy_session_calls(),
        initial_destroy,
        "Conflict must not destroy the session"
    );

    // @step And session s-1 remains open in the AgentViewStore
    assert!(
        app.agent_view_store()
            .open_sessions()
            .iter()
            .any(|c| c.id == sid("s-1")),
        "Conflict must keep s-1 open"
    );
}
