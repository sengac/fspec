//! Existing session approval fixtures, reused without the sessions package's
//! unrelated dev dependencies. No agent loops or model calls are started.
//!
//! Feature: spec/features/client-mcp-stale-approval-replies.feature
#[path = "../../sessions/tests/rpc410_hitl_wire_protocol_parity.rs"]
mod hitl_parity;
#[path = "../../sessions/tests/paused_chunk_delivery_rpc409.rs"]
mod pause_delivery;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checked_handle_rejects_replaced_and_duplicate_pause(
) -> Result<(), Box<dyn std::error::Error>> {
    use codelet_core::session_manager_handle::SessionManagerHandle;
    use codelet_tools::tool_pause::{PauseKind, PauseState};
    // @step Given a pending daemon interaction
    let dir = codelet_test_helpers::shared_temp_subdirectory()?;
    codelet_common::set_data_directory(dir.clone())?;
    let manager = codelet_sessions::SessionManager::new();
    // BUG-437: sessions need a start workspace.
    manager.set_default_project(codelet_test_helpers::shared_temp_subdirectory()?)?;
    manager.set_default_model("anthropic/claude-opus-4-5")?;
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let id = handle.create_session_checked(None)?;
    let session = manager.get_session(&id.value)?;
    let pause = PauseState {
        kind: PauseKind::Confirm,
        tool_name: "fixture".into(),
        message: "fixture only".into(),
        details: None,
        tool_call_id: None,
    };
    session.set_pause_state(Some(pause.clone()));
    // AMGR-020: the pending request's id comes with get_pause_state.
    let old = handle
        .get_pause_state(&id)?
        .ok_or("pending pause")?
        .route
        .request_id;
    // @step When that interaction changes before its checked response is dispatched
    session.set_pause_state(Some(pause));
    // @step Then the response is rejected without answering the newer interaction
    assert!(handle.pause_confirm(&id, &old, false).is_err());
    let current = handle
        .get_pause_state(&id)?
        .ok_or("pending pause")?
        .route
        .request_id;
    assert_ne!(current, old);
    handle.pause_confirm(&id, &current, false)?;
    assert!(handle.pause_confirm(&id, &current, false).is_err());
    Ok(())
}
