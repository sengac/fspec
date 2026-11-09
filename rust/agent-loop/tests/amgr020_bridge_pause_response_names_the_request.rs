//! Feature: spec/features/questions-and-approvals-are-requests-answered-by-id.feature
//!
//! AMGR-020: a bridge (remote dashboard) `pause_response` answers an
//! approval only when it names the pending request's id. The session's
//! bridge control handler (the one each turn registers) receives the
//! envelope through `handle_multiplexed_inbound`, which calls it inline, so
//! when the call returns the handler has finished (TEST-042 causality).

use codelet_rpc_types::RequestRoute;
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use codelet_tools::tool_pause::{PauseKind, PauseResponse, PauseState};
use std::sync::Arc;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Cancels whatever the session still waits on so the waiting thread never
/// outlives the test.
struct Release(Arc<BackgroundSession>);

impl Drop for Release {
    fn drop(&mut self) {
        self.0.cancel_pending_interactions();
    }
}

fn approval() -> PauseState {
    PauseState {
        kind: PauseKind::Triple,
        tool_name: "Bash".into(),
        message: "Command requires approval".into(),
        details: Some("echo amgr020 git reset --hard".into()),
        tool_call_id: None,
    }
}

async fn send(session: &Arc<BackgroundSession>, data: serde_json::Value) -> TestResult {
    let envelope = serde_json::json!({
        "service": "session",
        "type": "control",
        "session_id": session.id.to_string(),
        "data": data,
    });
    let injector: codelet_tools::InputInjector = Arc::new(|_| {});
    codelet_tools::handle_multiplexed_inbound(
        &envelope.to_string(),
        session.id,
        injector,
        Some(codelet_agent_loop::bridge_control::control_handler(
            session.clone(),
        )),
        None,
        None,
    )
    .await?;
    Ok(())
}

fn pending(session: &BackgroundSession) -> TestResult<RequestRoute> {
    Ok(session
        .pending_pause_route()
        .ok_or("the session has no pending approval")?)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_a_bridge_pause_response_must_name_the_request_id() -> TestResult {
    codelet_test_helpers::isolated_user_directory()?;
    let data = codelet_test_helpers::setup_temp_directory()?;
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(data.path().to_owned())?;
    // The session selects a model at creation; no request is ever sent.
    std::env::set_var("ANTHROPIC_API_KEY", "offline");
    let project = codelet_test_helpers::setup_temp_directory()?;
    let manager = SessionManager::new();
    let id = uuid::Uuid::new_v4().to_string();
    manager
        .create_session_with_id(
            &id,
            "anthropic/claude-sonnet-4-5",
            &project.path().to_string_lossy(),
            "amgr020",
        )
        .await?;
    let session = manager.get_session(&id)?;
    let _release = Release(session.clone());

    // @step Given a session waits on a blocklist approval
    session.set_pause_state(Some(approval()));
    let (answered, answer) = tokio::sync::oneshot::channel();
    let waiting = session.clone();
    std::thread::spawn(move || {
        let _ = answered.send(waiting.wait_for_pause_response());
    });
    let route = pending(&session)?;

    // @step When the bridge sends pause_response for it without a request id and then with the approval's id
    send(
        &session,
        serde_json::json!({"action": "pause_response", "response": "allow_once"}),
    )
    .await?;
    let after_first = pending(&session)?;
    send(
        &session,
        serde_json::json!({
            "action": "pause_response",
            "response": "deny",
            "request_id": route.request_id,
        }),
    )
    .await?;

    // @step Then the first is rejected leaving the approval pending and the second answers it
    assert_eq!(after_first, route);
    let response =
        codelet_test_helpers::deadlock_guard("the approval is answered", answer).await??;
    assert_eq!(response, PauseResponse::Denied);
    assert!(session.pending_pause_route().is_none());
    Ok(())
}
