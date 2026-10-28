//! Feature: spec/features/codex-invalid-checkpoint-state.feature

use super::*;
use crate::codex::{refreshing_client::RefreshingCodexClient, CodexResponsesModel};
use crate::session::ProviderSession;
use codelet_test_helpers::setup_temp_directory;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use serde_json::json;
use std::{fs, panic::resume_unwind, thread};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn poison<T: Send + 'static>(lock: Arc<Mutex<T>>) {
    let outcome = thread::spawn(move || {
        assert!(
            !lock.is_poisoned(),
            "fixture lock must initially be accessible"
        );
        let _guard = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        resume_unwind(Box::new("scripted lock owner termination"));
    })
    .join();
    assert!(outcome.is_err());
}

#[tokio::test]
async fn isolated_inaccessible_checkpoint_locks_fail_without_substituting_fresh_history(
) -> TestResult {
    for lock in ["state", "writer"] {
        // @step Given an existing durable Codex checkpoint with only its <lock> lock inaccessible
        let directory = setup_temp_directory()?;
        let path = directory.path().join("checkpoint.json");
        let session = ProviderSession::for_provider("codex");
        let history = &session
            .extension::<crate::codex::session::CodexSession>("codex")
            .ok_or("Codex extension missing")?
            .history;
        let initial = json!({"model":"shared-model","input":[{"role":"user","content":[{"type":"input_text","text":"inspect"}]}]});
        let destination = path.clone();
        session.bind_writers(|_| {
            let destination = destination.clone();
            Arc::new(move |metadata| {
                fs::write(&destination, serde_json::to_vec(&metadata)?).map_err(|error| {
                    crate::session::ExtensionError::Persistence(error.to_string())
                })?;
                Ok(())
            })
        })?;
        history.commit(&initial, &[])?;
        let previous = history.snapshot()?;
        let durable = fs::read(&path)?;
        if lock == "state" {
            poison(history.state.clone());
        } else {
            poison(history.writer.clone());
        }
        // @step When I restore or prepare its continuation
        let mut request = initial.clone();
        let result = history.prepare(&mut request);
        // @step Then the inaccessible lock returns its specific typed checkpoint failure
        assert!(
            matches!(result, Err(CompletionError::RequestError(ref error))
            if matches!(error.downcast_ref::<CheckpointError>(), Some(CheckpointError::Inaccessible(name)) if *name == lock)),
            "{lock} returned {result:?}"
        );
        if lock == "state" {
            drop(
                history
                    .writer
                    .lock()
                    .map_err(|_| "writer lock was also poisoned")?,
            );
        } else {
            assert_eq!(history.snapshot()?, previous);
        }
        // @step And no provider request or completion is emitted
        assert_eq!(request, initial);
        let server = wiremock::MockServer::start().await;
        let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .http_client(RefreshingCodexClient::new_api_key())
            .build()?;
        let model = CodexResponsesModel::new(client, "shared-model").with_history(history.clone());
        let result = model
            .stream(CompletionRequestBuilder::new(model.clone(), "inspect").build())
            .await;
        assert!(result.is_err(), "{lock} reached physical inference");
        assert!(server
            .received_requests()
            .await
            .ok_or("request capture unavailable")?
            .is_empty());
        // @step And a cold restore retains the last durable checkpoint and generation
        assert_eq!(fs::read(&path)?, durable);
        let cold = ProviderSession::for_provider("codex");
        cold.restore(&[fs::read_to_string(path)?])?;
        assert_eq!(
            cold.extension::<crate::codex::session::CodexSession>("codex")
                .ok_or("cold Codex missing")?
                .history
                .snapshot()?,
            previous
        );
    }
    Ok(())
}
