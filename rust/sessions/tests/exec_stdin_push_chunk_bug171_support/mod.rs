//! Shared harness for the BUG-171 exec-stdin push suites
//! (exec_stdin_push_chunk_bug171.rs, exec_stdin_detector_clear_bug171.rs,
//! exec_stdin_detector_cooldown_bug171.rs). REFAC-060 moved these helpers
//! here unchanged: one process-wide data dir, fake ANTHROPIC_API_KEY, and
//! the unified_exec run/close helpers.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use codelet_tools::unified_exec::{UnifiedExecArgs, UnifiedExecResult, UnifiedExecTool};
use rig::tool::Tool;
use uuid::Uuid;

/// Process-wide data dir (one for the whole test binary — the
/// persistence `SessionStore` caches its sessions_dir at first touch,
/// so per-test `set_data_directory` swaps would leave the cached store
/// pointing at a deleted tempdir).
static DATA_DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

pub fn data_dir() -> &'static std::path::PathBuf {
    DATA_DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("bug171-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create bug171 data dir");
        codelet_common::set_data_directory(dir.clone()).expect("set the bug171 data directory");
        dir
    })
}

/// Create a fresh BackgroundSession via the SessionManagerHandle bridge.
pub fn fresh_session(manager: &SessionManager) -> SessionId {
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    std::env::set_var("ANTHROPIC_API_KEY", "bug171-fake-key");
    assert_eq!(
        manager.set_default_model("anthropic/claude-opus-4-5"),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = manager;
    let sid = handle
        .create_session_checked(None)
        .expect("the handle creates the session");
    assert!(
        !sid.value.is_empty(),
        "create_session must succeed (fake env key + default model set)"
    );
    sid
}

/// Run a still-running command through the unified_exec tool, owned by
/// the given agent session (so the detector's emit lands on that
/// session's registered callback).
pub async fn run_still_running(agent: Uuid, command: &str) -> (UnifiedExecTool, String) {
    // BUG-447: this test drives tools without a session manager callback.
    codelet_tools::facade::declare_unrestricted_tool_access(
        "tests drive tools without a session manager",
    );
    let tool = UnifiedExecTool::new(agent);
    let result: UnifiedExecResult = tool
        .call(UnifiedExecArgs(serde_json::json!({
            "action": "run",
            "command": command,
            "yield_time_ms": 50
        })))
        .await
        .expect("run action must succeed");
    let session_id = result
        .session_id
        .expect("still-running command must carry a session_id");
    (tool, session_id)
}

/// Close the exec session; a failed close fails the test (BUG-415).
pub async fn close_exec(tool: &UnifiedExecTool, session_id: &str) {
    if let Err(error) = tool
        .call(UnifiedExecArgs(serde_json::json!({
            "action": "close",
            "session_id": session_id
        })))
        .await
    {
        panic!("closing exec session {session_id} failed: {error}");
    }
}
