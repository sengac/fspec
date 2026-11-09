//! Feature: spec/features/session-checkpoint-integration.feature
//!
//! GIT-021 — checkpoint operations on `BackgroundSession` require an isolated
//! session (one with a worktree path). Drives the real `checkpoint`,
//! `restore` and `list_checkpoints` methods on a session created through
//! `SessionManagerHandle::create_session` (Noop hooks, no agent loop).
//!
//! Replaces the retired codelet-napi source-shape test that grepped the
//! deleted `rust/napi/src/session_manager.rs` for these method signatures.
//! The isolated (worktree-backed) scenarios live in
//! `git021_checkpoint_requires_isolation/isolated.rs` (COV-057); the raw
//! ghost-commit primitives are covered by rust/git/tests/ghost_commit_tests.rs.
//!
//! multi-thread runtime is REQUIRED: `create_session` bridges sync->async via
//! `tokio::task::block_in_place`.

use std::path::PathBuf;
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::background_session::SessionError;
use codelet_sessions::SessionManager;
use tokio::sync::Mutex;

#[path = "git021_checkpoint_requires_isolation/isolated.rs"]
mod isolated;
#[path = "git021_checkpoint_requires_isolation/support.rs"]
mod support;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Serializes the tests that swap the process-global data directory
/// (`codelet_common::set_data_directory`), mirroring rpc418_compact_session.rs.
static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

// ============================================================================
// Scenario: Checkpoint fails for non-isolated session
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_fails_for_non_isolated_session() -> TestResult {
    let _guard = DATA_DIR_GUARD.lock().await;
    let data_dir = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    codelet_core::persistence::reset_stores_for_tests();
    // BUG-448: its own offline catalog and stub credentials — never the
    // machine's, nor whatever an earlier test in this binary left behind.
    std::fs::create_dir_all(data_dir.path().join("cache"))?;
    std::fs::write(
        data_dir.path().join("cache/models.json"),
        support::MODELS_FIXTURE,
    )?;
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    manager.set_default_project(codelet_test_helpers::shared_temp_subdirectory()?)?;

    // @step Given a non-isolated session without worktree path
    manager.set_default_model("anthropic/claude-opus-4-5")?;
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid = handle.create_session_checked(None)?;
    let session = manager.get_session(&sid.value)?;
    assert!(session.worktree_path().is_none());
    assert_eq!(session.effective_cwd(), PathBuf::from(&session.project));

    // @step When I call checkpoint with label "test"
    let result = session.checkpoint("test");

    // @step Then a NotIsolated error should be returned
    assert!(
        matches!(result, Err(SessionError::NotIsolated)),
        "checkpoint on a non-isolated session must return NotIsolated, got {result:?}"
    );

    // restore and list_checkpoints share the same isolation guard
    let restored = session.restore("test");
    assert!(
        matches!(restored, Err(SessionError::NotIsolated)),
        "restore on a non-isolated session must return NotIsolated, got {restored:?}"
    );
    let listed = session.list_checkpoints();
    assert!(
        matches!(listed, Err(SessionError::NotIsolated)),
        "list_checkpoints on a non-isolated session must return NotIsolated, got {listed:?}"
    );
    Ok(())
}
