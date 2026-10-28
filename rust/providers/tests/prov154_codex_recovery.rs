//! Feature: spec/features/codex-stream-recovery.feature
mod fixtures;
#[path = "fixtures/prov148.rs"]
pub mod stream_fixture;

use codelet_providers::codex::refreshing_client::RefreshingCodexClient;
use futures::StreamExt;
use http::Request;
use rig::http_client::HttpClientExt;
use serial_test::serial;
use std::time::{Duration, Instant};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn request(server: &MockServer) -> Result<Request<Vec<u8>>, http::Error> {
    Request::builder()
        .method("POST")
        .uri(format!("{}/recovery", server.uri()))
        .body(b"{}".to_vec())
}

#[tokio::test]
#[serial]
async fn unauthorized_setup_refreshes_credentials_once() -> TestResult {
    // @step Given valid-looking OAuth credentials rejected by the server
    // @step When refreshed credentials succeed or are rejected again
    // @step Then one refresh occurs and at most two setup attempts are made
    let (_home, _guard) = fixtures::setup_codex_home()?;
    for refreshed_status in [200, 401] {
        let server = MockServer::start().await;
        Mock::given(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                fixtures::build_token_response_json("id", "fresh-secret", "refresh-secret"),
            ))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(path("/recovery"))
            .and(header("authorization", "Bearer old-secret"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(path("/recovery"))
            .and(header("authorization", "Bearer fresh-secret"))
            .respond_with(ResponseTemplate::new(refreshed_status))
            .expect(1)
            .mount(&server)
            .await;
        let client = RefreshingCodexClient::new_oauth(
            "old-secret".into(),
            "refresh-secret".into(),
            "account".into(),
            Some(3600),
            server.uri(),
        );
        let outcome = client.send_streaming(request(&server)?).await;
        assert_eq!(outcome.is_ok(), refreshed_status == 200);
        if let Err(error) = outcome {
            assert!(!error.to_string().contains("secret"));
        }
        server.verify().await;
    }
    Ok(())
}

#[tokio::test]
async fn safe_retries_honor_advice_and_stop_within_budget() -> TestResult {
    // @step Given setup rejected with rate limiting, unavailable service, gateway failure or a permanent error
    // @step When Codex attempts recovery
    // @step Then retryable rejections use bounded delays and permanent errors never retry
    for (status, attempts) in [
        (429, 3),
        (502, 3),
        (503, 3),
        (504, 3),
        (400, 1),
        (403, 1),
        (500, 1),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(status).insert_header("retry-after", "0"))
            .expect(attempts)
            .mount(&server)
            .await;
        let result = RefreshingCodexClient::new_api_key()
            .send_streaming(request(&server)?)
            .await;
        assert!(result.is_err());
        server.verify().await;
    }
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "1"))
        .expect(3)
        .mount(&server)
        .await;
    let start = Instant::now();
    let result = RefreshingCodexClient::new_api_key()
        .send_streaming(request(&server)?)
        .await;
    assert!(result.is_err());
    assert!(start.elapsed() >= Duration::from_secs(2));
    server.verify().await;
    Ok(())
}

#[tokio::test]
async fn cancellation_aborts_network_waits_and_retry_backoff() -> TestResult {
    // @step Given a Codex request waiting on network data or retry advice
    // @step When the caller cancels the stream
    // @step Then no further requests occur and cancellation finishes promptly
    for status in [200, 429] {
        let server = MockServer::start().await;
        let response = ResponseTemplate::new(status).insert_header("retry-after", "1");
        let response = if status == 200 {
            response.set_delay(Duration::from_secs(10))
        } else {
            response
        };
        Mock::given(method("POST"))
            .respond_with(response)
            .expect(1)
            .mount(&server)
            .await;
        let client = RefreshingCodexClient::new_api_key();
        let result = tokio::time::timeout(
            Duration::from_millis(100),
            client.send_streaming(request(&server)?),
        )
        .await;
        assert!(result.is_err(), "cancellation must drop a pending wait");
        tokio::time::sleep(Duration::from_millis(1100)).await;
        server.verify().await;
    }
    Ok(())
}

#[tokio::test]
async fn accepted_streams_never_replay_tool_effects() -> TestResult {
    // @step Given a Codex continuation containing a completed tool result
    use codelet_providers::codex::{history::CodexHistory, CodexResponsesModel};
    use rig::{
        completion::{CompletionModel, CompletionRequestBuilder},
        message::{Message, ToolResultContent, UserContent},
        OneOrMany,
    };
    use serde_json::json;
    let server = stream_fixture::Server::start(
        vec![
            [
                stream_fixture::tool(),
                stream_fixture::terminal("completed", Some(true), None),
            ]
            .concat(),
            stream_fixture::text(),
            stream_fixture::terminal("completed", Some(true), None),
        ],
        false,
    )
    .await?;
    let history = CodexHistory::default();
    let model: CodexResponsesModel = server.models()?.0.with_history(history.clone());
    let mut initial = model
        .stream(CompletionRequestBuilder::new(model.clone(), "Hello").build())
        .await?;
    while let Some(event) = initial.next().await {
        event?;
    }
    let result = Message::User {
        content: OneOrMany::one(UserContent::tool_result_with_call_id(
            "fc_test",
            "call_test".into(),
            OneOrMany::one(ToolResultContent::text("done")),
        )),
    };
    let request = CompletionRequestBuilder::new(model.clone(), result)
        .messages(vec![Message::user("Hello"), Message::assistant("Hello")])
        .build();
    // @step When its accepted response disconnects
    let mut stream = model.stream(request).await?;
    let mut tools = 0;
    while let Some(event) = stream.next().await {
        if matches!(
            event?,
            rig::streaming::StreamedAssistantContent::ToolCall(_)
        ) {
            tools += 1;
        }
    }
    // @step Then same-transport replacement retains the completed result without executing that tool again
    assert_eq!(tools, 0);
    let requests = server.requests.lock().map_err(|_| "capture unavailable")?;
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[1], requests[2]);
    assert!(requests[2]["input"]
        .as_array()
        .ok_or("missing input")?
        .iter()
        .any(|item| item["type"] == json!("function_call_output") && item["output"] == "done"));
    Ok(())
}
