//! Feature: spec/features/codex-interruption-output.feature

#[path = "fixtures/prov160_environment.rs"]
mod environment;
#[path = "../../providers/tests/fixtures/prov160.rs"]
mod fixture;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_providers::{
    codex::{history::CodexHistory, session::CodexSession},
    session::ProviderSession,
};
use codelet_sessions::SessionManager;
use futures::StreamExt;
use rig::{
    completion::{CompletionModel, CompletionRequestBuilder},
    streaming::StreamedAssistantContent,
};
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_ordinary_interrupted_streaming_stays_silent_in_session_output(
) -> fixture::TestResult {
    // @step Given a Codex session forwarding provider notifications to session output
    let temporary = codelet_test_helpers::setup_temp_directory()?;
    let _environment = environment::TestEnvironment::install(temporary.path());
    let cache = temporary.path().join("cache");
    std::fs::create_dir_all(&cache)?;
    std::fs::write(
        cache.join("models.json"),
        r#"{"openai":{"id":"openai","name":"OpenAI","env":["OPENAI_API_KEY"],"models":{"o3":{"id":"o3","name":"o3","tool_call":true,"limit":{"context":200000,"output":100000}}}}}"#,
    )?;
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(temporary.path().to_owned())?;
    let manager = SessionManager::new();
    manager.set_default_model("openai/o3");
    let id = SessionManagerHandle::create_session(&manager, None);
    let session = manager.get_session(&id.value)?;
    let extensions = ProviderSession::for_provider("codex");
    session.prepare_provider_status(&extensions, "codex");
    let recovery = extensions
        .extension::<CodexSession>("codex")
        .ok_or("Codex extension missing")?
        .recovery
        .clone();
    let mut chunks = manager.chunks_tx().subscribe();
    let server = fixture::baseline::Server::start(vec![fixture::baseline::text()], true).await?;
    let model = fixture::model(&server.url, CodexHistory::default())?
        .with_recovery_handle(recovery.clone());
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "ordinary work").build())
        .await?;
    tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(event) = stream.next().await {
            if matches!(event?, StreamedAssistantContent::Text(_)) {
                return Ok::<_, Box<dyn std::error::Error>>(());
            }
        }
        Err("missing partial text".into())
    })
    .await??;

    // @step When an ordinary partial stream is dropped at the interruption boundary
    drop(stream);

    // @step Then session output contains no reconnection notification and recovery remains inactive
    assert_eq!(recovery.status(), None);
    assert!(!recovery.is_reconnecting());
    assert!(chunks.try_recv().is_err());
    assert_eq!(
        extensions.status("codex").ok_or("Codex status missing")?["recovery"],
        serde_json::Value::Null
    );
    Ok(())
}
