//! Feature: spec/features/selected-compaction-provider-identity.feature
use codelet_cli::session::Session;
use codelet_core::persistence::*;
use codelet_providers::codex::CodexResponsesModel;
use codelet_providers::request_admission::AdmissionLimits;
use codelet_providers::selected_request::SelectedRequestPreparer;
use codelet_providers::{CodexProvider, ProviderManager, ProviderType};
use rig::client::CompletionClient;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const MODEL: &str = "org/model/version";
const DAG: &str = "<dag-node depth=\"D1\" turns=\"0-0\" label=\"Work\">exact DAG</dag-node>";

fn runtime(
    provider: ProviderType,
    profile: Option<&str>,
) -> Result<Session, Box<dyn std::error::Error>> {
    let name = provider.as_str().to_owned();
    let mut manager = ProviderManager::for_testing(provider, Some(200_000), Some(4096));
    manager.set_model_direct_with_profile(
        &name,
        MODEL,
        profile,
        Some(200_000),
        Some(4096),
        None,
    )?;
    let mut session = Session::from_provider_manager(manager);
    if name == "codex" {
        let provider = CodexProvider::from_api_key("offline", MODEL)?;
        let extension = session
            .provider_session
            .extension::<codelet_providers::codex::session::CodexSession>("codex")
            .ok_or("Codex extension")?;
        let model = CodexResponsesModel::new(provider.client().clone(), MODEL)
            .with_history(extension.history.clone());
        install_model(&mut session, model);
    } else {
        let client: rig::providers::openai::CompletionsClient =
            rig::providers::openai::CompletionsClient::builder()
                .api_key("offline")
                .build()?;
        install_model(&mut session, client.completion_model(MODEL));
    }
    Ok(session)
}

fn install_model<Model: CompletionModel + 'static>(session: &mut Session, model: Model) {
    let template = CompletionRequestBuilder::new(model.clone(), "unused")
        .preamble("offline compaction admission".to_owned())
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

fn survives_commit_and_restore(provider: ProviderType, profile: Option<&str>) -> TestResult {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut session = runtime(provider.clone(), profile)?;
    let selection = session
        .provider_manager()
        .selected_model_string()
        .ok_or("selected identity")?;
    let mut manifest = create_session_with_provider("profile", directory.path(), &selection)?;
    append_message(&mut manifest, "user", "covered original")?;
    let source = session.capture_durable_compaction(manifest.id)?;
    let committed = session.commit_selected_compaction(source, Uuid::new_v4(), DAG, |_| Ok(()))?;
    reset_stores_for_tests();
    let durable = load_session(manifest.id)?;
    let mut restored = runtime(provider, profile)?;
    let envelopes = get_session_message_envelopes(manifest.id)?;
    restored.preflight_compaction_restore(&durable, &envelopes)?;
    restored.restore_compaction_boundary(&durable)?;
    let boundary = committed.compaction_commit.ok_or("committed boundary")?;
    assert_eq!(boundary.source.provider, selection);
    assert_eq!(boundary.checkpoint.provider, selection);
    assert_eq!(durable.provider, selection);
    assert_eq!(durable.compaction_commit, Some(boundary));
    session.ensure_compaction_inference_ready()?;
    restored.ensure_compaction_inference_ready()?;
    assert!(envelopes.join("").contains("exact DAG"));
    Ok(())
}

/// @step Given a persisted OpenAI profile selection matching the selected serializer
/// @step When the selected runtime commits and restores its admitted DAG
/// @step Then the exact profile identity remains authoritative and inference opens
#[test]
#[serial_test::serial]
fn openai_profile_selection_survives_commit_and_restore() -> TestResult {
    survives_commit_and_restore(ProviderType::OpenAI, Some("Strada"))
}

/// @step Given a persisted Codex selection matching the selected serializer and checkpoint
/// @step When the selected runtime commits and restores its admitted DAG
/// @step Then the exact persisted identity remains authoritative and inference opens
#[test]
#[serial_test::serial]
fn ordinary_codex_selection_survives_commit_and_restore() -> TestResult {
    survives_commit_and_restore(ProviderType::Codex, None)
}

/// @step Given a persisted selection for a different profile of the same provider and model
/// @step When the selected runtime attempts to commit its captured source
/// @step Then promotion is rejected and ordinary inference remains blocked
#[test]
#[serial_test::serial]
fn different_persisted_profile_cannot_authorize_selected_compaction() -> TestResult {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut session = runtime(ProviderType::OpenAI, Some("Strada"))?;
    for selection in [format!("openai:Other/{MODEL}"), format!("openai/{MODEL}")] {
        let mut manifest =
            create_session_with_provider("wrong profile", directory.path(), &selection)?;
        append_message(&mut manifest, "user", "covered original")?;
        let source = session.capture_durable_compaction(manifest.id)?;
        let result = session.commit_selected_compaction(source, Uuid::new_v4(), DAG, |_| {
            Err("wrong profile must not publish".into())
        });
        assert!(result.is_err());
        assert!(load_session(manifest.id)?.compaction_commit.is_none());
        assert_eq!(load_session(manifest.id)?.provider, selection);
        assert!(session.ensure_compaction_inference_ready().is_err());
    }
    Ok(())
}

/// @step Given an installed selected serializer for one profile
/// @step When the runtime changes profile without changing provider or model
/// @step Then promotion is rejected and ordinary inference remains blocked
#[test]
#[serial_test::serial]
fn changed_selected_profile_invalidates_the_installed_preparer() -> TestResult {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut session = runtime(ProviderType::OpenAI, Some("Strada"))?;
    session
        .provider_manager_mut()
        .set_model_direct_with_profile("openai", MODEL, None, Some(200_000), Some(4096), None)?;
    let selection = format!("openai/{MODEL}");
    let mut manifest =
        create_session_with_provider("changed profile", directory.path(), &selection)?;
    append_message(&mut manifest, "user", "covered original")?;
    let source = session.capture_durable_compaction(manifest.id)?;
    let mut published = false;
    let result = session.commit_selected_compaction(source, Uuid::new_v4(), DAG, |_| {
        published = true;
        Ok(())
    });
    assert!(result.is_err(), "stale profile preparer must be rejected");
    assert!(!published);
    assert!(load_session(manifest.id)?.compaction_commit.is_none());
    assert!(session.ensure_compaction_inference_ready().is_err());
    Ok(())
}
