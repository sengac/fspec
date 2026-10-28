//! Feature: spec/features/codex-latest-context-occupancy.feature

use codelet_core::{CompactionHook, TokenState};
use futures::StreamExt;
use rig::agent::{CancelSignal, StreamingPromptHook};
use rig::completion::{CompletionModel, CompletionRequestBuilder, GetTokenUsage};
use rig::message::Message;
use rig::streaming::StreamedAssistantContent;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[path = "fixtures/prov148.rs"]
pub mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Final = rig::providers::openai::responses_api::streaming::StreamingCompletionResponse;

fn terminal(input: u64, output: u64, end_turn: bool) -> String {
    let mut response = fixture::baseline::response();
    response["end_turn"] = json!(end_turn);
    response["usage"] = json!({"input_tokens":input,"output_tokens":output,
        "total_tokens":input+output,"input_tokens_details":{"cached_tokens":4},
        "output_tokens_details":{"reasoning_tokens":3}});
    fixture::event(json!({"type":"response.completed","response":response}))
}

async fn streamed(bodies: Vec<String>) -> Result<(Final, Vec<u64>), Box<dyn std::error::Error>> {
    let server = fixture::Server::start(bodies, false).await?;
    let model = server.models()?.0.with_request_admission(
        codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(10000),
            output_reservation: Some(100),
        },
    );
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "hello").build())
        .await?;
    let mut measured = Vec::new();
    let mut final_response = None;
    while let Some(event) = stream.next().await {
        match event? {
            StreamedAssistantContent::Usage(usage) => measured.push(usage.total_tokens),
            StreamedAssistantContent::Final(response) => final_response = Some(response),
            _ => {}
        }
    }
    Ok((final_response.ok_or("missing final response")?, measured))
}

async fn preflight(response: &Final, threshold: u64) -> TestResult {
    let state = Arc::new(Mutex::new(TokenState::default()));
    let hook = CompactionHook::new(state.clone(), threshold);
    let prompt = Message::user("next");
    <CompactionHook as StreamingPromptHook<codelet_providers::codex::CodexResponsesModel>>::on_stream_completion_response_finish(
        &hook, &prompt, response, CancelSignal::new()).await;
    let signal = CancelSignal::new();
    <CompactionHook as StreamingPromptHook<codelet_providers::codex::CodexResponsesModel>>::on_completion_call(
        &hook, &prompt, &[], signal.clone()).await;
    let actual = state
        .lock()
        .map_err(|_| "hook lock poisoned")?
        .compaction_needed;
    assert_eq!(
        actual,
        threshold < 80,
        "aggregate spend leaked into hook occupancy"
    );
    assert_eq!(
        signal.is_cancelled(),
        actual,
        "admission did not enforce measured occupancy"
    );
    Ok(())
}

#[tokio::test]
async fn missing_latest_usage_does_not_reuse_earlier_measured_context() -> TestResult {
    // @step Given a measured Codex response followed by a terminal without usage
    let mut unknown = fixture::baseline::response();
    unknown["end_turn"] = json!(true);
    unknown
        .as_object_mut()
        .ok_or("response object")?
        .remove("usage");
    let server = fixture::Server::start(
        vec![
            terminal(60, 20, false),
            fixture::event(json!({"type":"response.completed","response":unknown})),
        ],
        false,
    )
    .await?;
    let history = codelet_providers::codex::history::CodexHistory::default();
    let before = history.snapshot()?;
    let model = server
        .models()?
        .0
        .with_history(history.clone())
        .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(10000),
            output_reservation: Some(100),
        });
    // @step When the Responses stream is consumed
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "hello").build())
        .await?;
    let mut errors = Vec::new();
    let mut measured = Vec::new();
    let mut finals = 0;
    while let Some(event) = stream.next().await {
        match event {
            Err(error) => errors.push(error.to_string()),
            Ok(StreamedAssistantContent::Usage(usage)) => measured.push(usage.total_tokens),
            Ok(StreamedAssistantContent::Final(_)) => finals += 1,
            _ => {}
        }
    }
    // @step Then missing latest usage fails without publishing completion while earlier measured spend is retained
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("missing physical usage"), "{errors:?}");
    assert_eq!(finals, 0);
    assert_eq!(measured, vec![80]);
    // @step And the committed checkpoint remains unchanged
    assert_eq!(history.snapshot()?, before);
    Ok(())
}

#[tokio::test]
async fn aggregate_continuation_spend_cannot_veto_a_fitting_next_request() -> TestResult {
    // @step Given two physical Codex responses individually below the threshold
    let bodies = vec![terminal(60, 20, false), terminal(60, 20, true)];
    // @step And their combined measured usage exceeds that threshold
    let (response, measured) = streamed(bodies).await?;
    assert_eq!(
        response.token_usage().ok_or("missing spend")?.total_tokens,
        160
    );
    // @step When the Responses stream finishes and the compaction hook checks a small next request
    preflight(&response, 100).await?;
    // @step Then admission remains open and all measured spend is retained
    assert_eq!(measured, vec![80, 80]);
    Ok(())
}

#[tokio::test]
async fn actual_latest_occupancy_blocks_the_next_request() -> TestResult {
    // @step Given a Codex continuation whose latest measured context exceeds the threshold
    let (response, measured) =
        streamed(vec![terminal(10, 10, false), terminal(60, 20, true)]).await?;
    // @step When the Responses stream finishes and the compaction hook checks a small next request
    preflight(&response, 70).await?;
    // @step Then admission is cancelled and all measured spend is retained
    assert_eq!(measured, vec![20, 80]);
    assert_eq!(
        response.token_usage().ok_or("missing spend")?.total_tokens,
        100
    );
    Ok(())
}
