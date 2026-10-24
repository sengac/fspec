use codelet_core::{CompactionHook, RigAgent, TokenState};
use codelet_providers::{
    codex::history::CodexHistory,
    request_admission::{AdmissionLimits, RequestAdmissionError},
};
use futures::StreamExt;
use rig::{
    agent::AgentBuilder,
    completion::{CompletionError, CompletionModel, CompletionRequestBuilder},
    streaming::StreamedAssistantContent,
};
use serde_json::json;
use std::sync::{Arc, Mutex};

#[path = "fixtures/prov148.rs"]
mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn invalid_explicit_admission_limits_block_physical_inference() -> TestResult {
    for (capacity, reservation) in [
        (None, Some(10)),
        (Some(1000), None),
        (Some(0), Some(10)),
        (Some(1000), Some(0)),
        (Some(1000), Some(1000)),
        (Some(1000), Some(1001)),
    ] {
        // @step Given a Codex session with <limits> and an unchanged committed checkpoint
        let history = CodexHistory::default();
        history.commit(&json!({"model":"shared-model","input":[]}), &[])?;
        let previous = history.snapshot()?;
        let server =
            fixture::Server::start(vec![fixture::terminal("completed", None, None)], false).await?;
        let model = server
            .models()?
            .0
            .with_history(history.clone())
            .with_request_admission(AdmissionLimits {
                context_capacity: capacity,
                output_reservation: reservation,
            });
        // @step When I request a provider completion
        let mut stream = model
            .stream(CompletionRequestBuilder::new(model.clone(), "inspect").build())
            .await?;
        let mut failures = 0;
        while let Some(event) = stream.next().await {
            match event {
                Err(CompletionError::RequestError(error)) => {
                    // @step Then a typed admission failure is returned before a physical send
                    assert!(error.downcast_ref::<RequestAdmissionError>().is_some());
                    failures += 1;
                }
                Ok(StreamedAssistantContent::Final(_)) => {
                    return Err("invalid admission emitted completion".into())
                }
                Err(error) => return Err(error.into()),
                _ => {}
            }
        }
        // @step And no completion is emitted and the committed checkpoint remains unchanged
        assert_eq!(failures, 1);
        assert!(server
            .requests
            .lock()
            .map_err(|_| "capture lock poisoned")?
            .is_empty());
        assert_eq!(history.snapshot()?, previous);
    }
    Ok(())
}

#[tokio::test]
async fn inaccessible_admission_state_blocks_the_streaming_agent() -> TestResult {
    // @step Given an agent whose admission state lock is inaccessible
    let server =
        fixture::Server::start(vec![fixture::terminal("completed", None, None)], false).await?;
    let model = server.models()?.0.with_request_admission(AdmissionLimits {
        context_capacity: Some(10000),
        output_reservation: Some(100),
    });
    let state = Arc::new(Mutex::new(TokenState::default()));
    let inaccessible = state.clone();
    let stopped = std::thread::spawn(move || -> Result<(), &'static str> {
        let _guard = inaccessible
            .lock()
            .map_err(|_| "admission lock initially inaccessible")?;
        std::panic::resume_unwind(Box::new("scripted admission owner termination"));
    })
    .join();
    assert!(stopped.is_err());
    let hook = CompactionHook::new(state, 9000);
    let agent = RigAgent::with_default_depth(AgentBuilder::new(model).build());
    let mut history = vec![];
    // @step When its streaming completion hook checks a real provider request
    let stream = agent
        .prompt_streaming_with_history_and_hook("inspect", &mut history, hook)
        .await;
    futures::pin_mut!(stream);
    let mut failed = false;
    while let Some(event) = stream.next().await {
        match event {
            Err(_) => failed = true,
            Ok(rig::agent::MultiTurnStreamItem::FinalResponse(_)) => {
                return Err("unavailable guard emitted completion".into())
            }
            _ => {}
        }
    }
    // @step Then inference is cancelled before any physical request
    assert!(failed, "unavailable guard must explicitly cancel inference");
    // @step And no successful completion or substitute compaction is emitted
    assert!(server
        .requests
        .lock()
        .map_err(|_| "capture lock poisoned")?
        .is_empty());
    Ok(())
}
