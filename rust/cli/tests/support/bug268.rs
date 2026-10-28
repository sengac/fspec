use super::{runtime, wire};
use codelet_cli::{interactive::StreamEvent, session::Session};
use codelet_core::RigAgent;
use codelet_providers::copilot::{CopilotAuthJson, CopilotDeploymentType, CopilotProvider};
use codelet_providers::ProviderType;
use runtime::{Output, RuntimeFixture, TestResult};
use serde_json::{json, Value};
use wiremock::MockServer;

pub fn provider(server: &MockServer, model: &str) -> TestResult<CopilotProvider> {
    Ok(CopilotProvider::from_auth(
        CopilotDeploymentType::GitHubCom,
        CopilotAuthJson {
            github_oauth_token: "synthetic-logical-account".into(),
            copilot_token: Some("synthetic-copilot-token".into()),
            copilot_token_expires_at: Some(u64::MAX),
            endpoints_api: Some(server.uri()),
            enterprise_url: None,
        },
        model,
    )?)
}

pub async fn assert_request(
    server: &MockServer,
    model: &str,
    responses: bool,
    expected_count: usize,
) -> TestResult {
    let requests = server.received_requests().await.ok_or("missing requests")?;
    assert_eq!(requests.len(), expected_count);
    for request in requests {
        assert_eq!(
            request.url.path(),
            if responses {
                "/responses"
            } else {
                "/chat/completions"
            },
            "{model}"
        );
        let body: Value = request.body_json()?;
        assert_eq!(body["model"], model);
        assert!(body
            .get(if responses { "input" } else { "messages" })
            .is_some());
        assert!(body
            .get(if responses { "messages" } else { "input" })
            .is_none());
        assert_eq!(
            request.headers["authorization"],
            "Bearer synthetic-copilot-token"
        );
        assert_eq!(request.headers["openai-intent"], "conversation-edits");
        assert!(request.headers.contains_key("x-initiator"));
        assert!(request.headers["user-agent"].to_str()?.starts_with("rust/"));
    }
    Ok(())
}

pub async fn native_sequence(model: &str, responses: bool) -> TestResult {
    let fixture = RuntimeFixture::new()?;
    let directory = codelet_test_helpers::setup_temp_directory()?;
    let file = directory.path().join("evidence");
    std::fs::write(&file, "local tool evidence")?;
    let server = wire::server(true, &file.display().to_string()).await;
    let provider = provider(&server, model)?;
    let manager = fixture.manager(ProviderType::GitHubCopilot, json!({}))?;
    let session_id = fixture.session_id("github-copilot")?;
    let _admission = codelet_tools::tool_admission::register_auxiliary(
        session_id,
        codelet_common::get_data_dir()?,
    )?;
    codelet_tools::set_continue_armed(session_id, true);
    let agent = provider.create_rig_agent(session_id, None, None);
    let mut session = Session::from_provider_manager(manager);
    session.continue_enabled = true;
    session.continue_budget = 1;
    let output = Output::default();
    let outcome = runtime::run(RigAgent::new(agent, 8), &mut session, session_id, &output).await;
    codelet_tools::set_continue_armed(session_id, false);
    outcome?;
    {
        let chunks = output.0.lock().map_err(|error| error.to_string())?;
        assert!(chunks.iter().any(|event|matches!(event,StreamEvent::ToolResult(result) if result.content.contains("local tool evidence") && (result.id=="item-native" || result.id=="call-native"))));
        assert!(chunks.iter().any(
        |event| matches!(event,StreamEvent::Status(text) if text.contains("done: copilot complete"))
    ));
        assert!(!chunks.iter().any(|event|matches!(event,StreamEvent::ToolResult(result) if result.content.contains("invalid work_unit_id"))));
        assert_eq!(chunks.iter().filter(|event|matches!(event, StreamEvent::ContinueState(state) if state.reason == codelet_cli::interactive::ContinueStateReason::NudgeConsumed)).count(), 1);
    }
    assert_eq!(session.continue_nudges_used, 0);
    let manifest = codelet_core::persistence::load_session(session_id)?;
    assert_eq!(manifest.token_usage.cumulative_billed_input, 36);
    assert_eq!(manifest.token_usage.cumulative_billed_output, 9);
    let requests = server.received_requests().await.ok_or("missing capture")?;
    assert_eq!(requests[0].headers["x-initiator"], "user");
    assert_eq!(requests[1].headers["x-initiator"], "agent");
    let followup = String::from_utf8(requests[1].body.clone())?;
    assert!(followup.contains("local tool evidence") && followup.contains("call-native"));
    assert_request(&server, model, responses, 3).await
}
