//! Feature: spec/features/codex-reconnect-stream-consistency.feature
#[path = "fixtures/prov148.rs"]
mod fixture;
#[path = "fixtures/reconnect_consistency.rs"]
mod support;

use codelet_providers::codex::history::CodexHistory;
use fixture::{event, terminal, text};
use futures::StreamExt;
use rig::{
    completion::{CompletionModel, CompletionRequestBuilder},
    message::{AssistantContent, Message},
    streaming::StreamedAssistantContent,
};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use support::{model, SocketServer, TestResult};
use tokio::sync::Notify;

fn abandoned() -> String {
    [
        event(
            json!({"type":"response.output_text.delta", "delta":"Wrong answer",
            "output_index":0, "content_index":0}),
        ),
        event(
            json!({"type":"response.reasoning_summary_text.delta", "delta":"Wrong reasoning",
            "output_index":1, "summary_index":0}),
        ),
    ]
    .concat()
}

fn abandoned_item() -> String {
    let mut item = fixture::baseline::response()["output"][0].clone();
    item["content"][0]["text"] = json!("Wrong answer");
    [
        event(json!({"type":"response.output_item.done", "output_index":0, "item":item})),
        event(
            json!({"type":"response.reasoning_summary_text.delta", "delta":"Wrong reasoning",
            "output_index":1, "summary_index":0}),
        ),
    ]
    .concat()
}

fn answer(choice: &rig::OneOrMany<AssistantContent>) -> String {
    choice
        .iter()
        .filter_map(|content| match content {
            AssistantContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn scenario_replacement_output_agrees_with_next_turn_lineage_on_both_transports() -> TestResult
{
    for websocket in [false, true] {
        // @step Given an attempt streams divergent text and reasoning before disconnecting
        let bodies = vec![
            if websocket {
                abandoned_item()
            } else {
                abandoned()
            },
            terminal("completed", Some(true), None),
            terminal("completed", Some(true), None),
        ];
        let sse;
        let socket;
        let (url, requests) = if websocket {
            socket = SocketServer::start(bodies, None).await?;
            (&socket.url, socket.requests.clone())
        } else {
            sse = fixture::Server::start(bodies, false).await?;
            (&sse.url, sse.requests.clone())
        };
        let history = CodexHistory::default();
        let model = model(url, history.clone(), websocket)?;
        // @step When its replacement completes on the selected transport
        let request = CompletionRequestBuilder::new(model.clone(), "question").build();
        let mut stream = model.stream(request).await?;
        let mut events = Vec::new();
        let mut reasoning = String::new();
        let mut reasoning_start = 0;
        tokio::time::timeout(Duration::from_secs(4), async {
            while let Some(event) = stream.next().await {
                events.push(match event? {
                    StreamedAssistantContent::ResponseAttemptStarted => {
                        reasoning_start = reasoning.len();
                        "started"
                    }
                    StreamedAssistantContent::ResponseAttemptAborted => {
                        reasoning.truncate(reasoning_start);
                        "aborted"
                    }
                    StreamedAssistantContent::Text(_) => "text",
                    StreamedAssistantContent::ReasoningDelta {
                        reasoning: delta, ..
                    } => {
                        reasoning.push_str(&delta);
                        "reasoning"
                    }
                    StreamedAssistantContent::Usage(_) => "usage",
                    StreamedAssistantContent::Final(_) => "final",
                    _ => "other",
                });
            }
            Ok::<_, rig::completion::CompletionError>(())
        })
        .await??;
        // @step Then abandoned output is retracted and the successful answer appears once
        assert_eq!(answer(&stream.choice), "Hello");
        assert_eq!(reasoning, "");
        assert_eq!(
            events,
            [
                "started",
                "text",
                "reasoning",
                "aborted",
                "started",
                "text",
                "usage",
                "final"
            ]
        );
        assert!(stream.response.is_some());
        let checkpoint = serde_json::to_string(&history.snapshot()?)?;
        assert!(!checkpoint.contains("Wrong"));
        // @step And the next turn replays the successful answer without lineage failure
        let request = CompletionRequestBuilder::new(model.clone(), "follow up")
            .messages(vec![
                Message::user("question"),
                Message::Assistant {
                    id: None,
                    content: stream.choice,
                },
            ])
            .build();
        let mut next = model.stream(request).await?;
        tokio::time::timeout(Duration::from_secs(2), async {
            while let Some(event) = next.next().await {
                event?;
            }
            Ok::<_, rig::completion::CompletionError>(())
        })
        .await??;
        let requests = requests.lock().map_err(|_| "capture unavailable")?;
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0], requests[1]);
        assert_eq!(requests[2].to_string().matches("Hello").count(), 1);
        assert!(!requests[2].to_string().contains("Wrong"));
    }
    Ok(())
}

#[tokio::test]
async fn scenario_streaming_remains_live_before_the_attempt_terminates() -> TestResult {
    // @step Given an accepted WebSocket response whose terminal event is withheld
    let gate = Arc::new(Notify::new());
    let server = SocketServer::start(
        vec![[abandoned(), terminal("completed", Some(true), None)].concat()],
        Some(gate.clone()),
    )
    .await?;
    let model = model(&server.url, CodexHistory::default(), true)?;
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "question").build())
        .await?;
    // @step When partial text and reasoning arrive
    let mut seen_text = false;
    let mut seen_reasoning = false;
    tokio::time::timeout(Duration::from_secs(1), async {
        while !seen_text || !seen_reasoning {
            match stream
                .next()
                .await
                .ok_or("stream ended before terminal release")??
            {
                StreamedAssistantContent::Text(text) => seen_text = text.text == "Wrong answer",
                StreamedAssistantContent::ReasoningDelta { reasoning, .. } => {
                    seen_reasoning = reasoning == "Wrong reasoning"
                }
                _ => {}
            }
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await??;
    // @step Then both are delivered before the server releases the terminal event
    assert!(seen_text && seen_reasoning);
    gate.notify_one();
    while let Some(event) = stream.next().await {
        event?;
    }
    assert!(stream.response.is_some());
    Ok(())
}

#[tokio::test]
async fn scenario_continuation_retries_preserve_earlier_successful_physical_output() -> TestResult {
    // @step Given a successful nonterminal response followed by an interrupted continuation
    let server = fixture::Server::start(
        vec![
            [text(), terminal("completed", Some(false), None)].concat(),
            abandoned(),
            terminal("completed", Some(true), None),
        ],
        false,
    )
    .await?;
    // @step When the continuation replacement completes
    let observed = tokio::time::timeout(
        Duration::from_secs(4),
        fixture::collect(model(&server.url, CodexHistory::default(), false)?),
    )
    .await??;
    // @step Then only the interrupted continuation output is retracted
    assert!(observed.errors.is_empty(), "{observed:?}");
    assert_eq!(observed.text, "HelloHello");
    assert_eq!(observed.reasoning, "");
    assert_eq!(observed.finals.len(), 1);
    let requests = server.requests.lock().map_err(|_| "capture unavailable")?;
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[1], requests[2]);
    assert_ne!(requests[0], requests[1]);
    Ok(())
}
