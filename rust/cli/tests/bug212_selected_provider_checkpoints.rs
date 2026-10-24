//! Feature: spec/features/compaction-selected-provider-checkpoints.feature

use codelet_cli::session::Session;
use codelet_core::persistence::compaction_commit::CompactionCommitError;
use codelet_core::persistence::*;
use codelet_providers::codex::{
    history::CodexCheckpoint, session::CodexSession, CodexResponsesModel,
};
use codelet_providers::request_admission::AdmissionLimits;
use codelet_providers::selected_request::SelectedRequestPreparer;
use codelet_providers::{CodexProvider, ProviderManager, ProviderType};
use rig::client::CompletionClient;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const MODEL: &str = "qwen3.8-flash-next-iq3_s";
const DAG: &str = "<dag-node depth=\"D1\" turns=\"0-0\" label=\"Work\">exact DAG</dag-node>";

fn install<Model: CompletionModel + 'static>(session: &mut Session, model: Model) {
    let template = CompletionRequestBuilder::new(model.clone(), "unused")
        .preamble("offline selected admission".into())
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

fn runtime(switched: bool, profile: Option<&str>) -> Result<Session, Box<dyn std::error::Error>> {
    let initial = if switched {
        ProviderType::Codex
    } else {
        ProviderType::OpenAI
    };
    let mut manager = ProviderManager::for_testing(initial.clone(), Some(200_000), Some(4096));
    manager.set_model_direct(initial.as_str(), MODEL, Some(200_000), Some(4096), None)?;
    let mut session = Session::from_provider_manager(manager);
    if switched {
        session.reset_provider_state()?;
        session.reset_provider_state()?;
    }
    session
        .provider_manager_mut()
        .set_model_direct_with_profile("openai", MODEL, profile, Some(200_000), Some(4096), None)?;
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("offline-no-network")
            .build()?;
    install(&mut session, client.completion_model(MODEL));
    Ok(session)
}

fn persisted(
    session: &Session,
) -> Result<(tempfile::TempDir, SessionManifest), Box<dyn std::error::Error>> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let selection = session
        .provider_manager()
        .selected_model_string()
        .ok_or("selection")?;
    let mut manifest =
        create_session_with_provider("switched provider", directory.path(), &selection)?;
    append_message(&mut manifest, "user", "covered original")?;
    Ok((directory, manifest))
}

fn codex_state(
    session: &Session,
) -> Result<(u64, Option<CodexCheckpoint>), Box<dyn std::error::Error>> {
    let extension = session
        .provider_session
        .extension::<CodexSession>("codex")
        .ok_or("retained Codex extension")?;
    Ok((
        extension.history.generation()?,
        extension.history.snapshot()?,
    ))
}

#[test]
#[serial_test::serial]
fn scenario_openai_compaction_ignores_inactive_codex_state_after_switching_providers() -> TestResult
{
    // @step Given an OpenAI API selection after switching away from Codex with a newer transport generation
    for profile in [None, Some("Strada")] {
        let mut session = runtime(true, profile)?;
        let (_directory, manifest) = persisted(&session)?;
        let original = codex_state(&session)?;
        assert!(original.0 >= 2);
        let selection = manifest.provider.clone();
        // @step When successive admitted compactions are committed and published
        for generation in 1..=2 {
            let source = session.capture_durable_compaction(manifest.id)?;
            let mut published = false;
            let committed =
                session.commit_selected_compaction(source, Uuid::new_v4(), DAG, |_| {
                    published = true;
                    Ok(())
                })?;
            // @step Then each boundary retains the exact OpenAI API selection without Codex metadata
            let boundary = committed.compaction_commit.ok_or("boundary")?;
            assert!(published);
            assert_eq!(committed.provider, selection);
            assert_eq!(boundary.checkpoint.provider, selection);
            assert_eq!(boundary.checkpoint.model, MODEL);
            assert_eq!(boundary.identity.generation, generation);
            assert_eq!(boundary.checkpoint.payload, serde_json::json!({}));
            assert!(!boundary.publication_pending);
            // @step And inactive Codex state is unchanged and inference opens
            assert_eq!(codex_state(&session)?, original);
            session.ensure_compaction_inference_ready()?;
        }
    }
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_openai_restore_ignores_an_inactive_codex_extension() -> TestResult {
    // @step Given a committed OpenAI API profile boundary and a runtime previously using Codex
    let mut clean = runtime(false, Some("Strada"))?;
    let (_directory, manifest) = persisted(&clean)?;
    let source = clean.capture_durable_compaction(manifest.id)?;
    clean.commit_selected_compaction(source, Uuid::new_v4(), DAG, |_| Ok(()))?;
    reset_stores_for_tests();
    let durable = load_session(manifest.id)?;
    let envelopes = get_session_message_envelopes(manifest.id)?;
    let mut session = runtime(true, Some("Strada"))?;
    let original = codex_state(&session)?;
    // @step When the runtime preflights and restores the committed boundary
    session.preflight_compaction_restore(&durable, &envelopes)?;
    session.restore_compaction_boundary(&durable)?;
    // @step Then the OpenAI API generation is restored without requiring Codex metadata
    assert_eq!(
        session.token_tracker.input_tokens,
        durable.token_usage.current_context_tokens
    );
    assert_eq!(
        load_session(manifest.id)?.compaction_commit,
        durable.compaction_commit
    );
    // @step And inactive Codex state is unchanged and inference opens
    assert_eq!(codex_state(&session)?, original);
    session.ensure_compaction_inference_ready()?;
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_failed_openai_publication_reconciles_without_invoking_inactive_codex_state(
) -> TestResult {
    // @step Given an OpenAI API selection after switching away from Codex with a newer transport generation
    let mut session = runtime(true, Some("Strada"))?;
    let (_directory, manifest) = persisted(&session)?;
    let original = codex_state(&session)?;
    let source = session.capture_durable_compaction(manifest.id)?;
    // @step When admitted compaction is promoted but publication fails
    let failure = session
        .commit_selected_compaction(source, Uuid::new_v4(), DAG, |_| Err("disconnected".into()));
    let CompactionCommitError::Publication { identity, .. } =
        failure.err().ok_or("publication fails")?
    else {
        return Err("expected promoted publication failure, not inactive Codex rejection".into());
    };
    // @step Then inference stays blocked until the exact durable identity is reconciled
    assert!(session.ensure_compaction_inference_ready().is_err());
    let mut wrong = identity.clone();
    wrong.operation_id = Uuid::new_v4();
    assert!(session.reconcile_durable_compaction(&wrong).is_err());
    assert!(session.ensure_compaction_inference_ready().is_err());
    session.reconcile_durable_compaction(&identity)?;
    session.ensure_compaction_inference_ready()?;
    // @step And reconciliation leaves inactive Codex state unchanged
    assert_eq!(codex_state(&session)?, original);
    assert!(
        !load_session(manifest.id)?
            .compaction_commit
            .ok_or("boundary")?
            .publication_pending
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_selected_codex_compaction_still_rejects_invalid_checkpoint_lineage() -> TestResult {
    // @step Given a selected Codex runtime whose transport generation exceeds its durable generation
    let mut session = runtime(true, None)?;
    session.provider_manager_mut().set_model_direct(
        "codex",
        "gpt-6.1-sol",
        Some(200_000),
        Some(4096),
        None,
    )?;
    let provider = CodexProvider::from_api_key("offline-no-network", "gpt-6.1-sol")?;
    let history = session
        .provider_session
        .extension::<CodexSession>("codex")
        .ok_or("Codex extension")?
        .history
        .clone();
    install(
        &mut session,
        CodexResponsesModel::new(provider.client().clone(), "gpt-6.1-sol").with_history(history),
    );
    let (_directory, manifest) = persisted(&session)?;
    let original = codex_state(&session)?;
    let source = session.capture_durable_compaction(manifest.id)?;
    let mut published = false;
    // @step When the runtime attempts an admitted compaction
    let result = session.commit_selected_compaction(source, Uuid::new_v4(), DAG, |_| {
        published = true;
        Ok(())
    });
    // @step Then checkpoint lineage rejection prevents promotion and publication
    let failure = result.err().ok_or("invalid lineage must fail")?;
    assert!(
        failure
            .to_string()
            .contains("Codex checkpoint lineage mismatch"),
        "{failure}"
    );
    assert!(!published);
    assert!(load_session(manifest.id)?.compaction_commit.is_none());
    assert_eq!(codex_state(&session)?, original);
    // @step And ordinary inference remains blocked
    assert!(session.ensure_compaction_inference_ready().is_err());
    Ok(())
}
