#![doc = "Feature: spec/features/codex-reconnect-status.feature"]

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_providers::codex::{
    reconnect::{CodexRecoveryHandle, RecoveryStatus},
    refreshing_client::RefreshingCodexClient,
    session::CodexSession,
    CodexResponsesModel,
};
use codelet_providers::session::ProviderSession;
use codelet_rpc_types::{SessionStatus, StreamChunk};
use codelet_sessions::SessionManager;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::providers::openai;
use std::sync::Arc;
use std::time::Duration;

#[path = "../../providers/tests/fixtures/attempt_events.rs"]
mod attempt_events;
#[path = "fixtures/prov160_environment.rs"]
mod environment;
#[path = "../../providers/tests/fixtures/prov148.rs"]
mod transport;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn model(
    url: &str,
    recovery: CodexRecoveryHandle,
) -> Result<CodexResponsesModel, Box<dyn std::error::Error>> {
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(url)
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    Ok(CodexResponsesModel::new(client, "shared-model")
        .with_recovery_handle(recovery)
        .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        }))
}

async fn consume(model: CodexResponsesModel) -> TestResult {
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "original turn").build())
        .await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn reconnect_lifecycle_is_session_scoped_and_reaches_existing_status_output() -> TestResult {
    // @step Given two Codex sessions using the existing provider status observer
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
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("openai/o3");
    let active_id = SessionManagerHandle::create_session(manager.as_ref(), None);
    let other_id = SessionManagerHandle::create_session(manager.as_ref(), None);
    let active = manager.get_session(&active_id.value)?;
    let other = manager.get_session(&other_id.value)?;
    let active_extensions = ProviderSession::for_provider("codex");
    let other_extensions = ProviderSession::for_provider("codex");
    active.prepare_provider_status(&active_extensions, "codex");
    other.prepare_provider_status(&other_extensions, "codex");
    active.set_status(SessionStatus::Running);
    let recovery = active_extensions
        .extension::<CodexSession>("codex")
        .ok_or("active Codex extension missing")?
        .recovery
        .clone();
    let mut chunks = manager.chunks_tx().subscribe();
    assert_eq!(
        other_extensions
            .status("codex")
            .ok_or("other status missing")?["recovery"],
        serde_json::Value::Null
    );

    // @step When one session reconnects recovers exhausts or is cancelled
    let server = transport::Server::start(
        vec![
            transport::text(),
            transport::terminal("completed", Some(true), None),
        ],
        false,
    )
    .await?;
    tokio::time::timeout(
        Duration::from_secs(30),
        consume(model(&server.url, recovery.clone())?),
    )
    .await??;
    assert_eq!(recovery.status(), Some(RecoveryStatus::Recovered));
    assert!(!recovery.is_reconnecting());

    let server = transport::Server::start(vec![transport::text(); 6], false).await?;
    let failed = tokio::time::timeout(
        Duration::from_secs(60),
        consume(model(&server.url, recovery.clone())?),
    )
    .await?;
    assert!(failed.is_err());
    assert_eq!(recovery.status(), Some(RecoveryStatus::Failed));
    assert!(!recovery.is_reconnecting());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let offline_url = format!("http://{}", listener.local_addr()?);
    drop(listener);
    let offline = model(&offline_url, recovery.clone())?;
    let mut stream = offline
        .stream(CompletionRequestBuilder::new(offline.clone(), "original turn").build())
        .await?
        .filter(attempt_events::is_content);
    assert!(
        tokio::time::timeout(Duration::from_millis(200), stream.next())
            .await
            .is_err()
    );
    assert!(matches!(
        recovery.status(),
        Some(RecoveryStatus::WaitingForNetwork { .. })
    ));
    assert!(recovery.is_reconnecting());
    let status = active
        .provider_metadata_status()
        .ok_or("active status missing")?;
    assert!(!status["recovery"].is_null());
    drop(stream);
    assert_eq!(recovery.status(), Some(RecoveryStatus::Cancelled));
    assert!(!recovery.is_reconnecting());

    // @step Then only that session emits reconnect lifecycle status without daemon disconnect events or generic Continue recovery
    let mut notifications = Vec::new();
    while let Ok((received_id, chunk)) = chunks.try_recv() {
        assert_eq!(received_id.value, active_id.value);
        match chunk {
            StreamChunk::UserNotification { message, .. } => notifications.push(message),
            unexpected => return Err(format!("unexpected recovery output: {unexpected:?}").into()),
        }
    }
    for expected in ["Reconnecting", "Reconnected", "failed", "cancelled"] {
        assert!(
            notifications
                .iter()
                .any(|message| message.contains(expected)),
            "missing {expected}: {notifications:?}"
        );
    }
    assert!(!notifications
        .iter()
        .any(|message| message.contains("Continue") || message.contains("daemon")));
    assert_eq!(active.get_status(), SessionStatus::Running);
    assert_eq!(other.get_status(), SessionStatus::Idle);
    assert_eq!(
        other_extensions
            .status("codex")
            .ok_or("other status missing")?["recovery"],
        serde_json::Value::Null
    );
    Ok(())
}
