//! Shared helpers for the TOOL-022 P2 backend tests (`exec_stdin_prompt_p2.rs`).
//! A sub-directory module under `tests/` is not its own test binary, so the
//! exec-stdin-prompt feature keeps one test file.

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use codelet_tools::unified_exec::{UnifiedExecArgs, UnifiedExecResult, UnifiedExecTool};
use rig::tool::Tool;
use uuid::Uuid;

/// Process-wide data dir (one for the whole test binary — see module
/// docs for why it is NOT per-test).
pub static DATA_DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

pub fn data_dir() -> &'static std::path::PathBuf {
    DATA_DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("tool022-p2-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create tool022-p2 data dir");
        codelet_common::set_data_directory(dir.clone()).expect("set the tool022-p2 data directory");
        dir
    })
}

/// Create a fresh BackgroundSession via the SessionManagerHandle bridge.
/// The Noop hooks ensure no agent loop is spawned for the session.
pub fn fresh_session(manager: &SessionManager) -> SessionId {
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    std::env::set_var("ANTHROPIC_API_KEY", "tool022-fake-key");
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

/// Wait until `store.contains(id)` flips to the expected value (TEST-042:
/// polled between yields, under the deadlock guard).
pub async fn wait_store_state(id: &str, expect: bool) {
    let reached = codelet_test_helpers::wait_until("the exec store state", || async move {
        codelet_tools::unified_exec::global_store()
            .contains(id)
            .await
            == expect
    })
    .await;
    assert!(
        reached.is_ok(),
        "store.contains({id}) did not reach {expect}: {reached:?}"
    );
}

/// Wait until the quiet detector has stored its exec-stdin request on
/// `session` (TEST-042: it fires on its own cadence; the test waits on the
/// stored state, not a fixed time).
pub async fn wait_for_stored_request(
    session: &codelet_sessions::background_session::BackgroundSession,
) {
    let stored =
        codelet_test_helpers::wait_until("the detector stores the exec-stdin request", || {
            std::future::ready(matches!(session.get_exec_stdin_request(), Ok(Some(_))))
        })
        .await;
    assert!(stored.is_ok(), "{stored:?}");
}

/// Run a command through the unified_exec tool and return the
/// still-running session id (the detector is spawned with the reaper).
/// The tool is created with the OWNING agent session's Uuid so the
/// detector's emit lands on that session's registered callback.
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

/// Close the exec session (kills the child; the store entry is removed
/// by `close` itself).
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
