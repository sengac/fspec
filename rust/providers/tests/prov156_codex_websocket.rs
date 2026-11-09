//! Feature: spec/features/codex-websocket-transport.feature
use codelet_providers::codex::{
    refreshing_client::RefreshingCodexClient, websocket::CodexWebSocketHandle, CodexResponsesModel,
};
use codelet_providers::request_admission::AdmissionLimits;
use futures::{SinkExt, StreamExt};
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::providers::openai;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};

#[path = "fixtures/attempt_events.rs"]
mod attempt_events;
#[path = "fixtures/prov148.rs"]
pub mod fixture;
type TestResult = Result<(), Box<dyn std::error::Error>>;
type ServerResult = Result<Vec<Value>, Box<dyn std::error::Error + Send + Sync>>;
const DAG: &str = "<system-reminder><!-- type:compaction-dag -->committed DAG</system-reminder>";

fn response(id: &str, tools: bool) -> Value {
    let mut output = vec![
        json!({"type":"message","id":format!("msg_{id}"),"role":"assistant","content":[{"type":"output_text","text":"Hello"}]}),
        json!({"type":"reasoning","id":format!("reasoning_{id}"),"summary":[{"type":"summary_text","text":"Plan"}]}),
    ];
    if tools {
        output.push(json!({"type":"function_call","id":"fc_test","call_id":"call_test","name":"inspect","arguments":"{}","status":"completed"}));
    }
    json!({"type":"response.completed","response":{"id":id,"status":"completed","output":output,"usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}}})
}

async fn setup(
    turns: Vec<usize>,
    tools: bool,
) -> Result<(String, tokio::task::JoinHandle<ServerResult>), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}", listener.local_addr()?);
    let task = tokio::spawn(async move {
        codelet_test_helpers::deadlock_guard("the awaited block", async move {
            let mut requests = Vec::new();
            for count in turns {
                let (stream, _) = listener.accept().await?;
                let mut socket = accept_async(stream).await?;
                for _ in 0..count {
                    let message = socket.next().await.ok_or("request missing")??;
                    requests.push(serde_json::from_str(message.to_text()?)?);
                    socket
                        .send(Message::Text(
                            response(&format!("resp_{}", requests.len()), tools)
                                .to_string()
                                .into(),
                        ))
                        .await?;
                }
            }
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(requests)
        })
        .await?
    });
    Ok((url, task))
}

fn model(
    url: &str,
    handle: CodexWebSocketHandle,
) -> Result<CodexResponsesModel, Box<dyn std::error::Error>> {
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(url)
        .http_client(RefreshingCodexClient::new_api_key().with_websocket(handle))
        .build()?;
    Ok(
        CodexResponsesModel::new(client, "model").with_request_admission(AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        }),
    )
}

async fn complete(
    model: &CodexResponsesModel,
    prompt: &str,
    messages: &mut Vec<rig::message::Message>,
) -> TestResult {
    let request = CompletionRequestBuilder::new(model.clone(), prompt)
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
    messages.push(rig::message::Message::user(prompt));
    messages.push(rig::message::Message::assistant("Hello"));
    Ok(())
}

#[tokio::test]
async fn enabled_sessions_reuse_sockets_with_full_current_dag_input() -> TestResult {
    codelet_test_helpers::deadlock_guard("the awaited block", async {
        // @step Given an explicitly enabled Codex session and a local Responses socket
        let (url, task) = setup(vec![2], false).await?;
        let model = model(&url, CodexWebSocketHandle::default())?;
        let mut messages = vec![rig::message::Message::user(DAG)];
        // @step When two compatible requests extend the current DAG
        for prompt in ["inspect", "next suffix"] {
            complete(&model, prompt, &mut messages).await?;
        }
        let requests = task.await?.map_err(|error| error.to_string())?;
        // @step Then one connection carries both complete inputs without a previous response identity
        assert_eq!(requests.len(), 2);
        for request in &requests {
            assert_eq!(request["type"], "response.create");
            assert!(request.get("previous_response_id").is_none());
            for required in ["committed DAG", "mandatory instructions", "inspect"] {
                assert_eq!(request.to_string().matches(required).count(), 1);
            }
        }
        assert_eq!(requests[1].to_string().matches("next suffix").count(), 1);
        Ok(())
    })
    .await?
}

#[tokio::test]
async fn changed_history_and_reset_preserve_full_current_input() -> TestResult {
    codelet_test_helpers::deadlock_guard("the awaited block", async {
        // @step Given a successful reusable Codex socket response
        let (url, task) = setup(vec![2, 1], false).await?;
        let handle = CodexWebSocketHandle::default();
        let model = model(&url, handle.clone())?;
        let mut messages = vec![rig::message::Message::user(DAG)];
        complete(&model, "initial", &mut messages).await?;
        // @step When history changes and the session socket resets before another request
        complete(&model, "changed history", &mut messages).await?;
        handle.reset().await;
        complete(&model, "replacement", &mut messages).await?;
        let requests = task.await?.map_err(|error| error.to_string())?;
        // @step Then the next request contains full input without a stale previous response identity
        assert_eq!(requests.len(), 3);
        for (request, prompt) in requests
            .iter()
            .zip(["initial", "changed history", "replacement"])
        {
            assert!(request.get("previous_response_id").is_none());
            assert!(request.to_string().contains(prompt));
            assert!(request.to_string().contains("committed DAG"));
            assert!(!request["input"].as_array().ok_or("input")?.is_empty());
        }
        Ok(())
    })
    .await?
}

#[tokio::test]
async fn unsupported_handshakes_fail_without_transport_downgrade() -> TestResult {
    codelet_test_helpers::deadlock_guard("the awaited block", async {
        // @step Given a Codex server rejecting WebSocket upgrade as unsupported
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(426))
            .expect(1)
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(fixture::terminal("completed", None, None)),
            )
            .expect(0)
            .mount(&server)
            .await;
        // @step When an enabled session samples a response
        let observed =
            fixture::collect(model(&server.uri(), CodexWebSocketHandle::default())?).await?;
        // @step Then the rejected upgrade remains visible and no SSE request or completion is published
        assert_eq!(observed.errors.len(), 1);
        assert!(observed.errors[0].contains("426"), "{observed:?}");
        assert!(observed.finals.is_empty());
        assert!(observed.text.is_empty());
        assert_eq!(observed.tools, 0);
        assert_eq!(server.received_requests().await.ok_or("requests")?.len(), 1);
        Ok(())
    })
    .await?
}

#[tokio::test]
async fn authentication_fails_while_accepted_disconnects_recover_on_websocket() -> TestResult {
    codelet_test_helpers::deadlock_guard("the awaited block", async {
        // @step Given a rejected credential or a socket interrupted after accepting a request
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(401))
            .expect(1)
            .mount(&server)
            .await;
        // @step When the transport reports the failure
        let observed =
            fixture::collect(model(&server.uri(), CodexWebSocketHandle::default())?).await?;
        // @step Then authentication remains terminal while a bounded full-input WebSocket replacement completes without SSE
        assert_eq!(observed.errors.len(), 1);
        assert!(observed.finals.is_empty());
        assert_eq!(server.received_requests().await.ok_or("requests")?.len(), 1);
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}", listener.local_addr()?);
        let task = tokio::spawn(codelet_test_helpers::deadlock_guard(
            "the awaited block",
            async move {
                let mut requests = Vec::new();
                for attempt in 0..2 {
                    let (stream, _) = listener.accept().await?;
                    let mut socket = accept_async(stream).await?;
                    let request = socket.next().await.ok_or("request missing")??;
                    requests.push(serde_json::from_str::<Value>(request.to_text()?)?);
                    if attempt == 0 {
                        socket.close(None).await?;
                    } else {
                        socket
                            .send(Message::Text(
                                response("replacement", false).to_string().into(),
                            ))
                            .await?;
                    }
                }
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(requests)
            },
        ));
        let observed = fixture::collect(model(&url, CodexWebSocketHandle::default())?).await?;
        assert!(observed.errors.is_empty(), "{observed:?}");
        assert_eq!(observed.finals.len(), 1);
        let requests = task.await??.map_err(|error| error.to_string())?;
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0], requests[1]);
        assert_eq!(requests[0]["type"], "response.create");
        assert!(requests[0].get("previous_response_id").is_none());
        assert!(!requests[0]["input"].as_array().ok_or("input")?.is_empty());
        Ok(())
    })
    .await?
}

#[tokio::test]
async fn cancellation_retires_the_socket_within_a_bounded_deadline() -> TestResult {
    codelet_test_helpers::deadlock_guard("the awaited block", async {
        // @step Given an accepted Codex socket streaming partial output
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}", listener.local_addr()?);
        let task = tokio::spawn(codelet_test_helpers::deadlock_guard("the awaited block", async move {
            let (stream, _) = listener.accept().await?;
            let mut socket = accept_async(stream).await?;
            socket.next().await.ok_or("initial request missing")??;
            socket.send(Message::Text(json!({"type":"response.output_text.delta","item_id":"msg_partial","output_index":0,"content_index":0,"delta":"partial"}).to_string().into())).await?;
            let (fresh, _) = codelet_test_helpers::deadlock_guard("listener.accept()", listener.accept()).await??;
            let mut fresh = accept_async(fresh).await?;
            let request = fresh.next().await.ok_or("fresh request missing")??;
            fresh.send(Message::Text(response("resp_fresh", false).to_string().into())).await?;
            Ok::<Value, Box<dyn std::error::Error + Send + Sync>>(serde_json::from_str(request.to_text()?)?)
        }));
        let model = model(&url, CodexWebSocketHandle::default())?;
        let mut stream = model.stream(CompletionRequestBuilder::new(model.clone(), "hello").build()).await?.filter(attempt_events::is_content);
        assert!(stream.next().await.transpose()?.is_some());
        // @step When its consumer cancels without server acknowledgement
        drop(stream);
        // @step Then the socket is retired promptly and the next request starts a fresh full exchange
        let next = codelet_test_helpers::deadlock_guard("fixture::collect(model)", fixture::collect(model)).await??;
        assert!(next.errors.is_empty(), "{next:?}");
        assert_eq!(next.text, "Hello");
        let request = task.await??.map_err(|error| error.to_string())?;
        assert!(request.get("previous_response_id").is_none());
        assert!(!request["input"].as_array().ok_or("input")?.is_empty());
        Ok(())
    }).await?
}

#[tokio::test]
async fn socket_and_sse_transcripts_preserve_observable_provider_behavior() -> TestResult {
    codelet_test_helpers::deadlock_guard("the awaited block", async {
        // @step Given equivalent Codex transcripts over a socket and SSE and an ordinary Responses client
        let (url, task) = setup(vec![1], true).await?;
        let socket = model(&url, CodexWebSocketHandle::default())?;
        let server =
            fixture::Server::start(vec![fixture::event(response("resp_1", true)); 2], false)
                .await?;
        // @step When each consumes text reasoning usage and tool output
        let websocket = fixture::collect(socket).await?;
        let (sse, ordinary) = server.models()?;
        let http = fixture::collect(sse).await?;
        let _ = fixture::collect(ordinary).await?;
        task.await?.map_err(|error| error.to_string())?;
        // @step Then Codex produces equivalent observable events and ordinary Responses remains on HTTP
        assert!(websocket.errors.is_empty(), "{websocket:?}");
        assert!(http.errors.is_empty(), "{http:?}");
        assert_eq!(websocket.text, http.text);
        assert_eq!(websocket.tools, http.tools);
        assert_eq!(websocket.tools, 1);
        assert_eq!(websocket.reasoning, "Plan");
        assert_eq!(websocket.reasoning, http.reasoning);
        assert_eq!(websocket.usage, vec![20]);
        assert_eq!(websocket.usage, http.usage);
        assert_eq!(websocket.finals, vec![20]);
        assert_eq!(websocket.finals, http.finals);
        assert_eq!(
            server.requests.lock().map_err(|_| "requests lock")?.len(),
            2
        );
        Ok(())
    })
    .await?
}
