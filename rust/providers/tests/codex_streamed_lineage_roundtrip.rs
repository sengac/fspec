//! Feature: spec/features/codex-streamed-lineage-roundtrip.feature

use codelet_providers::codex::history::CodexHistory;
use codelet_providers::request_admission::AdmissionLimits;
use codelet_test_helpers::setup_temp_directory;
use futures::StreamExt;
use rig::agent::{
    AgentBuilder, CancelSignal, MultiTurnStreamItem, StreamingError, StreamingPromptHook,
};
use rig::completion::{CompletionModel, CompletionRequestBuilder, PromptError};
use rig::message::{AssistantContent, Message};
use rig::streaming::StreamingPrompt;
use serde_json::{json, Value};
use std::{
    fs,
    sync::{Arc, Mutex},
};

#[path = "fixtures/prov151.rs"]
pub mod fixture;
use fixture::{ReadTool, Server, TestResult};

const COMMENTARY: &str = "I’ll check the latest commit’s summary and changes.";

fn assistant(text: &str) -> Value {
    json!({"type":"message","id":"msg_commentary","role":"assistant","phase":"commentary",
        "content":[{"type":"output_text","text":text,"annotations":[],"logprobs":[]}]})
}

fn call() -> Value {
    fixture::item(
        "fc_1",
        "call_1",
        "read_file",
        "{ \"path\": \"a\", \"limit\": 10 }",
    )
}

fn limits() -> AdmissionLimits {
    AdmissionLimits {
        context_capacity: Some(100_000),
        output_reservation: Some(1000),
    }
}

fn completed(output: Vec<Value>) -> String {
    fixture::event(json!({"type":"response.completed","response":{
        "id":"resp_1","status":"completed","end_turn":true,"output":output,
        "usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}
    }}))
}

fn initial() -> Value {
    json!({"model":"shared-model","input":[{"role":"user","content":[{"type":"input_text","text":"inspect"}]}]})
}

fn projected(text: &str, arguments: Value) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    let mut content = text
        .split_inclusive(' ')
        .map(AssistantContent::text)
        .collect::<Vec<_>>();
    content.push(AssistantContent::tool_call_with_call_id(
        "fc_1",
        "call_1".into(),
        "read_file",
        arguments,
    ));
    Ok(vec![
        Message::user("inspect"),
        Message::Assistant {
            id: None,
            content: rig::OneOrMany::many(content)?,
        },
        Message::tool_result_with_call_id("fc_1", Some("call_1".into()), "real result"),
    ])
}

#[tokio::test]
async fn scenario_streamed_commentary_survives_a_real_multi_turn_tool_continuation() -> TestResult {
    let commentary = assistant(COMMENTARY);
    let body = [
        fixture::event(json!({"type":"response.output_text.delta","item_id":"msg_commentary","output_index":0,"content_index":0,"delta":"I’ll check "})),
        fixture::event(json!({"type":"response.output_text.delta","item_id":"msg_commentary","output_index":0,"content_index":0,"delta":"the latest commit’s summary and changes."})),
        completed(vec![commentary, call()]),
    ].concat();
    let server = Server::start(
        vec![body, completed(vec![assistant("Final answer")])],
        false,
    )
    .await?;
    let history = CodexHistory::default();
    let model = server
        .models()?
        .0
        .with_history(history)
        .with_request_admission(limits());
    let calls = Arc::new(Mutex::new(Vec::new()));
    let agent = AgentBuilder::new(model)
        .tool(ReadTool(calls.clone()))
        .build();
    let mut stream = agent.stream_prompt("inspect").multi_turn(2).await;
    let mut answer = None;
    while let Some(event) = stream.next().await {
        if let MultiTurnStreamItem::FinalResponse(response) = event? {
            answer = Some(response.response().to_owned());
        }
    }
    assert_eq!(answer.as_deref(), Some("Final answer"));
    assert_eq!(
        *calls.lock().map_err(|_| "calls lock")?,
        [json!({"path":"a","limit":10})]
    );
    let requests = server.requests.lock().map_err(|_| "requests lock")?;
    let input = requests.get(1).ok_or("continuation missing")?["input"]
        .as_array()
        .ok_or("input missing")?;
    assert_eq!(
        input
            .iter()
            .filter(|item| item["role"] == "assistant")
            .count(),
        1
    );
    assert_eq!(input[1]["content"][0]["text"], COMMENTARY);
    assert_eq!(input[2]["call_id"], "call_1");
    assert_eq!(input[3]["call_id"], "call_1");
    assert_eq!(
        serde_json::from_str::<Value>(input[3]["output"].as_str().ok_or("tool output missing")?)?,
        json!({"path":"a","limit":10})
    );
    Ok(())
}

#[tokio::test]
async fn scenario_lossless_history_projections_preserve_original_checkpoint_items() -> TestResult {
    let directory = setup_temp_directory()?;
    let path = directory.path().join("checkpoint.json");
    let history = CodexHistory::default();
    let mut first = assistant("I’ll check ");
    first["content"][0]["annotations"] = json!([{"type":"test_annotation","offset":0}]);
    first["content"][0]["logprobs"] = json!([{"token":"I’ll","logprob":-0.1}]);
    history.commit(
        &initial(),
        &[
            first,
            assistant("the latest commit’s summary and changes."),
            call(),
        ],
    )?;
    let original = history.snapshot()?;
    fs::write(&path, serde_json::to_vec(&original)?)?;
    let restored = CodexHistory::default();
    restored.restore(serde_json::from_slice(&fs::read(path)?)?)?;
    for history in [history, restored] {
        let server = Server::start(vec![completed(vec![])], false).await?;
        let model = server
            .models()?
            .0
            .with_history(history)
            .with_request_admission(limits());
        let request = CompletionRequestBuilder::new(model.clone(), "next")
            .messages(projected(COMMENTARY, json!({"limit":10,"path":"a"}))?)
            .build();
        let mut stream = model.stream(request).await?;
        while let Some(event) = stream.next().await {
            event?;
        }
        let requests = server.requests.lock().map_err(|_| "requests lock")?;
        let input = &requests.first().ok_or("continuation missing")?["input"];
        assert_eq!(input.as_array().ok_or("input missing")?.len(), 6);
        assert_eq!(
            input[1],
            serde_json::to_value(&original.as_ref().ok_or("checkpoint missing")?.items[1])?
        );
        assert_eq!(
            input[2],
            serde_json::to_value(&original.as_ref().ok_or("checkpoint missing")?.items[2])?
        );
        assert_eq!(input[3]["arguments"], call()["arguments"]);
        assert_eq!(input[4]["output"], "real result");
    }
    Ok(())
}

#[tokio::test]
async fn scenario_changed_projected_ancestry_remains_rejected() -> TestResult {
    for (text, arguments) in [
        ("changed assistant", json!({"path":"a","limit":10})),
        ("", json!({"path":"a","limit":10})),
        (COMMENTARY, json!({"path":"b","limit":10})),
    ] {
        let history = CodexHistory::default();
        history.commit(&initial(), &[assistant(COMMENTARY), call()])?;
        let previous = history.snapshot()?;
        let server = Server::start(vec![fixture::completed(vec![])], false).await?;
        let model = server
            .models()?
            .0
            .with_history(history.clone())
            .with_request_admission(limits());
        let request = CompletionRequestBuilder::new(model.clone(), "next")
            .messages(projected(text, arguments)?)
            .build();
        assert!(model.stream(request).await.is_err());
        assert_eq!(history.snapshot()?, previous);
        assert!(server
            .requests
            .lock()
            .map_err(|_| "requests lock")?
            .is_empty());
    }
    Ok(())
}

#[derive(Clone)]
struct CancelAfterResult;

impl<Model: CompletionModel> StreamingPromptHook<Model> for CancelAfterResult {
    async fn on_tool_result(
        &self,
        _: &str,
        _: Option<String>,
        _: &str,
        _: &str,
        signal: CancelSignal,
    ) {
        signal.cancel();
    }
}

#[tokio::test]
async fn scenario_cancellation_retains_commentary_with_the_completed_tool_outcome() -> TestResult {
    let server = Server::start(vec![completed(vec![assistant(COMMENTARY), call()])], false).await?;
    let history = CodexHistory::default();
    let model = server
        .models()?
        .0
        .with_history(history.clone())
        .with_request_admission(limits());
    let agent = AgentBuilder::new(model.clone())
        .tool(ReadTool(Arc::new(Mutex::new(Vec::new()))))
        .build();
    let mut stream = agent
        .stream_prompt("inspect")
        .with_hook(CancelAfterResult)
        .multi_turn(2)
        .await;
    let mut cancelled = None;
    while let Some(event) = stream.next().await {
        match event {
            Err(StreamingError::Prompt(error)) => {
                if let PromptError::PromptCancelled { chat_history } = *error {
                    cancelled = Some(*chat_history);
                    break;
                }
                return Err(error.into());
            }
            other => {
                other?;
            }
        }
    }
    let messages = cancelled.ok_or("cancellation history missing")?;
    let serialized = serde_json::to_string(&messages)?;
    assert!(
        serialized.contains(COMMENTARY),
        "cancellation dropped commentary"
    );
    let request = CompletionRequestBuilder::new(model.clone(), "next")
        .messages(messages)
        .build();
    let bytes = model.prepare_request(request)?;
    let body: Value = serde_json::from_slice(&bytes)?;
    assert_eq!(body["input"][1]["content"][0]["text"], COMMENTARY);
    assert_eq!(body["input"][2]["call_id"], "call_1");
    assert_eq!(body["input"][3]["call_id"], "call_1");
    Ok(())
}
