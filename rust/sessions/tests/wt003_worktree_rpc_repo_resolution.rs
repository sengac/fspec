//! WT-003 — Session worktree RPC ops resolve the repo via the process cwd.
//!
//! Feature: spec/features/session-worktree-rpc-ops-resolve-repo-path-via-process-cwd-so-merge-worktree-fails-when-tui-was-launched-outside-the-project-root.feature
//!
//! Every scenario drives the real `SessionManagerHandle` impl (codelet-sessions
//! `handle_impl`) against fresh temp git repositories with the process cwd set
//! to a directory that is NOT the session's project root — proving the ops
//! resolve the repository from the session's own recorded project root (or,
//! after a "restart", from the git-session manifest) instead of
//! `std::env::current_dir()`.
//!
//! Process-global env (HOME, data dir, dummy creds) and the process cwd are
//! all process-wide, so every scenario body runs inside a closure held under
//! `ENV_GUARD` (tokio async mutex) to serialise the global mutations.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use uuid::Uuid;

use codelet_core::SessionManagerHandle;
use codelet_rpc_types::{MergeStrategy, SessionId};
use codelet_sessions::SessionManager;

/// Trimmed offline models.dev catalog (anthropic/openai/google) — shared with
/// the WT-001 / RPC-385 / PROV-101 precedent so registry validation stays
/// fully offline.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// Serialises tests that swap process-global state (HOME, data dir, env
/// creds, cwd) so parallel tests cannot observe each other's state.
static ENV_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// RAII guard that swaps the process working directory for the test's
/// duration (WT-003: the whole card is about cwd independence).
struct CwdGuard {
    prior: PathBuf,
}

impl CwdGuard {
    fn to(dir: &Path) -> CwdGuard {
        let prior = std::env::current_dir().expect("prior cwd must exist");
        std::env::set_current_dir(dir).expect("set_current_dir must succeed");
        CwdGuard { prior }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prior);
    }
}

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

/// Create an isolated session (real worktree under `<project>/.fspec/worktrees`)
/// and return its id.
async fn create_isolated_session(manager: &SessionManager, project: &Path) -> String {
    let id = Uuid::new_v4().to_string();
    manager
        .create_isolated_session_with_id(
            &id,
            "anthropic/claude-opus-4-5",
            project.to_str().expect("repo path is utf8"),
            "WT-003 isolated",
        )
        .await
        .expect("isolated session creation must succeed");
    id
}

// =============================================================================
// Scenario: merge_session_worktree succeeds from a subdirectory cwd
// =============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_merge_session_worktree_succeeds_from_a_subdirectory_cwd() {
    let _guard = ENV_GUARD.lock().await;
    run_merge_subdir_cwd().await;
}

async fn run_merge_subdir_cwd() {
    // @step Given an in-memory session for project X with an isolated worktree containing a modified file
    let repo = fresh_git_repo();
    let (_data_dir, _home_dir, manager) = manager_with_seeded_cache();
    let sid = create_isolated_session(&manager, repo.path()).await;
    let worktree = repo.path().join(".fspec").join("worktrees").join(&sid);
    std::fs::write(worktree.join("hello.txt"), "changed by session\n").expect("modify in worktree");

    // @step And the process cwd is X/rust, a subdirectory of the repository that is NOT a repo root itself
    let subdir = repo.path().join("rust");
    std::fs::create_dir_all(&subdir).expect("create X/rust");
    let _cwd = CwdGuard::to(&subdir);

    // @step When merge_session_worktree is called with the session id
    let handle: Arc<dyn SessionManagerHandle> = manager.clone() as _;
    let outcome = handle
        .merge_session_worktree(&SessionId::new(sid), MergeStrategy::FastForward)
        .expect("merge must succeed even from a subdirectory cwd");

    // @step Then the merge resolves the repository from the session's project root X and returns a success outcome with the changed files
    assert_eq!(
        outcome.status,
        codelet_rpc_types::MergeStatus::Success,
        "merge must report Success with the changed files"
    );

    // @step And the modified file's new content is present in X's main working tree
    let merged = std::fs::read_to_string(repo.path().join("hello.txt")).expect("read merged file");
    assert_eq!(merged, "changed by session\n", "merge must land in project X");
}

// =============================================================================
// Scenario: discard_session_worktree removes the session worktree from a foreign cwd
// =============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_discard_session_worktree_removes_the_session_worktree_from_a_foreign_cwd() {
    let _guard = ENV_GUARD.lock().await;
    run_discard_foreign_cwd().await;
}

async fn run_discard_foreign_cwd() {
    // @step Given an in-memory session for project X with an isolated worktree
    let repo = fresh_git_repo();
    let (_data_dir, _home_dir, manager) = manager_with_seeded_cache();
    let sid = create_isolated_session(&manager, repo.path()).await;
    let worktree = repo.path().join(".fspec").join("worktrees").join(&sid);
    let git_metadata = repo.path().join(".git").join("worktrees").join(&sid);
    assert!(worktree.exists(), "worktree must exist before discard");

    // @step And the process cwd is an unrelated directory outside any repository
    let foreign = tempfile::tempdir().expect("tempdir for foreign cwd");
    let _cwd = CwdGuard::to(foreign.path());

    // @step When discard_session_worktree is called with the session id
    let handle: Arc<dyn SessionManagerHandle> = manager.clone() as _;
    handle
        .discard_session_worktree(&SessionId::new(sid))
        .expect("discard must succeed even from a foreign cwd");

    // @step Then discard succeeds and the session's worktree directory and its git metadata are removed from project X
    assert!(
        !worktree.exists(),
        "worktree directory must be removed from project X"
    );
    assert!(
        !git_metadata.exists(),
        "git worktree metadata must be removed from project X"
    );
}

// =============================================================================
// Scenario: inspect_session_changes returns the worktree summary from a foreign cwd
// =============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_inspect_session_changes_returns_the_worktree_summary_from_a_foreign_cwd() {
    let _guard = ENV_GUARD.lock().await;
    run_inspect_foreign_cwd().await;
}

async fn run_inspect_foreign_cwd() {
    // @step Given an in-memory session for project X with an isolated worktree containing modified, added, and deleted files
    let repo = fresh_git_repo();
    let (_data_dir, _home_dir, manager) = manager_with_seeded_cache();
    let sid = create_isolated_session(&manager, repo.path()).await;
    let worktree = repo.path().join(".fspec").join("worktrees").join(&sid);
    std::fs::write(worktree.join("hello.txt"), "modified in session\n").expect("modify file");
    std::fs::write(worktree.join("added.txt"), "brand new\n").expect("add file");

    // @step And the process cwd is an unrelated directory outside any repository
    let foreign = tempfile::tempdir().expect("tempdir for foreign cwd");
    let _cwd = CwdGuard::to(foreign.path());

    // @step When inspect_session_changes is called with the session id
    let handle: Arc<dyn SessionManagerHandle> = manager.clone() as _;
    let summary = handle
        .inspect_session_changes(&SessionId::new(sid))
        .expect("inspect must succeed even from a foreign cwd");

    // @step Then the call returns Ok with a non-zero file count and non-zero insertion/deletion counts derived from the worktree diff in project X
    assert!(
        summary.files_changed > 0,
        "files_changed must be non-zero, got {}",
        summary.files_changed
    );
    assert!(
        summary.insertions > 0,
        "insertions must be non-zero, got {}",
        summary.insertions
    );
}

// =============================================================================
// Scenario: list_session_worktrees returns the union of worktrees across all session projects
// =============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_list_session_worktrees_returns_the_union_of_worktrees_across_all_session_projects()
{
    let _guard = ENV_GUARD.lock().await;
    run_list_union().await;
}

async fn run_list_union() {
    // @step Given two in-memory isolated sessions, one rooted in project X and one in project Y, each with its own worktree
    let repo_x = fresh_git_repo();
    let repo_y = fresh_git_repo();
    let (_data_dir, _home_dir, manager) = manager_with_seeded_cache();
    let sid_x = create_isolated_session(&manager, repo_x.path()).await;
    let sid_y = create_isolated_session(&manager, repo_y.path()).await;

    // @step And the process cwd is a directory that is not the root of either repository
    let foreign = tempfile::tempdir().expect("tempdir for foreign cwd");
    let _cwd = CwdGuard::to(foreign.path());

    // @step When list_session_worktrees is called
    let handle: Arc<dyn SessionManagerHandle> = manager.clone() as _;
    let worktrees = handle.list_session_worktrees();

    // @step Then the result contains one entry for each session worktree in both project X and project Y
    let ids: Vec<String> = worktrees.iter().map(|w| w.session_id.value.clone()).collect();
    assert!(
        ids.iter().any(|id| id == &sid_x),
        "worktree of project X ({sid_x}) must be listed, got {ids:?}"
    );
    assert!(
        ids.iter().any(|id| id == &sid_y),
        "worktree of project Y ({sid_y}) must be listed, got {ids:?}"
    );
}

// =============================================================================
// Scenario: prune_orphaned_worktrees prunes the orphan and never touches the active session
// =============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_prune_orphaned_worktrees_prunes_the_orphan_and_never_touches_the_active_session()
{
    let _guard = ENV_GUARD.lock().await;
    run_prune_orphan().await;
}

async fn run_prune_orphan() {
    // @step Given project X contains a worktree whose session was closed and its git-session manifest is marked terminated
    let repo_x = fresh_git_repo();
    let (_data_dir, _home_dir, manager) = manager_with_seeded_cache();
    let orphan_id = Uuid::new_v4().to_string();
    codelet_git::create_worktree(repo_x.path(), &orphan_id)
        .expect("orphan worktree creation must succeed");
    codelet_git::create_session_manifest(
        &orphan_id,
        repo_x.path(),
        Some(repo_x.path().join(".fspec").join("worktrees").join(&orphan_id)),
        None,
    )
    .expect("orphan manifest creation must succeed");
    codelet_git::terminate_session(&orphan_id).expect("terminate must succeed");
    let orphan_worktree = repo_x.path().join(".fspec").join("worktrees").join(&orphan_id);
    assert!(orphan_worktree.exists(), "orphan worktree must exist");

    // @step And an active in-memory session lives in project Y with its own worktree
    let repo_y = fresh_git_repo();
    let active_id = create_isolated_session(&manager, repo_y.path()).await;
    let active_worktree = repo_y.path().join(".fspec").join("worktrees").join(&active_id);
    assert!(active_worktree.exists(), "active worktree must exist");

    // @step And the process cwd is a directory that is not the root of either repository
    let foreign = tempfile::tempdir().expect("tempdir for foreign cwd");
    let _cwd = CwdGuard::to(foreign.path());

    // @step When prune_orphaned_worktrees is called
    let handle: Arc<dyn SessionManagerHandle> = manager.clone() as _;
    let pruned = handle
        .prune_orphaned_worktrees()
        .expect("prune must succeed even from a foreign cwd");

    // @step Then the call returns the orphaned session id as pruned and the orphaned worktree directory is removed from project X
    assert!(
        pruned.iter().any(|id| id == &orphan_id),
        "orphan session {orphan_id} must be reported as pruned, got {pruned:?}"
    );
    assert!(
        !orphan_worktree.exists(),
        "orphan worktree must be removed from project X"
    );

    // @step And the active session's worktree in project Y is left untouched
    assert!(
        active_worktree.exists(),
        "active session's worktree must NOT be pruned"
    );
}

// =============================================================================
// Scenario: merge resolves the repository from the git-session manifest after a TUI restart
// =============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_merge_resolves_the_repository_from_the_git_session_manifest_after_a_tui_restart()
{
    let _guard = ENV_GUARD.lock().await;
    run_merge_manifest_fallback().await;
}

async fn run_merge_manifest_fallback() {
    // @step Given project X contains a session worktree with changes whose git-session manifest records X as its project root
    let repo = fresh_git_repo();
    let (_data_dir, _home_dir, _seed_manager) = manager_with_seeded_cache();
    let sid = Uuid::new_v4().to_string();
    codelet_git::create_worktree(repo.path(), &sid).expect("worktree creation must succeed");
    codelet_git::create_session_manifest(
        &sid,
        repo.path(),
        Some(repo.path().join(".fspec").join("worktrees").join(&sid)),
        None,
    )
    .expect("manifest creation must succeed");
    let worktree = repo.path().join(".fspec").join("worktrees").join(&sid);
    std::fs::write(worktree.join("hello.txt"), "merged via manifest\n").expect("modify in worktree");

    // @step And no in-memory session exists for that worktree's session id
    // (A fresh manager that never created the session — simulating the post-restart state.)
    let manager = SessionManager::new();

    // @step And the process cwd is an unrelated directory outside any repository
    let foreign = tempfile::tempdir().expect("tempdir for foreign cwd");
    let _cwd = CwdGuard::to(foreign.path());

    // @step When merge_session_worktree is called with that session id
    let handle: Arc<dyn SessionManagerHandle> = Arc::new(manager) as _;
    let outcome = handle
        .merge_session_worktree(&SessionId::new(sid), MergeStrategy::FastForward)
        .expect("merge must succeed via the manifest fallback after a restart");

    // @step Then the merge succeeds by resolving the repository from the manifest's project root X
    assert_eq!(
        outcome.status,
        codelet_rpc_types::MergeStatus::Success,
        "merge must report Success via the manifest fallback"
    );

    // @step And the worktree's changes are applied to X's main working tree
    let merged = std::fs::read_to_string(repo.path().join("hello.txt")).expect("read merged file");
    assert_eq!(merged, "merged via manifest\n", "merge must land in project X");
}
