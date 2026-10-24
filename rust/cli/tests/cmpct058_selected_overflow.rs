// Feature: spec/features/one-selected-overflow-dag-protocol.feature
use codelet_cli::compaction_operation::selected_overflow_protocol;
use codelet_cli::interactive::is_context_overflow_error;
use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::persistence::*;
use codelet_core::RigAgent;
use codelet_providers::{ProviderManager, ProviderType};
use rig::client::CompletionClient;
use rig::completion::{CompletionModel, GetTokenUsage};
use rig::message::Message;
use rig::wasm_compat::WasmCompatSend;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[derive(Default)]
struct Output(Mutex<Vec<StreamEvent>>);

impl StreamOutput for Output {
    fn emit(&self, event: StreamEvent) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event);
    }
}

async fn recovery<M: CompletionModel + 'static>(
    agent: RigAgent<M>,
    provider: ProviderType,
    server: &MockServer,
) -> Result<serde_json::Value, Box<dyn std::error::Error>>
where
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
{
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error":{"code":"context_length_exceeded","message":"declared rejection"}
        })))
        .mount(server)
        .await;
    let directory = codelet_test_helpers::setup_temp_directory()?;
    reset_stores_for_tests();
    let mut manifest =
        create_session_with_provider("overflow route", directory.path(), provider.as_str())?;
    append_message(&mut manifest, "user", "original question")?;
    append_message(&mut manifest, "assistant", "original answer")?;
    let originals = get_session_message_envelopes(manifest.id)?;
    let durable = serde_json::to_value(load_session(manifest.id)?)?;
    let mut session = Session::from_provider_manager(ProviderManager::for_testing(
        provider,
        Some(200_000),
        Some(4096),
    ));
    session.messages = vec![
        Message::user("original question"),
        Message::assistant("original answer"),
    ];
    let messages = serde_json::to_value(&session.messages)?;
    let output = Output::default();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        run_agent_stream(
            agent,
            "resume original task",
            &mut session,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(tokio::sync::Notify::new()),
            &output,
            manifest.id,
        ),
    )
    .await?;
    assert!(
        result.is_err(),
        "repeated physical overflow must terminate without success"
    );
    assert!(session.ensure_compaction_inference_ready().is_err());
    let requests = server
        .received_requests()
        .await
        .ok_or("physical requests")?;
    assert_eq!(
        requests.len(),
        usize::try_from(codelet_cli::interactive::MAX_COMPACTION_RETRIES)? + 1
    );
    assert!(!String::from_utf8_lossy(&requests[0].body).contains("Build a"));
    for request in &requests[1..] {
        let body: serde_json::Value = serde_json::from_slice(&request.body)?;
        assert!(body.to_string().contains("hierarchical summary DAG"));
        assert!(body.to_string().contains("inject_summary"));
        assert!(body.to_string().contains("SessionSearch(originals"));
    }
    assert!(!output
        .0
        .lock()
        .map_err(|error| error.to_string())?
        .iter()
        .any(|event| matches!(event, StreamEvent::CompactionComplete(_))));
    assert_eq!(serde_json::to_value(&session.messages)?, messages);
    assert_eq!(serde_json::to_value(load_session(manifest.id)?)?, durable);
    reset_stores_for_tests();
    assert_eq!(get_session_message_envelopes(manifest.id)?, originals);
    Ok(serde_json::to_value(session.inference_messages())?)
}

#[tokio::test]
#[serial_test::serial]
async fn equivalent_typed_overflow_routes_select_the_same_protocol(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given selected OpenAI and Anthropic adapters receiving structured overflow rejections
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    let openai = MockServer::start().await;
    let anthropic = MockServer::start().await;
    let openai_client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("offline")
            .base_url(openai.uri())
            .build()?;
    let anthropic_client: rig::providers::anthropic::Client =
        rig::providers::anthropic::Client::builder()
            .api_key("offline")
            .base_url(anthropic.uri())
            .build()?;
    // @step When each adapter executes standalone streaming overflow recovery
    let openai_view = recovery(
        RigAgent::new(openai_client.agent("o3").build(), 1),
        ProviderType::OpenAI,
        &openai,
    )
    .await?;
    let anthropic_view = recovery(
        RigAgent::new(
            anthropic_client
                .agent("claude-test")
                .max_tokens(4096)
                .build(),
            1,
        ),
        ProviderType::Claude,
        &anthropic,
    )
    .await?;
    // @step Then every retry sends the same in-view DAG construction instruction
    assert_eq!(openai_view, anthropic_view);
    // @step And no substitute completion occurs and ordinary inference stays blocked
    assert!(openai_view.to_string().contains("resume original task"));
    // @step And committed conversation and durable original envelopes remain unchanged
    Ok(())
}

#[test]
fn unknown_token_like_errors_do_not_select_compaction() {
    // @step Given an unrecognized error containing token-like wording
    let error = anyhow::anyhow!("invalid_request_error token maximum configuration")
        .context("input is too long for log buffer");
    // @step When overflow recovery routes the error
    let protocol = selected_overflow_protocol(is_context_overflow_error(&error), 0);
    // @step Then no DAG construction protocol is selected
    assert_eq!(protocol, None);
}
