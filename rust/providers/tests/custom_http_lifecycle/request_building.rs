//! custom_http_lifecycle_tests.rs: request body, URL, header and response parsing scenarios.

use super::*;

// =========================================================================
// Scenario: Build request body from messages
// Lines: pinned below for link-coverage.
// =========================================================================
#[tokio::test]
async fn build_request_body_from_messages() {
    // @step Given a Rhai script whose build_request produces {messages:[{role:"user",content:"hi"}]}
    let script = r#"
fn build_request(request) {
    #{ messages: [ #{ role: "user", content: "hi" } ] }
}
fn build_headers(config) { #{} }
fn build_url(config) { "" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn parse_stream_chunk(chunk) { #{} }
fn build_stream_request(ctx) { #{} }
fn map_error(status, body) { #{ type: "api", message: "" } }
"#;
    let (_tmp, cfg) = config_with_script("my-llm", script);
    let provider = build_provider(cfg, "smart");

    // @step When I call the request_bridge with a single user message "hi"
    let messages = vec![Message::user("hi")];
    let body_json = provider
        .invoke_build_request(&messages, &[], None)
        .await
        .expect("build_request returns JSON");

    // @step Then the resulting JSON body contains messages array with role "user" and content "hi"
    let messages_arr = body_json
        .get("messages")
        .and_then(|v| v.as_array())
        .expect("messages array present");
    assert_eq!(messages_arr.len(), 1);
    let first = &messages_arr[0];
    assert_eq!(first.get("role").and_then(|v| v.as_str()), Some("user"));
    assert_eq!(first.get("content").and_then(|v| v.as_str()), Some("hi"));
}

// =========================================================================
// Scenario: Build request URL from config
// =========================================================================
#[tokio::test]
async fn build_request_url_from_config() {
    // @step Given a config with base_url "https://api.example.com" and a script build_url returning "/v1/chat/completions"
    let script = r#"
fn build_request(request) { #{} }
fn build_headers(config) { #{} }
fn build_url(config) { config.base_url + "/v1/chat/completions" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn parse_stream_chunk(chunk) { #{} }
fn build_stream_request(ctx) { #{} }
fn map_error(status, body) { #{ type: "api", message: "" } }
"#;
    let (_tmp, cfg) = config_with_full_script(
        "my-llm",
        "https://api.example.com",
        "model-smart-v2",
        script,
    );
    let provider = build_provider(cfg, "smart");

    // @step When RhaiCustomProvider resolves the target URL
    let url = provider
        .invoke_build_url()
        .await
        .expect("build_url returns");

    // @step Then the URL equals "https://api.example.com/v1/chat/completions"
    assert_eq!(url, "https://api.example.com/v1/chat/completions");
}

// =========================================================================
// Scenario: Build HTTP headers including auth
// =========================================================================
#[tokio::test]
async fn build_http_headers_including_auth() {
    // @step Given a Rhai script whose build_headers returns a map with Authorization and Content-Type
    let script = r#"
fn build_request(request) { #{} }
fn build_headers(config) {
    #{
        "Authorization": "Bearer sk-xxx",
        "Content-Type": "application/json"
    }
}
fn build_url(config) { "" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn parse_stream_chunk(chunk) { #{} }
fn build_stream_request(ctx) { #{} }
fn map_error(status, body) { #{ type: "api", message: "" } }
"#;
    let (_tmp, cfg) = config_with_script("my-llm", script);
    let provider = build_provider(cfg, "smart");

    // @step When RhaiCustomProvider assembles outgoing HTTP headers
    let header_map = provider
        .invoke_build_headers()
        .await
        .expect("build_headers returns HeaderMap");

    // @step Then the HeaderMap contains Authorization "Bearer sk-xxx" and Content-Type "application/json"
    let auth = header_map
        .get("Authorization")
        .expect("Authorization header present");
    assert_eq!(auth.to_str().unwrap(), "Bearer sk-xxx");
    let ct = header_map
        .get("Content-Type")
        .expect("Content-Type header present");
    assert_eq!(ct.to_str().unwrap(), "application/json");
}

// =========================================================================
// Scenario: Parse plain text response
// =========================================================================
#[tokio::test]
async fn parse_plain_text_response() {
    // @step Given a Rhai script whose parse_response extracts content from choices[0].message.content and finish_reason
    let script = r#"
fn build_request(request) { #{} }
fn build_headers(config) { #{} }
fn build_url(config) { "" }
fn parse_response(raw) {
    let choice = raw.choices[0];
    let stop = if choice.finish_reason == "stop" { "end_turn" }
               else if choice.finish_reason == "tool_calls" { "tool_use" }
               else if choice.finish_reason == "length" { "max_tokens" }
               else { "end_turn" };
    #{
        content: choice.message.content,
        stop_reason: stop
    }
}
fn parse_stream_chunk(chunk) { #{} }
fn build_stream_request(ctx) { #{} }
fn map_error(status, body) { #{ type: "api", message: "" } }
"#;
    let (_tmp, cfg) = config_with_script("my-llm", script);
    let provider = build_provider(cfg, "smart");

    // @step When I parse the JSON {choices:[{message:{content:"hello"},finish_reason:"stop"}]}
    let json = serde_json::json!({
        "choices": [ { "message": { "content": "hello" }, "finish_reason": "stop" } ]
    });
    let response: CompletionResponse = provider
        .invoke_parse_response(&json)
        .await
        .expect("parse_response succeeds");

    // @step Then the CompletionResponse has content text "hello" and stop_reason EndTurn
    match &response.content {
        MessageContent::Text(t) => assert_eq!(t, "hello"),
        MessageContent::Parts(parts) => {
            assert_eq!(parts.len(), 1);
            match &parts[0] {
                ContentPart::Text { text } => assert_eq!(text, "hello"),
                other => panic!("expected text part, got {other:?}"),
            }
        }
    }
    assert_eq!(response.stop_reason, StopReason::EndTurn);
}

// =========================================================================
// Scenario: Parse tool call response
// =========================================================================
#[tokio::test]
async fn parse_tool_call_response() {
    // @step Given a Rhai script parsing a tool_call with name "read_file" and input {path:"a.txt"}
    let script = r#"
fn build_request(request) { #{} }
fn build_headers(config) { #{} }
fn build_url(config) { "" }
fn parse_response(raw) {
    let tc = raw.choices[0].message.tool_calls[0];
    #{
        content: [
            #{
                type: "tool_use",
                id: tc.id,
                name: tc.function.name,
                input: #{ path: "a.txt" }
            }
        ],
        stop_reason: "tool_use"
    }
}
fn parse_stream_chunk(chunk) { #{} }
fn build_stream_request(ctx) { #{} }
fn map_error(status, body) { #{ type: "api", message: "" } }
"#;
    let (_tmp, cfg) = config_with_script("my-llm", script);
    let provider = build_provider(cfg, "smart");

    // @step When I parse the response body
    let json = serde_json::json!({
        "choices": [
            {
                "message": {
                    "content": null,
                    "tool_calls": [
                        {
                            "id": "call_1",
                            "function": {
                                "name": "read_file",
                                "arguments": "{\"path\":\"a.txt\"}"
                            }
                        }
                    ]
                },
                "finish_reason": "tool_calls"
            }
        ]
    });
    let response: CompletionResponse = provider
        .invoke_parse_response(&json)
        .await
        .expect("parse_response succeeds");

    // @step Then the CompletionResponse contains MessageContent::ToolUse with name "read_file" and stop_reason ToolUse
    let parts = match &response.content {
        MessageContent::Parts(p) => p,
        MessageContent::Text(_) => panic!("expected Parts, got Text"),
    };
    assert_eq!(parts.len(), 1);
    match &parts[0] {
        ContentPart::ToolUse { name, input, .. } => {
            assert_eq!(name, "read_file");
            let input_obj = input.as_object().expect("input is JSON object");
            assert_eq!(
                input_obj.get("path").and_then(|v| v.as_str()),
                Some("a.txt")
            );
        }
        other => panic!("expected ToolUse, got {other:?}"),
    }
    assert_eq!(response.stop_reason, StopReason::ToolUse);
}
