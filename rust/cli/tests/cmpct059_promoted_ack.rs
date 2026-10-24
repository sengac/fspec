//! Feature: spec/features/promoted-compaction-durability-acknowledgement.feature

#[cfg(unix)]
#[test]
#[serial_test::serial]
fn runtime_retains_ownership_after_a_promoted_durability_acknowledgement_failure(
) -> Result<(), Box<dyn std::error::Error>> {
    use codelet_cli::session::Session;
    use codelet_core::persistence::*;
    use codelet_providers::request_admission::AdmissionLimits;
    use codelet_providers::selected_request::SelectedRequestPreparer;
    use codelet_providers::{ProviderManager, ProviderType};
    use rig::client::CompletionClient;
    use rig::completion::CompletionRequestBuilder;
    use std::os::unix::fs::PermissionsExt;

    // @step Given a selected runtime candidate whose manifest rename succeeds but directory synchronization fails
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("acknowledgement", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "covered original")?;
    let mut manager = ProviderManager::for_testing(ProviderType::OpenAI, Some(200_000), Some(4096));
    manager.set_model_direct("openai", "o3", Some(200_000), Some(4096), None)?;
    let mut session = Session::from_provider_manager(manager);
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("offline")
            .build()?;
    let model = client.completion_model("o3");
    let template = CompletionRequestBuilder::new(model.clone(), "unused")
        .max_tokens(4096)
        .build();
    let preparer = SelectedRequestPreparer::new(
        model,
        template,
        AdmissionLimits {
            context_capacity: Some(200_000),
            output_reservation: Some(4096),
        },
    );
    let source = session.capture_durable_compaction(manifest.id)?;
    let sessions = directory.path().join("sessions");
    let permissions = std::fs::metadata(&sessions)?.permissions();
    // @step When durable application reports the promoted identity
    let result = session.commit_durable_compaction(
        source,
        uuid::Uuid::new_v4(),
        "<dag-node depth=\"D1\" turns=\"0-0\" label=\"Work\">exact DAG</dag-node>",
        |messages, generation| {
            let admitted = preparer.prepare(messages, generation)?;
            std::fs::set_permissions(&sessions, std::fs::Permissions::from_mode(0o300)).map_err(
                |error| {
                    codelet_providers::request_admission::RequestAdmissionError::Preparation(
                        error.to_string(),
                    )
                },
            )?;
            Ok(admitted)
        },
        |_| Ok(()),
    );
    std::fs::set_permissions(&sessions, permissions)?;
    let failure = result
        .err()
        .ok_or("directory synchronization fails after rename")?;
    let identity = failure
        .promoted_identity()
        .ok_or("promoted identity")?
        .clone();
    // @step Then ordinary inference remains blocked and that exact identity can be reconciled without another promotion
    assert!(session.ensure_compaction_inference_ready().is_err());
    session.reconcile_durable_compaction(&identity)?;
    session.ensure_compaction_inference_ready()?;
    let durable = load_session(manifest.id)?;
    let boundary = durable.compaction_commit.ok_or("boundary")?;
    assert_eq!(boundary.identity, identity);
    assert_eq!(boundary.identity.generation, 1);
    assert!(!boundary.publication_pending);
    Ok(())
}
