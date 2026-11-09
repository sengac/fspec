//! Feature: spec/features/fspec-binary-footer-poller.feature
use super::*;

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_fspec_binary_session_receives_footer_state_update_on_first_poll_tick() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a session manager configured with the fspec binary's hook implementation (FspecAgentHooks)
    // (build_service wiring: hooks + the manager's chunks_tx registered as
    // the footer-poller emission target — the non-singleton manager the
    // binary owns.)
    let manager = manager_with_seeded_cache();
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));
    codelet_sessions::footer_poller::register_chunk_sender(manager.chunks_tx().clone());
    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step And a new session has been created in a git repository that has a branch checked out
    let repo = fresh_git_repo();
    let project = repo.path().to_str().expect("repo path is utf8");
    let sid = Uuid::new_v4();
    manager
        .create_session_with_id(
            &sid.to_string(),
            "anthropic/claude-opus-4-5",
            project,
            "wt002",
        )
        .await
        .expect("session creation under FspecAgentHooks must succeed");

    // @step When the footer poller's first tick runs
    // @step Then a FooterStateUpdate chunk is emitted on that manager's chunks broadcast for the session
    // (the first tick fires immediately — the first_run gate)
    let chunk = next_footer_chunk(&mut chunks_rx, &SessionId::new(sid.to_string()), |c| {
        matches!(c, StreamChunk::FooterStateUpdate { .. })
    })
    .await;

    // @step And the chunk carries the session's working directory and the checked-out branch name
    match chunk {
        StreamChunk::FooterStateUpdate {
            cwd,
            display_path: _,
            is_git_repo,
            branch,
        } => {
            assert_eq!(
                cwd, project,
                "chunk must carry the session's working directory"
            );
            assert!(is_git_repo, "the fresh repo must be reported as a git repo");
            assert_eq!(
                branch.as_deref(),
                Some("main"),
                "chunk must carry the checked-out branch"
            );
        }
        other => panic!("expected FooterStateUpdate chunk, got {other:?}"),
    }
}

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_session_in_git_repo_shows_branch_in_footer_without_any_bash_command_first() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a session manager configured with the fspec binary's hook implementation (FspecAgentHooks)
    let manager = manager_with_seeded_cache();
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));
    codelet_sessions::footer_poller::register_chunk_sender(manager.chunks_tx().clone());
    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step And a new session has been created in a git repository that has a branch checked out
    let repo = fresh_git_repo();
    let project = repo.path().to_str().expect("repo path is utf8");
    let sid = Uuid::new_v4();
    manager
        .create_session_with_id(
            &sid.to_string(),
            "anthropic/claude-opus-4-5",
            project,
            "wt002",
        )
        .await
        .expect("session creation under FspecAgentHooks must succeed");

    // @step When I observe the footer state for that session
    let chunk = next_footer_chunk(&mut chunks_rx, &SessionId::new(sid.to_string()), |c| {
        matches!(c, StreamChunk::FooterStateUpdate { .. })
    })
    .await;

    // @step Then the footer state reports the working directory and the branch name
    match chunk {
        StreamChunk::FooterStateUpdate {
            cwd,
            display_path: _,
            is_git_repo,
            branch,
        } => {
            assert_eq!(
                cwd, project,
                "footer state must report the working directory"
            );
            assert!(is_git_repo && branch.as_deref() == Some("main"));
            // The poller must have SEEDED the footer-cwd registry at creation
            // time — that is the value the first emission carries without
            // any tool ever running.
            assert_eq!(
                codelet_tools::footer_cwd::get_footer_cwd(sid).as_deref(),
                Some(project),
                "the footer CWD registry must be seeded with the project root at session creation"
            );
        }
        other => panic!("expected FooterStateUpdate chunk, got {other:?}"),
    }

    // @step And no Bash tool invocation is required for the branch to appear
    // (this test never invokes any tool — the branch arrived via the
    // creation-time poll alone)
}

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_worktree_session_reports_detached_git_state_not_a_blank_non_repo_state()
{
    let _guard = ENV_GUARD.lock().await;

    // @step Given a session manager configured with the fspec binary's hook implementation (FspecAgentHooks)
    let manager = manager_with_seeded_cache();
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));
    codelet_sessions::footer_poller::register_chunk_sender(manager.chunks_tx().clone());
    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step And a new isolated session whose worktree is in a detached HEAD state
    let repo = fresh_git_repo();
    let sid = Uuid::new_v4();
    let info = manager
        .create_isolated_session_with_id(
            &sid.to_string(),
            "anthropic/claude-opus-4-5",
            repo.path().to_str().expect("repo path is utf8"),
            "wt002-isolated",
        )
        .await
        .expect("isolated session creation under FspecAgentHooks must succeed");
    let worktree_path = info.worktree_path.clone();

    // @step When the footer poller ticks for that session
    let chunk = next_footer_chunk(&mut chunks_rx, &SessionId::new(sid.to_string()), |c| {
        matches!(c, StreamChunk::FooterStateUpdate { .. })
    })
    .await;

    // @step Then the emitted footer state has is_git_repo true with an empty (None) branch
    match chunk {
        StreamChunk::FooterStateUpdate {
            cwd,
            display_path: _,
            is_git_repo,
            branch,
        } => {
            assert!(
                is_git_repo,
                "WT-002: a detached-HEAD worktree is still a git repo — is_git_repo must be true (a false value collapses the branch to blank)"
            );
            assert!(
                branch.is_none(),
                "a detached-HEAD worktree has no branch — branch must be None, got {branch:?}"
            );

            // @step And the chunk carries the worktree path as the CWD
            assert_eq!(
                cwd, worktree_path,
                "the footer state must carry the worktree path so the TUI can render its detached indicator"
            );
        }
        other => panic!("expected FooterStateUpdate chunk, got {other:?}"),
    }
}
