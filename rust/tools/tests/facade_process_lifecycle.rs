//! Feature: spec/features/provider-facade-parity.feature

use codelet_tools::facade::wrapper::FacadeArgs;
use codelet_tools::facade::{CodexExecCommandFacade, ExecToolFacadeWrapper};
use codelet_tools::unified_exec::{set_exec_stdin_request_callback, ExecStdinRequest};
use rig::tool::Tool;
use serde_json::json;
use std::{sync::Arc, time::Duration};

#[tokio::test]
#[serial_test::serial]
async fn process_clear_survives_completion_of_the_originating_model_turn(
) -> Result<(), Box<dyn std::error::Error>> {
    let agent = uuid::Uuid::new_v4();
    let other = uuid::Uuid::new_v4();
    let (sender, mut events) = tokio::sync::mpsc::unbounded_channel();
    set_exec_stdin_request_callback(
        agent,
        Some(Arc::new(move |request| {
            let _ = sender.send(request);
        })),
    );
    let (other_sender, mut other_events) = tokio::sync::mpsc::unbounded_channel();
    set_exec_stdin_request_callback(
        other,
        Some(Arc::new(move |request| {
            let _ = other_sender.send(request);
        })),
    );
    let tool = ExecToolFacadeWrapper::new(Arc::new(CodexExecCommandFacade), agent);
    let result = tool
        .call(FacadeArgs(json!({"cmd":"sleep 5", "yield_time_ms":1})))
        .await?;
    let exec_id = result.session_id.ok_or("command must remain alive")?;
    let prompt: Option<ExecStdinRequest> =
        tokio::time::timeout(Duration::from_secs(8), events.recv())
            .await?
            .ok_or("observer closed")?;
    assert_eq!(prompt.ok_or("expected prompt")?.exec_session_id, exec_id);
    set_exec_stdin_request_callback(agent, None);
    let clear = tokio::time::timeout(Duration::from_secs(3), events.recv()).await?;
    assert_eq!(clear, Some(None));
    assert!(events.try_recv().is_err());
    assert!(other_events.try_recv().is_err());
    set_exec_stdin_request_callback(other, None);
    Ok(())
}
