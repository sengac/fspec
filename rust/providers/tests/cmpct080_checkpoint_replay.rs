//! Feature: spec/features/codex-checkpoint-replay-integrity.feature

use codelet_providers::{codex::history::CodexHistory, session::ProviderSession};
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};
use std::{fs, sync::Arc};

#[path = "fixtures/prov148.rs"]
mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn initial() -> Value {
    json!({"model":"shared-model","input":[{"role":"user","content":[{"type":"input_text","text":"inspect"}]}]})
}

#[tokio::test]
async fn a_genuinely_new_session_and_recorded_tool_outcomes_remain_supported() -> TestResult {
    use futures::StreamExt;
    use rig::{
        completion::{CompletionModel, CompletionRequestBuilder},
        message::{Message, ToolResultContent, UserContent},
        OneOrMany,
    };
    // @step Given a genuinely new Codex session without a checkpoint
    let history = CodexHistory::default();
    assert!(history.snapshot()?.is_none());
    let calls = json!([
        {"type":"function_call","id":"fc_1","call_id":"call_1","name":"inspect","arguments":"{}"},
        {"type":"function_call","id":"fc_2","call_id":"call_2","name":"inspect","arguments":"{}"}
    ]);
    let first = fixture::event(json!({"type":"response.completed","response":{
        "id":"resp_1","status":"completed","output":calls,"end_turn":true,
        "usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}
    }}));
    let server = fixture::Server::start(
        vec![first, fixture::terminal("completed", None, None)],
        false,
    )
    .await?;
    let model = server
        .models()?
        .0
        .with_history(history)
        .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        });
    // @step When I send its first request and continue with real recorded tool outcomes
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "inspect").build())
        .await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    let outcomes = Message::User {
        content: OneOrMany::many([
            UserContent::tool_result_with_call_id(
                "fc_1",
                "call_1".into(),
                OneOrMany::one(ToolResultContent::text("recorded result")),
            ),
            UserContent::tool_result_with_call_id(
                "fc_2",
                "call_2".into(),
                OneOrMany::one(ToolResultContent::text("recorded cancellation cancel-42")),
            ),
        ])?,
    };
    let request = CompletionRequestBuilder::new(model.clone(), outcomes)
        .messages(vec![Message::user("inspect")])
        .build();
    let mut stream = model.stream(request).await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    // @step Then the captured continuation contains each real result or recorded cancellation exactly once
    let requests = server
        .requests
        .lock()
        .map_err(|_| "capture lock poisoned")?;
    let continuation = requests.get(1).ok_or("continuation missing")?.to_string();
    assert_eq!(continuation.matches("recorded result").count(), 1);
    assert_eq!(
        continuation
            .matches("recorded cancellation cancel-42")
            .count(),
        1
    );
    // @step And no replacement outcome is fabricated
    assert!(!continuation.contains("Tool execution cancelled by interruption."));
    Ok(())
}

#[test]
fn checkpoint_commit_persistence_failure_preserves_prior_memory_and_disk() -> TestResult {
    // @step Given a Codex session with a checkpoint persisted in a real temporary store
    let directory = setup_temp_directory()?;
    let path = directory.path().join("checkpoint.json");
    let staging = directory.path().join("checkpoint.pending");
    let history = CodexHistory::default();
    let destination = path.clone();
    let pending = staging.clone();
    history.set_writer(Arc::new(move |checkpoint| {
        let bytes = serde_json::to_vec(&checkpoint).map_err(|error| error.to_string())?;
        fs::write(&pending, bytes).map_err(|error| error.to_string())?;
        fs::rename(&pending, &destination).map_err(|error| error.to_string())
    }))?;
    history.commit(&initial(), &[])?;
    let previous = history.snapshot()?;
    // @step And the next checkpoint save fails before durable promotion
    fs::create_dir(&staging)?;
    // @step When I commit a completed provider response
    let result = history.commit(
        &initial(),
        &[json!({"type":"message","role":"assistant","content":"uncommitted"})],
    );
    // @step Then I receive an explicit checkpoint persistence failure
    assert!(result.is_err());
    // @step And both the live checkpoint and a cold reload equal the previous durable checkpoint
    assert_eq!(
        history.snapshot()?,
        previous,
        "failed persistence changed live state"
    );
    let restored = CodexHistory::default();
    restored.restore(serde_json::from_slice(&fs::read(path)?)?)?;
    assert_eq!(restored.snapshot()?, previous);
    // @step And ordinary inference remains blocked until explicit resolution
    assert!(history.prepare(&mut initial()).is_err());
    Ok(())
}

#[test]
fn failed_reset_cannot_destroy_the_authoritative_checkpoint() -> TestResult {
    // @step Given a Codex session with a durable checkpoint and committed generation
    let directory = setup_temp_directory()?;
    let path = directory.path().join("checkpoint.json");
    let session = ProviderSession::for_provider("codex");
    let history = &session
        .extension::<codelet_providers::codex::session::CodexSession>("codex")
        .ok_or("Codex extension missing")?
        .history;
    history.commit(&initial(), &[])?;
    let previous = history.snapshot()?;
    fs::write(&path, serde_json::to_vec(&previous)?)?;
    // @step And its recorded generation reset cannot be persisted
    session.bind_writers(|_| {
        Arc::new(|_| {
            Err(codelet_providers::session::ExtensionError::Persistence(
                "save rejected before promotion".into(),
            ))
        })
    })?;
    // @step When I request the generation reset through the public session API
    let result = session.reset();
    assert!(result.is_err());
    // @step Then the public API returns an explicit persistence failure
    // @step And live and cold-resumed checkpoint state remain unchanged
    assert_eq!(
        history.snapshot()?,
        previous,
        "failed reset destroyed authoritative state"
    );
    assert_eq!(
        serde_json::from_slice::<Option<codelet_providers::codex::history::CodexCheckpoint>>(
            &fs::read(path)?
        )?,
        previous
    );
    // @step And no automatic fresh-session request is sent
    assert!(history.prepare(&mut initial()).is_err());
    Ok(())
}

#[tokio::test]
async fn replay_refuses_a_tool_call_with_no_recorded_outcome() -> TestResult {
    use futures::StreamExt;
    use rig::completion::{CompletionModel, CompletionRequestBuilder};
    for call in [
        json!({"type":"function_call","id":"fc_1","call_id":"call_1","name":"inspect","arguments":"{}"}),
        json!({"type":"custom_tool_call","id":"fc_1","call_id":"call_1","name":"inspect","input":"{}"}),
    ] {
        // @step Given a durable Codex checkpoint containing an unresolved <call> call
        let directory = setup_temp_directory()?;
        let path = directory.path().join("checkpoint.json");
        let history = CodexHistory::default();
        history.commit(&initial(), &[call])?;
        fs::write(&path, serde_json::to_vec(&history.snapshot()?)?)?;
        let previous = fs::read(&path)?;
        // @step And no real tool result or cancellation outcome was recorded
        let server =
            fixture::Server::start(vec![fixture::terminal("completed", None, None)], false).await?;
        let model = server.models()?.0.with_history(history.clone());
        // @step When I continue the session through the provider adapter
        let result = model
            .stream(CompletionRequestBuilder::new(model.clone(), "inspect").build())
            .await;
        // @step Then replay returns an explicit missing-outcome failure before sending a request
        let error = result.err().ok_or("unresolved tool call was accepted")?;
        match error {
            rig::completion::CompletionError::RequestError(error) => {
                assert!(matches!(
                    error.downcast_ref::<codelet_providers::codex::history::CheckpointError>(),
                    Some(codelet_providers::codex::history::CheckpointError::MissingOutcome(call_id))
                        if call_id == "call_1"
                ));
            }
            error => return Err(error.into()),
        }
        let requests = server
            .requests
            .lock()
            .map_err(|_| "capture lock poisoned")?;
        assert!(
            requests.is_empty(),
            "unresolved tool call was sent: {requests:?}"
        );
        // @step And no invented cancellation text is added to the request or checkpoint
        // @step And the durable checkpoint remains unchanged
        assert_eq!(fs::read(path)?, previous);
    }
    Ok(())
}
