//! Feature: spec/features/codex-idempotent-boundary-installation.feature

use codelet_providers::{
    codex::history::CheckpointError,
    request_admission::AdmissionLimits,
    session::{ExtensionError, ProviderSession},
};
use futures::StreamExt;
use rig::{
    completion::{CompletionModel, CompletionRequestBuilder},
    message::Message,
};
use serde_json::{json, Value};

#[path = "fixtures/prov148.rs"]
mod fixture;
#[path = "cmpct078/support.rs"]
mod support;
use support::{boundary, history, request};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn reinstalling_the_committed_boundary_preserves_a_completed_continuation() -> TestResult {
    // @step Given a real Codex response has extended the committed generation checkpoint
    let session = ProviderSession::for_provider("codex");
    let boundary = boundary()?;
    let metadata = [(
        "codexContinuation".into(),
        boundary["codexContinuation"].clone(),
    )]
    .into_iter()
    .collect();
    session.install_committed_generation(1, &metadata)?;
    let completed = fixture::event(json!({"type":"response.completed","response":{
        "id":"resp_continuation","status":"completed","end_turn":true,
        "usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20},"output":[
            {"type":"reasoning","summary":[],"encrypted_content":"opaque-continuation"},
            {"type":"message","role":"assistant","content":[{"type":"output_text","text":"continued answer"}]}
        ]
    }}));
    let server = fixture::Server::start(
        vec![completed, fixture::terminal("completed", None, None)],
        false,
    )
    .await?;
    let model = server
        .models()?
        .0
        .with_history(history(&session)?.clone())
        .with_request_admission(AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        });
    let dag = "<system-reminder><!-- type:compaction-dag -->committed DAG</system-reminder>";
    let mut stream = model
        .stream(
            CompletionRequestBuilder::new(model.clone(), "first continuation")
                .messages(vec![Message::user(dag)])
                .build(),
        )
        .await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    let checkpoint = history(&session)?.snapshot()?;
    let envelopes = [
        boundary.to_string(),
        json!({"codexContinuation":checkpoint}).to_string(),
    ];
    // @step When I reinstall the durable boundary and restore the same replay twice
    for _attempt in 0..2 {
        session.install_committed_generation(1, &metadata)?;
        // @step Then the complete continuation checkpoint remains unchanged after each restore
        assert_eq!(history(&session)?.snapshot()?, checkpoint);
        session.restore(&envelopes)?;
        assert_eq!(history(&session)?.snapshot()?, checkpoint);
    }
    let mut stream = model
        .stream(
            CompletionRequestBuilder::new(model.clone(), "next continuation")
                .messages(vec![
                    Message::user(dag),
                    Message::user("first continuation"),
                    Message::assistant("continued answer"),
                ])
                .build(),
        )
        .await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    // @step And the next physical request contains the committed DAG and complete continuation exactly once
    let captured = server.requests.lock().map_err(|_| "capture poisoned")?;
    assert_eq!(captured.len(), 2);
    let next = captured.get(1).ok_or("next request absent")?.to_string();
    for item in [
        "committed DAG",
        "first continuation",
        "continued answer",
        "opaque-continuation",
        "next continuation",
    ] {
        assert_eq!(next.matches(item).count(), 1, "duplicate or missing {item}");
    }
    assert_eq!(history(&session)?.generation()?, 1);
    Ok(())
}

#[tokio::test]
async fn replayed_metadata_cannot_replace_the_installed_same_generation_model() -> TestResult {
    // @step Given a completed continuation belongs to the installed committed model
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
    let previous = history(&session)?.snapshot()?;
    let authentic = json!({"codexContinuation":previous}).to_string();
    let mut replaced = serde_json::to_value(&previous)?;
    replaced["model"] = json!("other-model");
    // @step When retained same-generation replay metadata names a different model without a boundary envelope
    let result = session.restore(&[json!({"codexContinuation":replaced}).to_string()]);
    // @step Then restoration fails with typed lineage error before changing the checkpoint
    assert!(matches!(
        result,
        Err(ExtensionError::Checkpoint(CheckpointError::Lineage(_)))
    ));
    assert_eq!(history(&session)?.snapshot()?, previous);
    // @step And physical inference remains blocked until the authentic replay is restored
    let server = fixture::Server::start(vec![], false).await?;
    let model = server.models()?.0.with_history(history(&session)?.clone());
    let result = model
        .stream(CompletionRequestBuilder::new(model.clone(), "next").build())
        .await;
    assert!(matches!(
        result.err(),
        Some(rig::completion::CompletionError::RequestError(error))
        if matches!(error.downcast_ref::<CheckpointError>(), Some(CheckpointError::Blocked))
    ));
    assert!(server
        .requests
        .lock()
        .map_err(|_| "request lock")?
        .is_empty());
    session.restore(&[authentic])?;
    assert_eq!(history(&session)?.snapshot()?, previous);
    history(&session)?.prepare(&mut request("committed DAG"))?;
    Ok(())
}

#[test]
fn reinstalling_a_different_model_cannot_replace_same_generation_continuation() -> TestResult {
    // @step Given a completed continuation belongs to the committed Codex model
    let session = ProviderSession::for_provider("codex");
    let boundary = boundary()?;
    let mut metadata: std::collections::HashMap<String, Value> = [(
        "codexContinuation".into(),
        boundary["codexContinuation"].clone(),
    )]
    .into_iter()
    .collect();
    session.install_committed_generation(1, &metadata)?;
    history(&session)?.commit(&request("committed DAG"), &[])?;
    let checkpoint = history(&session)?.snapshot()?;
    // @step When I reinstall a same-generation boundary for a different model
    metadata
        .get_mut("codexContinuation")
        .ok_or("metadata missing")?["model"] = json!("other-model");
    let result = session.install_committed_generation(1, &metadata);
    // @step Then I receive a typed checkpoint failure with the prior committed state unchanged
    assert!(matches!(result, Err(ExtensionError::Checkpoint(_))));
    assert_eq!(history(&session)?.snapshot()?, checkpoint);
    Ok(())
}
