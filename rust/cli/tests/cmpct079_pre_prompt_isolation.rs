//! Feature: spec/features/pre-prompt-compaction-isolation.feature
use codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler;
use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::persistence::*;
use codelet_core::RigAgent;
use codelet_providers::{ProviderManager, ProviderType};
use rig::{client::CompletionClient, message::Message};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use wiremock::MockServer;

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

async fn exercise(cancel: bool) -> Result<(), Box<dyn std::error::Error>> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("pre-prompt", directory.path(), "openai/o3")?;
    append_message(&mut manifest, "user", "original question")?;
    append_message(&mut manifest, "assistant", "original answer")?;
    let mut manager = ProviderManager::for_testing(ProviderType::OpenAI, Some(200_000), Some(4096));
    manager.set_model_direct("openai", "o3", Some(200_000), Some(4096), None)?;
    let mut session = Session::from_provider_manager(manager);
    session.messages = vec![
        Message::user("original question"),
        Message::assistant("original answer"),
    ];
    session.turns = codelet_cli::interactive_helpers::convert_messages_to_turns(&session.messages);
    session.token_tracker.input_tokens = 195_000;
    session.token_tracker.cumulative_billed_input = 220_000;
    let messages = serde_json::to_value(&session.messages)?;
    let turns = session.turns.len();
    let tokens = serde_json::to_value(&session.token_tracker)?;
    let durable = serde_json::to_value(load_session(manifest.id)?)?;
    let started = Arc::new(AtomicBool::new(false));
    if cancel {
        let started = started.clone();
        set_compactor_sub_agent_handler(
            manifest.id,
            Some(Arc::new(move |_, _| {
                started.store(true, Ordering::Release);
                Box::pin(std::future::pending())
            })),
        );
    }
    let server = MockServer::start().await;
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("offline")
            .base_url(server.uri())
            .build()?;
    let agent = RigAgent::new(client.agent("o3").build(), 1);
    let output = Output::default();
    let busy = Arc::new(AtomicBool::new(false));
    let result = tokio::time::timeout(
        Duration::from_millis(200),
        run_agent_stream(
            agent,
            "resume work",
            &mut session,
            Arc::new(AtomicBool::new(false)),
            busy.clone(),
            Arc::new(tokio::sync::Notify::new()),
            &output,
            manifest.id,
        ),
    )
    .await;
    set_compactor_sub_agent_handler(manifest.id, None);
    if cancel {
        assert!(
            started.load(Ordering::Acquire),
            "actual pre-prompt builder must start"
        );
        assert!(result.is_err(), "cancellation drops the running builder");
    } else {
        assert!(result?.is_err(), "missing builder must fail closed");
    }
    assert_eq!(serde_json::to_value(&session.messages)?, messages);
    assert_eq!(session.turns.len(), turns);
    assert_eq!(serde_json::to_value(&session.token_tracker)?, tokens);
    assert_eq!(serde_json::to_value(load_session(manifest.id)?)?, durable);
    assert!(server
        .received_requests()
        .await
        .ok_or("requests unavailable")?
        .is_empty());
    assert!(!output
        .0
        .lock()
        .map_err(|error| error.to_string())?
        .iter()
        .any(|event| matches!(event, StreamEvent::CompactionComplete(_))));
    assert!(!busy.load(Ordering::Acquire));
    assert!(session.ensure_compaction_inference_ready().is_err());
    Ok(())
}

/// @step Given a persisted conversation above its selected compaction threshold
/// @step When pre-prompt compaction has no registered builder
/// @step Then authoritative messages turns counters and durable envelopes remain unchanged
/// @step And no provider request or completion occurs and inference stays blocked
#[tokio::test]
#[serial_test::serial]
async fn missing_pre_prompt_builder_preserves_authoritative_state(
) -> Result<(), Box<dyn std::error::Error>> {
    exercise(false).await
}

/// @step Given a persisted conversation above its selected compaction threshold
/// @step When pre-prompt compaction is cancelled while its builder is running
/// @step Then authoritative messages turns counters and durable envelopes remain unchanged
/// @step And no provider request or completion occurs and inference stays blocked
#[tokio::test]
#[serial_test::serial]
async fn cancelled_pre_prompt_builder_preserves_authoritative_state(
) -> Result<(), Box<dyn std::error::Error>> {
    exercise(true).await
}
