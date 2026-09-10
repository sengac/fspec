//! WT-005 — /worktrees slash command: session-worktree listing overlay +
//! Prune action wired to the backend prune RPC.
//!
//! Feature: spec/features/the-worktrees-slash-command-lists-session-worktrees-and-prunes-leaked-ones.feature
//!
//! Drives the App::dispatch routing for `SlashCommandSelected(Worktrees)`
//! through the `list_session_worktrees` round-trip and the
//! SessionWorktreesDialog opening + key handling, plus the
//! `PruneLeakedWorktrees` action routing into
//! `prune_orphaned_worktrees` and the outcome notices.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::views::agent::session_worktrees_dialog::{
    SessionWorktreesDialog, SessionWorktreesDialogOutcome, SESSION_WORKTREES_DIALOG_ID,
};
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, SessionWorktreeInfo};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn row(session_id: &str, dirty: bool) -> SessionWorktreeInfo {
    SessionWorktreeInfo {
        session_id: sid(session_id),
        worktree_path: format!("/tmp/repo/.fspec/worktrees/{session_id}"),
        base_commit: "abc1234".to_string(),
        head_commit: "def5678".to_string(),
        dirty,
    }
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
    timeout(Duration::from_secs(2), async {
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

fn session_scrollback_text(app: &App, id: &SessionId) -> String {
    let chunks = app
        .agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default();
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
        .join("\n")
}

fn render_dialog(dialog: &SessionWorktreesDialog) -> String {
    let mut term = Terminal::new(TestBackend::new(80, 24)).expect("Terminal::new");
    term.draw(|frame| {
        dialog.render(frame.area(), frame.buffer_mut());
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }
    joined
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The /worktrees slash command lists session worktrees
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_worktrees_lists_session_worktrees() {
    // @step Given a repository with two session worktrees for closed sessions
    let mock = Arc::new(MockBackend::new());
    mock.seed_session_worktrees(vec![row("w-1", true), row("w-2", false)]);
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    // @step And a TUI app running with a session
    let initial_list = mock.list_session_worktrees_calls();

    // @step When I run "/worktrees"
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Worktrees));
    drain_pending(&mut app).await;

    // @step Then the session worktrees dialog is shown
    wait_until(
        || mock.list_session_worktrees_calls() - initial_list == 1,
        "list_session_worktrees called once",
    )
    .await;
    wait_until(
        || app.compositor().contains(SESSION_WORKTREES_DIALOG_ID),
        "session-worktrees-dialog on compositor",
    )
    .await;

    // @step And the dialog lists one row per worktree with the session id and worktree path
    let dialog =
        SessionWorktreesDialog::new(vec![row("w-1", true), row("w-2", false)]);
    let text = render_dialog(&dialog);
    assert!(
        text.contains("w-1"),
        "row 1 session id missing:\n{text}"
    );
    assert!(
        text.contains("/tmp/repo/.fspec/worktrees/w-1"),
        "row 1 worktree path missing:\n{text}"
    );
    assert!(
        text.contains("w-2"),
        "row 2 session id missing:\n{text}"
    );

    // @step And dirty worktrees are marked dirty
    let marker_count = text.matches("[dirty]").count();
    assert_eq!(
        marker_count, 1,
        "exactly one [dirty] marker expected, got {marker_count}:\n{text}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The /worktrees slash command with no worktrees shows an
// empty-state row
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_worktrees_with_no_worktrees_shows_an_empty_state_row() {
    // @step Given a repository with no session worktrees
    let mock = Arc::new(MockBackend::new());
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    // @step And a TUI app running with a session
    // @step When I run "/worktrees"
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Worktrees));
    drain_pending(&mut app).await;

    // @step Then the session worktrees dialog is shown
    wait_until(
        || app.compositor().contains(SESSION_WORKTREES_DIALOG_ID),
        "session-worktrees-dialog on compositor",
    )
    .await;

    // @step And the dialog shows an empty-state row instead of worktree rows
    let dialog = SessionWorktreesDialog::new(Vec::new());
    let text = render_dialog(&dialog);
    assert!(
        text.contains("(no session worktrees)"),
        "empty-state row missing:\n{text}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// SessionWorktreesDialog key handling
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn session_worktrees_dialog_enter_on_prune_emits_prune_leaked_worktrees() {
    // @step Given a SessionWorktreesDialog with Prune focused
    let mut dialog =
        SessionWorktreesDialog::new(vec![row("w-1", true), row("w-2", false)]);
    assert_eq!(dialog.focused_button(), 0);

    // @step When the user presses Enter
    let outcome = dialog.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    // @step Then handle_key returns the Prune outcome
    assert!(
        matches!(outcome, SessionWorktreesDialogOutcome::Prune),
        "expected Prune outcome, got {outcome:?}"
    );
}

#[test]
fn session_worktrees_dialog_esc_emits_cancel_regardless_of_focus() {
    // @step Given a SessionWorktreesDialog
    let mut dialog = SessionWorktreesDialog::new(vec![row("w-1", true)]);
    dialog.handle_key(KeyCode::Tab, KeyModifiers::NONE); // move focus off Prune

    // @step When the user presses Esc
    let outcome = dialog.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    // @step Then handle_key returns the Cancel outcome
    assert!(
        matches!(outcome, SessionWorktreesDialogOutcome::Cancel),
        "expected Cancel outcome, got {outcome:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Pruning from the /worktrees dialog removes leaked worktrees
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pruning_from_the_worktrees_dialog_removes_leaked_worktrees() {
    // @step Given a repository with one leaked session worktree for a terminated session
    let mock = Arc::new(MockBackend::new());
    mock.seed_pruned_sessions(vec!["w-1".to_string()]);
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    // @step And a TUI app running with the session worktrees dialog open
    let initial_prune = mock.prune_orphaned_worktrees_calls();

    // @step When I press Prune in the dialog
    app.dispatch(Action::PruneLeakedWorktrees);
    drain_pending(&mut app).await;

    // @step Then the worktree directory is removed from the repository
    wait_until(
        || mock.prune_orphaned_worktrees_calls() - initial_prune == 1,
        "prune_orphaned_worktrees called once",
    )
    .await;

    // @step And I see the notice "[worktrees] pruned 1 leaked worktree(s)"
    wait_until(
        || {
            session_scrollback_text(&app, &sid("s-1"))
                .contains("[worktrees] pruned 1 leaked worktree(s)")
        },
        "pruned-1 notice in scrollback",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Pruning when nothing is leaked reports no leaked worktrees
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pruning_when_nothing_is_leaked_reports_no_leaked_worktrees() {
    // @step Given a repository with no leaked session worktrees
    let mock = Arc::new(MockBackend::new());
    // default seed: empty pruned list
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    // @step And a TUI app running with the session worktrees dialog open
    // @step When I press Prune in the dialog
    app.dispatch(Action::PruneLeakedWorktrees);
    drain_pending(&mut app).await;

    // @step Then I see the notice "[worktrees] no leaked worktrees to prune"
    wait_until(
        || {
            session_scrollback_text(&app, &sid("s-1"))
                .contains("[worktrees] no leaked worktrees to prune")
        },
        "no-leaked notice in scrollback",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Prune failures surface an error notice
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prune_failures_surface_an_error_notice() {
    // @step Given a TUI app running with the session worktrees dialog open
    let mock = Arc::new(MockBackend::new());
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    // @step And the backend prune call fails with "boom"
    mock.set_prune_orphaned_worktrees_error("boom".to_string());

    // @step When I press Prune in the dialog
    app.dispatch(Action::PruneLeakedWorktrees);
    drain_pending(&mut app).await;

    // @step Then I see the notice "[error] /worktrees prune: boom"
    wait_until(
        || {
            session_scrollback_text(&app, &sid("s-1"))
                .contains("[error] /worktrees prune: boom")
        },
        "error notice in scrollback",
    )
    .await;
}
