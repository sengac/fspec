#![doc = "Feature: spec/features/streamed-response-attempt-rollback.feature"]

#[path = "../../providers/tests/fixtures/prov148.rs"]
mod transport;

use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::RigAgent;
use codelet_providers::codex::session::CodexSession;
use codelet_providers::{ProviderManager, ProviderType};
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::message::{AssistantContent, Message};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

struct Output;

impl StreamOutput for Output {
    fn emit(&self, _: StreamEvent) {}
}

/// @step Given a Codex response streams text before its connection is lost
/// @step And its replacement response completes successfully
/// @step When the streaming agent finishes the turn
/// @step Then only the successful attempt remains in conversation history
/// @step And the next request passes checkpoint ancestry validation
#[tokio::test]
#[serial_test::serial]
async fn a_reconnected_response_can_be_followed_by_another_turn() -> transport::TestResult {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    codelet_core::persistence::reset_stores_for_tests();
    let manifest = codelet_core::persistence::create_session_with_provider(
        "reconnect lineage",
        directory.path(),
        "codex",
    )?;
    let server = transport::Server::start(
        vec![
            transport::text(),
            transport::terminal("completed", Some(true), None),
            transport::terminal("completed", None, None),
        ],
        false,
    )
    .await?;
    let manager = ProviderManager::for_testing(ProviderType::Codex, Some(272_000), Some(4096));
    let mut session = Session::from_provider_manager(manager);
    let history = session
        .provider_session
        .extension::<CodexSession>("codex")
        .ok_or("Codex session missing")?
        .history
        .clone();
    let model = server.models()?.0.with_history(history);
    let agent = rig::agent::AgentBuilder::new(model.clone()).build();
    tokio::time::timeout(
        Duration::from_secs(10),
        run_agent_stream(
            RigAgent::with_default_depth(agent),
            "inspect",
            &mut session,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(Notify::new()),
            &Output,
            manifest.id,
        ),
    )
    .await??;
    let text: String = session
        .messages
        .iter()
        .filter_map(|message| match message {
            Message::Assistant { content, .. } => Some(content.iter()),
            _ => None,
        })
        .flatten()
        .filter_map(|content| match content {
            AssistantContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, "Hello");
    let request = CompletionRequestBuilder::new(model.clone(), "continue")
        .messages(session.messages)
        .build();
    let mut response = model.stream(request).await?;
    while let Some(event) = response.next().await {
        event?;
    }
    assert_eq!(
        server
            .requests
            .lock()
            .map_err(|_| "requests poisoned")?
            .len(),
        3
    );
    Ok(())
}
