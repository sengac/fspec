//! Existing session approval fixtures, reused without the sessions package's
//! unrelated dev dependencies. No agent loops or model calls are started.
// The reused source-inspection fixture resolves files relative to its own
// sessions package, rather than this lightweight runner package.
macro_rules! env {
    ("CARGO_MANIFEST_DIR") => {
        concat!(::std::env!("CARGO_MANIFEST_DIR"), "/../sessions")
    };
}
#[path = "../../sessions/tests/rpc410_hitl_wire_protocol_parity.rs"]
mod hitl_parity;
#[path = "../../sessions/tests/paused_chunk_delivery_rpc409.rs"]
mod pause_delivery;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checked_handle_rejects_replaced_and_duplicate_pause(
) -> Result<(), Box<dyn std::error::Error>> {
    use codelet_core::session_manager_handle::SessionManagerHandle;
    use codelet_tools::tool_pause::{PauseKind, PauseState};
    let dir = tempfile::tempdir()?;
    let _ = codelet_common::set_data_directory(dir.path().to_path_buf());
    let manager = codelet_sessions::SessionManager::new();
    manager.set_default_model("anthropic/claude-opus-4-5");
    let handle: &dyn SessionManagerHandle = &manager;
    let id = handle.create_session(None);
    let session = manager.get_session(&id.value)?;
    let pause = PauseState {
        kind: PauseKind::Confirm,
        tool_name: "fixture".into(),
        message: "fixture only".into(),
        details: None,
    };
    session.set_pause_state(Some(pause.clone()));
    let old: serde_json::Value = serde_json::from_str(&handle.interaction_snapshot(&id)?)?;
    session.set_pause_state(Some(pause));
    assert!(handle
        .reply_interaction_checked(
            &id,
            "pause_confirm",
            old["pause"]["request_id"].as_str().ok_or("request id")?,
            r#"{"accept":false}"#
        )
        .is_err());
    let current: serde_json::Value = serde_json::from_str(&handle.interaction_snapshot(&id)?)?;
    let nonce = current["pause"]["request_id"]
        .as_str()
        .ok_or("request id")?;
    handle.reply_interaction_checked(&id, "pause_confirm", nonce, r#"{"accept":false}"#)?;
    assert!(handle
        .reply_interaction_checked(&id, "pause_confirm", nonce, r#"{"accept":false}"#)
        .is_err());
    Ok(())
}
