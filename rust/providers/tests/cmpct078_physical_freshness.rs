//! Feature: spec/features/codex-physical-request-freshness.feature

use codelet_providers::{
    codex::history::CheckpointError, request_admission::AdmissionLimits, session::ProviderSession,
};
use futures::StreamExt;
use rig::{
    completion::{CompletionError, CompletionModel, CompletionRequestBuilder},
    message::Message,
    streaming::StreamedAssistantContent,
};
use serde_json::json;

#[path = "fixtures/attempt_events.rs"]
mod attempt_events;
#[path = "fixtures/prov148.rs"]
mod fixture;
#[path = "cmpct078/support.rs"]
mod support;
use support::{boundary, history, request};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn invalidated_lazy_streams_cannot_send_a_previously_prepared_request() -> TestResult {
    for invalidation in ["failed restore", "generation replacement"] {
        // @step Given a Codex stream prepared against the committed checkpoint but not yet polled
        let session = ProviderSession::for_provider("codex");
        let boundary = boundary()?;
        let metadata = [(
            "codexContinuation".into(),
            boundary["codexContinuation"].clone(),
        )]
        .into_iter()
        .collect();
        session.install_committed_generation(1, &metadata)?;
        history(&session)?.commit(&request("committed DAG"), &[])?;
        let initial = history(&session)?.snapshot()?;
        let authentic = json!({"codexContinuation":initial}).to_string();
        let server =
            fixture::Server::start(vec![fixture::terminal("completed", None, None)], false).await?;
        let model = server
            .models()?
            .0
            .with_history(history(&session)?.clone())
            .with_request_admission(AdmissionLimits {
                context_capacity: Some(100_000),
                output_reservation: Some(1000),
            });
        let mut stream = model
            .stream(
                CompletionRequestBuilder::new(model.clone(), "stale prepared suffix")
                    .messages(vec![Message::user("committed DAG")])
                    .build(),
            )
            .await?;
        // @step When checkpoint authority changes through <invalidation> before the first poll
        if invalidation == "failed restore" {
            assert!(session.restore(&["{invalid".into()]).is_err());
        } else {
            let next = session.prepare_generation(2, "shared-model")?;
            session.install_committed_generation(2, &next)?;
        }
        let authoritative = history(&session)?.snapshot()?;
        // @step Then the prepared stream returns a typed checkpoint failure without a physical request or completion
        let event = stream.next().await.ok_or("checkpoint failure absent")?;
        assert!(
            matches!(event, Err(CompletionError::RequestError(ref error))
                if matches!(error.downcast_ref::<CheckpointError>(),
                    Some(CheckpointError::Blocked | CheckpointError::Lineage(_)))),
            "{invalidation} allowed prepared request: {event:?}"
        );
        assert!(stream.next().await.is_none());
        assert!(server
            .requests
            .lock()
            .map_err(|_| "capture lock")?
            .is_empty());
        // @step And the authoritative checkpoint remains unchanged until an authentic fresh request succeeds
        assert_eq!(history(&session)?.snapshot()?, authoritative);
        let dag = if invalidation == "failed restore" {
            session.restore(&[authentic])?;
            "committed DAG"
        } else {
            "replacement DAG"
        };
        let mut retry = model
            .stream(
                CompletionRequestBuilder::new(model.clone(), "fresh suffix")
                    .messages(vec![Message::user(dag)])
                    .build(),
            )
            .await?;
        let mut finals = 0;
        while let Some(event) = retry.next().await {
            if matches!(event?, StreamedAssistantContent::Final(_)) {
                finals += 1;
            }
        }
        assert_eq!(finals, 1);
        let captured = server.requests.lock().map_err(|_| "capture lock")?;
        assert_eq!(captured.len(), 1);
        let wire = captured.first().ok_or("fresh request absent")?.to_string();
        assert_eq!(wire.matches(dag).count(), 1);
        assert_eq!(wire.matches("fresh suffix").count(), 1);
        assert!(!wire.contains("stale prepared suffix"));
    }
    Ok(())
}

#[tokio::test]
async fn replaced_generations_cannot_receive_an_old_physical_response_checkpoint() -> TestResult {
    // @step Given a Codex physical response has started for the committed generation
    let session = ProviderSession::for_provider("codex");
    let metadata = session.prepare_generation(1, "shared-model")?;
    session.install_committed_generation(1, &metadata)?;
    let body = format!(
        "{}{}",
        fixture::text(),
        fixture::terminal("completed", None, None)
    );
    let server = fixture::Server::start(vec![body], false).await?;
    let model = server
        .models()?
        .0
        .with_history(history(&session)?.clone())
        .with_request_admission(AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        });
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "old generation").build())
        .await?
        .filter(attempt_events::is_content);
    assert!(matches!(
        stream.next().await.ok_or("response absent")??,
        StreamedAssistantContent::Text(_)
    ));
    // @step When a newer durable generation is installed before the response completes
    let next = session.prepare_generation(2, "shared-model")?;
    session.install_committed_generation(2, &next)?;
    let authoritative = history(&session)?.snapshot()?;
    // @step Then the old stream fails with typed lineage error without publishing completion
    let mut failures = 0;
    while let Some(event) = stream.next().await {
        match event {
            Err(CompletionError::RequestError(error))
                if matches!(
                    error.downcast_ref::<CheckpointError>(),
                    Some(CheckpointError::Lineage(_))
                ) =>
            {
                failures += 1
            }
            Err(error) => return Err(error.into()),
            Ok(StreamedAssistantContent::Final(_)) => {
                return Err("stale response published completion".into())
            }
            _ => {}
        }
    }
    assert_eq!(failures, 1);
    // @step And its response cannot overwrite the new generation checkpoint
    assert_eq!(history(&session)?.snapshot()?, authoritative);
    assert_eq!(server.requests.lock().map_err(|_| "capture lock")?.len(), 1);
    Ok(())
}
