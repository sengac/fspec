//! WT-006 — /merge-worktree with only gitignored changes emits
//! "nothing to merge".
//!
//! Feature: spec/features/merge-worktree-ignored-only-nothing-to-merge.feature
//!
//! The wire summary gains a `files_ignored` count. A worktree whose only
//! delta is gitignored artifacts (build output, logs) must still report
//! zero changed files, so the slash command stays honest: ignored-only
//! sessions emit the nothing-to-merge notice and never open the merge
//! dialog.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App};
use codelet_rpc_types::{SessionChangesSummary, SessionId};
use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn fresh_app(mock: Arc<MockBackend>) -> App {
    let backend: Arc<dyn codelet_fspec_tui::FspecBackend> = mock;
    App::new(backend)
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

/// Scenario: /merge-worktree with only ignored changes emits
/// "nothing to merge".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_merge_worktree_with_only_ignored_changes_emits_nothing_to_merge() {
    // @step Given an App with open session s-1 wired to a MockBackend whose inspect_session_changes returns SessionChangesSummary { files_changed: 0, insertions: 0, deletions: 0, commits: [], files_ignored: 3 }
    let mock = Arc::new(MockBackend::new());
    mock.seed_session_changes_summary(SessionChangesSummary {
        files_changed: 0,
        insertions: 0,
        deletions: 0,
        commits: vec![],
        files_ignored: 3,
    });
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    // @step When SlashCommandSelected(SlashCommandAction::MergeWorktree) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::MergeWorktree));
    drain_pending(&mut app).await;

    // @step Then within 1 second Action::EmitSessionNotice carrying "[merge] nothing to merge" for s-1 is observed on the action bus
    let chunks = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default();
    let text: String = chunks
        .iter()
        .flat_map(|c| c.lines.iter().map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>()))
        .collect::<Vec<String>>()
        .join("\n");
    assert!(
        text.contains("[merge] nothing to merge"),
        "WT-006: an ignored-only worktree must say 'nothing to merge', scrollback: {text}"
    );

    // @step And no merge-confirm-dialog is pushed onto the compositor
    assert!(
        !app.compositor().contains("merge-confirm-dialog"),
        "WT-006: ignored files must never open the merge dialog"
    );
}
