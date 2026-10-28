use codelet_providers::codex::{
    metadata::CodexMetadataHandle, refreshing_client::RefreshingCodexClient,
    websocket::CodexWebSocketHandle, CodexResponsesModel,
};
use codelet_providers::request_admission::AdmissionLimits;
use futures::{SinkExt, StreamExt};
use rig::providers::openai;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio_tungstenite::{accept_hdr_async, tungstenite::Message};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[path = "prov148.rs"]
pub mod baseline;
pub use baseline::{event, terminal, text, TestResult};

pub async fn consume(
    events: Vec<Value>,
    websocket: bool,
    headers: &[(&str, &str)],
    metadata: CodexMetadataHandle,
) -> TestResult {
    let body = events.into_iter().map(event).collect::<String>()
        + text().as_str()
        + terminal("completed", None, None).as_str();
    let mut transport = RefreshingCodexClient::new_api_key();
    let captured = Arc::new(Mutex::new(Vec::<Value>::new()));
    let mut socket_task = None;
    let mut http_server = None;
    let url = if websocket {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}", listener.local_addr()?);
        let recorded = captured.clone();
        let mut response_headers = http::HeaderMap::new();
        for (name, value) in headers {
            response_headers.insert(
                http::HeaderName::from_bytes(name.as_bytes())?,
                http::HeaderValue::from_str(value)?,
            );
        }
        socket_task = Some(tokio::spawn(async move {
            let (stream, _) = listener.accept().await?;
            #[allow(
                clippy::result_large_err,
                reason = "tungstenite requires an unboxed HTTP ErrorResponse in its handshake callback"
            )]
            let mut socket = accept_hdr_async(
                stream,
                move |_: &http::Request<()>, mut response: http::Response<()>| {
                    response.headers_mut().extend(response_headers);
                    Ok(response)
                },
            )
            .await?;
            let request = socket.next().await.ok_or("missing socket request")??;
            recorded
                .lock()
                .map_err(|_| "request capture poisoned")?
                .push(serde_json::from_str(request.to_text()?)?);
            for event in body
                .split("\n\n")
                .filter_map(|event| event.strip_prefix("data: "))
            {
                socket.send(Message::Text(event.to_owned().into())).await?;
            }
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        }));
        transport = transport.with_websocket(CodexWebSocketHandle::default());
        url
    } else {
        let server = MockServer::start().await;
        let mut response = ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string(body);
        for (name, value) in headers {
            response = response.insert_header(*name, *value);
        }
        Mock::given(method("POST"))
            .respond_with(response)
            .mount(&server)
            .await;
        let url = server.uri();
        http_server = Some(server);
        url
    };
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(url)
        .http_client(transport)
        .build()?;
    let model = CodexResponsesModel::new(client, "selected-model")
        .with_metadata_handle(metadata)
        .with_request_admission(AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        });
    let observed =
        tokio::time::timeout(std::time::Duration::from_secs(5), baseline::collect(model)).await??;
    assert!(observed.errors.is_empty(), "{observed:?}");
    assert_eq!(observed.text, "Hello");
    assert_eq!(observed.usage, vec![20]);
    assert_eq!(observed.finals, vec![20]);
    if let Some(task) = socket_task {
        task.await?.map_err(|error| error.to_string())?;
        assert_eq!(captured.lock().map_err(|_| "capture poisoned")?.len(), 1);
        assert_eq!(
            captured.lock().map_err(|_| "capture poisoned")?[0]["model"],
            "selected-model"
        );
    }
    if let Some(server) = http_server {
        let requests = server.received_requests().await.ok_or("missing requests")?;
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].body_json::<Value>()?["model"], "selected-model");
    }
    Ok(())
}

pub fn retry_attempts_and_treatments(buffering: Value) -> TestResult {
    use codelet_providers::codex::metadata::CodexPresentationEvent;
    let handle = CodexMetadataHandle::default();
    let presented = Arc::new(Mutex::new(Vec::new()));
    let recorded = presented.clone();
    handle.set_presentation_observer(Arc::new(move |event| {
        if let Ok(mut recorded) = recorded.lock() {
            recorded.push(event);
        }
    }));
    handle.apply_event(&buffering);
    handle.begin_response();
    let mut checks = vec![("new attempt clears buffering", !handle.status().buffering)];
    handle.apply_event(&buffering);
    assert!(handle.status().buffering);
    for enabled in ["true", "false"] {
        handle.apply_event(&serde_json::json!({"headers":{
            "x-codex-safety-buffering-faster-model":"previous-model"
        }}));
        handle.apply_event(&buffering);
        assert_eq!(
            handle.status().faster_model.as_deref(),
            Some("previous-model")
        );
        handle.apply_event(&serde_json::json!({"headers":{
            "x-codex-safety-buffering-enabled":enabled
        }}));
        handle.apply_event(&buffering);
        checks.push((enabled, handle.status().faster_model.is_none()));
    }
    assert_eq!(
        checks,
        vec![
            ("new attempt clears buffering", true),
            ("true", true),
            ("false", true)
        ]
    );
    assert_eq!(
        *presented.lock().map_err(|_| "presentation lock poisoned")?,
        vec![
            CodexPresentationEvent::Waiting(false),
            CodexPresentationEvent::Waiting(true),
            CodexPresentationEvent::Waiting(false),
            CodexPresentationEvent::Waiting(true)
        ]
    );
    handle.apply_event(&serde_json::json!({"type":"response.output_text.delta","delta":"Hello"}));
    handle.begin_response();
    handle.apply_event(&buffering);
    assert!(!handle.status().buffering);
    Ok(())
}

pub async fn tool_only_completion(metadata: CodexMetadataHandle) -> TestResult {
    let raw_terminal = baseline::terminal("completed", None, None);
    let mut terminal: Value = serde_json::from_str(
        raw_terminal
            .strip_prefix("data: ")
            .ok_or("terminal missing data prefix")?
            .trim(),
    )?;
    terminal["response"]["output"] = serde_json::json!([{
        "type":"function_call", "id":"fc_test", "call_id":"call_test",
        "name":"inspect", "arguments":"{}", "status":"completed"
    }]);
    let body = [baseline::tool(), baseline::event(terminal)].concat();
    let server = baseline::Server::start(vec![body], false).await?;
    let observed = baseline::collect(server.models()?.0.with_metadata_handle(metadata)).await?;
    assert!(observed.errors.is_empty(), "{observed:?}");
    assert!(observed.text.is_empty());
    assert_eq!(observed.tools, 1);
    Ok(())
}

pub async fn transport_header_advice(buffering: Value) -> TestResult {
    let mut observed = Vec::new();
    let mut expected = Vec::new();
    for websocket in [false, true] {
        for source in ["physical", "top-level", "nested", "conflicting"] {
            let handle = CodexMetadataHandle::default();
            let advice = Arc::new(Mutex::new(Vec::new()));
            let recorded = advice.clone();
            handle.set_observer(Arc::new(move |status| {
                if status.buffering {
                    if let Ok(mut recorded) = recorded.lock() {
                        recorded.push(status.faster_model);
                    }
                }
            }));
            let mut update = serde_json::json!({"type":"response.metadata"});
            let top_level = matches!(source, "top-level" | "conflicting");
            let nested = matches!(source, "nested" | "conflicting");
            if top_level {
                update["headers"] = serde_json::json!({
                    "x-codex-safety-buffering-faster-model":"event-advice",
                    "openai-model":"event-serving-model",
                    "x-request-id":"event-request"
                });
            }
            if nested {
                update["response"] = serde_json::json!({"headers":{
                    "x-codex-safety-buffering-faster-model":"nested-advice",
                    "openai-model":"nested-serving-model",
                    "x-request-id":"nested-request"
                }});
            }
            consume(
                vec![update, buffering.clone()],
                websocket,
                &[
                    ("x-codex-safety-buffering-faster-model", "physical-advice"),
                    ("openai-model", "physical-serving-model"),
                    ("x-request-id", "physical-request"),
                ],
                handle.clone(),
            )
            .await?;
            let diagnostic_source = if nested {
                "nested"
            } else if top_level {
                "event"
            } else {
                "physical"
            };
            assert_eq!(
                handle.status().server_model,
                Some(format!("{diagnostic_source}-serving-model")),
                "diagnostic model precedence: websocket={websocket}, source={source}"
            );
            assert_eq!(
                handle.status().request_id,
                Some(format!("{diagnostic_source}-request")),
                "diagnostic request precedence: websocket={websocket}, source={source}"
            );
            let expected_advice = if !websocket {
                Some("physical-advice".to_owned())
            } else {
                top_level.then(|| "event-advice".to_owned())
            };
            observed.push((
                websocket,
                source,
                advice
                    .lock()
                    .map_err(|_| "advice observer poisoned")?
                    .clone(),
            ));
            expected.push((websocket, source, vec![expected_advice]));
        }
    }
    assert_eq!(observed, expected, "transport-owned buffering advice");
    Ok(())
}
