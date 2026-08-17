// Feature: spec/features/openai-nonstreaming-request-loop.feature
//
// PROV-140 — when a profile disables streaming the OpenAI request path must
// (a) serialize `stream:false` and OMIT `stream_options`, and (b) adapt the
// single non-streaming JSON response into the SAME MultiTurnStreamItem stream
// the streaming path yields, so the interactive driver reuses its existing
// `match stream.next()` loop and tool calls still drive the multi-turn loop to
// completion. Today the ONLY request-build seam
// (patches/rig-core/.../openai/completion/streaming.rs:125) unconditionally
// merges `{"stream":true,"stream_options":{"include_usage":true}}`, so the
// non-streaming behaviours below are RED until the transport-level stream flag
// (spike Strategy B) is wired.
//
// These tests are OFFLINE: a local wiremock server stands in for the OpenAI
// endpoint (wiremock is a providers dev-dependency, see Cargo.toml:81). No live
// network occurs. Env mutation (OPENAI_STREAMING) is process-global, so every
// test is `#[serial]` and restores what it touches via `EnvGuard`.
//
// RED status per scenario:
//   * omits streaming options  -> FAILS: captured body has stream:true today.
//   * keeps streaming options   -> regression guard (passes today, by design).
//   * text reply adapted        -> FAILS: streaming path can't parse plain JSON,
//                                  so no Text item + no terminal FinalResponse.
//   * tool call multi-turn      -> FAILS: tool never executes (counter stays 0),
//                                  no final response is reached.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)]

use std::sync::atomic::{AtomicUsize, Ordering};

use codelet_providers::OpenAIProvider;
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::client::CompletionClient;
use rig::completion::ToolDefinition;
use rig::streaming::{StreamedAssistantContent, StreamingPrompt};
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serial_test::serial;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Save/restore the process-global env var these tests mutate.
struct EnvGuard {
    saved: Option<String>,
}

impl EnvGuard {
    fn capture() -> Self {
        Self {
            saved: std::env::var("OPENAI_STREAMING").ok(),
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.saved {
            Some(v) => std::env::set_var("OPENAI_STREAMING", v),
            None => std::env::remove_var("OPENAI_STREAMING"),
        }
    }
}

/// A minimal OpenAI-shaped SSE stream ending with `[DONE]`, so the streaming
/// path completes cleanly and the request is recorded for inspection.
const SSE_OK: &str = concat!(
    "data: {\"id\":\"c\",\"object\":\"chat.completion.chunk\",\"choices\":\
     [{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"ok\"},\"finish_reason\":null}]}\n\n",
    "data: {\"id\":\"c\",\"object\":\"chat.completion.chunk\",\"choices\":\
     [{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
    "data: {\"id\":\"c\",\"object\":\"chat.completion.chunk\",\"choices\":[],\
     \"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1,\"total_tokens\":2}}\n\n",
    "data: [DONE]\n\n"
);

/// Build an OpenAIProvider pointed at the local mock. The base URL carries an
/// explicit `/v1`, matching production `normalize_base_url` output, so requests
/// land on `POST /v1/chat/completions`.
fn provider_for(uri: &str) -> OpenAIProvider {
    OpenAIProvider::from_api_key_with_options(
        "sk-test-key-12345",
        "gpt-4o-mini",
        Some(&format!("{uri}/v1")),
        None,
    )
    .expect("OpenAIProvider constructs against a local mock base URL")
}

/// Parse the last request body the mock recorded as JSON.
async fn last_request_body(server: &MockServer) -> Value {
    let received = server
        .received_requests()
        .await
        .expect("mock server records requests");
    let last = received.last().expect("at least one request was sent");
    serde_json::from_slice(&last.body).expect("chat completion request body is JSON")
}

// =============================================================================
// Scenario: Streaming-disabled request omits streaming options
// =============================================================================
#[tokio::test]
#[serial]
async fn streaming_disabled_request_omits_streaming_options() {
    let _env = EnvGuard::capture();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(SSE_OK),
        )
        .mount(&server)
        .await;

    // @step Given an OpenAI completion request built with streaming disabled
    std::env::set_var("OPENAI_STREAMING", "false");
    let agent = provider_for(&server.uri())
        .client()
        .agent("gpt-4o-mini")
        .max_tokens(32)
        .preamble("terse")
        .build();

    // @step When the request body is serialized
    let mut stream = agent.stream_prompt("hi").multi_turn(2).await;
    while stream.next().await.is_some() {}
    let body = last_request_body(&server).await;

    // @step Then the body sets stream to false
    assert_eq!(
        body.get("stream"),
        Some(&json!(false)),
        "streaming-disabled request must serialize stream:false; got {body}"
    );

    // @step And the body omits stream_options
    assert!(
        body.get("stream_options").is_none(),
        "streaming-disabled request must omit stream_options; got {body}"
    );
}

// =============================================================================
// Scenario: Streaming-enabled request keeps streaming options (regression guard)
// =============================================================================
#[tokio::test]
#[serial]
async fn streaming_enabled_request_keeps_streaming_options() {
    let _env = EnvGuard::capture();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(SSE_OK),
        )
        .mount(&server)
        .await;

    // @step Given an OpenAI completion request built with streaming enabled
    std::env::remove_var("OPENAI_STREAMING");
    let agent = provider_for(&server.uri())
        .client()
        .agent("gpt-4o-mini")
        .max_tokens(32)
        .preamble("terse")
        .build();

    // @step When the request body is serialized
    let mut stream = agent.stream_prompt("hi").multi_turn(2).await;
    while stream.next().await.is_some() {}
    let body = last_request_body(&server).await;

    // @step Then the body sets stream to true
    assert_eq!(
        body.get("stream"),
        Some(&json!(true)),
        "streaming-enabled request must serialize stream:true; got {body}"
    );

    // @step And the body includes stream_options with include_usage
    assert_eq!(
        body.pointer("/stream_options/include_usage"),
        Some(&json!(true)),
        "streaming-enabled request must include stream_options.include_usage; got {body}"
    );
}

// =============================================================================
// Scenario: Non-streaming text reply is adapted into a Text then final item
// sequence
// =============================================================================
#[tokio::test]
#[serial]
async fn nonstreaming_text_reply_adapts_into_text_then_final_items() {
    let _env = EnvGuard::capture();
    let server = MockServer::start().await;

    // @step Given a non-streaming OpenAI response containing only assistant text
    std::env::set_var("OPENAI_STREAMING", "false");
    let response = json!({
        "id": "cmpl-text",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "gpt-4o-mini",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "The capital is Paris."},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    });
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;

    let agent = provider_for(&server.uri())
        .client()
        .agent("gpt-4o-mini")
        .max_tokens(64)
        .preamble("terse")
        .build();

    // @step When the non-streaming path adapts the response into stream items
    let mut stream = agent.stream_prompt("capital of France?").multi_turn(2).await;
    let mut text_seen = false;
    let mut final_seen = false;
    while let Some(item) = stream.next().await {
        match item {
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(t)))
                if t.text.contains("Paris") =>
            {
                text_seen = true;
            }
            Ok(MultiTurnStreamItem::FinalResponse(_)) => final_seen = true,
            _ => {}
        }
    }

    // @step Then a text item carries the assistant text
    assert!(
        text_seen,
        "non-streaming path must yield a Text item carrying the assistant text"
    );

    // @step And a final response item terminates the sequence
    assert!(
        final_seen,
        "non-streaming path must terminate the sequence with a FinalResponse item"
    );
}

// A tool with a process-global call counter so the test can prove the
// multi-turn loop actually executed it.
static TOOL_CALLS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, thiserror::Error)]
#[error("lookup tool error: {0}")]
struct LookupError(String);

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
struct LookupArgs {
    /// What to look up.
    query: String,
}

#[derive(Default)]
struct LookupTool;

impl Tool for LookupTool {
    const NAME: &'static str = "lookup";
    type Error = LookupError;
    type Args = LookupArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "lookup".to_string(),
            description: "Look up a fact. You MUST call this before answering.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {"query": {"type": "string", "description": "query"}},
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        TOOL_CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(format!("looked up: {}", args.query))
    }
}

// =============================================================================
// Scenario: Non-streaming tool call drives the multi-turn loop to completion
// =============================================================================
#[tokio::test]
#[serial]
async fn nonstreaming_tool_call_drives_multi_turn_to_completion() {
    let _env = EnvGuard::capture();
    TOOL_CALLS.store(0, Ordering::SeqCst);
    let server = MockServer::start().await;

    // @step Given a non-streaming OpenAI response requesting a tool call
    std::env::set_var("OPENAI_STREAMING", "false");
    let final_reply = json!({
        "id": "cmpl-final",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "gpt-4o-mini",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "All done."},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    });
    let tool_reply = json!({
        "id": "cmpl-tool",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "gpt-4o-mini",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "lookup", "arguments": "{\"query\":\"x\"}"}
                }]
            },
            "finish_reason": "tool_calls"
        }],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    });
    // wiremock matches mocks in mount order: the one-shot tool response is
    // mounted FIRST so it serves the first turn, then is exhausted and the
    // request falls through to the final text reply on the second turn.
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(tool_reply))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(final_reply))
        .mount(&server)
        .await;

    let agent = provider_for(&server.uri())
        .client()
        .agent("gpt-4o-mini")
        .max_tokens(128)
        .preamble("Use the lookup tool before answering.")
        .tool(LookupTool)
        .build();

    // @step When the non-streaming path runs the multi-turn loop
    let mut stream = agent.stream_prompt("look up x").multi_turn(4).await;
    let mut final_seen = false;
    while let Some(item) = stream.next().await {
        if let Ok(MultiTurnStreamItem::FinalResponse(_)) = item {
            final_seen = true;
        }
    }

    // @step Then the requested tool is executed
    assert!(
        TOOL_CALLS.load(Ordering::SeqCst) > 0,
        "the non-streaming multi-turn loop must execute the requested tool"
    );

    // @step And the loop continues to a final response
    assert!(
        final_seen,
        "the multi-turn loop must continue to a terminal FinalResponse"
    );
}
