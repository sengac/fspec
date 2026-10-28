//! Feature: spec/features/codex-sole-dag-memory-policy.feature
#[path = "fixtures/prov147.rs"]
pub mod baseline;
#[path = "fixtures/cmpct077.rs"]
pub mod fixture;

use fixture::{
    consume, consume_request, model, terminal, unsupported, PersistedHistory, TestResult,
};
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use serde_json::{json, Value};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

fn streamed(events: Vec<Value>) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(
            events
                .iter()
                .map(|event| format!("data: {event}\n\n"))
                .collect::<String>(),
        )
}

#[tokio::test]
async fn scenario_nonterminal_native_memory_snapshots_cannot_authorize_local_tools() -> TestResult {
    // @step Given a pending local tool followed by a nonterminal response containing native memory
    for family in [
        "compaction",
        "compaction_summary",
        "context_compaction",
        "compaction_trigger",
    ] {
        for kind in [
            "response.created",
            "response.in_progress",
            "response.queued",
        ] {
            let server = MockServer::start().await;
            let persisted = PersistedHistory::new()?;
            let history = persisted.history.clone();
            history.commit(&json!({"model":"shared-model","input":[]}), &[])?;
            let prior = history.snapshot()?;
            let bytes = persisted.bytes()?;
            let tool = json!({"type":"function_call","id":"fc_1","call_id":"call_1",
                "name":"read_file","arguments":"{}","status":"completed"});
            let native = json!({"type":family,"encrypted_content":"private-native"});
            let snapshot = json!({"type":kind,"response":{"id":"resp_1","status":"in_progress","output":[native]}});
            Mock::given(method("POST"))
                .respond_with(streamed(vec![
                    json!({"type":"response.output_item.done","output_index":0,"item":tool}),
                    snapshot,
                    json!({"type":"response.completed","response":terminal(vec![tool])}),
                ]))
                .expect(1)
                .mount(&server)
                .await;
            // @step When the stream later reports successful completion without native items
            let error = consume(&model(&server, history.clone())?, true).await?;
            // @step Then a typed unsupported error preserves the checkpoint without dispatch or completion
            unsupported(&error);
            assert_eq!(history.snapshot()?, prior, "{kind} {family}");
            assert_eq!(persisted.bytes()?, bytes, "{kind} {family}");
            server.verify().await;
        }
    }
    Ok(())
}

#[tokio::test]
async fn scenario_malformed_native_memory_terminals_still_return_typed_policy_errors() -> TestResult
{
    // @step Given a response containing native memory with invalid terminal lifecycle fields
    for invalid in ["id", "status", "usage", "error"] {
        for route in ["direct", "stream", "stream-completion"] {
            let server = MockServer::start().await;
            let persisted = PersistedHistory::new()?;
            let history = persisted.history.clone();
            history.commit(&json!({"model":"shared-model","input":[]}), &[])?;
            let prior = history.snapshot()?;
            let bytes = persisted.bytes()?;
            let mut raw = terminal(vec![
                json!({"type":"compaction","encrypted_content":"private-native"}),
            ]);
            let kind = if invalid == "error" {
                "response.failed"
            } else {
                "response.completed"
            };
            match invalid {
                "error" => raw["status"] = json!("failed"),
                "usage" => raw["usage"] = json!({"input_tokens":"invalid"}),
                key => {
                    raw.as_object_mut().ok_or("terminal object")?.remove(key);
                }
            }
            let template = if route == "direct" {
                ResponseTemplate::new(200).set_body_json(raw)
            } else {
                streamed(vec![json!({"type":kind,"response":raw})])
            };
            Mock::given(method("POST"))
                .respond_with(template)
                .expect(1)
                .mount(&server)
                .await;
            let model = model(&server, history.clone())?;
            let model = if route == "stream-completion" {
                model.with_streaming_completions()
            } else {
                model
            };
            // @step When streaming and direct completion responses are consumed
            let error = consume(&model, route == "stream").await?;
            // @step Then native memory is rejected with a typed unsupported error before lifecycle parsing
            unsupported(&error);
            assert_eq!(history.snapshot()?, prior, "{route} {invalid}");
            assert_eq!(persisted.bytes()?, bytes, "{route} {invalid}");
            server.verify().await;
        }
    }
    Ok(())
}

#[tokio::test]
async fn scenario_rejected_replacement_requests_retain_the_committed_checkpoint() -> TestResult {
    // @step Given a committed checkpoint and a request replacing its model or conversation prefix
    for replacement in ["model", "prefix", "dag"] {
        for route in ["direct", "stream", "stream-completion"] {
            let server = MockServer::start().await;
            let persisted = PersistedHistory::new()?;
            let history = persisted.history.clone();
            let prior_model = if replacement == "model" {
                "old-model"
            } else {
                "shared-model"
            };
            let prior_input = if replacement == "dag" {
                json!([])
            } else {
                json!([{"role":"user","content":"prior"}])
            };
            history.commit(
                &json!({"model":prior_model,"input":prior_input}),
                baseline::response()["output"].as_array().ok_or("output")?,
            )?;
            let prior = history.snapshot()?;
            let bytes = persisted.bytes()?;
            let raw = terminal(vec![
                json!({"type":"compaction","encrypted_content":"private-native"}),
            ]);
            let template = if route == "direct" {
                ResponseTemplate::new(200).set_body_json(raw)
            } else {
                streamed(vec![json!({"type":"response.completed","response":raw})])
            };
            Mock::given(method("POST"))
                .respond_with(template)
                .expect(1)
                .mount(&server)
                .await;
            let model = model(&server, history.clone())?;
            let model = if route == "stream-completion" {
                model.with_streaming_completions()
            } else {
                model
            };
            let prompt = if replacement == "dag" {
                "<system-reminder><!-- type:compaction-dag -->committed summary</system-reminder>"
            } else {
                "replacement"
            };
            let request = CompletionRequestBuilder::new(model.clone(), prompt).build();
            let error = consume_request(&model, route == "stream", request).await?;
            assert!(
                matches!(error, rig::completion::CompletionError::RequestError(source)
                if matches!(source.downcast_ref::<codelet_providers::codex::history::CheckpointError>(),
                    Some(codelet_providers::codex::history::CheckpointError::Lineage(_))))
            );
            assert_eq!(history.snapshot()?, prior);
            assert_eq!(persisted.bytes()?, bytes);
            assert!(server
                .received_requests()
                .await
                .ok_or("requests")?
                .is_empty());
            persisted.transition()?;
            let transitioned = history.snapshot()?;
            let transitioned_bytes = persisted.bytes()?;
            let request = CompletionRequestBuilder::new(model.clone(), prompt).build();
            // @step When the replacement response contains unsupported native memory
            let error = consume_request(&model, route == "stream", request).await?;
            unsupported(&error);
            // @step Then the original in-memory and durable checkpoint survive on every completion route
            assert_eq!(history.snapshot()?, transitioned, "{route} {replacement}");
            assert_eq!(
                persisted.bytes()?,
                transitioned_bytes,
                "{route} {replacement}"
            );
            server.verify().await;
            // @step And a subsequent supported response commits the replacement conversation
            server.reset().await;
            let template = if route == "direct" {
                ResponseTemplate::new(200).set_body_json(baseline::response())
            } else {
                streamed(vec![
                    json!({"type":"response.completed","response":baseline::response()}),
                ])
            };
            Mock::given(method("POST"))
                .respond_with(template)
                .expect(1)
                .mount(&server)
                .await;
            let request = CompletionRequestBuilder::new(model.clone(), prompt).build();
            if route == "stream" {
                let mut stream = model.stream(request).await?;
                while let Some(event) = stream.next().await {
                    event?;
                }
            } else {
                model.completion(request).await?;
            }
            let checkpoint = history.snapshot()?.ok_or("replacement checkpoint")?;
            assert_eq!(checkpoint.model.as_deref(), Some("shared-model"));
            assert_ne!(Some(checkpoint), transitioned, "{route} {replacement}");
            assert_ne!(
                persisted.bytes()?,
                transitioned_bytes,
                "{route} {replacement}"
            );
            let requests = server.received_requests().await.ok_or("requests")?;
            let wire: Value = serde_json::from_slice(&requests[0].body)?;
            assert!(!wire["input"].to_string().contains("prior"));
            server.verify().await;
        }
    }
    Ok(())
}
