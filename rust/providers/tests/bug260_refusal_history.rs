#![doc = "Feature: spec/features/codex-refusal-history-continuation.feature"]

#[path = "fixtures/prov148.rs"]
pub mod fixture;

use codelet_providers::codex::{history::CodexHistory, CodexResponsesModel};
use fixture::TestResult;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::message::{Message, ToolResultContent, UserContent};
use rig::streaming::StreamedAssistantContent;
use rig::OneOrMany;
use serde_json::{json, Value};

fn terminal(output: &[Value]) -> String {
    fixture::event(json!({"type":"response.completed","response":{
        "id":"response", "status":"completed", "end_turn":true, "output":output,
        "usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}
    }}))
}

fn output(mixed: bool) -> Vec<Value> {
    let mut content = vec![json!({"type":"refusal","refusal":"No"})];
    if mixed {
        content.insert(0, json!({"type":"output_text","text":"Before "}));
        content.push(json!({"type":"output_text","text":" after"}));
    }
    vec![
        json!({"type":"reasoning","id":"reason","summary":[],"encrypted_content":"synthetic+/=秘密"}),
        json!({"type":"message","role":"assistant","id":"answer","phase":"final_answer","content":content}),
    ]
}

async fn first_answer(model: &CodexResponsesModel) -> Result<String, Box<dyn std::error::Error>> {
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "hello").build())
        .await?;
    let mut text = String::new();
    while let Some(event) = stream.next().await {
        if let StreamedAssistantContent::Text(part) = event? {
            text.push_str(&part.text);
        }
    }
    Ok(text)
}

#[tokio::test]
async fn user_messages_and_nudges_retain_native_refusal_history() -> TestResult {
    // @step Given refusal only or mixed assistant output with optional readable reasoning
    for mixed in [false, true] {
        for readable in [false, true] {
            for prompt in ["Continue", "A different user question"] {
                let original = output(mixed);
                let server =
                    fixture::Server::start(vec![terminal(&original), terminal(&[])], false).await?;
                let model = server
                    .models()?
                    .0
                    .with_history(CodexHistory::default())
                    .with_completed_reasoning(readable);
                let visible = first_answer(&model).await?;
                assert_eq!(visible, if mixed { "Before No after" } else { "No" });
                // @step When a user message or continuation nudge follows the displayed answer
                let request = CompletionRequestBuilder::new(model.clone(), prompt)
                    .messages(vec![Message::user("hello"), Message::assistant(&visible)])
                    .build();
                let mut follow_up = model.stream(request).await?;
                while let Some(event) = follow_up.next().await {
                    event?;
                }
                // @step Then the provider receives the original refusal and opaque history unchanged
                let requests = server.requests.lock().map_err(|_| "capture poisoned")?;
                assert_eq!(requests.len(), 2);
                assert_eq!(requests[1]["input"][1], original[0]);
                assert_eq!(requests[1]["input"][2], original[1]);
                assert_eq!(
                    requests[1]["input"]
                        .as_array()
                        .ok_or("missing input")?
                        .len(),
                    4
                );
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn tool_results_retain_refusal_and_call_identity() -> TestResult {
    // @step Given mixed refusal output followed by a completed tool call
    for readable in [false, true] {
        let mut original = output(true);
        original.push(json!({"type":"function_call","id":"item","call_id":"call",
            "name":"inspect","arguments":"{}"}));
        let server =
            fixture::Server::start(vec![terminal(&original), terminal(&[])], false).await?;
        let model = server
            .models()?
            .0
            .with_history(CodexHistory::default())
            .with_completed_reasoning(readable);
        let visible = first_answer(&model).await?;
        // @step When the genuine tool result continues the displayed answer
        let result = Message::User {
            content: OneOrMany::one(UserContent::tool_result_with_call_id(
                "item",
                "call".into(),
                OneOrMany::one(ToolResultContent::text("genuine result")),
            )),
        };
        let request = CompletionRequestBuilder::new(model.clone(), result)
            .messages(vec![Message::user("hello"), Message::assistant(&visible)])
            .build();
        let mut follow_up = model.stream(request).await?;
        while let Some(event) = follow_up.next().await {
            event?;
        }
        // @step Then the physical follow up preserves refusal reasoning and the original call exactly once
        let requests = server.requests.lock().map_err(|_| "capture poisoned")?;
        let input = requests[1]["input"].as_array().ok_or("missing input")?;
        assert_eq!(&input[1..4], original.as_slice());
        assert_eq!(input.len(), 5);
        assert_eq!(input[4]["type"], "function_call_output");
        assert_eq!(input[4]["call_id"], "call");
        assert_eq!(input[4]["output"], "genuine result");
    }
    Ok(())
}

#[tokio::test]
async fn altered_readable_refusal_ancestry_remains_rejected() -> TestResult {
    // @step Given a committed native refusal answer
    for unknown_metadata in [false, true] {
        let mut original = output(false);
        if unknown_metadata {
            original[1]["content"][0]["unknown"] = json!("must retain");
        }
        let server = fixture::Server::start(vec![terminal(&original)], false).await?;
        let model = server.models()?.0.with_history(CodexHistory::default());
        first_answer(&model).await?;
        // @step When a follow up changes visible text or omits unknown content metadata
        let visible = if unknown_metadata { "No" } else { "Changed" };
        let request = CompletionRequestBuilder::new(model.clone(), "next")
            .messages(vec![Message::user("hello"), Message::assistant(visible)])
            .build();
        let outcome = model.stream(request).await;
        // @step Then ancestry validation rejects the request before inference
        let error = outcome.err().ok_or("altered ancestry accepted")?;
        assert!(error.to_string().contains("ancestry"), "{error}");
        assert_eq!(
            server
                .requests
                .lock()
                .map_err(|_| "capture poisoned")?
                .len(),
            1
        );
    }
    Ok(())
}
