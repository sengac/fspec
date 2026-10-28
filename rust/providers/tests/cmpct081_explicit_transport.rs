//! Feature: spec/features/codex-explicit-transport.feature

use codelet_providers::codex::{
    history::CodexHistory,
    refreshing_client::RefreshingCodexClient,
    websocket::{CodexWebSocketHandle, WebSocketUpgradeRejected},
    CodexResponsesModel,
};
use futures::{SinkExt, StreamExt};
use rig::{
    completion::{CompletionModel, CompletionRequestBuilder},
    providers::openai,
};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};

#[path = "fixtures/attempt_events.rs"]
mod attempt_events;
#[path = "fixtures/prov148.rs"]
pub mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn model(url: &str) -> Result<CodexResponsesModel, Box<dyn std::error::Error>> {
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(url)
        .http_client(
            RefreshingCodexClient::new_api_key().with_websocket(CodexWebSocketHandle::default()),
        )
        .build()?;
    Ok(
        CodexResponsesModel::new(client, "shared-model").with_request_admission(
            codelet_providers::request_admission::AdmissionLimits {
                context_capacity: Some(100_000),
                output_reservation: Some(1000),
            },
        ),
    )
}

fn completed() -> Message {
    Message::Text(
        json!({"type":"response.completed","response":{
            "id":"resp_1","status":"completed","output":[],"end_turn":true,
            "usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}
        }})
        .to_string()
        .into(),
    )
}

#[tokio::test]
async fn rejected_websocket_upgrades_never_downgrade_to_http() -> TestResult {
    for status in [404, 405, 426, 501] {
        // @step Given an explicitly selected Codex WebSocket transport
        let server = wiremock::MockServer::start().await;
        // @step And the scripted provider rejects its upgrade with status <status>
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(status))
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(fixture::terminal("completed", None, None)),
            )
            .mount(&server)
            .await;
        let history = CodexHistory::default();
        let previous = history.snapshot()?;
        // @step When I request a completion
        let model = model(&server.uri())?.with_history(history.clone());
        let error = tokio::time::timeout(Duration::from_secs(2), async {
            let mut stream = model
                .stream(CompletionRequestBuilder::new(model.clone(), "next").build())
                .await?
                .filter(attempt_events::is_content);
            let error = stream
                .next()
                .await
                .ok_or("missing transport failure")?
                .err()
                .ok_or("upgrade silently succeeded")?;
            assert!(stream.next().await.is_none(), "event after failed upgrade");
            Ok::<_, Box<dyn std::error::Error>>(error)
        })
        .await??;
        // @step Then I receive a typed transport failure identifying the rejected upgrade
        let mut source: Option<&(dyn std::error::Error + 'static)> = Some(&error);
        let mut typed = false;
        while let Some(error) = source {
            typed |= error.downcast_ref::<WebSocketUpgradeRejected>().is_some();
            source = error.source();
        }
        assert!(typed, "upgrade lost typed source: {error:?}");
        assert!(error.to_string().contains(&status.to_string()));
        // @step And the scripted provider records zero HTTP completion requests
        let requests = server
            .received_requests()
            .await
            .ok_or("capture unavailable")?;
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.method == "POST")
                .count(),
            0
        );
        // @step And no completion or checkpoint mutation is published
        assert_eq!(history.snapshot()?, previous);
    }
    Ok(())
}

#[tokio::test]
async fn every_explicitly_selected_full_current_dag_websocket_request_sends_full_input(
) -> TestResult {
    // @step Given an explicitly selected full-current-DAG Codex WebSocket session
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}", listener.local_addr()?);
    let task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await?;
        let mut socket = accept_async(stream).await?;
        let mut requests = Vec::new();
        for _ in 0..2 {
            let Some(Ok(Message::Text(body))) = socket.next().await else {
                return Err("request missing".into());
            };
            requests.push(serde_json::from_str::<Value>(&body)?);
            socket.send(completed()).await?;
        }
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(requests)
    });
    // @step When I send its first request and a compatible continuation on the same socket
    let model = model(&url)?;
    let dag = "<system-reminder><!-- type:compaction-dag -->committed DAG</system-reminder>";
    let mut messages = vec![rig::message::Message::user(dag)];
    for suffix in ["inspect", "next suffix"] {
        let request = CompletionRequestBuilder::new(model.clone(), suffix)
            .preamble("mandatory instructions".into())
            .messages(messages.clone())
            .build();
        let mut stream = model.stream(request).await?;
        let mut finals = 0;
        while let Some(event) = stream.next().await {
            if matches!(event?, rig::streaming::StreamedAssistantContent::Final(_)) {
                finals += 1;
            }
        }
        assert_eq!(finals, 1);
        messages.push(rig::message::Message::user(suffix));
    }
    let requests = tokio::time::timeout(Duration::from_secs(2), task)
        .await??
        .map_err(|error| error.to_string())?;
    // @step Then both captured requests contain their complete current DAG input
    for request in &requests {
        let wire = request.to_string();
        for required in ["committed DAG", "inspect", "mandatory instructions"] {
            assert_eq!(
                wire.matches(required).count(),
                1,
                "full context lost {required}"
            );
        }
    }
    assert_eq!(requests[1].to_string().matches("next suffix").count(), 1);
    // @step And neither request establishes or uses a previous-response delta baseline
    assert!(requests
        .iter()
        .all(|request| request.get("previous_response_id").is_none()));
    Ok(())
}

#[tokio::test]
async fn removed_delta_mode_cannot_silently_become_full_input() -> TestResult {
    // @step Given a Codex request explicitly selecting the removed delta mode
    let server = wiremock::MockServer::start().await;
    let model = model(&server.uri())?;
    let request = CompletionRequestBuilder::new(model.clone(), "current DAG")
        .additional_params(json!({"previous_response_id":"removed_delta_baseline"}))
        .build();
    // @step When I prepare the provider request
    let result = model.stream(request).await;
    // @step Then I receive an explicit unsupported-mode failure before transport activity
    assert!(
        result.is_err(),
        "removed previous-response delta selection was accepted"
    );
    // @step And no full-input request or baseline bootstrap is sent
    assert!(server
        .received_requests()
        .await
        .ok_or("capture unavailable")?
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn accepted_websocket_failures_recover_only_on_the_selected_transport() -> TestResult {
    // @step Given an explicitly selected Codex WebSocket request accepted by the provider
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}", listener.local_addr()?);
    let task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await?;
        let mut socket = accept_async(stream).await?;
        let first = socket.next().await.ok_or("missing accepted request")??;
        // @step When the scripted transport disconnects before successful completion
        socket.close(None).await?;
        let (stream, _) = listener.accept().await?;
        let mut socket = accept_async(stream).await?;
        let second = socket.next().await.ok_or("missing replacement request")??;
        socket.send(completed()).await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>((first, second))
    });
    let history = CodexHistory::default();
    let previous = history.snapshot()?;
    let observed = tokio::time::timeout(
        Duration::from_secs(4),
        fixture::collect(model(&url)?.with_history(history.clone())),
    )
    .await??;
    // @step Then a bounded replacement WebSocket request completes successfully
    assert!(observed.errors.is_empty(), "{observed:?}");
    assert_eq!(observed.finals.len(), 1);
    // @step And no HTTP request is sent and both WebSocket requests contain full input
    let (first, second) = task.await?.map_err(|error| error.to_string())?;
    assert_eq!(first, second);
    let request: Value = serde_json::from_str(first.to_text()?)?;
    assert!(request["input"].is_array());
    assert!(request.get("previous_response_id").is_none());
    // @step And the checkpoint changes only after the successful replacement
    assert_ne!(history.snapshot()?, previous);
    Ok(())
}
