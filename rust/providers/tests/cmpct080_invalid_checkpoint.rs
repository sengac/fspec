//! Feature: spec/features/codex-invalid-checkpoint-state.feature

use codelet_providers::{
    codex::{refreshing_client::RefreshingCodexClient, session::CodexSession, CodexResponsesModel},
    session::ProviderSession,
};
use codelet_test_helpers::setup_temp_directory;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use serde_json::json;
use std::{fs, panic::resume_unwind, sync::Arc, thread};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn invalid_checkpoint_state_is_not_treated_as_an_absent_checkpoint() -> TestResult {
    for failure in [
        "unsupported version",
        "terminated persistence",
        "missing checkpoint",
        "malformed",
        "protocol",
    ] {
        // @step Given an existing Codex session with <failure>
        let directory = setup_temp_directory()?;
        let path = directory.path().join("checkpoint.json");
        let session = ProviderSession::for_provider("codex");
        let history = &session
            .extension::<CodexSession>("codex")
            .ok_or("Codex extension missing")?
            .history;
        let initial = json!({"model":"shared-model","input":[{
            "role":"user","content":[{"type":"input_text","text":"inspect"}]
        }]});
        let destination = path.clone();
        session.bind_writers(|_| {
            let destination = destination.clone();
            Arc::new(move |metadata| {
                fs::write(&destination, serde_json::to_vec(&metadata)?).map_err(|error| {
                    codelet_providers::session::ExtensionError::Persistence(error.to_string())
                })?;
                Ok(())
            })
        })?;
        history.commit(&initial, &[])?;
        let previous = history.snapshot()?;
        let durable = fs::read(&path)?;
        if failure == "terminated persistence" {
            history.set_writer(Arc::new(|_| {
                resume_unwind(Box::new("scripted persistence writer termination"))
            }))?;
            let interrupted = history.clone();
            let request = initial.clone();
            assert!(thread::spawn(move || interrupted.commit(&request, &[]))
                .join()
                .is_err());
        }
        // @step When I restore or prepare its continuation
        let result = match failure {
            "unsupported version" => session.restore(&[json!({"codexContinuation":{
                "version":999,"items":[],"model":"shared-model"
            }})
            .to_string()]),
            "missing checkpoint" => session.restore(&[json!({
                "message":{"role":"user","content":"inspect"}
            })
            .to_string()]),
            "malformed" => session.restore(&["{invalid".into()]),
            "protocol" => {
                let mut checkpoint = serde_json::to_value(&previous)?;
                checkpoint["protocol"] = json!("removed-delta");
                session.restore(&[json!({"codexContinuation":checkpoint}).to_string()])
            }
            _ => session.restore(&[]),
        };
        // @step Then I receive an explicit checkpoint failure rather than a fresh session
        assert!(result.is_err(), "{failure} was treated as absent");
        // @step And no provider request or completion is emitted
        let mut request = initial.clone();
        assert!(
            history.prepare(&mut request).is_err(),
            "{failure} allowed inference"
        );
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
        assert!(result.is_err(), "{failure} reached physical inference");
        assert!(server
            .received_requests()
            .await
            .ok_or("capture unavailable")?
            .is_empty());
        // @step And the last durable checkpoint and committed generation remain authoritative
        assert_eq!(fs::read(&path)?, durable);
        let cold = ProviderSession::for_provider("codex");
        cold.restore(&[fs::read_to_string(&path)?])?;
        assert_eq!(
            cold.extension::<CodexSession>("codex")
                .ok_or("cold Codex missing")?
                .history
                .snapshot()?,
            previous
        );
        if failure != "terminated persistence" {
            assert_eq!(history.snapshot()?, previous);
        }
    }
    Ok(())
}
