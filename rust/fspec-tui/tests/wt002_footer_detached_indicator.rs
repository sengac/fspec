//! WT-002 — TUI footer renders a detached indicator for a detached-HEAD
//! worktree (and still collapses to a blank branch for non-repo CWDs).
//!
//! Feature: spec/features/tui-footer-detached-indicator.feature
//!
//! Drives `App::dispatch(Action::ChunkReceived(SessionId,
//! StreamChunk::FooterStateUpdate))` for the two git-state renderings the
//! WT-002 poller emits: `is_git_repo=true, branch=None` (detached-HEAD
//! worktree → visible `(detached)` indicator) and `is_git_repo=false`
//! (non-repo CWD → blank branch).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk};

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The footer renders a detached indicator for a detached-HEAD
// worktree instead of a blank branch
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn wt002_detached_worktree_footer_keeps_cwd_and_shows_detached_indicator() {
    // @step Given an App with an open session s-1
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::FooterStateUpdate { cwd: "<worktree>", display_path: "<worktree>", is_git_repo: true, branch: None })
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::FooterStateUpdate {
            cwd: "/home/u/.fspec/worktrees/s-1".to_string(),
            display_path: "/home/u/.fspec/worktrees/s-1".to_string(),
            is_git_repo: true,
            branch: None,
        },
    ));

    // @step Then the stored workspace info carries the worktree CWD
    let ws = app
        .agent_view_store()
        .workspace()
        .expect("workspace info must be set for a detached worktree");
    assert_eq!(
        ws.cwd, "/home/u/.fspec/worktrees/s-1",
        "the worktree CWD must reach the footer"
    );

    // @step And the footer shows the worktree path with a detached indicator rather than a blank branch
    assert_eq!(
        ws.git_branch.as_deref(),
        Some("(detached)"),
        "WT-002: is_git_repo=true + branch=None must render a visible detached indicator, not a blank branch"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The footer still collapses to a blank branch for a CWD that is
// not a git repository
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn wt002_non_repo_footer_still_collapses_to_a_blank_branch() {
    // @step Given an App with an open session s-1
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::FooterStateUpdate { cwd: "/tmp/not-a-repo", display_path: "/tmp/not-a-repo", is_git_repo: false, branch: None })
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::FooterStateUpdate {
            cwd: "/tmp/not-a-repo".to_string(),
            display_path: "/tmp/not-a-repo".to_string(),
            is_git_repo: false,
            branch: None,
        },
    ));
    // @step Then the stored workspace info carries the CWD
    let ws = app
        .agent_view_store()
        .workspace()
        .expect("workspace info must be set");
    assert_eq!(ws.cwd, "/tmp/not-a-repo");
    // @step And the footer shows no branch indicator for a non-repo CWD
    assert!(
        ws.git_branch.is_none(),
        "a non-repo CWD must not show a branch indicator"
    );
}
