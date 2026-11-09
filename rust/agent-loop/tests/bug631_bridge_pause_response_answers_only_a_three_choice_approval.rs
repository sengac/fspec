//! Feature: spec/features/a-pause-reply-answers-only-the-pause-kind-it-is-for.feature
//!
//! BUG-631: the bridge (remote dashboard) `pause_response` speaks the
//! three-choice vocabulary (allow_once / allow_session / deny), so it answers
//! a three-choice approval only. The session's bridge control handler
//! receives the envelope through `handle_multiplexed_inbound`, which calls it
//! inline, so when the call returns the handler has finished; cancelling the
//! pause then makes the wait return `Interrupted`, proving the rejected
//! reply sent nothing (TEST-042 causality).

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_a_bridge_pause_response_answers_only_a_three_choice_approval() -> TestResult {
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
            "bug631",
        )
        .await?;
    let session = manager.get_session(&id)?;
    let _release = Release(session.clone());

    // @step Given a session waiting on a WebSearch human action
    session.set_pause_state(Some(PauseState {
        kind: PauseKind::Continue,
        tool_name: "WebSearch".into(),
        message: "Page loaded at https://example.com. Interact with the page, then press Enter to capture screenshot.".into(),
        details: None,
        tool_call_id: None,
    }));
    let (answered, answer) = tokio::sync::oneshot::channel();
    let waiting = session.clone();
    std::thread::spawn(move || {
        let _ = answered.send(waiting.wait_for_pause_response());
    });
    let route = session
        .pending_pause_route()
        .ok_or("the session has no pending human action")?;

    // @step When the bridge sends pause_response "allow_once" naming its request id
    send(
        &session,
        serde_json::json!({
            "action": "pause_response",
            "response": "allow_once",
            "request_id": route.request_id,
        }),
    )
    .await?;

    // @step Then the human action is still pending under the same id and its tool is still waiting
    assert_eq!(session.pending_pause_route(), Some(route.clone()));
    assert_eq!(route.kind, codelet_rpc_types::RequestKind::HumanAction);
    session.cancel_pending_interactions();
    let response =
        codelet_test_helpers::deadlock_guard("the human action is cancelled", answer).await??;
    assert_eq!(response, PauseResponse::Interrupted);
    Ok(())
}
