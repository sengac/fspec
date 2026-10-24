//! Feature: spec/features/codex-custom-tool-dialect.feature
use codelet_providers::codex::custom_tools::{CustomToolRegistry, ToolRegistration};
use codelet_providers::codex::presentation::CodexPresentationHook;
use futures::StreamExt;
use rig::agent::AgentBuilder;
use rig::completion::ToolDefinition;
use rig::streaming::StreamingPrompt;
use rig::tool::Tool;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[path = "fixtures/prov151.rs"]
pub mod fixture;
use fixture::*;

#[derive(Clone)]
struct PatchTool(Arc<Mutex<Vec<Value>>>);

impl Tool for PatchTool {
    const NAME: &'static str = "apply_patch";
    type Error = std::io::Error;
    type Args = Value;
    type Output = Value;
    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Record patch".into(),
            parameters: json!({"type":"object"}),
        }
    }
    async fn call(&self, args: Value) -> Result<Value, Self::Error> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("patch lock"))?
            .push(args.clone());
        Ok(args)
    }
}

fn registry() -> Result<CustomToolRegistry, Box<dyn std::error::Error>> {
    Ok(CustomToolRegistry::new(vec![ToolRegistration {
        namespace: Some("edit".into()),
        name: "patch".into(),
        executor: "apply_patch".into(),
        grammar: None,
        raw_input: true,
    }])?)
}

#[tokio::test]
async fn raw_input_updates_remain_presentation_only() -> TestResult {
    // @step Given an enabled custom call arriving in separate Unicode input updates
    let input = "*** Begin Patch\n你好\n*** End Patch";
    let call = json!({"type":"custom_tool_call","id":"custom1","call_id":"call1","namespace":"edit","name":"patch","input":input,"status":"completed"});
    let body = [
        event(json!({"type":"response.output_item.added","item":{"type":"custom_tool_call","id":"custom1","namespace":"edit","name":"patch"}})),
        event(json!({"type":"response.custom_tool_call_input.delta","item_id":"custom1","delta":"*** Begin Patch\n"})),
        event(json!({"type":"response.custom_tool_call_input.delta","item_id":"custom1","delta":"你好\n*** End Patch"})),
        done(call.clone()), completed(vec![call]),
    ].concat();
    let server = Server::start(vec![body, terminal("completed", Some(true), None)], false).await?;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let updates = Arc::new(Mutex::new(Vec::new()));
    let captured = updates.clone();
    let before = calls.clone();
    let hook = CodexPresentationHook::new(
        Gate::default(),
        Some(Arc::new(move |update| {
            assert!(before
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_empty());
            captured
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(update);
        })),
    );
    let model = server.models()?.0.with_custom_tools(registry()?);
    let agent = AgentBuilder::new(model)
        .tool(PatchTool(calls.clone()))
        .build();
    // @step When partial input arrives before its completed item
    let mut stream = agent
        .stream_prompt("patch")
        .with_hook(hook)
        .multi_turn(2)
        .await;
    while let Some(item) = stream.next().await {
        item?;
    }
    // @step Then updates are visible but only the completed item becomes executable
    assert_eq!(updates.lock().map_err(|_| "updates lock")?.len(), 3);
    assert_eq!(
        *calls.lock().map_err(|_| "calls lock")?,
        [json!({"patch":input})]
    );
    let requests = server.requests.lock().map_err(|_| "requests lock")?;
    assert_eq!(requests[0]["tools"][0]["type"], "namespace");
    assert!(requests[1]["input"]
        .as_array()
        .ok_or("input array")?
        .iter()
        .any(|item| item["type"] == "custom_tool_call_output" && item["call_id"] == "call1"));
    Ok(())
}

#[tokio::test]
async fn existing_execution_authorization_applies_to_raw_tools() -> TestResult {
    // @step Given a custom patch call subject to an execution denial
    let call = json!({"type":"custom_tool_call","id":"custom1","call_id":"call1","namespace":"edit","name":"patch","input":"denied","status":"completed"});
    let server = Server::start(vec![completed(vec![call])], false).await?;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let model = server.models()?.0.with_custom_tools(registry()?);
    let agent = AgentBuilder::new(model)
        .tool(PatchTool(calls.clone()))
        .build();
    let gate = Gate {
        cancel: true,
        ..Gate::default()
    };
    // @step When the completed call reaches the existing tool dispatch hook
    let mut stream = agent.stream_prompt("patch").with_hook(gate.clone()).await;
    let mut rejected = false;
    while let Some(item) = stream.next().await {
        if item.is_err() {
            rejected = true;
            break;
        }
    }
    // @step Then the patch executor is not invoked
    assert!(rejected);
    assert!(calls.lock().map_err(|_| "calls lock")?.is_empty());
    assert_eq!(*gate.calls.lock().map_err(|_| "gate lock")?, 1);
    Ok(())
}
