//! Shared helpers for the TUI-099 session-ordering tests
//! (`tui099_session_list_ordering.rs`). A sub-directory module under `tests/`
//! is not its own test binary, so the feature keeps one test file.

use std::path::PathBuf;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{SessionId, SessionInfo};
use tokio::sync::Mutex;

/// PROV-132: Serialize tests that swap the process-global data directory.
pub static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

/// Offline models.dev catalog that knows `anthropic/claude-sonnet-4`
/// (shared with the RPC-422 tests).
const MODELS_FIXTURE: &str = include_str!("../fixtures/rpc422_models.json");

/// Helper: create a temp data directory with a seeded offline model cache and
/// a dummy anthropic credential, and return its path. BUG-409: without them
/// session creation fails in the fresh data dir (the BUG-186 pattern).
pub fn make_temp_data_dir() -> PathBuf {
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    let dir = tempfile::tempdir().expect("tempdir").keep();
    let cache_dir = dir.join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("write models fixture");
    dir
}

/// Helper: set the data directory and return a guard that cleans it up.
pub fn set_temp_data_dir(path: PathBuf) -> PathBuf {
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(path.clone()).expect("set_data_directory");
    // Reset the persistence singletons so they re-initialize against the new data dir.
    codelet_core::persistence::reset_stores_for_tests();
    path
}

/// Helper: construct a SessionInfo with a specific updated_at_ms.
pub fn make_session(id: &str, ts: Option<i64>) -> SessionInfo {
    SessionInfo {
        id: id.to_string(),
        name: format!("Session {id}"),
        status: "idle".to_string(),
        project: ".".to_string(),
        message_count: 0,
        provider_id: Some("anthropic".to_string()),
        model_id: Some("claude-sonnet-4".to_string()),
        is_isolated: false,
        worktree_path: None,
        role: None,
        updated_at_ms: ts,
        work_unit_focus: Default::default(),
    }
}

/// Helper: apply the expected sorting algorithm to a Vec<SessionInfo>.
/// This mirrors the sorting logic that list_sessions() should implement.
pub fn sort_sessions(sessions: &mut [SessionInfo]) {
    sessions.sort_by(|a, b| match (a.updated_at_ms, b.updated_at_ms) {
        (Some(ts_a), Some(ts_b)) => ts_b.cmp(&ts_a).then_with(|| a.id.cmp(&b.id)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.id.cmp(&b.id),
    });
}

/// BUG-399: create a session through the checked trait method; a
/// creation failure fails the test with its cause.
pub fn create_checked(handle: &dyn SessionManagerHandle) -> SessionId {
    handle
        .create_session_checked(None)
        .expect("the handle creates the session")
}
