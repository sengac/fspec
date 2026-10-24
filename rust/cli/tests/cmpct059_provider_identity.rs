//! Feature: spec/features/selected-compaction-provider-identity.feature
use codelet_cli::session::Session;
use codelet_core::persistence::*;
use codelet_providers::request_admission::AdmissionLimits;
use codelet_providers::selected_request::SelectedRequestPreparer;
use codelet_providers::{ProviderManager, ProviderType};
use rig::client::CompletionClient;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const MODEL: &str = "org/model/version";
const DAG: &str = "<dag-node depth=\"D1\" turns=\"0-0\" label=\"Work\">exact DAG</dag-node>";

fn runtime_for(provider: ProviderType) -> Result<Session, Box<dyn std::error::Error>> {
    let name = match &provider {
        ProviderType::Claude => "anthropic",
        ProviderType::Gemini => "google",
        provider => provider.as_str(),
    }
    .to_owned();
    let mut manager = ProviderManager::for_testing(provider, Some(200_000), Some(4096));
    manager.set_model_direct(&name, MODEL, Some(200_000), Some(4096), None)?;
    let mut session = Session::from_provider_manager(manager);
    match session.provider_manager().current_provider_type() {
        ProviderType::Claude => {
            let client: rig::providers::anthropic::Client =
                rig::providers::anthropic::Client::builder()
                    .api_key("offline")
                    .build()?;
            install_model(&mut session, client.completion_model(MODEL));
        }
        ProviderType::Gemini => {
            let client: rig::providers::gemini::Client = rig::providers::gemini::Client::builder()
                .api_key("offline")
                .build()?;
            install_model(&mut session, client.completion_model(MODEL));
        }
        _ => {
            let client: rig::providers::openai::CompletionsClient =
                rig::providers::openai::CompletionsClient::builder()
                    .api_key("offline")
                    .build()?;
            install_model(&mut session, client.completion_model(MODEL));
        }
    }
    Ok(session)
}

fn install_model<Model: CompletionModel + 'static>(session: &mut Session, model: Model) {
    let template = CompletionRequestBuilder::new(model.clone(), "unused")
        .max_tokens(4096)
        .build();
    session.install_selected_preparer(SelectedRequestPreparer::new(
        model,
        template,
        AdmissionLimits {
            context_capacity: Some(200_000),
            output_reservation: Some(4096),
        },
    ));
}

fn runtime() -> Result<Session, Box<dyn std::error::Error>> {
    runtime_for(ProviderType::OpenAI)
}

#[test]
#[serial_test::serial]
fn composite_persisted_selection_survives_commit_and_restore() -> TestResult {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    // @step Given a persisted provider and model selection matching the selected serializer
    let provider = format!("openai/{MODEL}");
    let mut manifest =
        create_session_with_provider("composite identity", directory.path(), &provider)?;
    append_message(&mut manifest, "user", "covered original")?;
    let mut session = runtime()?;
    let source = session.capture_durable_compaction(manifest.id)?;
    // @step When the selected runtime commits and restores its admitted DAG
    let committed = session.commit_selected_compaction(source, Uuid::new_v4(), DAG, |_| Ok(()))?;
    reset_stores_for_tests();
    let durable = load_session(manifest.id)?;
    let mut restored = runtime()?;
    restored.restore_compaction_boundary(&durable)?;
    // @step Then the exact persisted identity remains authoritative and inference opens
    let boundary = committed.compaction_commit.ok_or("committed boundary")?;
    assert_eq!(boundary.source.provider, provider);
    assert_eq!(boundary.checkpoint.provider, provider);
    assert_eq!(durable.provider, provider);
    assert_eq!(durable.compaction_commit, Some(boundary));
    session.ensure_compaction_inference_ready()?;
    restored.ensure_compaction_inference_ready()?;
    assert!(get_session_message_envelopes(manifest.id)?
        .join("")
        .contains("exact DAG"));
    Ok(())
}

fn rejects_identity(provider: &str, change_model: bool) -> TestResult {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut manifest =
        create_session_with_provider("identity rejection", directory.path(), provider)?;
    append_message(&mut manifest, "user", "covered original")?;
    let mut session = runtime()?;
    let source = session.capture_durable_compaction(manifest.id)?;
    if change_model {
        session.provider_manager_mut().set_model_direct(
            "openai",
            "different",
            Some(200_000),
            Some(4096),
            None,
        )?;
    }
    let result = session.commit_selected_compaction(source, Uuid::new_v4(), DAG, |_| {
        Err("rejected identities must not publish".into())
    });
    assert!(result
        .err()
        .ok_or("changed identity must reject")?
        .to_string()
        .contains("identity"));
    assert!(load_session(manifest.id)?.compaction_commit.is_none());
    assert_eq!(load_session(manifest.id)?.provider, provider);
    assert!(session.ensure_compaction_inference_ready().is_err());
    Ok(())
}

/// @step Given a persisted selection for a different provider
/// @step When the selected runtime attempts to commit its captured source
/// @step Then promotion is rejected and ordinary inference remains blocked
#[test]
#[serial_test::serial]
fn different_persisted_provider_cannot_authorize_selected_compaction() -> TestResult {
    rejects_identity(&format!("codex/{MODEL}"), false)
}

/// @step Given a persisted selection for a different complete model identifier
/// @step When the selected runtime attempts to commit its captured source
/// @step Then promotion is rejected and ordinary inference remains blocked
#[test]
#[serial_test::serial]
fn different_persisted_model_cannot_authorize_selected_compaction() -> TestResult {
    rejects_identity("openai/org/model/version/other", false)
}

/// @step Given a captured source and selected serializer for one model
/// @step When the runtime changes model before selected compaction
/// @step Then promotion is rejected and ordinary inference remains blocked
#[test]
#[serial_test::serial]
fn changed_selected_model_invalidates_the_installed_preparer() -> TestResult {
    rejects_identity(&format!("openai/{MODEL}"), true)
}

#[test]
#[serial_test::serial]
fn selected_provider_restore_requirements_are_checked_before_mutation() -> TestResult {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    // @step Given ordinary persisted non-Codex history and a selected Codex runtime
    let mut manifest = create_session_with_provider(
        "restore authority",
        directory.path(),
        "anthropic/claude-sonnet-4",
    )?;
    append_message(&mut manifest, "user", "ordinary persisted history")?;
    let envelopes = get_session_message_envelopes(manifest.id)?;
    let mut manager = ProviderManager::for_testing(ProviderType::Codex, Some(200_000), Some(4096));
    manager.set_model_direct("codex", MODEL, Some(200_000), Some(4096), None)?;
    let mut session = Session::from_provider_manager(manager);
    session
        .messages
        .push(rig::message::Message::user("live prefix"));
    session.token_tracker.input_tokens = 71;
    session.token_tracker.output_tokens = 17;
    let before = serde_json::to_value(&session.messages)?;
    let checkpoint = session
        .provider_session
        .extension::<codelet_providers::codex::session::CodexSession>("codex")
        .ok_or("selected Codex extension")?;
    assert!(checkpoint.history.snapshot()?.is_none());
    let generation = checkpoint.history.generation()?;
    // @step When the runtime preflights the authoritative replay
    let result = session.preflight_compaction_restore(&manifest, &envelopes);
    // @step Then the selected provider rejects its missing required checkpoint before mutation
    assert!(
        matches!(result,
            Err(codelet_core::persistence::compaction_commit::CompactionCommitError::Rejected(ref reason))
                if reason == &codelet_providers::codex::history::CheckpointError::Missing.to_string()),
        "selected Codex requirements were skipped: {result:?}"
    );
    // @step And existing messages checkpoint counters and durable history remain unchanged
    assert_eq!(serde_json::to_value(&session.messages)?, before);
    assert_eq!(session.token_tracker.input_tokens, 71);
    assert_eq!(session.token_tracker.output_tokens, 17);
    assert!(checkpoint.history.snapshot()?.is_none());
    assert_eq!(checkpoint.history.generation()?, generation);
    assert_eq!(
        serde_json::to_value(load_session(manifest.id)?)?,
        serde_json::to_value(&manifest)?
    );
    assert_eq!(get_session_message_envelopes(manifest.id)?, envelopes);
    Ok(())
}

fn alias_survives_commit_and_restore(alias: &str, provider: ProviderType) -> TestResult {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let selection = format!("{alias}/{MODEL}");
    let mut manifest = create_session_with_provider("alias", directory.path(), &selection)?;
    append_message(&mut manifest, "user", "covered original")?;
    let mut session = runtime_for(provider.clone())?;
    let source = session.capture_durable_compaction(manifest.id)?;
    let committed = session.commit_selected_compaction(source, Uuid::new_v4(), DAG, |_| Ok(()))?;
    reset_stores_for_tests();
    let durable = load_session(manifest.id)?;
    let mut restored = runtime_for(provider)?;
    restored.restore_compaction_boundary(&durable)?;
    let boundary = committed.compaction_commit.ok_or("boundary")?;
    assert_eq!(boundary.source.provider, selection);
    assert_eq!(boundary.checkpoint.provider, selection);
    assert_eq!(durable.provider, selection);
    session.ensure_compaction_inference_ready()?;
    restored.ensure_compaction_inference_ready()?;
    Ok(())
}

/// @step Given an anthropic persisted selection and the matching Claude runtime
/// @step When the selected runtime commits and restores its admitted DAG
/// @step Then the exact persisted identity remains authoritative and inference opens
#[test]
#[serial_test::serial]
fn anthropic_persisted_alias_identifies_the_selected_claude_runtime() -> TestResult {
    alias_survives_commit_and_restore("anthropic", ProviderType::Claude)
}

/// @step Given a google persisted selection and the matching Gemini runtime
/// @step When the selected runtime commits and restores its admitted DAG
/// @step Then the exact persisted identity remains authoritative and inference opens
#[test]
#[serial_test::serial]
fn google_persisted_alias_identifies_the_selected_gemini_runtime() -> TestResult {
    alias_survives_commit_and_restore("google", ProviderType::Gemini)
}
