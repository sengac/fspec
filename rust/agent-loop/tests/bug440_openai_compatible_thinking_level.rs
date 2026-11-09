//! BUG-440 — OpenAI-compatible providers send the selected thinking level.
//!
//! Feature: spec/features/openai-compatible-providers-send-the-thinking-level.feature

use codelet_agent_loop::thinking_config::{ThinkingTarget, ThinkingWire};
use codelet_agent_loop::thinking_level_detection::base_thinking_level;
use codelet_providers::local_reasoning::{LocalEngine, ThinkingLevelsDeclaration};
use codelet_providers::{OpenAIProvider, ProviderType};
use codelet_rpc_types::ThinkingLevel;
use rig::completion::Completion;
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

type TestResult = Result<(), String>;
type WireResult = Result<(), Box<dyn std::error::Error>>;

/// The thinking parameters `provider` (advertising none, low, medium and
/// high, MODEL-017) gets for the session level stored as `stored`.
fn config(provider: ProviderType, stored: ThinkingLevel) -> Result<Value, String> {
    let target = ThinkingTarget::new(
        ThinkingWire::for_dispatch(&provider, None, None),
        "provider/model",
        "model",
        advertised(),
    );
    let level = base_thinking_level(&stored)?.ok_or("no level is stored")?;
    target
        .config_for(Some(&level))?
        .ok_or_else(|| "no thinking parameter is sent".to_string())
}

#[test]
fn openai_and_copilot_levels_become_reasoning_effort() -> TestResult {
    // @step Given the providers "openai" and "github-copilot"
    for provider in [ProviderType::OpenAI, ProviderType::GitHubCopilot] {
        // @step When the Low, Medium, High and Off thinking levels are configured
        let configs = [
            config(provider.clone(), ThinkingLevel::Effort("low".to_string()))?,
            config(
                provider.clone(),
                ThinkingLevel::Effort("medium".to_string()),
            )?,
            config(provider.clone(), ThinkingLevel::Effort("high".to_string()))?,
            config(provider.clone(), ThinkingLevel::Off)?,
        ];
        // @step Then Low, Medium and High give reasoning_effort low, medium and high and Off gives reasoning_effort none
        assert_eq!(
            configs,
            [
                json!({"reasoning_effort": "low"}),
                json!({"reasoning_effort": "medium"}),
                json!({"reasoning_effort": "high"}),
                json!({"reasoning_effort": "none"}),
            ],
            "provider {provider:?}"
        );
    }
    Ok(())
}

fn chat_completion() -> Value {
    json!({
        "id": "chatcmpl-bug440",
        "object": "chat.completion",
        "created": 0,
        "model": "local-model",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "5"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    })
}

#[tokio::test]
async fn a_local_openai_compatible_server_receives_reasoning_effort_with_its_sampling_parameters(
) -> WireResult {
    // @step Given an OpenAI provider pointed at a local vLLM server whose model declares efforts "low, medium, high" (MODEL-016)
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(chat_completion()))
        .mount(&server)
        .await;
    let base_url = server.uri().replace("127.0.0.1", "localhost");
    let provider =
        OpenAIProvider::from_api_key_with_options("local", "local-model", Some(&base_url), None)?;
    let declaration: ThinkingLevelsDeclaration =
        serde_json::from_value(json!({"efforts": ["low", "medium", "high"]}))?;
    let local = ThinkingTarget::local_profile(
        ThinkingWire::for_dispatch(&ProviderType::OpenAI, None, Some("local")),
        "openai:local/local-model",
        "local-model",
        Ok(Some(declaration.resolve(
            "local",
            "local-model",
            LocalEngine::Vllm,
        )?)),
    );
    // @step When a request is sent with the High thinking level
    let level = base_thinking_level(&ThinkingLevel::Effort("high".to_string()))?
        .ok_or("no level is stored")?;
    let thinking = local
        .config_for(Some(&level))?
        .ok_or("no thinking parameter is sent")?;
    let agent = provider.create_rig_agent(uuid::Uuid::new_v4(), None, Some(thinking));
    agent
        .completion("what is 3 + 2", vec![])
        .await?
        .send()
        .await?;
    // @step Then the request body carries reasoning_effort "high" and the anti-ramble top_p, and no template toggle the model does not declare
    let requests = server
        .received_requests()
        .await
        .ok_or("no requests were recorded")?;
    let request = requests.first().ok_or("the server received no request")?;
    let body: Value = serde_json::from_slice(&request.body)?;
    assert_eq!(body["reasoning_effort"], json!("high"), "{body}");
    assert!(body.get("chat_template_kwargs").is_none(), "{body}");
    assert!(body["top_p"].is_number(), "{body}");
    Ok(())
}

/// MODEL-017: the levels the model advertises (none, low, medium, high); a
/// model that advertises none has no reasoning control.
fn advertised() -> Result<Option<codelet_providers::models::ReasoningCapability>, String> {
    let options: Vec<codelet_providers::models::ReasoningOption> = serde_json::from_value(
        serde_json::json!([{"type": "effort", "values": ["none", "low", "medium", "high"]}]),
    )
    .map_err(|error| error.to_string())?;
    codelet_providers::models::ReasoningCapability::resolve("provider/model", Some(&options))
        .map_err(|error| error.to_string())
}
