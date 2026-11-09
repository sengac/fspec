//! Feature: spec/features/implement-sessionmanagerhandle-for-codelet-sessions-sessionmanager.feature
use super::*;

// =============================================================================
// Scenario: Broadcast accessors round-trip chunks, logs, and status updates
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn scenario_broadcasts_round_trip() {
    // @step Given a fresh "SessionManager" wrapped as "Arc<dyn SessionManagerHandle>"
    let handle = make_handle();

    // @step When I subscribe via "handle.chunks_rx()", "handle.logs_rx()", and "handle.status_changes_rx()"
    let mut chunks_rx = handle.chunks_rx();
    let mut logs_rx = handle.logs_rx();
    let mut status_rx = handle.status_changes_rx();

    let sid = SessionId::new("abc");

    // @step And I publish "(SessionId::new(\"abc\"), StreamChunk::done())" via "handle.chunks_tx().send(...)"
    let chunks_tx = handle.chunks_tx();
    chunks_tx
        .send((sid.clone(), StreamChunk::done()))
        .expect("chunks_tx send must succeed when a subscriber exists");

    // @step And I publish "(SessionId::new(\"abc\"), SessionStatus::Idle)" via "handle.status_changes_tx().send(...)"
    let status_tx = handle.status_changes_tx();
    status_tx
        .send((sid.clone(), SessionStatus::Idle))
        .expect("status_changes_tx send must succeed when a subscriber exists");

    // @step And I publish a "LogRecord" via "handle.logs_tx().send(...)"
    let logs_tx = handle.logs_tx();
    logs_tx
        .send(LogRecord {
            level: "INFO".to_string(),
            target: "rpc-042".to_string(),
            message: "round-trip".to_string(),
            timestamp_ms: 0,
        })
        .expect("logs_tx send must succeed when a subscriber exists");

    // @step Then every subscriber observes the published item in arrival order
    let chunk_recv = codelet_test_helpers::deadlock_guard("chunks_rx.recv()", chunks_rx.recv())
        .await
        .expect("chunks_rx recv timed out")
        .expect("chunks_rx recv lagged or closed");
    assert_eq!(chunk_recv.0, sid);

    let status_recv = codelet_test_helpers::deadlock_guard("status_rx.recv()", status_rx.recv())
        .await
        .expect("status_rx recv timed out")
        .expect("status_rx recv lagged or closed");
    assert_eq!(status_recv, (sid.clone(), SessionStatus::Idle));

    let log_recv = codelet_test_helpers::deadlock_guard("logs_rx.recv()", logs_rx.recv())
        .await
        .expect("logs_rx recv timed out")
        .expect("logs_rx recv lagged or closed");
    assert_eq!(log_recv.message, "round-trip");

    // @step And no broadcast lag or "Closed" error is observed for a single-item publish
    // (All three `expect("... lagged or closed")` calls above passed.)
}

// =============================================================================
// Scenario: Active session tracking is manager-scoped and works without a
// real session row
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn scenario_active_session_tracking_round_trip() {
    // @step Given a fresh "SessionManager" wrapped as "Arc<dyn SessionManagerHandle>"
    let handle = make_handle();

    let sid = SessionId::new("00000000-0000-0000-0000-000000000001");

    // @step When I call "handle.set_active_session(&SessionId::new(\"00000000-0000-0000-0000-000000000001\"))"
    assert_eq!(handle.set_active_session(&sid), Ok(()));

    // @step Then "handle.get_active_session()" returns "Some(SessionId::new(\"00000000-0000-0000-0000-000000000001\"))"
    assert_eq!(handle.get_active_session(), Ok(Some(sid)));

    // @step When I call "handle.clear_active_session()"
    assert_eq!(handle.clear_active_session(), Ok(()));

    // @step Then "handle.get_active_session()" returns "None"
    assert_eq!(handle.get_active_session(), Ok(None));
}

// =============================================================================
// Scenario: Conversion helpers map every variant of tool_pause to its
// rpc-types peer
// =============================================================================
#[test]
fn scenario_conversion_helpers_map_every_variant() {
    // @step Given the conversions module is reachable as "codelet_sessions::conversions"
    // (Established by the `use` statements at the top of this file.)

    // @step When I call "pause_state_to_rpc(tool_pause::PauseState { kind: Continue, .. })"
    let from_continue: codelet_rpc_types::PauseState = pause_state_to_rpc(ToolPauseState {
        kind: ToolPauseKind::Continue,
        tool_name: "Bash".into(),
        message: "Press Enter to continue".into(),
        details: None,
        tool_call_id: None,
    });
    // @step Then the resulting "rpc_types::PauseState.kind" equals "rpc_types::PauseKind::HumanAction"
    assert_eq!(from_continue.kind, RpcPauseKind::HumanAction);

    // @step When I call "pause_state_to_rpc(tool_pause::PauseState { kind: Confirm, .. })"
    let from_confirm: codelet_rpc_types::PauseState = pause_state_to_rpc(ToolPauseState {
        kind: ToolPauseKind::Confirm,
        tool_name: "Bash".into(),
        message: "Approve?".into(),
        details: Some("tc-42".into()),
        tool_call_id: None,
    });
    // @step Then the resulting "rpc_types::PauseState.kind" equals "rpc_types::PauseKind::Confirm"
    assert_eq!(from_confirm.kind, RpcPauseKind::Confirm);
    assert!(from_confirm.prompt.contains("Bash"));
    assert!(from_confirm.prompt.contains("Approve?"));
    assert_eq!(from_confirm.details.as_deref(), Some("tc-42"));
    assert_eq!(from_confirm.tool_call_id, None);

    // @step When I call "pause_state_to_rpc(tool_pause::PauseState { kind: Triple, .. })"
    let from_triple: codelet_rpc_types::PauseState = pause_state_to_rpc(ToolPauseState {
        kind: ToolPauseKind::Triple,
        tool_name: "Bash".into(),
        message: "Pick".into(),
        details: None,
        tool_call_id: None,
    });
    // @step Then the resulting "rpc_types::PauseState.kind" equals "rpc_types::PauseKind::Triple"
    assert_eq!(from_triple.kind, RpcPauseKind::Triple);

    // @step When I call "approval_choice_to_pause_response(ApprovalChoice::Approve)"
    // @step Then the result is "tool_pause::PauseResponse::AllowOnce"
    assert_eq!(
        approval_choice_to_pause_response(ApprovalChoice::Approve),
        ToolPauseResponse::AllowOnce,
    );

    // @step When I call "approval_choice_to_pause_response(ApprovalChoice::ApproveSession)"
    // @step Then the result is "tool_pause::PauseResponse::AllowSession"
    assert_eq!(
        approval_choice_to_pause_response(ApprovalChoice::ApproveSession),
        ToolPauseResponse::AllowSession,
    );

    // @step When I call "approval_choice_to_pause_response(ApprovalChoice::Deny)"
    // @step Then the result is "tool_pause::PauseResponse::Denied"
    assert_eq!(
        approval_choice_to_pause_response(ApprovalChoice::Deny),
        ToolPauseResponse::Denied,
    );

    // @step When I call "confirm_accept_to_pause_response(true)"
    // @step Then the result is "tool_pause::PauseResponse::Approved"
    assert_eq!(
        confirm_accept_to_pause_response(true),
        ToolPauseResponse::Approved,
    );

    // @step When I call "confirm_accept_to_pause_response(false)"
    // @step Then the result is "tool_pause::PauseResponse::Denied"
    assert_eq!(
        confirm_accept_to_pause_response(false),
        ToolPauseResponse::Denied,
    );
}
