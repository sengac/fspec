//! RPC-057 — /merge-worktree slash command + MergeConfirmDialog end-to-end.
//!
//! Feature: spec/features/rpc057-merge-worktree-dispatch.feature
//!
//! Drives the App::dispatch routing for `SlashCommandSelected(MergeWorktree)`
//! through the `inspect_session_changes` round-trip and the MergeConfirmDialog
//! opening, focus traversal, key handling, and the post-confirm
//! `merge_session_worktree` / `discard_session_worktree` routing.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::views::agent::merge_confirm_dialog::{
    MergeConfirmDialog, MergeConfirmDialogOutcome,
};
use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{MergeOutcome, MergeStatus, SessionChangesSummary, SessionId};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
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

fn render_dialog(dialog: &MergeConfirmDialog) -> String {
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
// Scenario: /merge-worktree with no current session is a silent no-op
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_merge_worktree_with_no_session_is_silent_noop() {
    // @step Given an App with NO open AgentView session
    let mock = Arc::new(MockBackend::new());
    let mut app = fresh_app(mock.clone());

    let initial_inspect = mock.inspect_session_changes_calls();
    let initial_merge = mock.merge_session_worktree_calls();

    // @step When SlashCommandSelected(SlashCommandAction::MergeWorktree) is dispatched
    app.dispatch(Action::SlashCommandSelected(
        SlashCommandAction::MergeWorktree,
    ));
    drain_pending(&mut app).await;

    // @step Then no backend method is called
    assert_eq!(mock.inspect_session_changes_calls(), initial_inspect);
    assert_eq!(mock.merge_session_worktree_calls(), initial_merge);

    // @step And no scrollback notice is emitted
    // (No notices because no session — agent_view_store has no current session)

    // @step And the compositor contains no merge-confirm-dialog
    assert!(!app.compositor().contains("merge-confirm-dialog"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /merge-worktree on a session with no changes emits "nothing to merge"
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_merge_worktree_with_no_changes_emits_nothing_to_merge() {
    // @step Given an App with open session s-1 wired to a MockBackend whose inspect_session_changes returns zero changes
    let mock = Arc::new(MockBackend::new());
    mock.seed_session_changes_summary(SessionChangesSummary {
        files_changed: 0,
        insertions: 0,
        deletions: 0,
        commits: vec![],
    });
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    let initial_inspect = mock.inspect_session_changes_calls();

    // @step When SlashCommandSelected(SlashCommandAction::MergeWorktree) is dispatched
    app.dispatch(Action::SlashCommandSelected(
        SlashCommandAction::MergeWorktree,
    ));
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.inspect_session_changes is called exactly once with session_id "s-1"
    wait_until(
        || mock.inspect_session_changes_calls() - initial_inspect == 1,
        "inspect_session_changes called once",
    )
    .await;

    // @step And within 1 second Action::EmitSessionNotice carrying "[merge] nothing to merge" for s-1 is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1")).contains("[merge] nothing to merge"),
        "nothing-to-merge notice",
    )
    .await;

    // @step And no merge-confirm-dialog is pushed onto the compositor
    assert!(!app.compositor().contains("merge-confirm-dialog"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /merge-worktree on a session with changes opens the MergeConfirmDialog
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_merge_worktree_opens_dialog_on_changes() {
    // @step Given an App with open session s-1 wired to a MockBackend whose inspect_session_changes returns SessionChangesSummary { files_changed: 1, insertions: 4, deletions: 2, commits: ["abc1234"] }
    let mock = Arc::new(MockBackend::new());
    mock.seed_session_changes_summary(SessionChangesSummary {
        files_changed: 1,
        insertions: 4,
        deletions: 2,
        commits: vec!["abc1234".to_string()],
    });
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    let initial_inspect = mock.inspect_session_changes_calls();

    // @step When SlashCommandSelected(SlashCommandAction::MergeWorktree) is dispatched
    app.dispatch(Action::SlashCommandSelected(
        SlashCommandAction::MergeWorktree,
    ));
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.inspect_session_changes is called exactly once with session_id "s-1"
    wait_until(
        || mock.inspect_session_changes_calls() - initial_inspect == 1,
        "inspect_session_changes called once",
    )
    .await;

    // @step And within 1 second the compositor contains a layer with id "merge-confirm-dialog"
    wait_until(
        || app.compositor().contains("merge-confirm-dialog"),
        "merge-confirm-dialog on compositor",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: MergeConfirmDialog renders the change summary
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn merge_confirm_dialog_renders_change_summary() {
    // @step Given a MergeConfirmDialog seeded with SessionChangesSummary { files_changed: 2, insertions: 10, deletions: 3, commits: ["abc1234", "def5678"] }
    let dialog = MergeConfirmDialog::new(
        sid("s-1"),
        SessionChangesSummary {
            files_changed: 2,
            insertions: 10,
            deletions: 3,
            commits: vec!["abc1234".to_string(), "def5678".to_string()],
        },
    );

    // @step When the dialog is rendered into a 80x24 buffer
    let text = render_dialog(&dialog);

    // @step Then the rendered text contains "2 files changed"
    assert!(
        text.contains("2 files changed"),
        "missing '2 files changed': {text}"
    );
    // @step And the rendered text contains "+10"
    assert!(text.contains("+10"), "missing '+10': {text}");
    // @step And the rendered text contains "-3"
    assert!(text.contains("-3"), "missing '-3': {text}");
    // @step And the rendered text contains "Merge"
    assert!(text.contains("Merge"));
    // @step And the rendered text contains "Discard"
    assert!(text.contains("Discard"));
    // @step And the rendered text contains "Cancel"
    assert!(text.contains("Cancel"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: MergeConfirmDialog opens with Merge focused; Tab cycles forward
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn merge_confirm_dialog_tab_cycles_focus_forward() {
    // @step Given a fresh MergeConfirmDialog
    let mut dialog = MergeConfirmDialog::new(
        sid("s-1"),
        SessionChangesSummary {
            files_changed: 1,
            insertions: 1,
            deletions: 0,
            commits: vec![],
        },
    );

    // @step Then the focused button index equals 0
    assert_eq!(dialog.focused_button(), 0);
    // @step When the user presses Tab
    dialog.handle_key(KeyCode::Tab, KeyModifiers::NONE);
    // @step Then the focused button index equals 1
    assert_eq!(dialog.focused_button(), 1);
    // @step When the user presses Tab
    dialog.handle_key(KeyCode::Tab, KeyModifiers::NONE);
    // @step Then the focused button index equals 2
    assert_eq!(dialog.focused_button(), 2);
    // @step When the user presses Tab
    dialog.handle_key(KeyCode::Tab, KeyModifiers::NONE);
    // @step Then the focused button index equals 0
    assert_eq!(dialog.focused_button(), 0);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: MergeConfirmDialog handle_key Enter on Merge emits MergeConfirmed
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn merge_confirm_dialog_enter_on_merge_emits_merge() {
    // @step Given a fresh MergeConfirmDialog for session s-1 with the Merge button focused
    let mut dialog = MergeConfirmDialog::new(
        sid("s-1"),
        SessionChangesSummary {
            files_changed: 1,
            insertions: 1,
            deletions: 0,
            commits: vec![],
        },
    );
    assert_eq!(dialog.focused_button(), 0);

    // @step When the user presses Enter
    let outcome = dialog.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    // @step Then handle_key returns MergeConfirmDialogOutcome::Merge
    assert!(matches!(outcome, MergeConfirmDialogOutcome::Merge { .. }));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: MergeConfirmDialog handle_key Enter on Discard emits DiscardConfirmed
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn merge_confirm_dialog_enter_on_discard_emits_discard() {
    // @step Given a fresh MergeConfirmDialog for session s-1 with the Discard button focused
    let mut dialog = MergeConfirmDialog::new(
        sid("s-1"),
        SessionChangesSummary {
            files_changed: 1,
            insertions: 1,
            deletions: 0,
            commits: vec![],
        },
    );
    dialog.handle_key(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(dialog.focused_button(), 1);

    // @step When the user presses Enter
    let outcome = dialog.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    // @step Then handle_key returns MergeConfirmDialogOutcome::Discard
    assert!(matches!(outcome, MergeConfirmDialogOutcome::Discard { .. }));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: MergeConfirmDialog handle_key Esc emits Cancel regardless of focus
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn merge_confirm_dialog_esc_emits_cancel() {
    // @step Given a fresh MergeConfirmDialog for session s-1 with the Merge button focused
    let mut dialog = MergeConfirmDialog::new(
        sid("s-1"),
        SessionChangesSummary {
            files_changed: 1,
            insertions: 1,
            deletions: 0,
            commits: vec![],
        },
    );

    // @step When the user presses Esc
    let outcome = dialog.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    // @step Then handle_key returns MergeConfirmDialogOutcome::Cancel
    assert!(matches!(outcome, MergeConfirmDialogOutcome::Cancel));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Action::MergeConfirmed routes through backend.merge_session_worktree (Success)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn merge_confirmed_success_emits_success_notice() {
    // @step Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    let mock = Arc::new(MockBackend::new());
    // @step And the backend's merge_session_worktree returns Ok(MergeOutcome { status: Success, conflicts: [], merge_commit: Some("abc1234") })
    mock.seed_merge_outcome(MergeOutcome {
        status: MergeStatus::Success,
        conflicts: vec![],
        merge_commit: Some("abc1234".to_string()),
    });
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;
    app.dispatch(Action::OpenMergeConfirmDialog {
        session_id: sid("s-1"),
        summary: SessionChangesSummary {
            files_changed: 1,
            insertions: 1,
            deletions: 0,
            commits: vec!["abc1234".to_string()],
        },
    });
    drain_pending(&mut app).await;
    assert!(app.compositor().contains("merge-confirm-dialog"));

    let initial = mock.merge_session_worktree_calls();

    // @step When Action::MergeConfirmed { session_id: "s-1" } is dispatched
    app.dispatch(Action::MergeConfirmed {
        session_id: sid("s-1"),
    });
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.merge_session_worktree is called exactly once with session_id "s-1"
    wait_until(
        || mock.merge_session_worktree_calls() - initial == 1,
        "merge_session_worktree called once",
    )
    .await;

    // @step And within 1 second the compositor no longer contains a layer with id "merge-confirm-dialog"
    wait_until(
        || !app.compositor().contains("merge-confirm-dialog"),
        "dialog popped",
    )
    .await;

    // @step And within 1 second Action::EmitSessionNotice for s-1 with text starting with "[merge] success" is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1")).contains("[merge] success"),
        "success notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Action::MergeConfirmed with NoChanges emits nothing-to-merge notice
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn merge_confirmed_no_changes_emits_nothing_to_merge_notice() {
    // @step Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    let mock = Arc::new(MockBackend::new());
    // @step And the backend's merge_session_worktree returns Ok(MergeOutcome { status: NoChanges, conflicts: [], merge_commit: None })
    mock.seed_merge_outcome(MergeOutcome {
        status: MergeStatus::NoChanges,
        conflicts: vec![],
        merge_commit: None,
    });
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;
    app.dispatch(Action::OpenMergeConfirmDialog {
        session_id: sid("s-1"),
        summary: SessionChangesSummary {
            files_changed: 0,
            insertions: 0,
            deletions: 0,
            commits: vec![],
        },
    });
    drain_pending(&mut app).await;

    // @step When Action::MergeConfirmed { session_id: "s-1" } is dispatched
    app.dispatch(Action::MergeConfirmed {
        session_id: sid("s-1"),
    });
    drain_pending(&mut app).await;

    // @step Then within 1 second Action::EmitSessionNotice for s-1 with text "[merge] nothing to merge" is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1")).contains("[merge] nothing to merge"),
        "nothing-to-merge notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Action::MergeConfirmed with Conflict seeds the input
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn merge_confirmed_conflict_seeds_input() {
    // @step Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    let mock = Arc::new(MockBackend::new());
    // @step And the backend's merge_session_worktree returns Ok(MergeOutcome { status: Conflict, conflicts: ["src/a.rs", "src/b.rs"], merge_commit: None })
    mock.seed_merge_outcome(MergeOutcome {
        status: MergeStatus::Conflict,
        conflicts: vec!["src/a.rs".to_string(), "src/b.rs".to_string()],
        merge_commit: None,
    });
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;
    app.dispatch(Action::OpenMergeConfirmDialog {
        session_id: sid("s-1"),
        summary: SessionChangesSummary {
            files_changed: 2,
            insertions: 0,
            deletions: 0,
            commits: vec![],
        },
    });
    drain_pending(&mut app).await;

    // @step When Action::MergeConfirmed { session_id: "s-1" } is dispatched
    app.dispatch(Action::MergeConfirmed {
        session_id: sid("s-1"),
    });
    drain_pending(&mut app).await;

    // @step Then within 1 second Action::SeedPendingInput for s-1 with text containing "Merge produced conflicts" is observed on the action bus
    // @step And the seeded text contains "src/a.rs"
    // @step And the seeded text contains "src/b.rs"
    wait_until(
        || {
            app.agent_view_store()
                .session_context_for(&sid("s-1"))
                .map(|ctx| {
                    ctx.input_draft.contains("Merge produced conflicts")
                        && ctx.input_draft.contains("src/a.rs")
                        && ctx.input_draft.contains("src/b.rs")
                })
                .unwrap_or(false)
        },
        "SeedPendingInput with conflict context",
    )
    .await;

    // @step And the compositor no longer contains a layer with id "merge-confirm-dialog"
    wait_until(
        || !app.compositor().contains("merge-confirm-dialog"),
        "dialog popped",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Action::MergeConfirmed with Err emits error notice
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn merge_confirmed_err_emits_error_notice() {
    // @step Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    let mock = Arc::new(MockBackend::new());
    // @step And the backend's merge_session_worktree returns Err("worktree not found")
    mock.set_merge_session_worktree_error("worktree not found".to_string());
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;
    app.dispatch(Action::OpenMergeConfirmDialog {
        session_id: sid("s-1"),
        summary: SessionChangesSummary {
            files_changed: 1,
            insertions: 1,
            deletions: 0,
            commits: vec![],
        },
    });
    drain_pending(&mut app).await;

    // @step When Action::MergeConfirmed { session_id: "s-1" } is dispatched
    app.dispatch(Action::MergeConfirmed {
        session_id: sid("s-1"),
    });
    drain_pending(&mut app).await;

    // @step Then within 1 second Action::EmitSessionNotice for s-1 with text "[error] /merge-worktree: worktree not found" is observed on the action bus
    wait_until(
        || {
            session_scrollback_text(&app, &sid("s-1"))
                .contains("[error] /merge-worktree: worktree not found")
        },
        "error notice",
    )
    .await;

    // @step And no Action::SeedPendingInput is observed
    let draft = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .map(|ctx| ctx.input_draft.clone())
        .unwrap_or_default();
    assert!(
        !draft.contains("Merge produced conflicts"),
        "Err path must not seed pending input; draft={draft:?}"
    );

    // @step And the compositor no longer contains a layer with id "merge-confirm-dialog"
    assert!(!app.compositor().contains("merge-confirm-dialog"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Action::DiscardConfirmed routes through backend.discard_session_worktree
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discard_confirmed_routes_through_backend() {
    // @step Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    let mock = Arc::new(MockBackend::new());
    // @step And the backend's discard_session_worktree returns Ok(())
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;
    app.dispatch(Action::OpenMergeConfirmDialog {
        session_id: sid("s-1"),
        summary: SessionChangesSummary {
            files_changed: 1,
            insertions: 1,
            deletions: 0,
            commits: vec![],
        },
    });
    drain_pending(&mut app).await;

    let initial = mock.discard_session_worktree_calls();

    // @step When Action::DiscardConfirmed { session_id: "s-1" } is dispatched
    app.dispatch(Action::DiscardConfirmed {
        session_id: sid("s-1"),
    });
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.discard_session_worktree is called exactly once with session_id "s-1"
    wait_until(
        || mock.discard_session_worktree_calls() - initial == 1,
        "discard_session_worktree called once",
    )
    .await;

    // @step And within 1 second the compositor no longer contains a layer with id "merge-confirm-dialog"
    wait_until(
        || !app.compositor().contains("merge-confirm-dialog"),
        "dialog popped",
    )
    .await;

    // @step And within 1 second Action::EmitSessionNotice for s-1 with text "[discard] worktree discarded" is observed on the action bus
    wait_until(
        || session_scrollback_text(&app, &sid("s-1")).contains("[discard] worktree discarded"),
        "discard notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Action::DiscardConfirmed with Err emits error notice
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discard_confirmed_err_emits_error_notice() {
    // @step Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    let mock = Arc::new(MockBackend::new());
    // @step And the backend's discard_session_worktree returns Err("worktree not found")
    mock.set_discard_session_worktree_error("worktree not found".to_string());
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;
    app.dispatch(Action::OpenMergeConfirmDialog {
        session_id: sid("s-1"),
        summary: SessionChangesSummary {
            files_changed: 1,
            insertions: 1,
            deletions: 0,
            commits: vec![],
        },
    });
    drain_pending(&mut app).await;

    // @step When Action::DiscardConfirmed { session_id: "s-1" } is dispatched
    app.dispatch(Action::DiscardConfirmed {
        session_id: sid("s-1"),
    });
    drain_pending(&mut app).await;

    // @step Then within 1 second Action::EmitSessionNotice for s-1 with text "[error] /merge-worktree discard: worktree not found" is observed on the action bus
    wait_until(
        || {
            session_scrollback_text(&app, &sid("s-1"))
                .contains("[error] /merge-worktree discard: worktree not found")
        },
        "discard error notice",
    )
    .await;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Action::CancelMergeDialog pops dialog without firing backend
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_merge_dialog_pops_without_backend_call() {
    // @step Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    let mock = Arc::new(MockBackend::new());
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;
    app.dispatch(Action::OpenMergeConfirmDialog {
        session_id: sid("s-1"),
        summary: SessionChangesSummary {
            files_changed: 1,
            insertions: 1,
            deletions: 0,
            commits: vec![],
        },
    });
    drain_pending(&mut app).await;

    let initial_merge = mock.merge_session_worktree_calls();
    let initial_discard = mock.discard_session_worktree_calls();

    // @step When Action::CancelMergeDialog is dispatched
    app.dispatch(Action::CancelMergeDialog);
    drain_pending(&mut app).await;

    // @step Then the compositor no longer contains a layer with id "merge-confirm-dialog"
    assert!(!app.compositor().contains("merge-confirm-dialog"));

    // @step And no backend method is called
    assert_eq!(mock.merge_session_worktree_calls(), initial_merge);
    assert_eq!(mock.discard_session_worktree_calls(), initial_discard);

    // @step And no scrollback notice is emitted
    let text = session_scrollback_text(&app, &sid("s-1"));
    assert!(!text.contains("[merge]"));
    assert!(!text.contains("[discard]"));
    assert!(!text.contains("[error]"));
}
