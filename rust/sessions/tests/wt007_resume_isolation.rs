//! WT-007 — /resume silently drops worktree isolation for
//! formerly-isolated sessions.
//!
//! Feature: spec/features/isolated-sessions-are-indistinguishable-after-restart-resume-loses-worktree-isolation.feature
//!
//! Drives a real `SessionManager` against a fresh git repository (temp
//! dir, HOME redirected to a temp dir so the `~/.fspec/git-sessions`
//! manifest stays hermetic) and resumes sessions via
//! `create_session_from_manifest` — the same path `/resume` uses.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use serial_test::serial;
use uuid::Uuid;
use codelet_sessions::SessionManager;

/// Trimmed offline models.dev catalog (anthropic/openai/google) — shared
/// with the WT-001 / WT-005 / RPC-385 precedent so registry validation
/// stays fully offline.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

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
    // RPC-423 precedent: reset the persistence singletons BEFORE pointing
    // the data directory at the fresh temp dir, so SessionStore re-initialises
    // against THIS test's dir (it caches `sessions_dir` at first use).
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(data_dir.path().to_path_buf())
        .expect("set data directory");
    std::env::set_var("HOME", home_dir.path());
    let manager = Arc::new(SessionManager::new());
    (data_dir, home_dir, manager)
}

/// Read the `~/.fspec/git-sessions/<id>.json` manifest written by
/// `codelet_git::create_session_manifest`.
fn read_git_manifest(home: &Path, id: &str) -> Option<codelet_git::SessionManifest> {
    let path = home.join(".fspec").join("git-sessions").join(format!("{id}.json"));
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Path of the `~/.fspec/git-sessions/<id>.json` manifest file.
fn git_manifest_path(home: &Path, id: &str) -> PathBuf {
    home.join(".fspec")
        .join("git-sessions")
        .join(format!("{id}.json"))
}

/// Delete the session's worktree the way `codelet_git::remove_worktree`
/// does: the worktree dir itself plus the git admin dir
/// (`.git/worktrees/<id>`) so BOTH re-isolation preconditions are gone.
fn remove_worktree_artifacts(repo: &Path, id: &str) {
    codelet_git::remove_worktree(repo, id).expect("remove_worktree must succeed");
}

// =============================================================================
// Scenario: Resuming a formerly-isolated session restores worktree isolation
// =============================================================================

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resuming_a_formerly_isolated_session_restores_worktree_isolation() {
    // @step Given a temp git repo with a persisted session manifest for an isolated session (git manifest at git-sessions/<id>.json with worktree_path and base_commit, and both the worktree dir and the .git/worktrees/<id> admin dir exist)
    let repo = fresh_git_repo();
    let (data_dir, home_dir, manager) = manager_with_seeded_cache();
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
    let worktree_path = info.worktree_path.clone();
    let base_commit = info.base_commit.clone();
    let git_manifest = read_git_manifest(home_dir.path(), &id)
        .expect("git-session manifest must exist");
    assert_eq!(
        git_manifest.worktree_path.as_deref(),
        Some(Path::new(&worktree_path)),
        "precondition: git manifest records the worktree"
    );
    // Simulate a restart: drop the in-memory session.
    manager
        .destroy_session(&id)
        .expect("destroy_session must succeed");
    // WT-005's terminate-on-close flipped `terminated` — a process RESTART
    // (the scenario under test) leaves the session un-terminated, so reset
    // the flag to model that.
    let mut manifest = read_git_manifest(home_dir.path(), &id)
        .expect("git manifest must exist after isolated creation");
    manifest.terminated = false;
    codelet_git::write_manifest(&manifest).expect("write manifest");
    let persistence_manifest = codelet_core::persistence::load_session(
        Uuid::parse_str(&id).expect("valid uuid"),
    )
    .expect("persistence manifest must exist after isolated creation (WT-005)");
    // Subscribe to the manager's chunk broadcast BEFORE the resume so the
    // IsolationStateChange emitted during re-isolation is observable.
    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step When the manager resumes the session via create_session_from_manifest
    manager
        .create_session_from_manifest(&persistence_manifest, "anthropic/claude-opus-4-5")
        .await
        .expect("create_session_from_manifest must succeed");

    // @step Then the resumed session carries worktree_path and base_commit and the IsolationStateChange(true, worktree, base) chunk is broadcast
    // and the IsolationStateChange(true, worktree, base) chunk is broadcast
    let session = manager
        .get_session(&id)
        .expect("resumed session must be in memory");
    assert_eq!(
        session.worktree_path().as_deref(),
        Some(Path::new(&worktree_path)),
        "WT-007: a resumed formerly-isolated session must carry its worktree_path"
    );
    assert_eq!(
        session.base_commit().as_deref(),
        Some(base_commit.as_str()),
        "WT-007: a resumed formerly-isolated session must carry its base_commit"
    );
    // Receive the IsolationStateChange chunk broadcast by the resume path.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut isolation_chunk = None;
    while std::time::Instant::now() < deadline {
        match chunks_rx.recv().await {
            Ok((
                sid,
                codelet_rpc_types::StreamChunk::IsolationStateChange {
                    is_isolated,
                    worktree_path,
                    base_commit,
                    ..
                },
            )) if sid.value == id => {
                isolation_chunk = Some((is_isolated, worktree_path, base_commit));
                break;
            }
            Ok(_) => {}
            Err(_) => break,
        }
        tokio::task::yield_now().await;
    }
    let (is_isolated, chunk_worktree, chunk_base) = isolation_chunk
        .expect("WT-007: resuming a formerly-isolated session must broadcast IsolationStateChange");
    assert!(is_isolated, "the isolation chunk must report isolated=true");
    assert_eq!(
        chunk_worktree.as_deref(),
        Some(worktree_path.as_str()),
        "the isolation chunk must carry the worktree path"
    );
    assert_eq!(
        chunk_base.as_deref(),
        Some(base_commit.as_str()),
        "the isolation chunk must carry the base commit (WT-001 contract)"
    );
    let effective_cwd = session.effective_cwd();
    assert_eq!(
        effective_cwd,
        Path::new(&worktree_path).to_path_buf(),
        "WT-007: resumed isolated session's effective cwd must be the worktree"
    );
    // The git manifest must survive the resume un-terminated (the session
    // is isolated again — nothing to prune).
    let manifest_after = read_git_manifest(home_dir.path(), &id)
        .expect("git manifest must survive resume");
    assert!(
        !manifest_after.terminated,
        "resuming an isolated session must not terminate the git manifest"
    );
    let _ = data_dir;
}

// =============================================================================
// Scenario: Resuming an isolated session whose worktree was deleted falls
// back to non-isolated
// =============================================================================

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resuming_an_isolated_session_whose_worktree_was_deleted_falls_back_to_non_isolated() {
    // @step Given a persisted isolated session whose git manifest still claims a worktree_path, but the worktree directory has been deleted
    let repo = fresh_git_repo();
    let (_data_dir, home_dir, manager) = manager_with_seeded_cache();
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
    manager
        .destroy_session(&id)
        .expect("destroy_session must succeed");
    remove_worktree_artifacts(repo.path(), &id);
    // WT-005's terminate-on-close already flipped `terminated`; reset it so
    // the assertion below proves the RESUME path (not the close path) marks
    // the strayed manifest terminated.
    let mut manifest = read_git_manifest(home_dir.path(), &id)
        .expect("git manifest must still claim the worktree");
    manifest.terminated = false;
    codelet_git::write_manifest(&manifest).expect("write manifest");
    assert!(
        !Path::new(&info.worktree_path).exists(),
        "precondition: worktree dir is gone"
    );
    let persistence_manifest = codelet_core::persistence::load_session(
        Uuid::parse_str(&id).expect("valid uuid"),
    )
    .expect("persistence manifest must exist");

    // @step When the manager resumes the session via create_session_from_manifest
    manager
        .create_session_from_manifest(&persistence_manifest, "anthropic/claude-opus-4-5")
        .await
        .expect("create_session_from_manifest must succeed");

    // @step Then the session resumes non-isolated with a warning and the git manifest is marked terminated so /worktrees prune can reclaim it
    let session = manager
        .get_session(&id)
        .expect("resumed session must be in memory");
    assert!(
        session.worktree_path().is_none(),
        "WT-007: with the worktree gone the session must resume non-isolated"
    );
    assert!(
        session.base_commit().is_none(),
        "WT-007: with the worktree gone the session must not carry a base_commit"
    );
    let manifest_after = read_git_manifest(home_dir.path(), &id)
        .expect("git manifest must still exist (only the flag flips)");
    assert!(
        manifest_after.terminated,
        "WT-007: a resume that cannot re-isolate must mark the git manifest terminated"
    );
}

// =============================================================================
// Scenario: Resuming a never-isolated session keeps the non-isolated resume
// behavior
// =============================================================================

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resuming_a_never_isolated_session_keeps_the_non_isolated_resume_behavior() {
    // @step Given a persisted session with no git-session manifest (or one whose worktree_path is null)
    let (_data_dir, home_dir, manager) = manager_with_seeded_cache();
    let id = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(
            &id,
            "anthropic/claude-opus-4-5",
            std::env::temp_dir().to_str().expect("temp dir is utf8"),
            "Plain test",
        )
        .await
        .expect("non-isolated session creation must succeed");
    assert!(
        !git_manifest_path(home_dir.path(), &id).exists(),
        "precondition: non-isolated sessions have no git-session manifest"
    );
    manager
        .destroy_session(&id)
        .expect("destroy_session must succeed");
    let persistence_manifest = codelet_core::persistence::load_session(
        Uuid::parse_str(&id).expect("valid uuid"),
    )
    .expect("persistence manifest must exist");

    // @step When the manager resumes the session via create_session_from_manifest
    manager
        .create_session_from_manifest(&persistence_manifest, "anthropic/claude-opus-4-5")
        .await
        .expect("create_session_from_manifest must succeed");

    // @step Then the session resumes non-isolated exactly as before (IsolationStateChange(false, None), footer poller with the project cwd)
    let session = manager
        .get_session(&id)
        .expect("resumed session must be in memory");
    assert!(
        session.worktree_path().is_none(),
        "a never-isolated session must resume without a worktree"
    );
    assert!(
        session.base_commit().is_none(),
        "a never-isolated session must resume without a base_commit"
    );
    assert!(
        !git_manifest_path(home_dir.path(), &id).exists(),
        "resuming a never-isolated session must not create a git-session manifest"
    );
}
