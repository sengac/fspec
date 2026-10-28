//! Feature: spec/features/runtime-request-settlement.feature

use codelet_core::persistence::*;
use codelet_core::{CompactionHook, RigAgent, TokenState};
use codelet_test_helpers::setup_temp_directory;
use futures::StreamExt;
use rig::agent::AgentBuilder;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[path = "fixtures/prov148.rs"]
pub mod fixture;

#[path = "fixtures/cmpct069_anthropic.rs"]
pub mod anthropic_fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn terminal(identity: &str, end_turn: bool) -> String {
    let mut response = fixture::baseline::response();
    response["id"] = json!(identity);
    response["end_turn"] = json!(end_turn);
    response["usage"] = json!({"input_tokens":60,"output_tokens":20,"total_tokens":80,
        "input_tokens_details":{"cached_tokens":4},"output_tokens_details":{"reasoning_tokens":3}});
    fixture::event(json!({"type":"response.completed","response":response}))
}

#[tokio::test]
#[serial_test::serial]
async fn ordinary_provider_finish_hooks_settle_measured_cache_usage() -> TestResult {
    // @step Given a scripted Anthropic stream with measured fresh input cache and output usage
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let session = create_session("ordinary provider", directory.path())?;
    let (server, model) = anthropic_fixture::model().await?;
    let agent = RigAgent::new(AgentBuilder::new(model).max_tokens(200).build(), 3)
        .with_settlement_session(session.id);
    let hook = CompactionHook::new(Arc::new(Mutex::new(TokenState::default())), u64::MAX);
    // @step When its real adapter completes through the runtime finish hook
    let mut history = vec![];
    let stream = agent
        .prompt_streaming_with_history_and_hook("hello", &mut history, hook)
        .await;
    futures::pin_mut!(stream);
    while let Some(event) = stream.next().await {
        event?;
    }
    server.verify().await;
    // @step Then a cold manifest reload retains exactly one settlement with complete cache evidence
    reset_stores_for_tests();
    let restored = load_session(session.id)?;
    assert_eq!(restored.request_settlements.len(), 1);
    let usage = restored
        .request_settlements
        .values()
        .next()
        .ok_or("settlement missing")?;
    assert_eq!(usage.input_tokens, 600);
    assert_eq!(usage.output_tokens, 100);
    assert_eq!(usage.cache_read_input_tokens, Some(400));
    assert_eq!(usage.cache_creation_input_tokens, Some(50));
    assert_eq!(restored.token_usage.cumulative_billed_input, 1050);
    assert_eq!(restored.token_usage.cumulative_billed_output, 100);
    // @step And current occupancy agrees with the measured physical request
    assert_eq!(restored.token_usage.current_context_tokens, 1150);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn runtime_finish_hooks_settle_every_physical_request() -> TestResult {
    // @step Given a scripted provider stream with two measured physical requests
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let session = create_session("physical continuations", directory.path())?;
    let server = fixture::Server::start(
        vec![terminal("first", false), terminal("second", true)],
        false,
    )
    .await?;
    let model = server.models()?.0.with_request_admission(
        codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(10000),
            output_reservation: Some(100),
        },
    );
    let agent =
        RigAgent::new(AgentBuilder::new(model).build(), 3).with_settlement_session(session.id);
    let hook = CompactionHook::new(Arc::new(Mutex::new(TokenState::default())), u64::MAX);
    // @step When real request finish hooks complete through the selected adapter
    let mut history = vec![];
    let stream = agent
        .prompt_streaming_with_history_and_hook("hello", &mut history, hook.clone())
        .await;
    futures::pin_mut!(stream);
    while let Some(event) = stream.next().await {
        event?;
    }
    // @step Then both requests are durably settled once without reading display counters
    reset_stores_for_tests();
    let restored = load_session(session.id)?;
    assert_eq!(restored.token_usage.cumulative_billed_input, 120);
    assert_eq!(restored.token_usage.cumulative_billed_output, 40);
    assert_eq!(restored.request_settlements.len(), 2);
    assert_eq!(restored.token_usage.current_context_tokens, 80);
    // @step And cache and reasoning evidence survives a cold store reload
    assert!(
        restored
            .request_settlements
            .values()
            .all(|usage| usage.reasoning_tokens == Some(3)
                && usage.cache_read_input_tokens == Some(4))
    );
    Ok(())
}
