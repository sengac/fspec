#![doc = "Feature: spec/features/codex-napi-session-recovery.feature"]

#[path = "../../napi/src/provider_session.rs"]
mod provider_session;
#[path = "../../providers/tests/fixtures/prov148.rs"]
pub mod transport;

use codelet_cli::session::Session;
use codelet_providers::codex::{
    history::CodexHistory, refreshing_client::RefreshingCodexClient, session::CodexSession,
    CodexResponsesModel,
};
use codelet_providers::request_admission::AdmissionLimits;
use codelet_providers::session::ProviderSession;
use codelet_providers::{CodexProvider, ProviderManager, ProviderType};
use codelet_sessions::background_session::BackgroundSession;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::providers::openai;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use transport::TestResult;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

async fn request(model: &CodexResponsesModel) -> TestResult {
    let model = model.clone().with_history(CodexHistory::default());
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "continue").build())
        .await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    Ok(())
}

/// @step Given a NAPI Codex session has routing state from a completed turn
/// @step When the next user turn prepares its provider
/// @step Then the new request retains session affinity without the previous turn state
/// @step And a late response from the previous turn cannot restore that turn state
#[tokio::test]
#[serial_test::serial]
async fn scenario_napi_dispatch_clears_routing_state_from_the_previous_turn() -> TestResult {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    let background = Arc::new(BackgroundSession::new(
        uuid::Uuid::new_v4(),
        "routing".into(),
        directory.path().display().to_string(),
        Some("codex".into()),
        Some("shared-model".into()),
        Session::from_provider_manager(ProviderManager::for_testing(
            ProviderType::Codex,
            Some(272_000),
            Some(4096),
        )),
        mpsc::channel(1).0,
        None,
        None,
        None,
        broadcast::channel(32).0,
        broadcast::channel(32).0,
        broadcast::channel(32).0,
    ));
    let mut extensions = ProviderSession::for_provider("codex");
    let routing = extensions
        .extension::<CodexSession>("codex")
        .ok_or("Codex extension missing")?
        .routing
        .clone();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .insert_header("x-codex-turn-state", "previous-turn")
                .set_body_string(transport::terminal("completed", None, None)),
        )
        .mount(&server)
        .await;
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("routing-key")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    let model = CodexResponsesModel::new(client, "shared-model")
        .with_request_admission(AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        })
        .with_routing_handle(routing.clone());
    request(&model).await?;
    request(&model).await?;
    let late_response = routing.bind_account("Bearer routing-key");
    let _provider = provider_session::prepare_provider(
        CodexProvider::from_api_key("routing-key", "shared-model")?,
        &background,
        &mut extensions,
    );
    late_response.observe_turn_state("late-previous-turn");
    request(&model).await?;
    let requests = server.received_requests().await.ok_or("requests missing")?;
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[1].headers["x-codex-turn-state"], "previous-turn");
    assert!(
        !requests[2].headers.contains_key("x-codex-turn-state"),
        "a new NAPI user turn must not reuse the previous turn state"
    );
    let first: Value = requests[0].body_json()?;
    let second: Value = requests[2].body_json()?;
    assert_eq!(first["prompt_cache_key"], second["prompt_cache_key"]);
    Ok(())
}
