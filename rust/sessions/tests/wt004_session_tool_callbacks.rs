//! WT-004 — Isolation path-validation callback is never registered in the
//! fspec binary.
//!
//! Feature: spec/features/isolation-path-validation-callback-is-never-registered-in-the-fspec-binary-isolated-sessions-are-not-actually-pinned-to-their-worktree.feature
//!
//! These scenarios drive the real, NAPI-free tool-callbacks module
//! (`codelet_sessions::session_tool_callbacks`) exactly the way the fspec
//! binary's `build_service` registers it: a NON-singleton `SessionManager`
//! is registered via `register_manager`, then the shared callback functions
//! are used as the tool layer's effective-cwd / work-unit-stage /
//! block-notification callbacks. The tool-layer validation goes through the
//! real `codelet_tools::facade` entry points (`get_isolation_context` and
//! `validate_and_resolve_path_with_isolation`), so a green test proves the
//! boundary is live under the fspec binary rather than in the NAPI adapter
//! only.

//!
//! This file holds the isolation-lookup scenarios; the stage-none,
//! block-notification and build_service wiring scenarios live in
//! wt004_tool_callbacks_wiring.rs, and the shared helpers in
//! `wt004_session_tool_callbacks_support` (REFAC-053).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod wt004_session_tool_callbacks_support;

use std::path::Path;
use std::sync::Arc;

use uuid::Uuid;

use codelet_sessions::SessionManager;
use codelet_tools::facade::{get_isolation_context, validate_and_resolve_path_with_isolation};
use codelet_tools::ToolError;
use wt004_session_tool_callbacks_support::{
    create_plain, fresh_git_repo, manager_with_seeded_cache, register_tool_callbacks, ENV_GUARD,
};

/// Create an isolated session through the registered manager's in-memory
/// API (the same call the fspec binary drives via RPC).
async fn create_isolated(manager: &Arc<SessionManager>, repo: &Path) -> String {
    let id = Uuid::new_v4().to_string();
    manager
        .create_isolated_session_with_id(
            &id,
            "anthropic/claude-opus-4-5",
            repo.to_str().expect("repo path is utf8"),
            "wt004",
        )
        .await
        .expect("isolated session creation must succeed");
    id
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_session_under_the_fspec_binary_exposes_its_isolation_context_to_the_tool_layer(
) {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a fresh git repository with one committed file
    let repo = fresh_git_repo();

    // @step And a non-singleton session manager whose tool callbacks are registered the way build_service registers them
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);

    // @step When an isolated session is created against that repository through the registered manager
    let session_id = create_isolated(&manager, repo.path()).await;

    // @step Then the tool layer resolves an isolation context for that session with the worktree path of the created worktree
    let ctx = get_isolation_context(Uuid::parse_str(&session_id).expect("session id is a uuid"))
        .expect("BUG-404: the isolation lookup for a known session must succeed")
        .expect("WT-004: the tool layer must resolve an isolation context for an isolated session under the fspec binary");
    let worktree = repo
        .path()
        .join(".fspec")
        .join("worktrees")
        .join(&session_id);
    assert_eq!(
        ctx.worktree_path, worktree,
        "worktree_path must be the created worktree"
    );

    // @step And the isolation context's blocked project path is the repository root
    assert_eq!(
        ctx.blocked_project_path,
        repo.path().to_path_buf(),
        "blocked_project_path must be the session's project root"
    );
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolation_lookup_through_the_registered_manager_ignores_the_global_singleton() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a non-singleton session manager registered as the tool-callbacks manager
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);

    // @step And an isolated session created in that manager against a fresh git repository
    let repo = fresh_git_repo();
    let session_id = create_isolated(&manager, repo.path()).await;

    // @step When the tool layer resolves the isolation context for that session id
    // (a second manager has never seen this session — it was created in the
    // registered manager, exactly like the fspec binary does)
    let ctx = get_isolation_context(Uuid::parse_str(&session_id).expect("session id is a uuid"))
        .expect("BUG-404: the isolation lookup for a known session must succeed")
        .expect("WT-004: the registered manager must be consulted, not any other manager");

    // @step Then the lookup succeeds even though a second, unregistered manager has no such session
    assert!(
        codelet_sessions::SessionManager::new()
            .get_session(&session_id)
            .is_err(),
        "sanity: the session must NOT exist in a second manager"
    );

    // @step And the returned context matches the session's worktree path and project root
    assert_eq!(
        ctx.blocked_project_path,
        repo.path().to_path_buf(),
        "the context must come from the registered manager's session record"
    );
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_write_tool_of_an_isolated_session_blocks_absolute_paths_into_the_original_project(
) {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a non-singleton session manager registered as the tool-callbacks manager
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);

    // @step And an isolated session created against a fresh git repository that contains a tracked source file
    let repo = fresh_git_repo();
    let session_id = create_isolated(&manager, repo.path()).await;
    let target = repo.path().join("src").join("main.rs");
    let before = codelet_test_helpers::source_composition::read_tree_or_file(&target)
        .expect("read tracked source file");

    // @step When a file operation for that session validates an absolute path inside the original project root
    let ctx = get_isolation_context(Uuid::parse_str(&session_id).expect("session id is a uuid"))
        .expect("BUG-404: the isolation lookup for a known session must succeed")
        .expect("isolation context must be resolved");
    let result =
        validate_and_resolve_path_with_isolation(&target.to_string_lossy(), Some(&ctx), "write");

    // @step Then the validation fails with an error naming the blocked project path
    match result {
        Err(ToolError::Validation { tool, message }) => {
            assert_eq!(tool, "write");
            assert!(
                message.contains("original project"),
                "error must name the blocked project path, got: {message}"
            );
        }
        other => panic!("expected a write validation error, got {other:?}"),
    }

    // @step And the original project file is not modified
    assert_eq!(
        codelet_test_helpers::source_composition::read_tree_or_file(&target)
            .expect("re-read tracked source file"),
        before,
        "the original project file must not be touched by the blocked write"
    );
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_write_tool_of_an_isolated_session_resolves_relative_paths_into_the_worktree() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a non-singleton session manager registered as the tool-callbacks manager
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);

    // @step And an isolated session created against a fresh git repository
    let repo = fresh_git_repo();
    let session_id = create_isolated(&manager, repo.path()).await;

    // @step When a file operation for that session validates a relative path
    let ctx = get_isolation_context(Uuid::parse_str(&session_id).expect("session id is a uuid"))
        .expect("BUG-404: the isolation lookup for a known session must succeed")
        .expect("isolation context must be resolved");
    let resolved = validate_and_resolve_path_with_isolation("src/feature.rs", Some(&ctx), "write")
        .expect("a relative path inside the worktree must be allowed");

    // @step Then the resolved path lands inside the session's worktree directory
    let worktree = repo
        .path()
        .join(".fspec")
        .join("worktrees")
        .join(&session_id);
    assert!(
        resolved.starts_with(&worktree),
        "relative path must resolve inside the worktree, got {resolved:?} (worktree {worktree:?})"
    );
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_non_isolated_session_keeps_unrestricted_path_access() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a non-singleton session manager registered as the tool-callbacks manager
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);

    // @step And a plain non-isolated session created in that manager
    let repo = fresh_git_repo();
    let session_id = create_plain(&manager, repo.path().to_str().expect("utf8")).await;

    // @step When the tool layer resolves the isolation context for that session
    let ctx = get_isolation_context(Uuid::parse_str(&session_id).expect("session id is a uuid"))
        .expect("BUG-404: the isolation lookup for a known session must succeed");

    // @step Then it gets no isolation context
    assert!(
        ctx.is_none(),
        "non-isolated sessions must NOT carry an isolation context (they keep full access)"
    );

    // @step And a path validation for any absolute path outside the project succeeds unchanged
    let outside = tempfile::tempdir().expect("tempdir outside the project");
    let path = outside.path().join("loose.txt");
    let resolved =
        validate_and_resolve_path_with_isolation(path.to_str().expect("utf8"), None, "read")
            .expect("no isolation => allow-all paths");
    assert_eq!(
        resolved, path,
        "without an isolation context the path must pass through unchanged"
    );
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_unknown_session_ids_refuse_isolation_lookups() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a non-singleton session manager registered as the tool-callbacks manager
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);

    // @step When the tool layer resolves the isolation context for a session id that no manager knows
    let unknown = Uuid::new_v4();

    // @step Then the lookup fails naming the session instead of returning no isolation context
    let error = get_isolation_context(unknown).expect_err(
        "BUG-404: an unknown session id must refuse the isolation lookup, never degrade to no isolation",
    );
    assert!(error.contains(&unknown.to_string()), "{error}");
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_work_unit_stage_lookup_under_the_fspec_binary_reads_the_sessions_work_unit_context(
) {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a non-singleton session manager registered as the tool-callbacks manager
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);

    // @step And a session with a work unit context whose status is testing
    let repo = fresh_git_repo();
    // BUG-405: binding validates the card against the session project.
    std::fs::create_dir_all(repo.path().join("spec")).expect("spec dir");
    std::fs::write(
        repo.path().join("spec/work-units.json"),
        serde_json::to_vec(&serde_json::json!({"workUnits": {"WT-999": {
            "id": "WT-999", "title": "Test card", "type": "story", "status": "testing"
        }}}))
        .expect("cards json"),
    )
    .expect("seed the session project's cards");
    let session_id = create_plain(&manager, repo.path().to_str().expect("utf8")).await;
    let session = manager.get_session(&session_id).expect("session exists");
    session
        .set_work_unit_context(
            Some("WT-999".to_string()),
            Some("Test card".to_string()),
            Some("testing".to_string()),
        )
        .expect("BUG-405: binding the work unit must succeed for a session its manager owns");

    // @step When the tool layer asks for that session's work unit stage
    let stage = codelet_sessions::session_tool_callbacks::work_unit_stage(session_id);

    // @step Then it returns testing
    assert_eq!(
        stage.map(|stage| stage.map(|stage| stage.stage)),
        Ok(Some("testing".to_string())),
        "the stage lookup must return the session's work unit context status"
    );
}
