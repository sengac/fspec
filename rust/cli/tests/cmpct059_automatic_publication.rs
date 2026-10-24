//! Feature: spec/features/automatic-dag-context-publication.feature
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

const DAG: &str = "<dag-node depth=\"D1\" turns=\"0-0\" label=\"Work\">exact DAG</dag-node>";
type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Observer {
    reject: bool,
    identity: Mutex<Option<CompactionIdentity>>,
    completions: Mutex<usize>,
}

impl Observer {
    fn record_completion(&self) -> Result<(), String> {
        let identity = self.identity.lock().map_err(|error| error.to_string())?;
        let identity = identity.as_ref().ok_or("publication precedes completion")?;
        let boundary = load_session(identity.session_id)?
            .compaction_commit
            .ok_or("committed boundary")?;
        assert!(!self.reject, "failed publication cannot emit completion");
        assert_eq!(&boundary.identity, identity);
        assert!(!boundary.publication_pending);
        *self.completions.lock().map_err(|error| error.to_string())? += 1;
        Ok(())
    }
}

impl StreamOutput for Observer {
    fn emit(&self, event: StreamEvent) {
        if matches!(event, StreamEvent::CompactionComplete(_)) {
            assert_eq!(self.record_completion(), Ok(()));
        }
    }

    fn publish_committed_context(
        &self,
        identity: &CompactionIdentity,
        snapshot: CommittedContextSnapshot,
        context: RequestContextSnapshot,
    ) -> Result<(), String> {
        let boundary = load_session(identity.session_id)?
            .compaction_commit
            .ok_or("authoritative boundary")?;
        assert_eq!(&boundary.identity, identity);
        assert!(boundary.publication_pending);
        assert_eq!(snapshot.generation, identity.generation as i64);
        assert_eq!(context.generation, snapshot.generation);
        assert_eq!(context.model, boundary.checkpoint.model);
        assert_eq!(context.input_tokens, Some(boundary.input_tokens));
        assert_eq!(context.output_tokens, Some(0));
        assert_eq!(snapshot.tokens.input_tokens as u64, boundary.input_tokens);
        assert_eq!(
            serde_json::to_value(snapshot.context_fill).map_err(|error| error.to_string())?,
            serde_json::to_value(context.context_fill().ok_or("usable limits")?)
                .map_err(|error| error.to_string())?
        );
        assert!(snapshot.reconciled);
        assert_eq!(
            *self.completions.lock().map_err(|error| error.to_string())?,
            0
        );
        *self.identity.lock().map_err(|error| error.to_string())? = Some(identity.clone());
        if self.reject {
            Err("observer rejected committed context".into())
        } else {
            Ok(())
        }
    }
}

#[tokio::test]
#[serial_test::serial]
async fn automatic_builder_publication_refreshes_the_authoritative_context() -> TestResult {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    for reject in [false, true] {
        // @step Given a durable session and a fitting candidate from the selected builder
        let mut manifest =
            create_session_with_provider("automatic publication", directory.path(), "openai")?;
        append_message(&mut manifest, "user", "covered original")?;
        let mut manager =
            ProviderManager::for_testing(ProviderType::OpenAI, Some(200_000), Some(4096));
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
            Some(Arc::new(|_, _| Box::pin(async { Ok(DAG.into()) }))),
        );
        let observer = Observer {
            reject,
            identity: Mutex::new(None),
            completions: Mutex::new(0),
        };
        let tokens = Arc::new(Mutex::new(TokenState::default()));
        let busy = Arc::new(AtomicBool::new(false));
        // @step When automatic compaction promotes the candidate
        let result =
            run_compactor_sub_agent_round(&mut session, manifest.id, &tokens, &busy, &observer)
                .await;
        set_compactor_sub_agent_handler(manifest.id, None);
        let identity = observer
            .identity
            .lock()
            .map_err(|error| error.to_string())?
            .clone()
            .ok_or("observer must receive authoritative identity")?;
        let boundary = load_session(manifest.id)?
            .compaction_commit
            .ok_or("boundary")?;
        assert_eq!(boundary.identity, identity);
        assert_eq!(
            load_session(manifest.id)?.compaction.ok_or("DAG")?.summary,
            DAG
        );
        assert_eq!(identity.generation, 1);
        if reject {
            // @step And publication failure retains the exact pending identity and inference barrier
            assert!(result
                .err()
                .ok_or("publication must fail")?
                .to_string()
                .contains("observer rejected"));
            assert!(boundary.publication_pending);
            assert!(session.ensure_compaction_inference_ready().is_err());
            assert_eq!(
                *observer
                    .completions
                    .lock()
                    .map_err(|error| error.to_string())?,
                0
            );
            let before = serde_json::to_value(load_session(manifest.id)?)?;
            assert!(run_compactor_sub_agent_round(
                &mut session,
                manifest.id,
                &tokens,
                &busy,
                &observer,
            )
            .await
            .is_err());
            assert_eq!(serde_json::to_value(load_session(manifest.id)?)?, before);
            session.reconcile_durable_compaction(&identity)?;
            session.ensure_compaction_inference_ready()?;
            assert_eq!(
                load_session(manifest.id)?
                    .compaction_commit
                    .ok_or("reconciled")?
                    .identity,
                identity
            );
            assert_eq!(
                *observer
                    .completions
                    .lock()
                    .map_err(|error| error.to_string())?,
                0
            );
        } else {
            // @step Then observers receive the committed generation and context before completion
            result?;
            assert!(!boundary.publication_pending);
            session.ensure_compaction_inference_ready()?;
            assert_eq!(
                *observer
                    .completions
                    .lock()
                    .map_err(|error| error.to_string())?,
                1
            );
            assert_eq!(
                tokens
                    .lock()
                    .map_err(|error| error.to_string())?
                    .input_tokens,
                boundary.input_tokens
            );
        }
    }
    Ok(())
}
