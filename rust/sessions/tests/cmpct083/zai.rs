use super::{
    fixture::TestResult,
    runtime::{self, Output},
};
use codelet_cli::session::Session;
use codelet_core::RigAgent;
use codelet_providers::{LlmProvider, ProviderType};
use rig::client::CompletionClient;
use std::sync::Arc;
use wiremock::MockServer;

pub async fn send(
    row: &str,
    server: &MockServer,
    inner: &mut Session,
    id: uuid::Uuid,
    output: &Output,
) -> TestResult {
    assert_eq!(
        inner.provider_manager().current_provider_type(),
        &ProviderType::ZAI
    );
    assert_eq!(inner.current_provider_name(), "zai");
    assert_eq!(inner.current_model_id().as_deref(), Some("glm-4.7"));
    let provider = inner.provider_manager().get_zai()?;
    assert_eq!(
        inner.provider_manager().max_output_tokens(),
        provider.max_output_tokens(),
        "selected Z.AI reservation must match its unmodified native builder allowance"
    );
    assert_eq!(provider.name(), "zai");
    assert_eq!(provider.model(), "glm-4.7");
    let plan = row == "zai-plan";
    assert_eq!(provider.is_plan_endpoint(), plan);
    let path = if plan {
        "/api/coding/paas/v4"
    } else {
        "/api/paas/v4"
    };
    assert_eq!(provider.base_url(), format!("https://api.z.ai{path}"));
    assert_eq!(provider.client().base_url(), provider.base_url());
    let key = if plan {
        "offline-gate-plan-key"
    } else {
        "offline-gate-key"
    };
    assert_eq!(
        provider.client().headers()["authorization"],
        format!("Bearer {key}")
    );
    let mut agent = provider.create_rig_agent(id, Some("mandatory gate instructions"), None);
    assert_eq!(agent.model.model, provider.model());
    assert_eq!(
        agent.additional_params,
        Some(serde_json::json!({"temperature":1.0,"top_p":0.95}))
    );
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key(key)
            .http_headers(provider.client().headers().clone())
            .base_url(format!("{}{path}", server.uri()))
            .build()?;
    let mut model = client.completion_model(provider.model());
    model.stream = agent.model.stream;
    model.strict_tools = agent.model.strict_tools;
    model.tool_result_array_content = agent.model.tool_result_array_content;
    agent.model = Arc::new(model);
    runtime::prepare_turn(inner, id, output)?;
    runtime::run_agent(RigAgent::new(agent, 3), inner, id, output).await?;
    let requests = server
        .received_requests()
        .await
        .ok_or("Z.AI physical capture")?;
    assert_eq!(
        requests.len(),
        1,
        "Z.AI must not fall back to an alternate request"
    );
    assert_eq!(requests[0].url.path(), format!("{path}/chat/completions"));
    assert_eq!(
        requests[0].headers["authorization"],
        format!("Bearer {key}")
    );
    let wire: serde_json::Value = requests[0].body_json()?;
    assert_eq!(wire["model"], "glm-4.7");
    assert_eq!(wire["temperature"], 1.0);
    assert_eq!(wire["top_p"], 0.95);
    assert_eq!(wire["max_completion_tokens"], provider.max_output_tokens());
    assert!(wire["tools"]
        .as_array()
        .ok_or("Z.AI native tools")?
        .iter()
        .any(|tool| tool["function"]["name"] == "read_file"));
    Ok(())
}
