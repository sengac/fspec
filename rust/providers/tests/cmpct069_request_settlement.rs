//! Feature: spec/features/physical-request-settlement.feature

use codelet_core::compaction_hook::{CompactionHook, TokenState};
use codelet_core::persistence::*;
use codelet_test_helpers::setup_temp_directory;
use rig::agent::{CancelSignal, StreamingPromptHook};
use rig::message::Message;
use rig::providers::anthropic::{self, completion::CompletionModel};
use serde_json::json;
use std::sync::{Arc, Mutex};

#[path = "fixtures/cmpct069_anthropic.rs"]
mod anthropic_fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn hook(
    session: uuid::Uuid,
) -> codelet_core::request_settlement_hook::RequestSettlementHook<CompactionHook> {
    codelet_core::request_settlement_hook::RequestSettlementHook::new(
        CompactionHook::new(Arc::new(Mutex::new(TokenState::default())), u64::MAX),
        session,
    )
}

fn response(
    input: u64,
) -> Result<anthropic::streaming::StreamingCompletionResponse, serde_json::Error> {
    serde_json::from_value(json!({"usage": {
        "input_tokens": input, "output_tokens": 70,
        "cache_read_input_tokens": 400, "cache_creation_input_tokens": 50
    }}))
}

async fn begin(hook: &impl StreamingPromptHook<CompletionModel>) {
    hook.on_completion_call(&Message::user("hello"), &[], CancelSignal::new())
        .await;
}

async fn finish(
    hook: &impl StreamingPromptHook<CompletionModel>,
    input: u64,
) -> Result<CancelSignal, serde_json::Error> {
    let signal = CancelSignal::new();
    hook.on_stream_completion_response_finish(
        &Message::user("hello"),
        &response(input)?,
        signal.clone(),
    )
    .await;
    Ok(signal)
}

fn configure(path: &std::path::Path) -> Result<(), String> {
    codelet_common::set_data_directory(path.to_path_buf())?;
    reset_stores_for_tests();
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn physical_requests_settle_once_independently_of_context_replacement() -> TestResult {
    // @step Given measured intermediate and retry requests with stable physical identities
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let session = create_session("settlement", directory.path())?;
    let hook = hook(session.id);
    begin(&hook).await;
    finish(&hook, 600).await?;
    begin(&hook).await;
    finish(&hook, 200).await?;
    // @step When duplicate terminal events and a context replacement are persisted
    finish(&hook, 200).await?;
    update_session_manifest(session.id, |current| {
        current.token_usage.current_context_tokens = 123;
        Ok(())
    })?;
    reset_stores_for_tests();
    let restored = load_session(session.id)?;
    // @step Then each physical request is billed once and the replaced context stays separate
    assert_eq!(restored.token_usage.cumulative_billed_input, 1700);
    assert_eq!(restored.token_usage.cumulative_billed_output, 140);
    assert_eq!(restored.token_usage.current_context_tokens, 123);
    // @step And cache evidence survives a cold store reload
    let encoded = serde_json::to_value(restored)?;
    let ledger = encoded["request_settlements"]
        .as_object()
        .ok_or("request ledger missing")?;
    assert_eq!(ledger.len(), 2);
    assert!(ledger
        .values()
        .all(|usage| usage["cache_read_input_tokens"] == 400));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn failed_or_cancelled_requests_never_invent_usage() -> TestResult {
    use futures::StreamExt;
    use rig::{agent::AgentBuilder, client::CompletionClient};
    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

    // @step Given a physical request without measured final usage
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let session = create_session("cancelled", directory.path())?;
    let measured = hook(session.id);
    begin(&measured).await;
    finish(&measured, 600).await?;
    let previous = load_session(session.id)?;
    // @step When the request fails or is cancelled before final measurement
    for interruption in ["failure", "cancellation"] {
        let server = MockServer::start().await;
        let template = if interruption == "failure" {
            ResponseTemplate::new(500).set_body_string("scripted provider failure")
        } else {
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(concat!(
                    "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"pending\",\"role\":\"assistant\",\"model\":\"test-model\",\"content\":[],\"usage\":{\"input_tokens\":600,\"output_tokens\":0}}}\n\n",
                    "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
                    "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"partial\"}}\n\n"
                ))
        };
        Mock::given(method("POST"))
            .respond_with(template)
            .expect(1)
            .mount(&server)
            .await;
        let model: CompletionModel = anthropic::Client::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .build()?
            .completion_model("test-model");
        let agent =
            codelet_core::RigAgent::new(AgentBuilder::new(model).max_tokens(200).build(), 3)
                .with_settlement_session(session.id);
        let mut history = vec![];
        {
            let stream = agent
                .prompt_streaming_with_history_and_hook(
                    "interrupt",
                    &mut history,
                    CompactionHook::new(Arc::new(Mutex::new(TokenState::default())), u64::MAX),
                )
                .await;
            futures::pin_mut!(stream);
            let mut observed = false;
            while let Some(event) = stream.next().await {
                if interruption == "failure" {
                    if event.is_err() {
                        observed = true;
                        break;
                    }
                } else if matches!(
                    event?,
                    rig::agent::MultiTurnStreamItem::StreamAssistantItem(
                        rig::streaming::StreamedAssistantContent::Text(_)
                    )
                ) {
                    observed = true;
                    break;
                }
            }
            assert!(observed, "{interruption} stimulus did not occur");
        }
        server.verify().await;
        // @step Then no spend is fabricated and previous measured requests remain settled
        reset_stores_for_tests();
        let restored = load_session(session.id)?;
        assert_eq!(restored.request_settlements, previous.request_settlements);
        assert_eq!(restored.token_usage, previous.token_usage);
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn conflicting_replay_is_rejected_without_changing_settlement() -> TestResult {
    // @step Given a physical request was settled with measured usage
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let session = create_session("conflict", directory.path())?;
    let hook = hook(session.id);
    begin(&hook).await;
    finish(&hook, 600).await?;
    let before = load_session(session.id)?;
    // @step When a replay reuses its identity with different measured counts
    let signal = finish(&hook, 601).await?;
    // @step Then settlement fails explicitly and the committed ledger remains unchanged
    assert!(signal.is_cancelled());
    let after = load_session(session.id)?;
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.token_usage, before.token_usage);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn persistence_failure_blocks_successful_settlement() -> TestResult {
    use futures::StreamExt;
    use rig::agent::{AgentBuilder, MultiTurnStreamItem};

    // @step Given a measured final response and an unavailable session manifest
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let session = create_session("unavailable", directory.path())?;
    let (server, model) = anthropic_fixture::model().await?;
    let agent = codelet_core::RigAgent::new(AgentBuilder::new(model).max_tokens(200).build(), 3)
        .with_settlement_session(session.id);
    std::fs::remove_file(
        directory
            .path()
            .join("sessions")
            .join(format!("{}.json", session.id)),
    )?;
    // @step When the real finish hook attempts request settlement
    let mut history = vec![];
    let stream = agent
        .prompt_streaming_with_history_and_hook(
            "hello",
            &mut history,
            CompactionHook::new(Arc::new(Mutex::new(TokenState::default())), u64::MAX),
        )
        .await;
    futures::pin_mut!(stream);
    let mut failures = Vec::new();
    let mut completions = 0;
    while let Some(event) = stream.next().await {
        match event {
            Err(error) => failures.push(error.to_string()),
            Ok(MultiTurnStreamItem::FinalResponse(_)) => completions += 1,
            _ => {}
        }
    }
    // @step Then inference is cancelled and no successful settlement is claimed
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(failures[0].contains("settlement"), "{failures:?}");
    assert_eq!(completions, 0);
    assert!(load_session(session.id).is_err());
    server.verify().await;
    Ok(())
}
