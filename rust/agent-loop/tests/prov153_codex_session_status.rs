//! Feature: spec/features/codex-session-metadata-status.feature

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_providers::codex::{refreshing_client::RefreshingCodexClient, CodexResponsesModel};
use codelet_rpc_types::StreamChunk;
use codelet_sessions::SessionManager;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::providers::openai;
use std::sync::Arc;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn session_presentation_retains_diagnostics_without_routine_notices(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a Codex session connected to a local response stream
    let temporary = tempfile::tempdir()?;
    std::env::set_var("OPENAI_API_KEY", "test-key");
    let cache = temporary.path().join("cache");
    std::fs::create_dir_all(&cache)?;
    std::fs::write(
        cache.join("models.json"),
        r#"{"openai":{"id":"openai","name":"OpenAI","env":["OPENAI_API_KEY"],"models":{"o3":{"id":"o3","name":"o3","tool_call":true,"limit":{"context":200000,"output":100000}}}}}"#,
    )?;
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(temporary.path().to_path_buf())?;
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("openai/o3");
    let session_id = SessionManagerHandle::create_session(manager.as_ref(), None);
    let session = manager.get_session(&session_id.value)?;
    let mut chunks = manager.chunks_tx().subscribe();
    let extensions = codelet_providers::session::ProviderSession::for_provider("codex");
    assert!(session.provider_metadata_status().is_none());
    session.prepare_provider_status(&extensions, "openai");
    assert!(session.provider_metadata_status().is_none());
    assert!(chunks.try_recv().is_err());
    session.prepare_provider_status(&extensions, "codex");
    let handle = extensions
        .extension::<codelet_providers::codex::session::CodexSession>("codex")
        .ok_or("Codex metadata hook missing")?
        .metadata
        .clone();
    let server = MockServer::start().await;
    let body = concat!(
        "data: {\"type\":\"response.metadata\",\"safety_buffering\":true}\n\n",
        "data: {\"type\":\"response.output_text.delta\",\"item_id\":\"message\",\"output_index\":0,\"content_index\":0,\"delta\":\"visible\"}\n\n",
        "data: {\"type\":\"response.metadata\",\"safety_buffering\":false}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp-test\",\"object\":\"response\",\"status\":\"completed\",\"created_at\":1,\"model\":\"serving-model\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1,\"total_tokens\":2}}}\n\n"
    );
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/event-stream")
                .insert_header("X-Request-ID", "req-session")
                .insert_header("OpenAI-Model", "serving-model")
                .insert_header("X-Codex-Turn-State", "private-route")
                .set_body_string(body),
        )
        .mount(&server)
        .await;
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("private-credential")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    let model = CodexResponsesModel::new(client, "selected-model")
        .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        })
        .with_metadata_handle(handle);
    // @step When header metadata and buffering updates arrive
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "Hello").build())
        .await?;
    while stream.next().await.transpose()?.is_some() {}
    // @step Then the session retains safe status without routine user notifications
    let status = session
        .provider_metadata_status()
        .ok_or("session status missing")?;
    assert_eq!(status["request_id"], "req-session");
    assert_eq!(status["server_model"], "serving-model");
    assert_eq!(status["buffering"], false);
    let mut notifications = Vec::new();
    while let Ok((received_id, chunk)) = chunks.try_recv() {
        assert_eq!(received_id.value, session_id.value);
        if let StreamChunk::UserNotification { message, .. } = chunk {
            assert!(!message.contains("private-"));
            notifications.push(message);
        }
    }
    assert!(notifications.is_empty());
    // @step And a non-Codex session emits no Codex notifications
    session.prepare_provider_status(&extensions, "openai");
    assert!(session.provider_metadata_status().is_none());
    assert!(chunks.try_recv().is_err());
    Ok(())
}
