#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/custom-provider-http-request-response-lifecycle.feature
//!
//! This test file validates the acceptance criteria for PROV-063:
//! `RhaiCustomProvider` + `request_bridge` + `response_bridge` that wire
//! the 7 required Rhai functions from PROV-062 through an async HTTP
//! request/response lifecycle.
//!
//! These tests import symbols that will only exist AFTER PROV-063 is
//! implemented (`RhaiCustomProvider`, the `request_bridge` and
//! `response_bridge` submodules of `codelet_providers::custom`). They
//! therefore fail to compile until the production code lands — this is
//! the red phase.

#[path = "custom_http_test_helpers.rs"]
mod helpers;

use std::sync::Arc;

use codelet_common::{ContentPart, Message, MessageContent};
use codelet_providers::custom::request_bridge::messages_to_rhai;
use codelet_providers::custom::response_bridge::rhai_to_completion_response;
use codelet_providers::custom::{ProviderConfig, RhaiCustomProvider, ScriptLoader};
use codelet_providers::{CompletionResponse, LlmProvider, ProviderError, StopReason};

use helpers::{config_with_full_script, config_with_script, FULL_HAPPY_SCRIPT};
use rhai::{Dynamic, Map};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// =========================================================================
// Helpers local to this test file
// =========================================================================

/// Build a fresh `RhaiCustomProvider` from a config + inline script using a
/// newly-allocated `ScriptLoader`. Panics on construction failure because
/// each test is expected to provide a well-formed script.
fn build_provider(cfg: ProviderConfig, model_alias: &str) -> RhaiCustomProvider {
    let loader = Arc::new(ScriptLoader::with_default_engine());
    RhaiCustomProvider::new(Arc::new(cfg), loader, model_alias.to_string())
        .expect("construct RhaiCustomProvider")
}

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

// =========================================================================
// Scenario: Map HTTP 401 to auth error
// =========================================================================
#[tokio::test]
async fn map_http_401_to_auth_error() {
    // @step Given a Rhai script whose map_error returns an auth error for status 401
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string("{\"error\":\"unauthorized\"}"),
        )
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
        .respond_with(
            ResponseTemplate::new(200).set_body_string("{\"choices\":[]}"),
        )
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

// =========================================================================
// Scenario: Provider name reflects config
// =========================================================================
#[tokio::test]
async fn provider_name_reflects_config() {
    // @step Given a ProviderConfig with name "my-llm"
    let (_tmp, cfg) = config_with_full_script(
        "my-llm",
        "https://api.example.com",
        "model-smart-v2",
        FULL_HAPPY_SCRIPT,
    );

    // @step When I construct a RhaiCustomProvider from that config
    let provider = build_provider(cfg, "smart");

    // @step Then provider.name() returns "my-llm"
    assert_eq!(provider.name(), "my-llm");
}

// =========================================================================
// Scenario: Provider context window reflects selected model
// =========================================================================
#[tokio::test]
async fn provider_context_window_reflects_selected_model() {
    // @step Given a config with a model "big" defining context_window 200000 and max_output_tokens 8192
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let script_path = helpers::write_script(tmp.path(), "p.rhai", FULL_HAPPY_SCRIPT);

    let mut models = std::collections::HashMap::new();
    models.insert(
        "big".to_string(),
        codelet_providers::custom::ModelDef {
            id: "model-big".to_string(),
            context_window: 200_000,
            max_output_tokens: 8_192,
            supports_tools: true,
            supports_streaming: true,
            supports_thinking: false,
        },
    );

    let cfg = ProviderConfig {
        name: "my-llm".to_string(),
        display_name: "My LLM".to_string(),
        base_url: "https://api.example.com".to_string(),
        script: script_path.to_string_lossy().to_string(),
        facade: None,
        api_key_env_var: None,
        auth: codelet_providers::custom::AuthConfig::Bearer {
            env_var: "MY_KEY".to_string(),
            token_prefix: "Bearer".to_string(),
        },
        models,
        defaults: None,
        system_prompt: None,
        tool_style: codelet_providers::custom::ToolStyle::Openai,
        api_style: codelet_providers::custom::ApiStyle::OpenaiChat,
        headers: std::collections::HashMap::new(),
        env_prefix: None,
        resolved_tools: None,
    };

    // @step When I construct a RhaiCustomProvider selecting model "big"
    let provider = build_provider(cfg, "big");

    // @step Then provider.context_window() equals 200000 and provider.max_output_tokens() equals 8192
    assert_eq!(provider.context_window(), 200_000);
    assert_eq!(provider.max_output_tokens(), 8_192);
}

// =========================================================================
// Scenario: Request bridge preserves multi-turn message structure
// =========================================================================
#[tokio::test]
async fn request_bridge_preserves_multi_turn_message_structure() {
    // @step Given a conversation with user then assistant then user turns
    let messages = vec![
        Message::user("first"),
        Message::assistant("second"),
        Message::user("third"),
    ];

    // @step When I convert the messages through the request_bridge
    let dyn_value: Dynamic = messages_to_rhai(&messages).expect("messages_to_rhai");
    let array = dyn_value
        .into_typed_array::<Dynamic>()
        .expect("resulting Dynamic is a Rhai Array");

    // @step Then the resulting Rhai array has three entries in the correct order with matching roles and contents
    assert_eq!(array.len(), 3);

    let expect = [
        ("user", "first"),
        ("assistant", "second"),
        ("user", "third"),
    ];
    for (i, (role, content)) in expect.iter().enumerate() {
        let m = array[i]
            .clone()
            .try_cast::<Map>()
            .expect("each array entry is a Map");
        let actual_role = m
            .get("role")
            .cloned()
            .unwrap_or(Dynamic::UNIT)
            .into_string()
            .expect("role is string");
        assert_eq!(&actual_role, role);
        let actual_content = m
            .get("content")
            .cloned()
            .unwrap_or(Dynamic::UNIT)
            .into_string()
            .expect("content is string");
        assert_eq!(&actual_content, content);
    }
}

// =========================================================================
// Scenario: Response bridge preserves structured tool call input
// =========================================================================
#[tokio::test]
async fn response_bridge_preserves_structured_tool_call_input() {
    // @step Given a Rhai response map with tool_call input {path:"a.txt", mode:"read"}
    let mut input_map = Map::new();
    input_map.insert("path".into(), Dynamic::from("a.txt".to_string()));
    input_map.insert("mode".into(), Dynamic::from("read".to_string()));

    let mut tool_use_part = Map::new();
    tool_use_part.insert("type".into(), Dynamic::from("tool_use".to_string()));
    tool_use_part.insert("id".into(), Dynamic::from("call_1".to_string()));
    tool_use_part.insert("name".into(), Dynamic::from("read_file".to_string()));
    tool_use_part.insert("input".into(), Dynamic::from_map(input_map));

    let content_array: rhai::Array = vec![Dynamic::from_map(tool_use_part)];

    let mut response_map = Map::new();
    response_map.insert("content".into(), Dynamic::from_array(content_array));
    response_map.insert("stop_reason".into(), Dynamic::from("tool_use".to_string()));

    let response_dynamic = Dynamic::from_map(response_map);

    // @step When I convert it through the response_bridge
    let (response, _usage) =
        rhai_to_completion_response(response_dynamic).expect("response_bridge succeeds");

    // @step Then the ToolUseContent input is a serde_json::Value object with fields path="a.txt" and mode="read"
    let parts = match &response.content {
        MessageContent::Parts(p) => p,
        MessageContent::Text(_) => panic!("expected Parts, got Text"),
    };
    assert_eq!(parts.len(), 1);
    match &parts[0] {
        ContentPart::ToolUse { id, name, input } => {
            assert_eq!(id, "call_1");
            assert_eq!(name, "read_file");
            let obj = input.as_object().expect("input is JSON object");
            assert_eq!(obj.get("path").and_then(|v| v.as_str()), Some("a.txt"));
            assert_eq!(obj.get("mode").and_then(|v| v.as_str()), Some("read"));
        }
        other => panic!("expected ToolUse, got {other:?}"),
    }
    assert_eq!(response.stop_reason, StopReason::ToolUse);
}
