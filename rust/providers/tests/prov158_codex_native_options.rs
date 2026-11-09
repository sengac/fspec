//! Feature: spec/features/codex-sole-dag-memory-policy.feature

use codelet_providers::codex::NativeCapabilities;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use serde_json::{json, Value};

#[path = "fixtures/prov148.rs"]
pub mod fixture;

fn terminal(output: Vec<Value>) -> String {
    fixture::event(json!({"type":"response.completed","response":{
        "id":"resp_native","status":"completed","end_turn":true,"output":output,
        "usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}
    }}))
}

#[tokio::test]
async fn hosted_search_replays_without_native_memory_activation() -> fixture::TestResult {
    let opaque = json!({"type":"reasoning","id":"reasoning","summary":[],"encrypted_content":"synthetic-private"});
    // @step Given a Codex model with hosted search enabled and encrypted reasoning output
    let search = json!({"type":"web_search_call","id":"search","status":"completed","action":{"type":"search","query":"Rust"}});
    let server = fixture::Server::start(
        vec![
            terminal(vec![search.clone(), opaque.clone()]),
            terminal(vec![]),
            fixture::terminal("completed", Some(true), None),
        ],
        false,
    )
    .await?;
    let model = server
        .models()?
        .0
        .with_native_capabilities(NativeCapabilities {
            hosted_search: true,
        });
    let first = fixture::collect(model.clone()).await?;
    assert!(first.errors.is_empty(), "{:?}", first.errors);
    assert_eq!(first.tools, 0);
    assert!(first.reasoning.contains("search"));
    // @step When a successful response is followed by another user request
    let mut stream = model
        .stream(
            CompletionRequestBuilder::new(model.clone(), "next")
                .messages(vec![rig::message::Message::user("Hello")])
                .build(),
        )
        .await?;
    while stream.next().await.transpose()?.is_some() {}
    fixture::collect(server.models()?.0).await?;
    let requests = server.requests.lock().map_err(|_| "requests lock")?;
    assert!(requests[0]["tools"]
        .as_array()
        .ok_or("tools")?
        .iter()
        .any(|tool| tool["type"] == "web_search"));
    assert!(!requests[0]["input"]
        .as_array()
        .ok_or("input")?
        .iter()
        .any(|item| item["type"] == "compaction_trigger"));
    // @step Then supported items replay without native memory activation or opaque disclosure
    let input = requests[1]["input"].as_array().ok_or("input")?;
    assert!(input.contains(&search));
    assert!(input.contains(&opaque));
    assert!(requests[1]["input"]
        .as_array()
        .ok_or("input")?
        .iter()
        .any(|item| item["content"].to_string().contains("next")));
    assert!(!requests[2]["tools"]
        .as_array()
        .is_some_and(|tools| tools.iter().any(|tool| tool["type"] == "web_search")));
    assert!(!requests[2]["input"]
        .as_array()
        .ok_or("input")?
        .iter()
        .any(|item| item["type"] == "compaction_trigger"));
    Ok(())
}
