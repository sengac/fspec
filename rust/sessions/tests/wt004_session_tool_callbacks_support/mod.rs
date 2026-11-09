//! Shared helpers for the WT-004 tool-callbacks test files
//! (wt004_session_tool_callbacks.rs and wt004_tool_callbacks_wiring.rs),
//! split out by REFAC-053.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use codelet_sessions::SessionManager;
use codelet_tools::facade::{
    set_block_notification_callback, set_get_effective_cwd_callback,
    set_get_work_unit_stage_callback,
};

/// Trimmed offline models.dev catalog (anthropic/openai/google) — shared
/// with the RPC-385 / WT-001 / WT-002 precedent so registry validation stays
/// fully offline.
const MODELS_FIXTURE: &str = include_str!("../fixtures/prov101_models.json");

/// Serialises tests that swap process-global state (data dir, HOME, the
/// tool-callbacks manager slot, the footer-poller chunk-sender slot, and
/// codelet-tools' callback OnceLocks) so parallel tests cannot observe
/// each other's state.
pub static ENV_GUARD: Mutex<()> = Mutex::const_new(());

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
/// `main` and one committed file.
pub fn fresh_git_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir for git repo");
    let path = dir.path();
    run_git(path, &["init", "-b", "main"]);
    run_git(path, &["config", "user.email", "test@example.com"]);
    run_git(path, &["config", "user.name", "Test User"]);
    std::fs::write(path.join("hello.txt"), "hello\n").expect("write hello.txt");
    std::fs::create_dir_all(path.join("src")).expect("create src dir");
    std::fs::write(path.join("src").join("main.rs"), "fn main() {}\n").expect("write src/main.rs");
    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "initial commit"]);
    dir
}

/// Process-wide hermetic dirs, created ONCE per test binary:
/// `codelet_common::set_data_directory` is first-call-wins, so a per-test
/// data dir would leak into later tests whose tempdir is already dropped
/// (manifest writes then fail with ENOENT).
static SHARED_DIR: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();

fn shared_dir() -> &'static std::path::Path {
    SHARED_DIR
        .get_or_init(|| tempfile::TempDir::new().expect("tempdir for data dir"))
        .path()
}

/// Build a manager rooted in the shared hermetic data dir with the offline
/// models.dev fixture pre-seeded, with HOME redirected so the session
/// manifest writes stay hermetic.
pub fn manager_with_seeded_cache() -> Arc<SessionManager> {
    set_dummy_credentials();
    let cache_dir = shared_dir().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("seed models.json");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(shared_dir().to_path_buf())
        .expect("set_data_directory must accept the shared dir");
    let home_dir = shared_dir().join("home");
    std::fs::create_dir_all(&home_dir).expect("create home dir");
    std::env::set_var("HOME", &home_dir);
    // The manager owns its sessions (owning-manager back-references set).
    SessionManager::new()
}

/// Wire the tool-callbacks manager slot + the codelet-tools OnceLocks the
/// way `build_service` does it (WT-004 architecture note). Idempotent per
/// process: the OnceLocks accept the first set only, `register_manager`
/// replaces the previous registration.
pub fn register_tool_callbacks(manager: &Arc<SessionManager>) {
    codelet_sessions::session_tool_callbacks::register_manager(Arc::clone(manager));
    set_get_effective_cwd_callback(codelet_sessions::session_tool_callbacks::isolation_context);
    set_get_work_unit_stage_callback(codelet_sessions::session_tool_callbacks::work_unit_stage);
    set_block_notification_callback(
        codelet_sessions::session_tool_callbacks::emit_block_notification,
    );
}

/// Create a plain (non-isolated) session through the registered manager.
pub async fn create_plain(manager: &Arc<SessionManager>, project: &str) -> String {
    let id = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(&id, "anthropic/claude-opus-4-5", project, "wt004-plain")
        .await
        .expect("plain session creation must succeed");
    id
}
