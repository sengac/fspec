//! Feature: spec/features/codex-physical-request-freshness.feature

use codelet_providers::{
    codex::{
        history::CheckpointError, refreshing_client::RefreshingCodexClient, CodexResponsesModel,
    },
    request_admission::AdmissionLimits,
    session::ProviderSession,
};
use rig::{
    completion::{CompletionError, CompletionModel, CompletionRequestBuilder},
    message::Message,
    providers::openai,
};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

#[path = "fixtures/prov147.rs"]
pub mod fixture;
#[path = "cmpct078/support.rs"]
pub mod support;
use support::{history, request};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn nonstreaming_responses_cannot_outlive_their_checkpoint_authority() -> TestResult {
    for invalidation in ["failed restore", "generation replacement"] {
        // @step Given a nonstreaming Codex request is waiting for its physical response
        let session = ProviderSession::for_provider("codex");
        let initial_metadata = session.prepare_generation(1, "shared-model")?;
        session.install_committed_generation(1, &initial_metadata)?;
        history(&session)?.commit(&request("committed DAG"), &[])?;
        let original = history(&session)?.snapshot()?;
        let authentic = json!({"codexContinuation":original}).to_string();
        let next_metadata = session.prepare_generation(2, "shared-model")?;
        let authoritative = if invalidation == "failed restore" {
            original
        } else {
            serde_json::from_value(next_metadata["codexContinuation"].clone())?
        };
        let writes = Arc::new(AtomicUsize::new(0));
        let observed = writes.clone();
        session.bind_writers(|_| {
            let observed = observed.clone();
            Arc::new(move |_| {
                observed.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        })?;
        let server = MockServer::start().await;
        let changing_session = session.clone();
        Mock::given(method("POST"))
            .and(path("/responses"))
            .respond_with(move |_: &wiremock::Request| {
                // @step When checkpoint authority changes through <invalidation> before the response arrives
                if invalidation == "failed restore" {
                    assert!(changing_session.restore(&["{invalid".into()]).is_err());
                } else {
                    assert!(
                        changing_session
                            .install_committed_generation(2, &next_metadata)
                            .is_ok(),
                        "new committed boundary must install before response arrives"
                    );
                }
                ResponseTemplate::new(200).set_body_json(fixture::response())
            })
            .up_to_n_times(1)
            .expect(1)
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture::response()))
            .expect(1)
            .with_priority(2)
            .mount(&server)
            .await;
        let client = openai::Client::<RefreshingCodexClient>::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .http_client(RefreshingCodexClient::new_api_key())
            .build()?;
        let model = CodexResponsesModel::new(client, "shared-model")
            .with_history(history(&session)?.clone())
            .with_request_admission(AdmissionLimits {
                context_capacity: Some(100_000),
                output_reservation: Some(1000),
            });
        let result = model
            .completion(
                CompletionRequestBuilder::new(model.clone(), "stale prepared suffix")
                    .messages(vec![Message::user("committed DAG")])
                    .build(),
            )
            .await;
        // @step Then completion returns a typed checkpoint failure without publishing the stale response
        assert!(
            matches!(result, Err(CompletionError::RequestError(ref error))
                if matches!(error.downcast_ref::<CheckpointError>(),
                    Some(CheckpointError::Blocked | CheckpointError::Lineage(_)))),
            "{invalidation} published a stale completion: {result:?}"
        );
        // @step And the authoritative checkpoint and persistence observer remain unchanged
        assert_eq!(history(&session)?.snapshot()?, authoritative);
        assert_eq!(writes.load(Ordering::SeqCst), 0);
        assert_eq!(
            server
                .received_requests()
                .await
                .ok_or("capture disabled")?
                .len(),
            1
        );
        // @step And an authentic fresh request succeeds without a replacement transport
        let dag = if invalidation == "failed restore" {
            session.restore(&[authentic])?;
            "committed DAG"
        } else {
            "replacement DAG"
        };
        let response = model
            .completion(
                CompletionRequestBuilder::new(model.clone(), "fresh suffix")
                    .messages(vec![Message::user(dag)])
                    .build(),
            )
            .await?;
        assert_eq!(
            response.choice.first(),
            rig::message::AssistantContent::text("Hello")
        );
        assert_eq!(writes.load(Ordering::SeqCst), 1);
        let captured = server.received_requests().await.ok_or("capture disabled")?;
        assert_eq!(captured.len(), 2);
        let fresh: Value = captured.last().ok_or("fresh request absent")?.body_json()?;
        assert_eq!(fresh["model"], "shared-model");
        let wire = fresh.to_string();
        assert_eq!(wire.matches(dag).count(), 1);
        assert_eq!(wire.matches("fresh suffix").count(), 1);
        assert!(!wire.contains("stale prepared suffix"));
    }
    Ok(())
}
