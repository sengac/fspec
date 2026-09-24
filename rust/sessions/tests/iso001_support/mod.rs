//! Shared offline harness for the ISO-001 isolated-session file-operations
//! suite (`iso001_isolated_session_file_operations.rs`).
//!
//! Precedent: `wt004_session_tool_callbacks.rs` — a NON-singleton
//! `SessionManager` with the offline models.dev fixture seeded, HOME redirected
//! for hermetic manifest writes, and the tool-callbacks OnceLocks wired the
//! way `build_service` wires them (WT-004 architecture note).
//!
//! The harnesses return the `ENV_GUARD` lock guard so each test HOLDS it for
//! its whole duration (the tool-callbacks OnceLocks and manager slot are
//! process-global and must not be observed mid-swap by another test).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use codelet_rpc_types::{SessionId, StreamChunk};
use codelet_sessions::SessionManager;
use uuid::Uuid;

/// Trimmed offline models.dev catalog (anthropic/openai/google) — shared with
/// the RPC-385 / WT-001 / WT-002 precedent so registry validation stays
/// fully offline.
const MODELS_FIXTURE: &str = include_str!("../fixtures/prov101_models.json");

/// Serialises tests that swap process-global state (data dir, HOME, the
/// tool-callbacks manager slot, the footer-poller chunk-sender slot, and
/// codelet-tools' callback OnceLocks) so parallel tests cannot observe
/// each other's state.
pub(crate) static ENV_GUARD: std::sync::LazyLock<std::sync::Arc<tokio::sync::Mutex<()>>> =
    std::sync::LazyLock::new(|| std::sync::Arc::new(tokio::sync::Mutex::new(())));

/// Process-wide hermetic dirs, created ONCE per test binary:
/// `codelet_common::set_data_directory` is first-call-wins.
pub(crate) static SHARED_DIR: std::sync::OnceLock<tempfile::TempDir> =
    std::sync::OnceLock::new();

fn shared_dir() -> &'static Path {
    SHARED_DIR
        .get_or_init(|| tempfile::TempDir::new().expect("tempdir for data dir"))
        .path()
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

/// Create a fresh git repository (temp dir) with the branch explicitly named
/// `main` and one committed file under `src/`.
pub fn fresh_git_repo(src_file: &str, content: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir for git repo");
    let path = dir.path();
    run_git(path, &["init", "-b", "main"]);
    run_git(path, &["config", "user.email", "test@example.com"]);
    run_git(path, &["config", "user.name", "Test User"]);
    std::fs::create_dir_all(path.join("src")).expect("create src dir");
    std::fs::write(path.join("src").join(src_file), content).expect("write src file");
    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "initial commit"]);
    dir
}

/// Build a manager rooted in the shared hermetic data dir with the offline
/// models.dev fixture pre-seeded, HOME redirected for hermetic manifests.
fn manager_with_seeded_cache() -> Arc<SessionManager> {
    set_dummy_credentials();
    let cache_dir = shared_dir().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("seed models.json");
    let _ = codelet_common::set_data_directory(shared_dir().to_path_buf());
    let home_dir = shared_dir().join("home");
    std::fs::create_dir_all(&home_dir).expect("create home dir");
    std::env::set_var("HOME", &home_dir);
    Arc::new(SessionManager::new())
}

/// Wire the tool-callbacks manager slot + the codelet-tools OnceLocks the
/// way `build_service` does it (WT-004 architecture note). Idempotent per
/// process: the OnceLocks accept the first set only, `register_manager`
/// replaces the previous registration.
fn register_tool_callbacks(manager: &Arc<SessionManager>) {
    codelet_sessions::session_tool_callbacks::register_manager(Arc::clone(manager));
    codelet_tools::facade::set_get_effective_cwd_callback(
        codelet_sessions::session_tool_callbacks::isolation_context,
    );
    codelet_tools::facade::set_get_work_unit_stage_callback(
        codelet_sessions::session_tool_callbacks::work_unit_stage,
    );
    codelet_tools::facade::set_block_notification_callback(
        codelet_sessions::session_tool_callbacks::emit_block_notification,
    );
}

/// Shared wiring step (manager + callbacks + chunk-sender registration).
async fn wired_manager() -> (
    tokio::sync::OwnedMutexGuard<()>,
    Arc<SessionManager>,
    tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
) {
    let guard = ENV_GUARD.clone().lock_owned().await;
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);
    // Register the manager's chunks_tx as the block-notification emission
    // target (mirrors build_service's WT-002 registration) and hold a
    // subscriber so tests can observe the emitted chunks.
    codelet_sessions::footer_poller::register_chunk_sender(manager.chunks_tx().clone());
    (guard, manager.clone(), manager.chunks_tx().subscribe())
}

/// Create an isolated session through the registered manager's in-memory API
/// (the same call the fspec binary drives via RPC). Returns the session UUID.
async fn create_isolated_sync(
    manager: &Arc<SessionManager>,
    repo: &Path,
) -> (Uuid, PathBuf) {
    let id = Uuid::new_v4();
    let info = manager
        .create_isolated_session_with_id(
            &id.to_string(),
            "anthropic/claude-opus-4-5",
            repo.to_str().expect("repo path is utf8"),
            "iso001",
        )
        .await
        .expect("isolated session creation must succeed");
    (id, PathBuf::from(info.worktree_path))
}

/// Create a plain (non-isolated) session through the registered manager.
async fn create_plain_sync(manager: &Arc<SessionManager>, repo: &Path) -> Uuid {
    let id = Uuid::new_v4();
    manager
        .create_session_with_id(
            &id.to_string(),
            "anthropic/claude-opus-4-5",
            repo.to_str().expect("repo path is utf8"),
            "iso001-plain",
        )
        .await
        .expect("plain session creation must succeed");
    id
}

/// RAII harness for ISOLATED-scenario tests: fresh repo with one committed
/// `src/<src_file>`, non-singleton manager with tool callbacks registered,
/// an isolated session, its worktree path, and the env guard held until Drop.
#[allow(dead_code)]
pub struct IsoEnv {
    guard: tokio::sync::OwnedMutexGuard<()>,
    pub manager: Arc<SessionManager>,
    pub chunks_rx: tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
    pub repo: PathBuf,
    pub session_id: Uuid,
    pub worktree: PathBuf,
    _repo_keepalive: tempfile::TempDir,
}

pub async fn iso_env(src_file: &str, content: &str) -> IsoEnv {
    let (guard, manager, rx) = wired_manager().await;
    let repo = fresh_git_repo(src_file, content);
    let (session_id, worktree) = create_isolated_sync(&manager, repo.path()).await;
    let repo_path = repo.path().to_path_buf();
    IsoEnv {
        guard,
        manager,
        chunks_rx: rx,
        repo: repo_path,
        session_id,
        worktree,
        _repo_keepalive: repo,
    }
}

/// RAII harness for NON-ISOLATED-scenario tests.
#[allow(dead_code)]
pub struct PlainEnv {
    guard: tokio::sync::OwnedMutexGuard<()>,
    pub manager: Arc<SessionManager>,
    pub repo: PathBuf,
    pub session_id: Uuid,
    _repo_keepalive: tempfile::TempDir,
}

pub async fn plain_env(src_file: &str, content: &str) -> PlainEnv {
    let (guard, manager, _rx) = wired_manager().await;
    let repo = fresh_git_repo(src_file, content);
    let session_id = create_plain_sync(&manager, repo.path()).await;
    let repo_path = repo.path().to_path_buf();
    PlainEnv {
        guard,
        manager,
        repo: repo_path,
        session_id,
        _repo_keepalive: repo,
    }
}

/// A unique file under the shared hermetic dir standing in for "/tmp/<name>"
/// (a per-process unique sub-directory, never the repo, so isolation logic
/// treats it as "elsewhere on the filesystem").
pub fn unique_tmp_file(name: &str, content: &str) -> PathBuf {
    let dir = shared_dir().join("loose").join(Uuid::new_v4().to_string());
    std::fs::create_dir_all(&dir).expect("create loose dir");
    let file = dir.join(name);
    std::fs::write(&file, content).expect("write loose file");
    file
}

/// A unique directory under the shared hermetic dir (for /tmp-style dirs).
pub fn unique_tmp_dir() -> PathBuf {
    let dir = shared_dir().join("loose").join(Uuid::new_v4().to_string());
    std::fs::create_dir_all(&dir).expect("create loose dir");
    dir
}

/// Pull the next `UserNotification` chunk for `sid` off `rx`.
///
/// Each `recv` is bounded by the remaining budget so a missing notification
/// fails in `timeout` instead of blocking forever (the broadcast recv only
/// unblocks on a new item or a closed channel).
pub async fn next_user_notification(
    rx: &mut tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
    sid: &SessionId,
    timeout: std::time::Duration,
) -> StreamChunk {
    let deadline = tokio::time::Instant::now() + timeout;
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Ok((id, chunk))) => {
                if id == *sid && matches!(chunk, StreamChunk::UserNotification { .. }) {
                    return chunk;
                }
            }
            Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(tokio::sync::broadcast::error::RecvError::Closed)) => {
                panic!("chunks broadcast closed before the expected chunk arrived");
            }
            // Timed out waiting for the next item — re-check the deadline.
            Err(_) => {
                if tokio::time::Instant::now() >= deadline {
                    break;
                }
            }
        }
    }
    panic!("timed out waiting for a UserNotification chunk for {sid}");
}
