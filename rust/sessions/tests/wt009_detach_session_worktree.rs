//! WT-009 — `detach_session_worktree` backend RPC (codelet-sessions).
//!
//! Feature: spec/features/isolation-only-opens-the-create-session-dialog-and-merge-worktree-never-closes-the-session-ux-contract-broken-in-the-rust-tui.feature
//!
//! Covers the architecture-note behaviour of the new
//! `SessionManagerHandle::detach_session_worktree` RPC: clearing the
//! session's worktree_path/base_commit, deleting the git-session
//! manifest (so the worktree becomes prunable), emitting
//! `IsolationStateChange(false, None)`, and erroring for non-isolated or
//! unknown sessions.
//!
//! Scenarios drive a real `SessionManager` against a fresh git
//! repository (temp dir, HOME redirected to a temp dir so the
//! `~/.fspec/git-sessions` manifest writes stay hermetic).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use codelet_core::SessionManagerHandle;
use codelet_rpc_types::{SessionId, StreamChunk};
use codelet_sessions::SessionManager;
use uuid::Uuid;

/// Trimmed offline models.dev catalog (anthropic/openai/google) — shared
/// with the WT-001 / WT-003 / RPC-385 precedent so registry validation
/// stays fully offline.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// Serialises tests that swap process-global state (HOME, data dir, env
/// creds) so parallel tests cannot observe each other's state.
static ENV_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

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

/// Build a manager rooted in a fresh data dir with the offline models.dev
/// fixture pre-seeded, with HOME redirected to a temp dir so the
/// `~/.fspec/git-sessions` manifest writes stay hermetic.
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

/// Create an isolated session (real worktree under `<project>/.fspec/worktrees`)
/// and return its id.
async fn create_isolated_session(manager: &SessionManager, project: &Path) -> String {
    let id = Uuid::new_v4().to_string();
    manager
        .create_isolated_session_with_id(
            &id,
            "anthropic/claude-opus-4-5",
            project.to_str().expect("repo path is utf8"),
            "WT-009 detached",
        )
        .await
        .expect("isolated session creation must succeed");
    id
}

/// Path of the `~/.fspec/git-sessions/<id>.json` manifest file.
fn git_manifest_path(home: &Path, id: &str) -> std::path::PathBuf {
    home.join(".fspec")
        .join("git-sessions")
        .join(format!("{id}.json"))
}

/// Pull the next `IsolationStateChange` chunk for `sid` off the broadcast,
/// skipping chunks for other sessions. Times out after 5 seconds.
async fn next_isolation_chunk(
    rx: &mut tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
    sid: &SessionId,
) -> StreamChunk {
    use std::time::Duration;
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
// Rule 1: detach_session_worktree clears the session's worktree path,
// deletes the git-session manifest (worktree becomes prunable), and emits
// IsolationStateChange(false, None) — the session stays alive.
// =============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_detach_session_worktree_clears_isolation_and_marks_worktree_prunable() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a git repository with one committed file and an isolated session
    let repo = fresh_git_repo();
    let (_data_dir, home_dir, manager) = manager_with_seeded_cache();
    let sid = create_isolated_session(&manager, repo.path()).await;
    let worktree = repo.path().join(".fspec").join("worktrees").join(&sid);
    assert!(worktree.exists(), "worktree must exist before detach");
    assert!(
        git_manifest_path(home_dir.path(), &sid).exists(),
        "git-session manifest must exist before detach"
    );

    // Precondition: the session is isolated (effective cwd = worktree).
    let session_before = manager.get_session(&sid).expect("session in memory");
    assert!(
        session_before.worktree_path().is_some(),
        "session must start isolated"
    );

    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step When detach_session_worktree is called with the session id
    let handle: Arc<dyn SessionManagerHandle> = manager.clone() as _;
    let result = handle.detach_session_worktree(&SessionId::new(sid.clone()));
    assert!(
        result.is_ok(),
        "detach_session_worktree must succeed for an isolated session: {result:?}"
    );

    // @step Then the session's worktree path and base commit are cleared and its effective cwd is the project root
    let session_after = manager.get_session(&sid).expect("session still in memory");
    assert!(
        session_after.worktree_path().is_none(),
        "detach must clear worktree_path"
    );
    assert!(
        session_after.base_commit().is_none(),
        "detach must clear base_commit"
    );
    assert_eq!(
        session_after.effective_cwd(),
        repo.path().to_path_buf(),
        "after detach the effective cwd must be the project root"
    );

    // @step And the git-session manifest is deleted so the worktree becomes prunable
    assert!(
        !git_manifest_path(home_dir.path(), &sid).exists(),
        "detach must delete the git-session manifest"
    );
    let active: std::collections::HashSet<String> = std::collections::HashSet::new();
    assert!(
        codelet_git::is_orphaned(&sid, &active).expect("is_orphaned"),
        "without a manifest the session worktree must be prunable"
    );

    // @step And the manager emits IsolationStateChange(false, None) for the session
    let chunk = next_isolation_chunk(&mut chunks_rx, &SessionId::new(sid.clone())).await;
    match chunk {
        StreamChunk::IsolationStateChange {
            is_isolated,
            worktree_path,
            ..
        } => {
            assert!(!is_isolated, "detach must emit is_isolated=false");
            assert!(worktree_path.is_none(), "detach must emit worktree_path=None");
        }
        other => panic!("expected IsolationStateChange chunk, got {other:?}"),
    }

    // @step And the session stays alive in memory (detach keeps the session running)
    assert!(
        manager.get_session(&sid).is_ok(),
        "detach must NOT destroy the session"
    );
}

// =============================================================================
// Rule 1 (error paths): detaching a non-isolated session or an unknown
// session returns an error and leaves the isolation state untouched.
// =============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_detach_session_worktree_errors_for_non_isolated_sessions() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a git repository with a NON-isolated session
    let repo = fresh_git_repo();
    let (_data_dir, _home_dir, manager) = manager_with_seeded_cache();
    let id = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(&id, "anthropic/claude-opus-4-5", repo.path().to_str().unwrap(), "plain")
        .await
        .expect("plain session creation must succeed");

    // @step When detach_session_worktree is called with that session id
    let handle: Arc<dyn SessionManagerHandle> = manager.clone() as _;
    let result = handle.detach_session_worktree(&SessionId::new(id.clone()));

    // @step Then the call returns an error mentioning the session is not isolated
    let err = result.expect_err("detaching a non-isolated session must fail");
    assert!(
        err.to_lowercase().contains("not isolated"),
        "error must mention the session is not isolated, got: {err}"
    );

    // @step And the session is unchanged (still non-isolated, still alive)
    let session = manager.get_session(&id).expect("session still in memory");
    assert!(session.worktree_path().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_detach_session_worktree_errors_for_unknown_sessions() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a session id the manager has never seen
    let (_data_dir, _home_dir, manager) = manager_with_seeded_cache();
    let unknown = Uuid::new_v4().to_string();

    // @step When detach_session_worktree is called with that id
    let result = manager.detach_session_worktree(&SessionId::new(unknown));

    // @step Then the call returns an error
    assert!(
        result.is_err(),
        "detaching an unknown session must fail, got {result:?}"
    );
}
