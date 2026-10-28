//! Feature: spec/features/codex-committed-checkpoint-precedence.feature

use codelet_providers::{
    codex::history::CheckpointError,
    request_admission::AdmissionLimits,
    session::{ExtensionError, ProviderSession},
};
use codelet_test_helpers::setup_temp_directory;
use futures::StreamExt;
use rig::{
    completion::{CompletionModel, CompletionRequestBuilder},
    message::Message,
};
use serde_json::{json, Value};
use std::fs;

#[path = "fixtures/prov148.rs"]
pub mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "cmpct078/support.rs"]
pub mod support;
use support::{boundary, history, request};

#[tokio::test]
async fn retained_pre_promotion_checkpoints_cannot_roll_back_committed_replay() -> TestResult {
    // @step Given a durable committed Codex boundary followed by a genuine older checkpoint
    let directory = setup_temp_directory()?;
    let session = ProviderSession::for_provider("codex");
    history(&session)?.commit(&request("replaced request"), &[json!({
        "type":"message","role":"assistant","content":[{"type":"output_text","text":"replaced assistant"}]
    })])?;
    let older = history(&session)?.snapshot()?;
    let boundary = boundary()?;
    let metadata = [(
        "codexContinuation".into(),
        boundary["codexContinuation"].clone(),
    )]
    .into_iter()
    .collect();
    session.install_committed_generation(1, &metadata)?;
    let path = directory.path().join("ordered-replay.json");
    let envelopes = vec![
        boundary.to_string(),
        json!({"codexContinuation":older,"message":{"role":"assistant","content":[]}}).to_string(),
    ];
    fs::write(&path, serde_json::to_vec(&envelopes)?)?;
    let durable = fs::read(&path)?;
    // @step When I restore the ordered replay through the public provider session API
    session.restore(&serde_json::from_slice::<Vec<String>>(&durable)?)?;
    // @step Then the committed generation remains authoritative without modifying the replay
    assert_eq!(
        history(&session)?.generation()?,
        1,
        "retained checkpoint rolled back committed generation"
    );
    assert_eq!(fs::read(path)?, durable);
    // @step And the next physical request contains only the committed DAG and complete compatible suffix
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
    let dag = "<system-reminder><!-- type:compaction-dag -->committed DAG</system-reminder>";
    let mut stream = model
        .stream(
            CompletionRequestBuilder::new(model.clone(), "complete compatible suffix")
                .messages(vec![Message::user(dag)])
                .build(),
        )
        .await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    let captured = server.requests.lock().map_err(|_| "capture poisoned")?;
    let wire = captured
        .first()
        .ok_or("physical request absent")?
        .to_string();
    assert_eq!(wire.matches("committed DAG").count(), 1);
    assert_eq!(wire.matches("complete compatible suffix").count(), 1);
    assert!(!wire.contains("replaced"));
    Ok(())
}

#[tokio::test]
async fn incompatible_retained_checkpoint_metadata_fails_before_state_mutation() -> TestResult {
    for incompatibility in [
        "unsupported version",
        "unsupported protocol",
        "future generation",
        "same-generation model",
        "absent checkpoint",
    ] {
        // @step Given a committed Codex generation and retained checkpoint with <incompatibility>
        let session = ProviderSession::for_provider("codex");
        let boundary = boundary()?;
        let metadata = [(
            "codexContinuation".into(),
            boundary["codexContinuation"].clone(),
        )]
        .into_iter()
        .collect();
        session.install_committed_generation(1, &metadata)?;
        let prior = history(&session)?.snapshot()?;
        let mut checkpoint = boundary["codexContinuation"].clone();
        match incompatibility {
            "unsupported version" => checkpoint["version"] = json!(999),
            "unsupported protocol" => checkpoint["protocol"] = json!("removed-delta"),
            "future generation" => checkpoint["generation"] = json!(2),
            "same-generation model" => checkpoint["model"] = json!("other-model"),
            _ => checkpoint = Value::Null,
        }
        let envelopes = [
            boundary.to_string(),
            json!({"codexContinuation":checkpoint}).to_string(),
        ];
        // @step When I restore the ordered replay through the public provider session API
        let result = session.restore(&envelopes);
        // @step Then I receive a typed checkpoint failure with the prior committed state unchanged
        assert!(
            matches!(result, Err(ExtensionError::Checkpoint(_))),
            "{incompatibility} was accepted: {result:?}"
        );
        assert_eq!(history(&session)?.snapshot()?, prior);
        // @step And ordinary inference remains blocked without sending an alternate request
        let server =
            fixture::Server::start(vec![fixture::terminal("completed", None, None)], false).await?;
        let model = server.models()?.0.with_history(history(&session)?.clone());
        let result = model
            .stream(CompletionRequestBuilder::new(model.clone(), "next").build())
            .await;
        assert!(
            matches!(result.err(), Some(rig::completion::CompletionError::RequestError(error))
            if matches!(error.downcast_ref::<CheckpointError>(), Some(CheckpointError::Blocked)))
        );
        assert!(server
            .requests
            .lock()
            .map_err(|_| "capture poisoned")?
            .is_empty());
    }
    Ok(())
}

#[test]
fn same_generation_checkpoints_retain_genuine_post_promotion_continuation() -> TestResult {
    // @step Given a committed Codex boundary followed by its genuine same-generation checkpoint
    let session = ProviderSession::for_provider("codex");
    let boundary = boundary()?;
    let metadata = [(
        "codexContinuation".into(),
        boundary["codexContinuation"].clone(),
    )]
    .into_iter()
    .collect();
    session.install_committed_generation(1, &metadata)?;
    let reasoning =
        json!({"type":"reasoning","summary":[],"encrypted_content":"opaque-post-promotion"});
    let call = json!({"type":"function_call","call_id":"call-1","name":"inspect","arguments":"{}"});
    let outcome =
        json!({"type":"function_call_output","call_id":"call-1","output":"recorded result"});
    history(&session)?.commit(&request("committed DAG"), &[reasoning, call, outcome])?;
    let checkpoint = history(&session)?.snapshot()?;
    let envelopes = [
        boundary.to_string(),
        json!({"codexContinuation":checkpoint}).to_string(),
    ];
    let original = envelopes.clone();
    let restored = ProviderSession::for_provider("codex");
    // @step When I restore the ordered replay through the public provider session API
    restored.restore(&envelopes)?;
    // @step Then compatible opaque reasoning and recorded tool outcomes survive exactly once
    assert_eq!(history(&restored)?.snapshot()?, checkpoint);
    let mut next = request("committed DAG");
    next["input"]
        .as_array_mut()
        .ok_or("input missing")?
        .extend([
            json!({"type":"function_call","call_id":"call-1","name":"inspect","arguments":"{}"}),
            json!({"type":"function_call_output","call_id":"call-1","output":"recorded result"}),
        ]);
    history(&restored)?.prepare(&mut next)?;
    assert_eq!(next.to_string().matches("opaque-post-promotion").count(), 1);
    assert_eq!(next.to_string().matches("recorded result").count(), 1);
    // @step And the committed generation remains authoritative without modifying the replay
    assert_eq!(history(&restored)?.generation()?, 1);
    assert_eq!(envelopes, original);
    Ok(())
}
