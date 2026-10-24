//! Feature: spec/features/durable-versioned-dag-boundary.feature
use codelet_cli::compactor_sub_agent::{
    run_compactor_sub_agent_round, set_compactor_sub_agent_handler,
};
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::persistence::compaction_boundary::CompactionIdentity;
use codelet_core::persistence::*;
use codelet_core::TokenState;
use codelet_providers::request_admission::AdmissionLimits;
use codelet_providers::selected_request::SelectedRequestPreparer;
use codelet_providers::{ProviderManager, ProviderType};
use codelet_rpc_types::{CommittedContextSnapshot, RequestContextSnapshot};
use rig::client::CompletionClient;
use rig::completion::CompletionRequestBuilder;
use std::sync::{atomic::AtomicBool, Arc, Mutex};

#[derive(Default)]
struct FailedPublication(Mutex<Option<CompactionIdentity>>);

impl StreamOutput for FailedPublication {
    fn emit(&self, event: StreamEvent) {
        assert!(!matches!(event, StreamEvent::CompactionComplete(_)));
    }

    fn publish_committed_context(
        &self,
        identity: &CompactionIdentity,
        snapshot: CommittedContextSnapshot,
        context: RequestContextSnapshot,
    ) -> Result<(), String> {
        let manifest = load_session(identity.session_id)?;
        let boundary = manifest.compaction_commit.ok_or("missing boundary")?;
        assert_eq!(&boundary.identity, identity);
        assert!(boundary.publication_pending);
        assert_eq!(snapshot.generation, identity.generation as i64);
        assert_eq!(context.input_tokens, Some(boundary.input_tokens));
        assert_eq!(context.output_tokens, Some(0));
        assert!(snapshot.reconciled);
        *self.0.lock().map_err(|error| error.to_string())? = Some(identity.clone());
        Err("observer unavailable".into())
    }
}

#[tokio::test]
#[serial_test::serial]
async fn selected_builder_publication_failure_retains_exact_promoted_ownership(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a selected builder returning an admissible DAG
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut manifest =
        create_session_with_provider("selected publication", directory.path(), "openai")?;
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
    session.install_selected_preparer(SelectedRequestPreparer::new(
        model,
        template,
        AdmissionLimits {
            context_capacity: Some(200_000),
            output_reservation: Some(4096),
        },
    ));
    set_compactor_sub_agent_handler(
        manifest.id,
        Some(Arc::new(|_, _| {
            Box::pin(async {
                Ok(
                    "<dag-node depth=\"D1\" turns=\"0-0\" label=\"Work\">exact DAG</dag-node>"
                        .into(),
                )
            })
        })),
    );
    let output = FailedPublication::default();
    // @step When its identity-bearing context publication fails
    let result = run_compactor_sub_agent_round(
        &mut session,
        manifest.id,
        &Arc::new(Mutex::new(TokenState::default())),
        &Arc::new(AtomicBool::new(false)),
        &output,
    )
    .await;
    set_compactor_sub_agent_handler(manifest.id, None);
    let failure = result
        .err()
        .ok_or("failed observer cannot acknowledge durable application")?;
    assert!(
        failure.to_string().contains("observer unavailable"),
        "{failure}"
    );
    let identity = output
        .0
        .lock()
        .map_err(|error| error.to_string())?
        .clone()
        .ok_or("publication identity")?;
    // @step Then the durable generation remains pending and ordinary inference stays blocked
    assert!(session.ensure_compaction_inference_ready().is_err());
    assert!(
        load_session(manifest.id)?
            .compaction_commit
            .ok_or("boundary")?
            .publication_pending
    );
    // @step And exact identity reconciliation succeeds without another promotion or completion
    session.reconcile_durable_compaction(&identity)?;
    session.ensure_compaction_inference_ready()?;
    assert_eq!(
        load_session(manifest.id)?
            .compaction_commit
            .ok_or("boundary")?
            .identity,
        identity
    );
    Ok(())
}
