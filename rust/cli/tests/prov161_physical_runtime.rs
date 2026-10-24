//! Feature: spec/features/provider-runtime-physical-requests.feature

#[path = "support/prov161_gated_sse.rs"]
mod gated_sse;
#[path = "support/prov161_runtime.rs"]
mod support;

use codelet_cli::interactive::StreamEvent;
use codelet_cli::interactive_helpers::prompt_provider;
use codelet_cli::session::Session;
use codelet_core::RigAgent;
use codelet_providers::ProviderType;
use rig::client::CompletionClient;
use rig::message::{AssistantContent, Message, Reasoning, ToolCall, ToolFunction};
use serde_json::{json, Value};
use support::{limits, Output, RuntimeFixture, TestResult, BUDGET, PROMPT};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[tokio::test]
#[serial_test::serial]
async fn scenario_resolved_output_budgets_reach_physical_requests() -> TestResult {
    // @step Given an OpenAI-compatible model with a saved maximum output override
    let mut fixture = RuntimeFixture::new()?;
    let server = MockServer::start().await;
    fixture.environment("OPENAI_API_KEY", Some("offline-runtime-key"));
    fixture.environment("OPENAI_BASE_URL", Some(&server.uri()));
    fixture.environment("OPENAI_MAX_OUTPUT_TOKENS", None);
    fixture.environment("OPENAI_STREAMING", None);
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id":"runtime-response", "object":"chat.completion", "created":0,
            "model":"runtime-test-model", "choices":[{"index":0,
                "message":{"role":"assistant", "content":"verified"}, "finish_reason":"stop"}],
            "usage":{"prompt_tokens":20, "completion_tokens":2, "total_tokens":22}
        })))
        .expect(1)
        .mount(&server)
        .await;
    let manager = fixture.manager(ProviderType::OpenAI, json!({"maxOutputTokens":BUDGET}))?;
    let session_id = fixture.session_id("openai")?;
    let selected = RigAgent::with_default_depth(
        manager
            .get_openai(session_id)?
            .create_rig_agent(session_id, None, None),
    )
    .with_output_limit(Some(u64::try_from(manager.max_output_tokens())?));

    // @step When the selected agent prepares a physical request
    let admitted = selected
        .selected_request_preparer(PROMPT, limits(&manager)?)
        .await?
        .prepare(&[Message::user(PROMPT)], 1)?;
    let prepared: Value = serde_json::from_slice(admitted.request_bytes())?;
    let answer = prompt_provider(&manager, PROMPT).await?;
    let requests = server
        .received_requests()
        .await
        .ok_or("missing HTTP capture")?;
    let physical: Value = serde_json::from_slice(&requests[0].body)?;

    // @step Then the serialized output limit and admission reservation equal the effective saved budget
    assert_eq!(manager.max_output_tokens(), usize::try_from(BUDGET)?);
    assert_eq!(admitted.output_reservation(), BUDGET);
    assert_eq!(prepared["max_completion_tokens"], BUDGET);
    assert_eq!(physical["max_completion_tokens"], BUDGET);
    assert!(physical.get("max_tokens").is_none());
    assert_eq!(answer, "verified");
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_streaming_preferences_control_progressive_display() -> TestResult {
    // @step Given a provider with progressive streaming disabled and a streaming-required transport
    let fixture = RuntimeFixture::new()?;
    let mut manager = fixture.manager(
        ProviderType::Codex,
        json!({"streaming":false, "maxOutputTokens":BUDGET}),
    )?;
    manager.override_model_limits(None, Some(usize::try_from(BUDGET)?));
    let server = gated_sse::GatedSse::start(support::codex_events()).await?;
    let model = support::codex_model(&server.url, limits(&manager)?).await?;
    let selected = RigAgent::with_default_depth(
        rig::agent::AgentBuilder::new(model)
            .preamble("Codex runtime instructions")
            .max_tokens(8192)
            .build(),
    )
    .with_output_limit(None);
    let session_id = fixture.session_id("codex")?;
    let mut session = Session::from_provider_manager(manager);
    let output = Output::default();

    // @step When a response containing multiple assistant text chunks is delivered
    let observe_pending = async {
        tokio::time::timeout(std::time::Duration::from_secs(5), server.deltas_sent).await??;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        {
            let events = output.0.lock().map_err(|error| error.to_string())?;
            assert!(
                !events
                    .iter()
                    .any(|event| matches!(event, StreamEvent::Text(_))),
                "assistant deltas must remain hidden before response completion"
            );
        }
        server
            .complete
            .send(())
            .map_err(|()| "SSE endpoint closed")?;
        TestResult::Ok(())
    };
    let (run, observed) = tokio::join!(
        support::run(selected, &mut session, session_id, &output),
        observe_pending,
    );
    run?;
    observed?;

    // @step Then assistant text is buffered until completion without disabling the required transport
    {
        let events = output.0.lock().map_err(|error| error.to_string())?;
        let text: Vec<&str> = events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            text,
            vec!["Hello world"],
            "disabled progressive display must not leak individual deltas"
        );
        let done = events
            .iter()
            .position(|event| matches!(event, StreamEvent::Done(_)))
            .ok_or("missing stream completion")?;
        let text_position = events
            .iter()
            .position(|event| matches!(event, StreamEvent::Text(_)))
            .ok_or("missing buffered text")?;
        assert!(text_position < done, "buffered text must flush before Done");
    }
    let body: Value = serde_json::from_slice(&server.request.await??)?;
    assert_eq!(
        body["stream"], true,
        "Codex transport must remain streaming"
    );
    assert!(serde_json::to_string(&session.messages)?.contains("Hello world"));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_codex_output_budgets_omit_unsupported_wire_parameters() -> TestResult {
    // @step Given a Codex model with an effective output budget
    let fixture = RuntimeFixture::new()?;
    let mut manager = fixture.manager(ProviderType::Codex, json!({"maxOutputTokens":BUDGET}))?;
    manager.override_model_limits(None, Some(usize::try_from(BUDGET)?));
    let server = MockServer::start().await;
    support::codex_response(&server).await;
    let model = support::codex_model(&server.uri(), limits(&manager)?).await?;
    let selected = RigAgent::with_default_depth(
        rig::agent::AgentBuilder::new(model)
            .preamble("Codex runtime instructions")
            .max_tokens(8192)
            .build(),
    )
    .with_output_limit(None);
    let session_id = fixture.session_id("codex")?;

    // @step When the selected agent prepares a physical request
    let admitted = selected
        .selected_request_preparer(PROMPT, limits(&manager)?)
        .await?
        .prepare(&[Message::user(PROMPT)], 1)?;
    let prepared: Value = serde_json::from_slice(admitted.request_bytes())?;
    let mut session = Session::from_provider_manager(manager);
    support::run(selected, &mut session, session_id, &Output::default()).await?;
    let requests = server
        .received_requests()
        .await
        .ok_or("missing Codex HTTP capture")?;
    let physical: Value = serde_json::from_slice(&requests[0].body)?;

    // @step Then admission reserves that budget without serializing a forbidden maximum output parameter
    assert_eq!(admitted.output_reservation(), BUDGET);
    assert_eq!(
        session.provider_manager().max_output_tokens(),
        usize::try_from(BUDGET)?
    );
    for body in [&prepared, &physical] {
        assert!(body.get("max_output_tokens").is_none());
        assert!(body.get("max_completion_tokens").is_none());
        assert!(body.get("max_tokens").is_none());
        assert_eq!(body["stream"], true);
    }
    Ok(())
}

async fn signed_history<M: rig::completion::CompletionModel + 'static>(
    agent: rig::agent::Agent<M>,
    manager: codelet_providers::ProviderManager,
) -> TestResult<Value> {
    let session = Session::from_provider_manager(manager);
    let history = vec![
        Message::user("Earlier question"),
        Message::Assistant {
            id: None,
            content: rig::OneOrMany::many(vec![
                AssistantContent::Reasoning(
                    Reasoning::new("signed reasoning evidence")
                        .with_signature(Some("provider-signed-evidence".into())),
                ),
                AssistantContent::text("Earlier answer"),
                AssistantContent::ToolCall(
                    ToolCall::new(
                        "signed-call".into(),
                        ToolFunction::new("inspect".into(), json!({})),
                    )
                    .with_signature(Some("gemini-thought-signature".into())),
                ),
            ])?,
        },
        Message::tool_result("signed-call", "Earlier tool result"),
        Message::user(PROMPT),
    ];
    let original = serde_json::to_value(&history)?;
    let admitted = RigAgent::with_default_depth(agent)
        .with_preserve_thinking(session.preserve_thinking_enabled)
        .selected_request_preparer(PROMPT, limits(session.provider_manager())?)
        .await?
        .prepare(&history, 1)?;
    assert_eq!(serde_json::to_value(&history)?, original);
    Ok(serde_json::from_slice(admitted.request_bytes())?)
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_signed_reasoning_defaults_survive_automatic_settings() -> TestResult {
    // @step Given Anthropic and Gemini selections without a thinking preservation override
    let fixture = RuntimeFixture::new()?;
    let server = MockServer::start().await;
    let claude = fixture.manager(ProviderType::Claude, json!({"maxOutputTokens":4096}))?;
    let anthropic: rig::providers::anthropic::Client = rig::providers::anthropic::Client::builder()
        .api_key("offline-runtime-key")
        .base_url(server.uri())
        .build()?;
    let gemini = fixture.manager(ProviderType::Gemini, json!({"maxOutputTokens":4096}))?;
    let google: rig::providers::gemini::Client = rig::providers::gemini::Client::builder()
        .api_key("offline-runtime-key")
        .base_url(server.uri())
        .build()?;

    // @step When their outgoing histories are prepared
    let anthropic_body = signed_history(
        anthropic
            .agent("runtime-test-model")
            .max_tokens(4096)
            .build(),
        claude,
    )
    .await?;
    let gemini_body = signed_history(
        google.agent("runtime-test-model").max_tokens(4096).build(),
        gemini,
    )
    .await?;

    // @step Then signed reasoning blocks remain present by default
    assert!(anthropic_body
        .to_string()
        .contains("signed reasoning evidence"));
    assert!(anthropic_body
        .to_string()
        .contains("provider-signed-evidence"));
    assert!(gemini_body
        .to_string()
        .contains("signed reasoning evidence"));
    assert!(gemini_body.to_string().contains("gemini-thought-signature"));
    assert!(server
        .received_requests()
        .await
        .ok_or("capture")?
        .is_empty());
    Ok(())
}
