#![doc = "Feature: spec/features/codex-napi-session-recovery.feature"]

#[path = "../../providers/tests/fixtures/attempt_events.rs"]
mod attempt_events;
#[path = "fixtures/prov160_environment.rs"]
mod environment;
#[path = "../../napi/src/provider_session.rs"]
mod provider_session;

use codelet_cli::interactive::provider_recovery::wait_for_stall_deadline;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_providers::codex::{reconnect::RecoveryStatus, session::CodexSession};
use codelet_providers::request_admission::AdmissionLimits;
use codelet_providers::session::ProviderSession;
use codelet_providers::CodexProvider;
use codelet_rpc_types::{SessionStatus, StreamChunk};
use codelet_sessions::SessionManager;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use std::{ffi::OsString, sync::Arc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct OfflineProxy(Vec<(&'static str, Option<OsString>)>);

impl OfflineProxy {
    async fn install() -> Result<Self, std::io::Error> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let proxy = format!("http://{}", listener.local_addr()?);
        let mut previous = Vec::new();
        for (name, value) in [
            ("HTTPS_PROXY", proxy.as_str()),
            ("https_proxy", proxy.as_str()),
            ("NO_PROXY", ""),
            ("no_proxy", ""),
        ] {
            previous.push((name, std::env::var_os(name)));
            std::env::set_var(name, value);
        }
        Ok(Self(previous))
    }
}

impl Drop for OfflineProxy {
    fn drop(&mut self) {
        for (name, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_napi_dispatch_shares_recovery_state_with_its_owning_session() -> TestResult {
    // @step Given two NAPI sessions dispatching Codex after starting with another provider
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
    active.set_status(SessionStatus::Running);
    let mut extensions = ProviderSession::for_provider("openai");
    let mut other_extensions = ProviderSession::for_provider("openai");
    let _proxy = OfflineProxy::install().await?;
    let provider = CodexProvider::from_api_key("test-key", "shared-model")?.with_request_admission(
        AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        },
    );
    let provider = provider_session::prepare_provider(provider, &active, &mut extensions);
    let _other_provider = provider_session::prepare_provider(
        CodexProvider::from_api_key("test-key", "shared-model")?,
        &other,
        &mut other_extensions,
    );
    let model = provider.completion_model();
    let mut chunks = manager.chunks_tx().subscribe();

    // @step When one session waits for an unavailable network beyond its generation stall deadline
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "original turn").build())
        .await?
        .filter(attempt_events::is_content);
    {
        let pending = stream.next();
        tokio::pin!(pending);
        let watchdog = wait_for_stall_deadline(&extensions, "codex", Duration::from_millis(30));
        tokio::pin!(watchdog);
        tokio::select! {
            event = &mut pending => return Err(format!("offline request unexpectedly ended: {event:?}").into()),
            _ = &mut watchdog => return Err("NAPI recovery fired the generic generation stall watchdog".into()),
            _ = tokio::time::sleep(Duration::from_millis(150)) => {},
        }
    }

    // @step Then its watchdog remains pending and only its session emits reconnect status
    let recovery = &extensions
        .extension::<CodexSession>("codex")
        .ok_or("NAPI dispatch did not initialize the Codex extension")?
        .recovery;
    assert!(matches!(
        recovery.status(),
        Some(RecoveryStatus::WaitingForNetwork { .. })
    ));
    assert!(!active
        .provider_metadata_status()
        .ok_or("active provider status missing")?["recovery"]
        .is_null());
    let (received_id, chunk) = chunks.try_recv()?;
    assert_eq!(received_id.value, active_id.value);
    assert!(
        matches!(chunk, StreamChunk::UserNotification { message, .. } if message.contains("Reconnecting"))
    );
    assert_eq!(
        other_extensions
            .extension::<CodexSession>("codex")
            .ok_or("other Codex extension missing")?
            .recovery
            .status(),
        None
    );

    // @step And cancelling the provider stream clears its recovery wait without changing the other session
    drop(stream);
    assert_eq!(recovery.status(), Some(RecoveryStatus::Cancelled));
    let (received_id, chunk) = chunks.try_recv()?;
    assert_eq!(received_id.value, active_id.value);
    assert!(
        matches!(chunk, StreamChunk::UserNotification { message, .. } if message.contains("cancelled"))
    );
    assert!(chunks.try_recv().is_err());
    tokio::time::timeout(
        Duration::from_millis(200),
        wait_for_stall_deadline(&extensions, "codex", Duration::from_millis(30)),
    )
    .await?;
    assert_eq!(active.get_status(), SessionStatus::Running);
    assert_eq!(other.get_status(), SessionStatus::Idle);
    assert_eq!(
        other_extensions
            .extension::<CodexSession>("codex")
            .ok_or("other Codex extension missing")?
            .recovery
            .status(),
        None
    );
    Ok(())
}
