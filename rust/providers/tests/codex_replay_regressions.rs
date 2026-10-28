use codelet_providers::codex::{
    custom_tools::{CustomToolRegistry, ToolRegistration},
    history::{CheckpointError, CodexHistory},
    NativeCapabilities,
};
use codelet_test_helpers::setup_temp_directory;
use futures::StreamExt;
use rig::{
    agent::{AgentBuilder, MultiTurnStreamItem},
    completion::{CompletionError, CompletionModel, CompletionRequestBuilder},
    message::{AssistantContent, Message},
    streaming::StreamingPrompt,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[path = "fixtures/prov151.rs"]
pub mod fixture;
use fixture::{ReadTool, Server, TestResult};

fn assistant(text: &str) -> Value {
    json!({"type":"message","id":format!("msg_{text}"),"role":"assistant",
        "content":[{"type":"output_text","text":text,"annotations":[]}]})
}

fn call() -> Value {
    json!({"type":"function_call","id":"item_1","call_id":"call_1",
        "name":"read_file","arguments":"{\"path\":\"a\"}"})
}

fn reasoning() -> Value {
    json!({"type":"reasoning","id":"rs_1","summary":[],"encrypted_content":"opaque"})
}

#[tokio::test]
async fn reasoning_between_assistant_messages_survives_real_tool_continuation() -> TestResult {
    let output = vec![assistant("First"), reasoning(), assistant("Second"), call()];
    let server = Server::start(
        vec![
            fixture::completed(output.clone()),
            fixture::completed(vec![assistant("Done")]),
        ],
        false,
    )
    .await?;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let agent = AgentBuilder::new(server.models()?.0)
        .tool(ReadTool(calls.clone()))
        .build();
    let mut stream = agent.stream_prompt("inspect").multi_turn(2).await;
    let mut answer = None;
    while let Some(event) = stream.next().await {
        if let MultiTurnStreamItem::FinalResponse(response) = event? {
            answer = Some(response.response().to_owned());
        }
    }
    assert_eq!(answer.as_deref(), Some("Done"));
    assert_eq!(calls.lock().map_err(|_| "calls lock")?.len(), 1);
    let requests = server.requests.lock().map_err(|_| "requests lock")?;
    let input = requests[1]["input"].as_array().ok_or("input missing")?;
    assert_eq!(&input[1..5], output);
    assert_eq!(input[5]["call_id"], "call_1");
    assert_eq!(input.len(), 6);
    Ok(())
}

proptest::proptest! {
    #[test]
    fn reasoning_interleaving_preserves_original_items(
        first in "[a-zA-Z0-9 ]{1,40}", second in "[a-zA-Z0-9 ]{1,40}"
    ) {
        let check = || -> TestResult {
            let initial = json!({"model":"shared-model","input":[{"type":"message",
                "role":"user","content":[{"type":"input_text","text":"inspect"}]}]});
            let output = vec![assistant(&first), reasoning(), assistant(&second), call()];
            let history = CodexHistory::default();
            history.commit(&initial, &output)?;
            let saved = history.snapshot()?;
            let result = json!({"type":"function_call_output","call_id":"call_1","output":"real"});
            let mut request = json!({"model":"shared-model","input":[initial["input"][0],
                assistant(&format!("{first}{second}")),call(),result]});
            history.prepare(&mut request)?;
            assert_eq!(&request["input"].as_array().ok_or("input missing")?[1..5], output);
            assert_eq!(request["input"][5], result);
            let prepared = request.clone();
            history.prepare(&mut request)?;
            assert_eq!(request, prepared);
            assert_eq!(history.snapshot()?, saved);
            request["input"][1]["content"][0]["text"] = json!(format!("{first}!"));
            let changed = request.clone();
            assert!(history.prepare(&mut request).is_err());
            assert_eq!(request, changed);
            assert_eq!(history.snapshot()?, saved);
            Ok(())
        };
        check().map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
    }
}

#[tokio::test]
async fn hosted_search_notices_do_not_become_assistant_ancestry() -> TestResult {
    let search = json!({"type":"web_search_call","id":"ws_1","status":"completed",
        "action":{"type":"search","query":"test"}});
    let mut answer = assistant("Found");
    answer["content"][0]["annotations"] = json!([{"type":"url_citation",
        "url":"https://example.com","title":"Source","start_index":0,"end_index":5}]);
    let output = vec![search.clone(), answer, call()];
    let body = [
        fixture::event(json!({"type":"response.web_search_call.in_progress","item_id":"ws_1"})),
        fixture::done(search),
        fixture::completed(output.clone()),
    ]
    .concat();
    let server = Server::start(
        vec![body, fixture::completed(vec![assistant("Done")])],
        false,
    )
    .await?;
    let model = server
        .models()?
        .0
        .with_native_capabilities(NativeCapabilities {
            hosted_search: true,
        });
    let agent = AgentBuilder::new(model)
        .tool(ReadTool(Arc::new(Mutex::new(Vec::new()))))
        .build();
    let mut stream = agent.stream_prompt("inspect").multi_turn(2).await;
    let mut answer = None;
    while let Some(event) = stream.next().await {
        if let MultiTurnStreamItem::FinalResponse(response) = event? {
            answer = Some(response.response().to_owned());
        }
    }
    assert_eq!(answer.as_deref(), Some("Done"));
    let requests = server.requests.lock().map_err(|_| "requests lock")?;
    let input = requests[1]["input"].as_array().ok_or("input missing")?;
    assert_eq!(&input[1..4], output);
    assert_eq!(input.len(), 5);
    assert!(!serde_json::to_string(input)?.contains("[Codex"));
    Ok(())
}

#[tokio::test]
async fn unnamespaced_function_alias_retains_wire_identity_after_restart() -> TestResult {
    let directory = setup_temp_directory()?;
    let path = directory.path().join("checkpoint.json");
    let registry = CustomToolRegistry::new(vec![ToolRegistration {
        namespace: None,
        name: "read_alias".into(),
        executor: "read_file".into(),
        grammar: None,
        raw_input: false,
    }])?;
    let initial = json!({"model":"shared-model","input":[{"type":"message","role":"user",
        "content":[{"type":"input_text","text":"inspect"}]}]});
    let mut wire_call = call();
    wire_call["name"] = json!("read_alias");
    let history = CodexHistory::default();
    history.commit(&initial, std::slice::from_ref(&wire_call))?;
    std::fs::write(&path, serde_json::to_vec(&history.snapshot()?)?)?;
    let restored = CodexHistory::default();
    restored.restore(serde_json::from_slice(&std::fs::read(path)?)?)?;
    for history in [history, restored] {
        let server = Server::start(vec![], false).await?;
        let model = server
            .models()?
            .0
            .with_history(history)
            .with_custom_tools(registry.clone());
        let request = CompletionRequestBuilder::new(model.clone(), "next")
            .messages(vec![
                Message::user("inspect"),
                Message::Assistant {
                    id: None,
                    content: rig::OneOrMany::one(AssistantContent::tool_call_with_call_id(
                        "item_1",
                        "call_1".into(),
                        "read_file",
                        json!({"path":"a"}),
                    )),
                },
                Message::tool_result_with_call_id("item_1", Some("call_1".into()), "real result"),
            ])
            .build();
        let body: Value = serde_json::from_slice(&model.prepare_request(request)?)?;
        assert_eq!(body["input"][1]["name"], "read_alias");
        assert_eq!(body["input"][2]["call_id"], "call_1");
        assert_eq!(body["input"][2]["output"], "real result");
    }
    Ok(())
}

#[test]
fn interrupted_checkpoint_requires_a_genuine_outcome_and_remains_recoverable() -> TestResult {
    let directory = setup_temp_directory()?;
    let path = directory.path().join("checkpoint.json");
    let initial = json!({"model":"shared-model","input":[{"type":"message","role":"user",
        "content":[{"type":"input_text","text":"inspect"}]}]});
    let history = CodexHistory::default();
    history.commit(&initial, &[call()])?;
    let saved = history.snapshot()?;
    std::fs::write(&path, serde_json::to_vec(&saved)?)?;
    let restored = CodexHistory::default();
    restored.restore(serde_json::from_slice(&std::fs::read(path)?)?)?;
    for history in [history, restored] {
        let new_user = json!({"type":"message","role":"user","content":[{"type":"input_text","text":"review"}]});
        let mut request =
            json!({"model":"shared-model","input":[initial["input"][0],call(),new_user]});
        let original = request.clone();
        let error = history
            .prepare(&mut request)
            .err()
            .ok_or("missing outcome accepted")?;
        assert!(matches!(error, CompletionError::RequestError(ref source)
            if matches!(source.downcast_ref::<CheckpointError>(), Some(CheckpointError::MissingOutcome(identity)) if identity == "call_1")));
        assert_eq!(request, original);
        assert_eq!(history.snapshot()?, saved);
        request["input"].as_array_mut().ok_or("input missing")?.insert(2,
            json!({"type":"function_call_output","call_id":"call_1","output":"Tool execution cancelled before a result was recorded"}));
        history.prepare(&mut request)?;
        assert_eq!(request["input"].as_array().ok_or("input missing")?.len(), 4);
        assert_eq!(request["input"][2]["call_id"], "call_1");
        assert_eq!(request["input"][3], new_user);
    }
    Ok(())
}
