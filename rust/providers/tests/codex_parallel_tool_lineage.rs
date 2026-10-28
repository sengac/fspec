//! Feature: spec/features/codex-parallel-tool-lineage.feature

use codelet_providers::codex::history::{CheckpointError, CodexHistory};
use codelet_test_helpers::setup_temp_directory;
use futures::StreamExt;
use rig::{
    completion::{CompletionError, CompletionModel, CompletionRequestBuilder},
    message::{AssistantContent, Message},
};
use serde_json::{json, Value};
use std::fs;

#[path = "fixtures/prov148.rs"]
pub mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn initial() -> Value {
    json!({"model":"shared-model","input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"inspect"}]}]})
}

fn call(number: usize) -> Value {
    json!({"type":"function_call","id":format!("item-{number}"),
        "call_id":format!("call-{number}"),"name":"inspect",
        "arguments":format!("{{ \"number\": {number} }}")})
}

fn outcome(number: usize) -> Value {
    json!({"type":"function_call_output","call_id":format!("call-{number}"),
        "output":format!("result-{number}")})
}

fn messages() -> Vec<Message> {
    let mut messages = vec![Message::user("inspect")];
    for number in 1..=3 {
        messages.push(Message::Assistant {
            id: None,
            content: rig::OneOrMany::one(AssistantContent::tool_call_with_call_id(
                format!("item-{number}"),
                format!("call-{number}"),
                "inspect",
                json!({"number":number}),
            )),
        });
        messages.push(Message::tool_result_with_call_id(
            format!("item-{number}"),
            Some(format!("call-{number}")),
            format!("result-{number}"),
        ));
    }
    messages
}

fn interleaved() -> Value {
    json!({"model":"shared-model","input":[initial()["input"][0],
        call(1),outcome(1),call(2),outcome(2),call(3),outcome(3)]})
}

#[tokio::test]
async fn scenario_interleaved_parallel_tool_outcomes_preserve_committed_lineage() -> TestResult {
    let directory = setup_temp_directory()?;
    let path = directory.path().join("checkpoint.json");
    let history = CodexHistory::default();
    history.commit(
        &initial(),
        &[
            call(1),
            call(2),
            call(3),
            outcome(1),
            outcome(2),
            outcome(3),
        ],
    )?;
    let original = history.snapshot()?;
    fs::write(&path, serde_json::to_vec(&original)?)?;
    let restored = CodexHistory::default();
    restored.restore(serde_json::from_slice(&fs::read(&path)?)?)?;
    for history in [history, restored] {
        let server =
            fixture::Server::start(vec![fixture::terminal("completed", None, None)], false).await?;
        let model = server.models()?.0.with_history(history.clone());
        let request = CompletionRequestBuilder::new(model.clone(), "next")
            .messages(messages())
            .build();
        let mut stream = model.stream(request).await?;
        while let Some(event) = stream.next().await {
            event?;
        }
        let requests = server.requests.lock().map_err(|_| "capture poisoned")?;
        let input = requests.first().ok_or("request missing")?["input"]
            .as_array()
            .ok_or("input missing")?;
        let saved = &original.as_ref().ok_or("checkpoint missing")?.items;
        assert_eq!(
            &input[..saved.len()],
            serde_json::to_value(saved)?
                .as_array()
                .ok_or("saved input missing")?
        );
        assert_eq!(input.len(), saved.len() + 1);
        assert_eq!(
            input.last().ok_or("prompt missing")?["content"][0]["text"],
            "next"
        );
    }
    Ok(())
}

#[test]
fn scenario_pending_parallel_calls_retain_new_interleaved_outcomes() -> TestResult {
    for recorded_outcomes in 0..=3 {
        let history = CodexHistory::default();
        let mut output = vec![call(1), call(2), call(3)];
        output.extend((1..=recorded_outcomes).map(outcome));
        history.commit(&initial(), &output)?;
        let original = history.snapshot()?;
        let mut request = interleaved();
        history.prepare(&mut request)?;
        assert_eq!(
            request["input"],
            json!([
                initial()["input"][0],
                call(1),
                call(2),
                call(3),
                outcome(1),
                outcome(2),
                outcome(3)
            ])
        );
        assert_eq!(history.snapshot()?, original);
    }
    Ok(())
}

#[tokio::test]
async fn scenario_changed_parallel_tool_ancestry_remains_rejected() -> TestResult {
    for change in [
        "call arguments",
        "call identity",
        "call order",
        "result payload",
        "result order",
        "missing result",
        "intervening text",
        "unrelated outcome",
    ] {
        let history = CodexHistory::default();
        history.commit(
            &initial(),
            &[
                call(1),
                call(2),
                call(3),
                outcome(1),
                outcome(2),
                outcome(3),
            ],
        )?;
        let original = history.snapshot()?;
        let server = fixture::Server::start(Vec::new(), false).await?;
        let model = server.models()?.0.with_history(history.clone());
        let mut projected = messages();
        match change {
            "call arguments" | "call identity" => {
                let Message::Assistant { content, .. } = &mut projected[3] else {
                    return Err("assistant missing".into());
                };
                let AssistantContent::ToolCall(call) = content.first_mut() else {
                    return Err("call missing".into());
                };
                if change == "call arguments" {
                    call.function.arguments = json!({"number":99});
                } else {
                    call.call_id = Some("changed".into());
                }
            }
            "call order" => projected.swap(1, 3),
            "result payload" => {
                projected[4] = Message::tool_result_with_call_id(
                    "item-2",
                    Some("call-2".into()),
                    "changed result",
                )
            }
            "result order" => projected.swap(2, 4),
            "missing result" => {
                projected.remove(4);
            }
            "intervening text" => projected.insert(3, Message::assistant("new ancestry")),
            _ => projected.insert(
                3,
                Message::tool_result_with_call_id("other", Some("other".into()), "unrelated"),
            ),
        }
        let request = CompletionRequestBuilder::new(model.clone(), "next")
            .messages(projected)
            .build();
        let error = model
            .stream(request)
            .await
            .err()
            .ok_or("changed ancestry accepted")?;
        assert!(
            matches!(error, CompletionError::RequestError(ref source)
            if matches!(source.downcast_ref::<CheckpointError>(), Some(CheckpointError::Lineage(_)))),
            "{change}: {error}"
        );
        assert_eq!(history.snapshot()?, original);
        assert!(server
            .requests
            .lock()
            .map_err(|_| "capture poisoned")?
            .is_empty());
    }
    Ok(())
}
