#![doc = "Feature: spec/features/model-switch-durable-identity.feature"]

use codelet_core::persistence::{
    append_message, get_session_message_envelopes, load_session, reset_stores_for_tests,
    SessionManifest, SessionStore,
};
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_providers::request_admission::AdmissionLimits;
use codelet_providers::selected_request::SelectedRequestPreparer;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use rig::client::CompletionClient;
use rig::completion::CompletionRequestBuilder;
use serial_test::serial;
use std::ffi::OsString;
use std::sync::Arc;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const PROFILE_MODEL: &str = "org/model/version";

#[path = "fixtures/model_switch_failure_paths.rs"]
mod failure_paths;

struct Fixture {
    directory: tempfile::TempDir,
    environment: Vec<(&'static str, Option<OsString>)>,
    manager: Arc<SessionManager>,
    session_id: SessionId,
    original: SessionManifest,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        let environment = [
            "FSPEC_USER_DIR",
            "OPENAI_API_KEY",
            "OPENAI_BASE_URL",
            "OPENAI_MODEL",
            "OPENAI_CONTEXT_WINDOW",
            "OPENAI_STREAMING",
            "ANTHROPIC_API_KEY",
        ]
        .into_iter()
        .map(|key| (key, std::env::var_os(key)))
        .collect();
        std::env::set_var("FSPEC_USER_DIR", directory.path());
        std::env::set_var("OPENAI_API_KEY", "offline");
        std::env::set_var("ANTHROPIC_API_KEY", "offline");
        codelet_common::set_data_directory(directory.path().to_owned())?;
        reset_stores_for_tests();
        let cache = directory.path().join("cache");
        std::fs::create_dir_all(&cache)?;
        std::fs::write(
            cache.join("models.json"),
            include_str!("fixtures/prov129_codex_models.json"),
        )?;
        std::fs::write(
            directory.path().join("fspec-config.json"),
            serde_json::to_vec(&serde_json::json!({
                "providers": {"openai": {"profiles": {"Strada": {
                    "baseUrl": "http://127.0.0.1:1/v1",
                    "apiKey": "offline",
                    "contextWindow": 200000,
                    "customModels": [{"id": PROFILE_MODEL, "maxOutputTokens": 4096}]
                }}}}
            }))?,
        )?;
        let manager = Arc::new(SessionManager::new());
        manager.set_default_model("openai/gpt-4o");
        let session_id = SessionManagerHandle::create_session(manager.as_ref(), None);
        let mut original = load_session(Uuid::parse_str(&session_id.value)?)?;
        append_message(&mut original, "user", "original evidence")?;
        Ok(Self {
            directory,
            environment,
            manager,
            session_id,
            original,
        })
    }

    async fn assert_unchanged(&self) -> TestResult {
        let persisted = load_session(self.original.id)?;
        assert_eq!(persisted.provider, self.original.provider);
        assert_eq!(persisted.revision, self.original.revision);
        let session = self.manager.get_session(&self.session_id.value)?;
        let inner = session.inner.lock().await;
        assert_eq!(
            inner.provider_manager().selected_model_string().as_deref(),
            Some("openai/gpt-4o")
        );
        let model = self.manager.get_session_model(&self.session_id);
        assert_eq!(model.provider_id, "openai");
        assert_eq!(model.model_id, "gpt-4o");
        assert_eq!(model.context_window, 128000);
        assert_eq!(model.max_output_tokens, 16384);
        Ok(())
    }

    async fn switch_and_compact(&self, provider: &str, model: &str) -> TestResult {
        self.manager.set_model(&self.session_id, provider, model)?;
        self.compact_selected(provider, model).await
    }

    async fn compact_selected(&self, provider: &str, model: &str) -> TestResult {
        let selection = format!("{provider}/{model}");
        let durable = load_session(self.original.id)?;
        assert_eq!(durable.provider, selection);
        assert_eq!(durable.messages.len(), self.original.messages.len());
        let session = self.manager.get_session(&self.session_id.value)?;
        let mut inner = session.inner.lock().await;
        assert_eq!(
            inner.provider_manager().selected_model_string().as_deref(),
            Some(selection.as_str())
        );
        let client: rig::providers::openai::CompletionsClient =
            rig::providers::openai::CompletionsClient::builder()
                .api_key("offline")
                .build()?;
        let completion_model = client.completion_model(model);
        let request = CompletionRequestBuilder::new(completion_model.clone(), "unused").build();
        inner.install_selected_preparer(SelectedRequestPreparer::new(
            completion_model,
            request,
            AdmissionLimits {
                context_capacity: Some(200000),
                output_reservation: Some(4096),
            },
        ));
        let source = inner.capture_durable_compaction(self.original.id)?;
        assert_eq!(source.provider, selection);
        assert_eq!(source.model, model);
        let committed = inner.commit_selected_compaction(
            source,
            Uuid::new_v4(),
            "<dag-node depth=\"D1\" turns=\"0-0\" label=\"Work\">original evidence</dag-node>",
            |_| Ok(()),
        )?;
        inner.ensure_compaction_inference_ready()?;
        let cold_store = SessionStore::new()?;
        let restored = cold_store.load(self.original.id)?;
        assert_eq!(restored.provider, selection);
        assert_eq!(restored.compaction_commit, committed.compaction_commit);
        assert_eq!(restored.messages.len(), self.original.messages.len());
        drop(inner);
        let manager =
            codelet_sessions::model_resolution::resolve_provider_manager(&selection).await?;
        let mut cold_runtime = codelet_cli::session::Session::from_provider_manager(manager);
        let envelopes = get_session_message_envelopes(self.original.id)?;
        cold_runtime.preflight_compaction_restore(&restored, &envelopes)?;
        cold_runtime.restore_compaction_boundary(&restored)?;
        cold_runtime.ensure_compaction_inference_ready()?;
        Ok(())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        reset_stores_for_tests();
        for (key, value) in &self.environment {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

/// @step Given a persisted session with an original message and a different selected model
/// @step When the session switches to an OpenAI profile model
/// @step Then its durable provider and runtime selection include the same profile and model
/// @step And a captured compaction can commit and survive cold restoration
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn switching_to_profile_preserves_identity_through_compaction() -> TestResult {
    let fixture = Fixture::new()?;
    fixture
        .switch_and_compact("openai:Strada", PROFILE_MODEL)
        .await
}

/// @step Given a persisted session with an original message and a different selected model
/// @step When the session switches to a cloud model from another provider
/// @step Then its durable provider and runtime selection include the same provider and model
/// @step And a captured compaction can commit and survive cold restoration
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn switching_to_cloud_preserves_identity_through_compaction() -> TestResult {
    let fixture = Fixture::new()?;
    fixture
        .switch_and_compact("anthropic", "claude-sonnet-5")
        .await
}

/// @step Given a persisted session with an original message and a different selected model
/// @step When the session selects a missing profile
/// @step Then the switch fails without changing the durable identity or runtime selection and limits
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn rejected_selection_preserves_durable_identity() -> TestResult {
    let fixture = Fixture::new()?;
    assert!(fixture
        .manager
        .set_model(&fixture.session_id, "openai:Missing", PROFILE_MODEL)
        .is_err());
    fixture.assert_unchanged().await
}

/// @step Given a persisted session with a committed compaction generation
/// @step When the session attempts to switch to a different selected identity
/// @step Then the switch fails without changing the durable identity or runtime selection and limits
/// @step And the prior request environment remains unchanged
/// @step And the committed generation remains valid
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn switching_committed_identity_preserves_ancestry() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.compact_selected("openai", "gpt-4o").await?;
    fixture.original = load_session(fixture.original.id)?;
    let environment = failure_paths::request_environment();
    let result = fixture
        .manager
        .set_model(&fixture.session_id, "openai:Strada", PROFILE_MODEL);
    assert!(
        result.is_err(),
        "switching must not rewrite a committed provider identity"
    );
    fixture.assert_unchanged().await?;
    assert_eq!(failure_paths::request_environment(), environment);
    let durable = load_session(fixture.original.id)?;
    assert_eq!(
        durable.compaction_commit,
        fixture.original.compaction_commit
    );
    let session = fixture.manager.get_session(&fixture.session_id.value)?;
    session
        .inner
        .lock()
        .await
        .capture_durable_compaction(fixture.original.id)?;
    Ok(())
}

/// @step Given a persisted session whose inner session is locked
/// @step When the session attempts to switch model
/// @step Then the switch fails without changing the durable identity or runtime selection and limits
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn busy_session_preserves_durable_identity() -> TestResult {
    let fixture = Fixture::new()?;
    let session = fixture.manager.get_session(&fixture.session_id.value)?;
    let guard = session.inner.lock().await;
    assert!(fixture
        .manager
        .set_model(&fixture.session_id, "openai:Strada", PROFILE_MODEL)
        .is_err());
    drop(guard);
    fixture.assert_unchanged().await
}

/// @step Given a persisted session whose manifest is readable but cannot be updated
/// @step When the session attempts to switch model
/// @step Then the switch fails without changing the durable identity or runtime selection and limits
/// @step And the prior request environment remains unchanged
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn durable_write_failure_preserves_runtime_selection() -> TestResult {
    let fixture = Fixture::new()?;
    let environment = failure_paths::request_environment();
    let permissions = failure_paths::DirectoryPermissions::set(
        &fixture.directory.path().join("sessions"),
        0o500,
    )?;
    assert_eq!(
        load_session(fixture.original.id)?.provider,
        fixture.original.provider
    );
    let result = fixture
        .manager
        .set_model(&fixture.session_id, "openai:Strada", PROFILE_MODEL);
    drop(permissions);
    assert!(
        result
            .as_ref()
            .is_err_and(|error| error.contains("manifest I/O failed")),
        "a failed durable commit must reject the switch: {result:?}"
    );
    fixture.assert_unchanged().await?;
    assert_eq!(failure_paths::request_environment(), environment);
    Ok(())
}
