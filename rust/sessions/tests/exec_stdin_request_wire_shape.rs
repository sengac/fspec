//! Feature: spec/features/exec-stdin-request-wire-shape.feature
//!
//! Moved out of exec_stdin_prompt_p2.rs (COV-075: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_rpc_types::ExecStdinRequest;

/// Wire-shape round-trip lock for the TOOL-022 P2 request (the TUI probe
/// path deserializes this exact shape over JSON in the websocket transport).
#[test]
fn wire_exec_stdin_request_round_trips_through_serde_json() {
    // @step Given a wire ExecStdinRequest { exec_session_id, command, quiet_seconds, ts_ms }
    let request = ExecStdinRequest {
        exec_session_id: "exec-live".to_string(),
        command: "git commit".to_string(),
        quiet_seconds: 5,
        ts_ms: 1_700_000_000_000,
    };

    // @step When the value is serialized to JSON and deserialized back
    let json = serde_json::to_string(&request).expect("serialize");
    let back: ExecStdinRequest = serde_json::from_str(&json).expect("deserialize");

    // @step Then the deserialized value equals the original
    assert_eq!(back, request, "round-trip must preserve the value exactly");
}
