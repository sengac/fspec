#[path = "fixtures/prov151.rs"]
pub mod fixture;
use fixture::*;
use futures::StreamExt;
use rig::agent::{AgentBuilder, MultiTurnStreamItem};
use rig::streaming::{StreamedUserContent, StreamingPrompt};
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::test]
async fn completed_read_only_calls_wait_for_response_validation() -> TestResult {
    let server = Server::start(
        vec![done(item(
            "fc_a",
            "call_a",
            "read_file",
            "{\"path\":\"a\"}",
        ))],
        true,
    )
    .await?;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let gate = Gate::default();
    let agent = AgentBuilder::new(server.models()?.0)
        .tool(ReadTool(calls.clone()))
        .build();
    let mut stream = agent.stream_prompt("read").with_hook(gate.clone()).await;
    let waiting = tokio::time::timeout(Duration::from_millis(150), async {
        while let Some(event) = stream.next().await {
            event?;
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await;
    assert!(waiting.is_err());
    assert!(calls.lock().map_err(|_| "calls lock")?.is_empty());
    assert_eq!(*gate.calls.lock().map_err(|_| "gate lock")?, 0);
    assert_eq!(*gate.results.lock().map_err(|_| "result lock")?, 0);
    Ok(())
}

#[tokio::test]
async fn duplicate_items_and_terminal_reconciliation_execute_once() -> TestResult {
    // @step Given two interleaved completed calls with repeated item and call identities
    // @step When a successful terminal response repeats their output
    // @step Then each call executes once with its own arguments and matching result
    let first = item("fc_a", "call_a", "read_file", "{\"path\":\"a\"}");
    let second = item("fc_b", "call_b", "read_file", "{\"path\":\"b\"}");
    let alias = item("fc_alias", "call_a", "read_file", "{\"path\":\"a\"}");
    let body = [
        done(first.clone()),
        done(second.clone()),
        done(first.clone()),
        done(alias),
        completed(vec![first, second]),
    ]
    .concat();
    let server = Server::start(vec![body, terminal("completed", Some(true), None)], false).await?;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let agent = AgentBuilder::new(server.models()?.0)
        .tool(ReadTool(calls.clone()))
        .build();
    let mut stream = agent.stream_prompt("read").multi_turn(2).await;
    let mut results = Vec::new();
    while let Some(event) = stream.next().await {
        if let MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(result)) = event?
        {
            results.push(result.call_id);
        }
    }
    assert_eq!(
        *calls.lock().map_err(|_| "calls lock")?,
        [json!({"path":"a"}), json!({"path":"b"})]
    );
    assert_eq!(results, [Some("call_a".into()), Some("call_b".into())]);
    assert_eq!(
        server.requests.lock().map_err(|_| "requests lock")?.len(),
        2
    );
    Ok(())
}

#[tokio::test]
async fn mutation_tools_remain_terminal_gated() -> TestResult {
    // @step Given a completed mutation item followed by a failed Codex response
    // @step When the agent consumes the response
    // @step Then no mutation executes and the failure is surfaced
    let body = [
        done(item("fc_a", "call_a", "shell", "{}")),
        terminal("failed", None, None),
    ]
    .concat();
    let observed = previous::run(body).await?;
    assert_eq!(observed.tools, 0);
    assert_eq!(observed.errors.len(), 1);
    Ok(())
}

#[tokio::test]
async fn invalid_and_unfinished_calls_never_execute() -> TestResult {
    // @step Given malformed arguments or a non-completed tool item
    // @step When the Codex stream terminates
    // @step Then no invalid or unfinished tool executes
    for args in ["{", "null", "[]", "42"] {
        let call = item("fc_a", "call_a", "read_file", args);
        let observed = previous::run([done(call.clone()), completed(vec![call])].concat()).await?;
        assert_eq!(observed.tools, 0, "arguments {args}");
        assert!(!observed.errors.is_empty());
    }
    let mut call = item("fc_a", "call_a", "read_file", "{}");
    call["status"] = json!("in_progress");
    let observed = previous::run([done(call.clone()), completed(vec![call])].concat()).await?;
    assert_eq!(observed.tools, 0);
    Ok(())
}

#[tokio::test]
async fn late_failure_prevents_read_only_execution_without_replay() -> TestResult {
    let body = [
        done(item("fc_a", "call_a", "read_file", "{}")),
        terminal("failed", None, None),
    ]
    .concat();
    let server = Server::start(vec![body], false).await?;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let agent = AgentBuilder::new(server.models()?.0)
        .tool(ReadTool(calls.clone()))
        .build();
    let mut stream = agent.stream_prompt("read").multi_turn(2).await;
    let mut results = 0;
    let mut errors = 0;
    while let Some(event) = stream.next().await {
        match event {
            Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(_))) => {
                results += 1
            }
            Err(_) => {
                errors += 1;
                break;
            }
            _ => {}
        }
    }
    assert!(calls.lock().map_err(|_| "calls lock")?.is_empty());
    assert_eq!(results, 0);
    assert_eq!(errors, 1);
    assert_eq!(
        server.requests.lock().map_err(|_| "requests lock")?.len(),
        1
    );
    Ok(())
}

#[tokio::test]
async fn permission_cancellation_still_prevents_execution() -> TestResult {
    // @step Given a completed read-only call rejected by the existing hook
    // @step When the agent dispatches the call
    // @step Then cancellation prevents tool execution
    let call = item("fc_a", "call_a", "read_file", "{}");
    let server = Server::start(
        vec![[done(call.clone()), completed(vec![call])].concat()],
        false,
    )
    .await?;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let gate = Gate {
        cancel: true,
        ..Gate::default()
    };
    let agent = AgentBuilder::new(server.models()?.0)
        .tool(ReadTool(calls.clone()))
        .build();
    let mut stream = agent.stream_prompt("read").with_hook(gate.clone()).await;
    let mut cancelled = false;
    while let Some(event) = stream.next().await {
        if event.is_err() {
            cancelled = true;
            break;
        }
    }
    assert!(cancelled);
    assert!(calls.lock().map_err(|_| "calls lock")?.is_empty());
    assert_eq!(*gate.calls.lock().map_err(|_| "gate lock")?, 1);
    Ok(())
}

#[tokio::test]
async fn ordinary_responses_does_not_opt_into_early_dispatch() -> TestResult {
    // @step Given an ordinary Responses completed tool item on an open stream
    // @step When the agent waits for output
    // @step Then the tool does not execute before the ordinary provider finishes
    let server = Server::start(vec![done(item("fc_a", "call_a", "read_file", "{}"))], true).await?;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let agent = AgentBuilder::new(server.models()?.1)
        .tool(ReadTool(calls.clone()))
        .build();
    let mut stream = agent.stream_prompt("read").await;
    let _ = tokio::time::timeout(Duration::from_millis(150), async {
        while let Some(event) = stream.next().await {
            let _ = event;
        }
    })
    .await;
    assert!(calls.lock().map_err(|_| "calls lock")?.is_empty());
    Ok(())
}
