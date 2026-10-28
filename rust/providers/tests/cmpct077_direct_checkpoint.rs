//! Feature: spec/features/codex-sole-dag-memory-policy.feature
#[path = "fixtures/prov147.rs"]
pub mod baseline;
#[path = "fixtures/cmpct077.rs"]
pub mod fixture;

use fixture::{model, PersistedHistory, TestResult};
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use serde_json::json;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn scenario_malformed_direct_completions_preserve_the_committed_checkpoint() -> TestResult {
    malformed_completion(false).await
}

#[tokio::test]
async fn scenario_malformed_streamed_completions_preserve_the_committed_checkpoint() -> TestResult {
    malformed_completion(true).await
}

fn response(raw: serde_json::Value, streaming: bool) -> ResponseTemplate {
    if streaming {
        ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string(format!(
                "data: {}\n\n",
                json!({"type":"response.completed","response":raw})
            ))
    } else {
        ResponseTemplate::new(200).set_body_json(raw)
    }
}

async fn malformed_completion(streaming: bool) -> TestResult {
    // @step Given a committed checkpoint and a replacement response with malformed completion data
    // @step Given a committed checkpoint and a streamed replacement response with malformed completion data
    for invalid in [
        "usage",
        "id",
        "empty-output",
        "object",
        "model",
        "created_at",
    ] {
        let server = MockServer::start().await;
        let persisted = PersistedHistory::new()?;
        let history = persisted.history.clone();
        history.commit(&json!({"model":"shared-model","input":[]}), &[])?;
        let prior = history.snapshot()?;
        let bytes = persisted.bytes()?;
        let model = model(&server, history.clone())?;
        let model = if streaming {
            model.with_streaming_completions()
        } else {
            model
        };
        let mut raw = baseline::response();
        match invalid {
            "usage" => raw["usage"]["input_tokens"] = json!("invalid"),
            "empty-output" => raw["output"] = json!([]),
            key => {
                raw.as_object_mut().ok_or("response object")?.remove(key);
            }
        }
        Mock::given(method("POST"))
            .respond_with(response(raw, streaming))
            .expect(1)
            .mount(&server)
            .await;
        let request = CompletionRequestBuilder::new(model.clone(), "replacement").build();
        // @step When a direct completion cannot be decoded or converted to an assistant response
        // @step When a streamed completion cannot be decoded or converted to an assistant response
        assert!(model.completion(request).await.is_err(), "{invalid}");
        // @step Then the in-memory and durable checkpoint remain unchanged without replay
        assert_eq!(history.snapshot()?, prior, "{invalid}");
        assert_eq!(persisted.bytes()?, bytes, "{invalid}");
        server.verify().await;
        // @step And a subsequent valid completion commits the replacement conversation
        // @step And a subsequent valid streamed completion commits the replacement conversation
        server.reset().await;
        Mock::given(method("POST"))
            .respond_with(response(baseline::response(), streaming))
            .expect(1)
            .mount(&server)
            .await;
        let request = CompletionRequestBuilder::new(model.clone(), "replacement").build();
        model.completion(request).await?;
        assert_ne!(history.snapshot()?, prior, "{invalid}");
        assert_ne!(persisted.bytes()?, bytes, "{invalid}");
        server.verify().await;
    }
    Ok(())
}
