//! Feature: spec/features/compaction-switched-provider-replay.feature

#[path = "../../core/tests/support/compaction_boundary.rs"]
mod boundary;
#[path = "cmpct060/support.rs"]
pub mod support;

use codelet_core::persistence::*;
use codelet_core::SessionManagerHandle;
use codelet_providers::codex::session::CodexSession;
use codelet_providers::request_admission::AdmissionLimits;
use codelet_providers::selected_request::SelectedRequestPreparer;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use rig::client::CompletionClient;
use rig::completion::CompletionRequestBuilder;

const SCENARIO: &str = "scenario_full_openai_replay_preserves_selected_checkpoint_ownership";
const MODEL: &str = "o3";
const DAG: &str = "<dag-node depth=\"D1\" turns=\"0-0\" label=\"Work\">exact DAG</dag-node>";

struct ApiKey(Option<std::ffi::OsString>);

impl Drop for ApiKey {
    fn drop(&mut self) {
        match &self.0 {
            Some(value) => std::env::set_var("OPENAI_API_KEY", value),
            None => std::env::remove_var("OPENAI_API_KEY"),
        }
    }
}

async fn restore(manager: &SessionManager, id: &SessionId) -> support::TestResult {
    let background = manager.get_session(&id.value)?;
    let before = load_session(background.id)?;
    let inactive = {
        let inner = background.inner.lock().await;
        inner
            .provider_session
            .extension::<CodexSession>("codex")
            .map(|extension| -> Result<_, Box<dyn std::error::Error>> {
                Ok((
                    extension.history.generation()?,
                    extension.history.snapshot()?,
                ))
            })
            .transpose()?
    };
    // @step When SessionManager repeatedly restores the authoritative replay
    for _attempt in 0..2 {
        manager.restore_session_messages(id, get_session_message_envelopes(background.id)?)?;
        let inner = background.inner.lock().await;
        // @step Then the selected DAG and usage are restored without changing inactive Codex state
        assert!(serde_json::to_string(&inner.messages)?.contains("exact DAG"));
        assert_eq!(
            inner.token_tracker.input_tokens,
            before.token_usage.current_context_tokens
        );
        let checkpoint = inner
            .provider_session
            .extension::<CodexSession>("codex")
            .map(|extension| -> Result<_, Box<dyn std::error::Error>> {
                Ok((
                    extension.history.generation()?,
                    extension.history.snapshot()?,
                ))
            })
            .transpose()?;
        assert_eq!(checkpoint, inactive);
        inner.ensure_compaction_inference_ready()?;
        assert_eq!(
            load_session(background.id)?.compaction_commit,
            before.compaction_commit
        );
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_full_openai_replay_preserves_selected_checkpoint_ownership() -> support::TestResult
{
    let _key = ApiKey(std::env::var_os("OPENAI_API_KEY"));
    std::env::set_var("OPENAI_API_KEY", "offline");
    if let Some(id) = support::cold_process_session()? {
        // @step And a fresh process can resume the same durable OpenAI API identity
        let manager = support::cold_resume(&id)?;
        let background = manager.get_session(&id.value)?;
        assert_eq!(load_session(background.id)?.provider, "openai/o3");
        assert_eq!(
            background.inner.lock().await.current_provider_name(),
            "openai"
        );
        return restore(&manager, &id).await;
    }
    // @step Given a committed OpenAI API session retaining an inactive Codex extension
    let (directory, manager, id, _) = support::manager().await?;
    let background = manager.get_session(&id.value)?;
    let mut manifest = load_session(background.id)?;
    append_message(&mut manifest, "user", "covered original")?;
    update_session_manifest(background.id, |current| {
        current.provider = "openai/o3".into();
        Ok(())
    })?;
    {
        let mut inner = background.inner.lock().await;
        inner.provider_session.ensure_provider("codex");
        inner.reset_provider_state()?;
        inner.reset_provider_state()?;
        inner.provider_manager_mut().set_model_direct(
            "openai",
            MODEL,
            Some(200_000),
            Some(4096),
            None,
        )?;
        let client: rig::providers::openai::CompletionsClient =
            rig::providers::openai::CompletionsClient::builder()
                .api_key("offline")
                .build()?;
        let model = client.completion_model(MODEL);
        let template = CompletionRequestBuilder::new(model.clone(), "unused")
            .max_tokens(4096)
            .build();
        inner.install_selected_preparer(SelectedRequestPreparer::new(
            model,
            template,
            AdmissionLimits {
                context_capacity: Some(200_000),
                output_reservation: Some(4096),
            },
        ));
        let source = inner.capture_durable_compaction(background.id)?;
        inner.commit_selected_compaction(source, uuid::Uuid::new_v4(), DAG, |_| Ok(()))?;
    }
    // @step When SessionManager repeatedly restores the authoritative replay
    // @step Then the selected DAG and usage are restored without changing inactive Codex state
    restore(&manager, &id).await?;
    drop(manager);
    support::run_cold_process(directory.path(), &id, SCENARIO).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_direct_restore_rejects_corrupt_owned_provider_checkpoints_before_mutation(
) -> support::TestResult {
    // @step Given a committed boundary explicitly owning Codex checkpoint metadata
    let (_directory, manager, id, _) = support::manager().await?;
    let background = manager.get_session(&id.value)?;
    let mut manifest = load_session(background.id)?;
    append_message(&mut manifest, "user", "covered original")?;
    let mut inner = background.inner.lock().await;
    inner.provider_session.ensure_provider("codex");
    let model = inner.current_model_id().ok_or("selected model")?;
    let mut candidate = boundary::candidate(&manifest)?;
    candidate.source = inner.capture_durable_compaction(manifest.id)?;
    candidate.checkpoint.model = model.clone();
    candidate.checkpoint.payload =
        serde_json::to_value(inner.provider_session.prepare_generation(1, &model)?)?;
    let promoted = compaction_commit::commit_compaction(candidate, &model, |_, _| Ok(()))?;
    manifest = compaction_commit::publish_compaction(&promoted, |_| Ok(()))?;
    // @step And its retained replay contains a future checkpoint generation
    append_message_with_metadata(&mut manifest, "assistant", "", [
        ("provider".into(), serde_json::json!("codex")),
        ("message".into(), serde_json::json!({"role":"assistant","content":[]})),
        ("codexContinuation".into(), serde_json::json!({
            "version": 1, "generation": 2, "protocol": "full-current-dag", "model": model, "items": []
        })),
    ].into())?;
    let extension = inner
        .provider_session
        .extension::<CodexSession>("codex")
        .ok_or("Codex extension")?;
    let history = extension.history.clone();
    let before = (history.generation()?, history.snapshot()?);
    let messages = inner.messages.clone();
    let tokens = serde_json::to_value(&inner.token_tracker)?;
    // @step When the boundary is restored without a separate preflight call
    let result = inner.restore_compaction_boundary(&manifest);
    // @step Then restore fails without changing messages tokens or checkpoint state
    assert!(result.is_err());
    assert_eq!((history.generation()?, history.snapshot()?), before);
    assert_eq!(inner.messages, messages);
    assert_eq!(serde_json::to_value(&inner.token_tracker)?, tokens);
    // @step And ordinary inference remains blocked
    assert!(inner.ensure_compaction_inference_ready().is_err());
    Ok(())
}
