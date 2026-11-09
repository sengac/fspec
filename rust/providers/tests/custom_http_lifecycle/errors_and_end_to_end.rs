//! custom_http_lifecycle_tests.rs: HTTP error mapping, Rhai runtime errors and the end-to-end request.

use super::*;

// =========================================================================
// Scenario: Map HTTP 401 to auth error
// =========================================================================
#[tokio::test]
async fn map_http_401_to_auth_error() {
    // @step Given a Rhai script whose map_error returns an auth error for status 401
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_string("{\"error\":\"unauthorized\"}"))
        .mount(&server)
        .await;

    let script = format!(
        r#"
fn build_request(request) {{ #{{ messages: request.messages }} }}
fn build_headers(config) {{ #{{ "Content-Type": "application/json" }} }}
fn build_url(config) {{ "{base}/v1/chat/completions" }}
fn parse_response(raw) {{ #{{ content: "", stop_reason: "end_turn" }} }}
fn parse_stream_chunk(chunk) {{ #{{}} }}
fn build_stream_request(ctx) {{ #{{}} }}
fn map_error(status, body) {{
    if status == 401 {{
        #{{ type: "auth", message: "unauthorized" }}
    }} else {{
        #{{ type: "api", message: "other" }}
    }}
}}
"#,
        base = server.uri()
    );
    let (_tmp, cfg) = config_with_full_script("my-llm", &server.uri(), "model-smart-v2", &script);
    let provider = build_provider(cfg, "smart");

    // @step When the HTTP response returns status 401 with body "{\"error\":\"unauthorized\"}"
    let messages = vec![Message::user("hi")];
    let err = provider
        .complete_with_tools(&messages, &[])
        .await
        .expect_err("expected auth error");

    // @step Then I receive ProviderError::Auth whose message contains "unauthorized"
    match err {
        ProviderError::Authentication { message, .. } => {
            assert!(
                message.contains("unauthorized"),
                "message did not contain 'unauthorized': {message}"
            );
        }
        other => panic!("expected Authentication error, got {other:?}"),
    }
}

// =========================================================================
// Scenario: Map HTTP 429 to rate limit error
// =========================================================================
#[tokio::test]
async fn map_http_429_to_rate_limit_error() {
    // @step Given a Rhai script whose map_error returns rate_limit for status 429
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(429).set_body_string("slow down"))
        .mount(&server)
        .await;

    let script = format!(
        r#"
fn build_request(request) {{ #{{ messages: request.messages }} }}
fn build_headers(config) {{ #{{ "Content-Type": "application/json" }} }}
fn build_url(config) {{ "{base}/v1/chat/completions" }}
fn parse_response(raw) {{ #{{ content: "", stop_reason: "end_turn" }} }}
fn parse_stream_chunk(chunk) {{ #{{}} }}
fn build_stream_request(ctx) {{ #{{}} }}
fn map_error(status, body) {{
    if status == 429 {{
        #{{ type: "rate_limit", message: "rate limited" }}
    }} else {{
        #{{ type: "api", message: "other" }}
    }}
}}
"#,
        base = server.uri()
    );
    let (_tmp, cfg) = config_with_full_script("my-llm", &server.uri(), "model-smart-v2", &script);
    let provider = build_provider(cfg, "smart");

    // @step When the HTTP response returns status 429
    let messages = vec![Message::user("hi")];
    let err = provider
        .complete_with_tools(&messages, &[])
        .await
        .expect_err("expected rate limit error");

    // @step Then I receive ProviderError::RateLimit
    assert!(
        matches!(err, ProviderError::RateLimit { .. }),
        "expected RateLimit, got {err:?}"
    );
}

// =========================================================================
// Scenario: Surface Rhai runtime errors as provider errors
// =========================================================================
#[tokio::test]
async fn surface_rhai_runtime_errors_as_provider_errors() {
    // @step Given a Rhai script whose parse_response throws a runtime error
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"choices\":[]}"))
        .mount(&server)
        .await;

    let script = format!(
        r#"
fn build_request(request) {{ #{{ messages: request.messages }} }}
fn build_headers(config) {{ #{{ "Content-Type": "application/json" }} }}
fn build_url(config) {{ "{base}/v1/chat/completions" }}
fn parse_response(raw) {{ throw "boom from parse_response"; }}
fn parse_stream_chunk(chunk) {{ #{{}} }}
fn build_stream_request(ctx) {{ #{{}} }}
fn map_error(status, body) {{ #{{ type: "api", message: "other" }} }}
"#,
        base = server.uri()
    );
    let (_tmp, cfg) = config_with_full_script("my-llm", &server.uri(), "model-smart-v2", &script);
    let provider = build_provider(cfg, "smart");

    // @step When I complete with that provider
    let messages = vec![Message::user("hi")];
    let err = provider
        .complete_with_tools(&messages, &[])
        .await
        .expect_err("expected runtime error surfaced as provider error");

    // @step Then I receive ProviderError::Api and the process does not crash
    match err {
        ProviderError::Api { message, .. } => {
            assert!(
                message.contains("boom") || message.to_lowercase().contains("parse_response"),
                "expected message to reference the Rhai error; got: {message}"
            );
        }
        other => panic!("expected Api error from Rhai runtime failure, got {other:?}"),
    }
}

// =========================================================================
// Scenario: Complete end-to-end request against mock server
// =========================================================================
#[tokio::test]
async fn complete_end_to_end_request_against_mock_server() {
    // @step Given a wiremock server responding to /v1/chat/completions with a valid OpenAI-style success payload
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [
                {
                    "message": { "content": "pong" },
                    "finish_reason": "stop"
                }
            ]
        })))
        .mount(&server)
        .await;

    let script = format!(
        r#"
fn build_request(request) {{
    #{{ messages: request.messages }}
}}
fn build_headers(config) {{ #{{ "Content-Type": "application/json" }} }}
fn build_url(config) {{ "{base}/v1/chat/completions" }}
fn parse_response(raw) {{
    #{{
        content: raw.choices[0].message.content,
        stop_reason: "end_turn"
    }}
}}
fn parse_stream_chunk(chunk) {{ #{{}} }}
fn build_stream_request(ctx) {{ #{{}} }}
fn map_error(status, body) {{ #{{ type: "api", message: "other" }} }}
"#,
        base = server.uri()
    );
    let (_tmp, cfg) = config_with_full_script("my-llm", &server.uri(), "model-smart-v2", &script);
    let provider = build_provider(cfg, "smart");

    // @step When RhaiCustomProvider.complete_with_tools is called with a single user message
    let messages = vec![Message::user("ping")];
    let response = provider
        .complete_with_tools(&messages, &[])
        .await
        .expect("completion succeeds");

    // @step Then the returned CompletionResponse contains the mock server's content text
    let text = match &response.content {
        MessageContent::Text(t) => t.clone(),
        MessageContent::Parts(parts) => parts
            .iter()
            .filter_map(|p| match p {
                ContentPart::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(""),
    };
    assert_eq!(text, "pong");
    assert_eq!(response.stop_reason, StopReason::EndTurn);
}
