//! WT-001 — IsolationStateChange stream chunk drops `base_commit`.
//!
//! Feature: spec/features/isolationstatechange-stream-chunk-drops-base-commit-tui-isolation-badge-can-never-show-the-worktree-base.feature
//!
//! Scenario 1 drives a real `SessionManager::create_isolated_session_with_id`
//! against a fresh git repository (temp dir) and asserts the
//! `IsolationStateChange` chunk on the manager-owned chunks broadcast
//! carries the worktree's base commit SHA (business rule 1). Scenario 2
//! pins the non-isolated creation path to the base_commit-free emit
//! (business rule 2).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use uuid::Uuid;

use codelet_rpc_types::{SessionId, StreamChunk};
use codelet_sessions::SessionManager;

/// Trimmed offline models.dev catalog (anthropic/openai/google) — shared
/// with the RPC-385 / PROV-101 precedent so registry validation stays
/// fully offline.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// Serialises tests that swap process-global env (data dir, HOME, creds)
/// so parallel tests cannot observe each other's state.
static ENV_GUARD: Mutex<()> = Mutex::const_new(());

/// Set dummy credentials so `ProviderCredentials::detect()` passes offline.
fn set_dummy_credentials() {
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");
}

/// Run a `git` CLI command in `cwd` and fail the test if it exits non-zero.
fn run_git(cwd: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to spawn git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Create a fresh git repository (temp dir) with one committed file.
fn fresh_git_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir for git repo");
    let path = dir.path();
    run_git(path, &["init"]);
    run_git(path, &["config", "user.email", "test@example.com"]);
    run_git(path, &["config", "user.name", "Test User"]);
    std::fs::write(path.join("hello.txt"), "hello\n").expect("write hello.txt");
    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "initial commit"]);
    dir
}

/// Read the repository's current HEAD SHA.
fn head_sha(cwd: &Path) -> String {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(cwd)
        .output()
        .expect("failed to spawn git rev-parse");
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert!(!sha.is_empty(), "repo must have a HEAD commit");
    sha
}

/// Build a manager rooted in a fresh data dir with the offline models.dev
/// fixture pre-seeded, with HOME redirected to a temp dir so the
/// `~/.fspec/git-sessions` manifest writes from
/// `create_isolated_session_with_id` stay hermetic.
fn manager_with_seeded_cache() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    Arc<SessionManager>,
) {
    set_dummy_credentials();
    let data_dir = tempfile::tempdir().expect("tempdir for data dir");
    let home_dir = tempfile::tempdir().expect("tempdir for HOME");
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("seed models.json");
    // SessionStore caches `sessions_dir` at first use — reset the
    // persistence singletons BEFORE pointing the data directory at this
    // test's fresh temp dir so session-manifest writes stay hermetic.
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(data_dir.path().to_path_buf())
        .expect("set data directory");
    std::env::set_var("HOME", home_dir.path());
    let manager = Arc::new(SessionManager::new());
    (data_dir, home_dir, manager)
}

/// Pull the next `IsolationStateChange` chunk for `sid` off the broadcast,
/// skipping chunks for other sessions. Times out after 5 seconds.
async fn next_isolation_chunk(
    rx: &mut tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
    sid: &SessionId,
) -> StreamChunk {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        match rx.recv().await {
            Ok((id, chunk)) => {
                if id == *sid {
                    return chunk;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                panic!("chunks broadcast closed before IsolationStateChange arrived");
            }
        }
    }
    panic!("timed out waiting for IsolationStateChange chunk for {sid}");
}

// =============================================================================
// Scenario: Isolated session creation chunk carries the worktree base commit
// =============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn isolated_session_creation_chunk_carries_the_worktree_base_commit() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a fresh git repository with one committed file
    let repo = fresh_git_repo();
    let base_sha = head_sha(repo.path());

    // @step When an isolated session is created via create_isolated_session_with_id against that repository
    let (_data_dir, _home_dir, manager) = manager_with_seeded_cache();
    let mut chunks_rx = manager.chunks_tx().subscribe();
    let id = Uuid::new_v4().to_string();
    let info = manager
        .create_isolated_session_with_id(
            &id,
            "anthropic/claude-opus-4-5",
            repo.path().to_str().expect("repo path is utf8"),
            "Isolated test",
        )
        .await
        .expect("isolated session creation must succeed");

    // @step Then the IsolationStateChange chunk received on the chunks broadcast has is_isolated true, the worktree path, and a base_commit equal to the repository's HEAD at fork time
    let chunk = next_isolation_chunk(&mut chunks_rx, &SessionId::new(id.clone()))
        .await;
    match chunk {
        StreamChunk::IsolationStateChange {
            is_isolated,
            worktree_path,
            base_commit,
        } => {
            assert!(
                is_isolated,
                "chunk must report the session as isolated"
            );
            assert_eq!(
                worktree_path.as_deref(),
                Some(info.worktree_path.as_str()),
                "worktree_path must match the created worktree"
            );
            assert_eq!(
                base_commit.as_deref(),
                Some(base_sha.as_str()),
                "WT-001: base_commit must carry the forked HEAD SHA on the chunk"
            );
        }
        other => panic!("expected IsolationStateChange chunk, got {other:?}"),
    }
}

// =============================================================================
// Scenario: Non-isolated session creation still emits a base_commit-free
// IsolationStateChange
// =============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_isolated_session_creation_still_emits_a_base_commit_free_isolation_state_change() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a SessionManager with a subscriber on the chunks broadcast
    let (_data_dir, _home_dir, manager) = manager_with_seeded_cache();
    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step When a non-isolated session is created via create_session_with_id
    let id = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(&id, "anthropic/claude-opus-4-5", ".", "Plain test")
        .await
        .expect("non-isolated session creation must succeed");

    // @step Then the IsolationStateChange chunk received on the chunks broadcast has is_isolated false, no worktree path, and base_commit None
    let chunk = next_isolation_chunk(&mut chunks_rx, &SessionId::new(id.clone()))
        .await;
    match chunk {
        StreamChunk::IsolationStateChange {
            is_isolated,
            worktree_path,
            base_commit,
        } => {
            assert!(
                !is_isolated,
                "non-isolated session chunk must have is_isolated false"
            );
            assert!(
                worktree_path.is_none(),
                "non-isolated session chunk must not carry a worktree path"
            );
            assert!(
                base_commit.is_none(),
                "non-isolated session chunk must carry base_commit None"
            );
        }
        other => panic!("expected IsolationStateChange chunk, got {other:?}"),
    }
}
