#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/custom-provider-script-shadowing-builtin-providers.feature
//!
//! PROV-095 — regression test for the screenshot-captured Rhai failure:
//!
//! ```text
//! API Error: Streaming error: ProviderError: [claude-rhai] API error:
//!   script 'build_request' failed:
//!   For loop expects iterable type (line 51, position 16)
//! ```
//!
//! Background
//! ----------
//! An earlier TUI-level end-to-end test (`e2e/prov-095-rhai-dispatch.test.ts`)
//! claimed to guard this regression but is gated behind a real
//! `ANTHROPIC_API_KEY` and the developer-installed
//! `~/.fspec/providers/claude_rhai.rhai` fixture. On machines without
//! both prerequisites it silently skips — so it never actually
//! exercised the code path that produced the error in the screenshot.
//! That test was, in effect, theatre.
//!
//! This file replaces that theatre with an offline, deterministic,
//! zero-network Rust integration test. It:
//!
//!   1. Embeds the **exact `build_request` function body** that ships
//!      inside `~/.fspec/providers/claude_rhai.rhai` (verbatim copy).
//!      Any deviation in the real-world bridge that causes
//!      `request.messages` / `request.tools` to arrive as a non-iterable
//!      Dynamic will reproduce the screenshot's error here.
//!   2. Invokes the build path through
//!      `RhaiCustomProvider::invoke_build_request`, which is the same
//!      call site `RhaiCustomProviderModel::completion` uses. No HTTP,
//!      no API key, no mock server — just the bridge + engine.
//!   3. Covers the realistic shapes produced by
//!      `rig_messages_to_internal`:
//!         - System preamble only
//!         - Preamble + single user turn
//!         - Preamble + user + assistant + user multi-turn
//!         - Empty chat history (empty Vec<Message>)
//!         - Message whose content is structured `Parts` (array-shaped)
//!      …plus tool invocation (populated `&[ToolDefinition]`) so the
//!      `for tool in request.tools` loop at line 77 is also covered.
//!   4. Asserts the result is a serializable JSON body with the
//!      expected Anthropic-Messages-shaped keys, and that **no**
//!      `ProviderError` is returned — a Rhai iteration failure surfaces
//!      here as `ProviderError::Api { message: "…For loop expects
//!      iterable…" }` and the test fails loudly.
//!
//! If the production bridge ever regresses such that `request.messages`
//! or `request.tools` is handed to Rhai as a value that the engine does
//! not recognise as iterable, every scenario in this file fails with a
//! helpful diagnostic — unlike the previous e2e test, which would have
//! silently skipped.

#[path = "custom_http_test_helpers.rs"]
mod helpers;

use std::sync::Arc;

use codelet_common::{ContentPart, Message, MessageContent, MessageRole};
use codelet_providers::custom::{RhaiCustomProvider, ScriptLoader};
use codelet_providers::ProviderError;
use codelet_tools::ToolDefinition;
use helpers::config_with_full_script;
use serde_json::{json, Value as JsonValue};

// ---------------------------------------------------------------------------
// The EXACT body that ships in ~/.fspec/providers/claude_rhai.rhai today
// (copied verbatim from that file so the `for msg in request.messages` loop
// lands on the same line/position Rhai reports in the error we are guarding
// against — "line 51, position 16"). Trailing whitespace and comments are
// preserved so the test script's line numbers match the production script
// as closely as possible.
// ---------------------------------------------------------------------------
const CLAUDE_RHAI_SCRIPT: &str = r#"//! Custom Rhai provider: claude-rhai (verbatim copy for regression guard)
//! Only the 7 required functions are included; no network helpers run.

fn api_token() {
    "sk-ant-test-key-not-used-offline"
}

// =====================================================================
// 1. build_url(config) -> String
// =====================================================================
fn build_url(config) {
    config.base_url + "/v1/messages"
}

// =====================================================================
// 2. build_headers(config) -> Map
// =====================================================================
fn build_headers(config) {
    #{
        "Authorization":    "Bearer " + api_token(),
        "Content-Type":     "application/json",
        "anthropic-version": "2023-06-01",
        "anthropic-beta":   "prompt-caching-2024-07-31,interleaved-thinking-2025-05-14,context-1m-2025-08-07",
        "User-Agent":       "claude-cli/1.0",
        "x-app":            "cli"
    }
}

// =====================================================================
// 3. build_request(request) -> JSON body
//    request = #{ messages: [...], tools: [...] }
// =====================================================================
fn build_request(request) {
    let body = #{};
    // Real models.dev data for Claude Opus 4.7:
    //   id=claude-opus-4-7, context=1_000_000, output=128_000,
    //   family=claude-opus, release_date=2026-04-16, knowledge=2026-01-31
    body.model = "claude-opus-4-7";
    body.max_tokens = 128000;

    let conversation = [];
    let system_parts = [];

    for msg in request.messages {
        if msg.role == "system" {
            if type_of(msg.content) == "string" {
                system_parts.push(#{ type: "text", text: msg.content });
            } else if type_of(msg.content) == "array" {
                for part in msg.content {
                    if type_of(part) == "string" {
                        system_parts.push(#{ type: "text", text: part });
                    } else if type_of(part.text) != "()" {
                        system_parts.push(#{ type: "text", text: part.text });
                    }
                }
            }
        } else {
            let m = #{ role: msg.role, content: msg.content };
            conversation.push(m);
        }
    }

    if system_parts.len() > 0 {
        body.system = system_parts;
    }
    body.messages = conversation;

    if request.tools.len() > 0 {
        let tool_list = [];
        for tool in request.tools {
            tool_list.push(#{
                name: tool.name,
                description: tool.description,
                input_schema: tool.input_schema
            });
        }
        body.tools = tool_list;
    }

    body
}

// =====================================================================
// 4. parse_response(raw) -> #{ content, stop_reason }
// =====================================================================
fn parse_response(raw) {
    #{ content: [], stop_reason: "end_turn" }
}

// =====================================================================
// 5. parse_stream_chunk(chunk) -> stream event map
// =====================================================================
fn parse_stream_chunk(chunk) {
    #{ kind: "ignore" }
}

// =====================================================================
// 6. build_stream_request(request) -> JSON body (streaming)
// =====================================================================
fn build_stream_request(request) {
    let body = build_request(request);
    body.stream = true;
    body
}

// =====================================================================
// 7. map_error(status, body) -> #{ type, message }
// =====================================================================
fn map_error(status, body) {
    #{ type: "api", message: "HTTP " + status }
}
"#;

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

/// Sanity-check that the embedded copy of the script still contains the
/// Rhai `for msg in request.messages {` loop, indented with 4 spaces so
/// the `request` identifier lands at column 16 — the exact position
/// Rhai reports in the production error
/// (`line XX, position 16`). The line number does not need to match the
/// production file byte-for-byte (we strip the long `//!` docblock at
/// the top of the real file) — but the **position** on that line must,
/// otherwise the regression guard is weaker than the error it claims to
/// cover.
///
/// If someone edits the embedded script and inadvertently shifts the
/// indentation, this guard fails before the scenario tests so the drift
/// cannot silently weaken downstream assertions.
fn assert_for_loop_position_matches_production_error() {
    let lines: Vec<&str> = CLAUDE_RHAI_SCRIPT.split('\n').collect();
    let for_loop_line = lines
        .iter()
        .enumerate()
        .find(|(_, line)| line.trim_start() == "for msg in request.messages {")
        .map(|(idx, line)| (idx + 1, *line))
        .unwrap_or_else(|| {
            panic!(
                "embedded script no longer contains a `for msg in request.messages {{` loop; \
                 the regression guard is meaningless without it"
            )
        });

    let (line_no, line_text) = for_loop_line;
    let indent = line_text.len() - line_text.trim_start().len();
    // Column 1 = first char. "for msg in " is 11 chars. Indent of 4 spaces
    // places "request" at column 4 + 11 + 1 = 16. That is the position the
    // production error reports ("position 16") regardless of which line
    // holds the loop.
    assert_eq!(
        indent, 4,
        "embedded script at line {line_no}: `for` keyword indent is {indent}, expected 4 so that `request` lands at column 16"
    );
    let request_column = indent + "for msg in ".len() + 1;
    assert_eq!(
        request_column, 16,
        "embedded script at line {line_no}: `request` identifier lands at column {request_column}, \
         expected 16 to match the production error message"
    );
}

/// Build a fresh `RhaiCustomProvider` pointing at the embedded script.
///
/// Returns the provider together with the `TempDir` backing its
/// on-disk script. Callers must keep the `TempDir` alive for the
/// lifetime of the provider — `RhaiCustomProvider::new` only reads
/// the script path eagerly through `ScriptLoader::load`, but the
/// loader caches by path so if the tempdir is dropped mid-test any
/// second invocation that re-reads from disk would fail.
fn build_provider() -> (tempfile::TempDir, RhaiCustomProvider) {
    let (tmp, cfg) =
        config_with_full_script("claude-rhai", "https://api.anthropic.com", "claude-opus-4-7", CLAUDE_RHAI_SCRIPT);
    let loader = Arc::new(ScriptLoader::with_default_engine());
    let provider = RhaiCustomProvider::new(Arc::new(cfg), loader, "smart".to_string())
        .expect("construct RhaiCustomProvider with embedded claude_rhai.rhai script");
    (tmp, provider)
}

/// Invoke `build_request` and fail with a helpful diagnostic when the
/// Rhai engine emits "For loop expects iterable type". Returning the
/// body lets downstream assertions inspect the JSON shape.
async fn build_request_or_fail(
    provider: &RhaiCustomProvider,
    messages: &[Message],
    tools: &[ToolDefinition],
    scenario: &str,
) -> JsonValue {
    match provider.invoke_build_request(messages, tools, None).await {
        Ok(body) => body,
        Err(ProviderError::Api { message, .. }) if message.contains("For loop expects iterable") => {
            panic!(
                "PROV-095 REGRESSION ({scenario}): the production Rhai bridge surfaced the \
                 exact error from the screenshot — `{message}`. The `request` map passed to \
                 `build_request` no longer exposes `messages`/`tools` as iterable Rhai arrays."
            );
        }
        Err(other) => {
            panic!(
                "PROV-095 ({scenario}): invoke_build_request returned an unexpected error: {other:?}"
            );
        }
    }
}

/// Assert the body produced by the script has the minimum Anthropic
/// Messages shape: `model`, `max_tokens`, and a `messages` array. We do
/// not assert on the contents of the messages array here — that varies
/// by scenario — only on the structural invariants that every scenario
/// shares.
fn assert_valid_anthropic_body(body: &JsonValue, scenario: &str) {
    let obj = body
        .as_object()
        .unwrap_or_else(|| panic!("{scenario}: body is not a JSON object: {body}"));
    assert_eq!(
        obj.get("model").and_then(JsonValue::as_str),
        Some("claude-opus-4-7"),
        "{scenario}: body.model missing or wrong"
    );
    assert!(
        obj.get("max_tokens").and_then(JsonValue::as_i64).is_some(),
        "{scenario}: body.max_tokens missing or not integer"
    );
    assert!(
        obj.get("messages").and_then(JsonValue::as_array).is_some(),
        "{scenario}: body.messages missing or not an array: {body}"
    );
}

// =========================================================================
// Scenario: The embedded script still indents the for loop so the
// `request` identifier lands at column 16 (defensive guard against
// silent test-weakening — "position 16" is the coordinate the
// production error message reports).
// =========================================================================
#[test]
fn embedded_script_pins_for_loop_column_to_production_error_position() {
    // @step Given the embedded verbatim copy of ~/.fspec/providers/claude_rhai.rhai
    // @step When I inspect the `for msg in request.messages {` line
    // @step Then the `request` identifier lands at column 16,
    //       matching the position reported in the production error
    assert_for_loop_position_matches_production_error();
}

// =========================================================================
// Scenario: build_request succeeds when chat history contains only a
// single user turn (the exact shape that drove the screenshot's
// "what is 3 + 2?" prompt).
// =========================================================================
#[tokio::test]
async fn build_request_succeeds_with_single_user_turn() {
    // @step Given the claude-rhai build_request script from the screenshot failure
    let (_script_tmp, provider) = build_provider();

    // @step When I invoke build_request with a single user message "what is 3 + 2?"
    let messages = vec![Message::user("what is 3 + 2?")];
    let body = build_request_or_fail(&provider, &messages, &[], "single_user_turn").await;

    // @step Then the Rhai for-loop at line 51 does NOT fail with "For loop expects iterable type"
    // @step And the resulting body is a valid Anthropic Messages payload with one user message
    assert_valid_anthropic_body(&body, "single_user_turn");
    let msgs = body["messages"].as_array().expect("messages array");
    assert_eq!(msgs.len(), 1, "single_user_turn: expected exactly 1 message");
    assert_eq!(msgs[0]["role"].as_str(), Some("user"));
    assert_eq!(msgs[0]["content"].as_str(), Some("what is 3 + 2?"));
}

// =========================================================================
// Scenario: build_request succeeds with system preamble + user turn.
// This is the shape `rig_messages_to_internal` produces when the rig
// `CompletionRequest.preamble` is set — the path the TUI actually
// exercises when the user sends their first prompt.
// =========================================================================
#[tokio::test]
async fn build_request_succeeds_with_system_preamble_plus_user_turn() {
    // @step Given a system preamble and a user question
    let (_script_tmp, provider) = build_provider();
    let messages = vec![
        Message::system("You are a helpful assistant."),
        Message::user("what is 3 + 2?"),
    ];

    // @step When I invoke build_request
    let body = build_request_or_fail(&provider, &messages, &[], "system_plus_user").await;

    // @step Then the Rhai for-loop iterates successfully over both messages
    assert_valid_anthropic_body(&body, "system_plus_user");
    // @step And the system text is lifted out into body.system
    let system = body["system"]
        .as_array()
        .expect("system_plus_user: body.system should be an array");
    assert_eq!(system.len(), 1);
    assert_eq!(system[0]["type"].as_str(), Some("text"));
    assert_eq!(
        system[0]["text"].as_str(),
        Some("You are a helpful assistant.")
    );
    // @step And the user message appears in body.messages
    let msgs = body["messages"].as_array().expect("messages array");
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["role"].as_str(), Some("user"));
}

// =========================================================================
// Scenario: build_request succeeds across a multi-turn conversation.
// =========================================================================
#[tokio::test]
async fn build_request_succeeds_with_multi_turn_conversation() {
    // @step Given a 4-turn conversation (system, user, assistant, user)
    let (_script_tmp, provider) = build_provider();
    let messages = vec![
        Message::system("You are a helpful assistant."),
        Message::user("what is 3 + 2?"),
        Message::assistant("3 + 2 = 5"),
        Message::user("and what is 5 + 5?"),
    ];

    // @step When I invoke build_request
    let body = build_request_or_fail(&provider, &messages, &[], "multi_turn").await;

    // @step Then the for-loop does not fail and all three non-system messages are present
    assert_valid_anthropic_body(&body, "multi_turn");
    let msgs = body["messages"].as_array().expect("messages array");
    assert_eq!(msgs.len(), 3, "multi_turn: expected 3 non-system messages");
    let roles: Vec<&str> = msgs
        .iter()
        .map(|m| m["role"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(roles, vec!["user", "assistant", "user"]);
}

// =========================================================================
// Scenario: build_request succeeds when chat history is empty. This is
// the shape the TUI passes on the first `/model` dispatch before any
// user prompt has been submitted — an empty Rhai Array must still be
// iterable (`for msg in []` is a no-op, not a type error).
// =========================================================================
#[tokio::test]
async fn build_request_succeeds_with_empty_chat_history() {
    // @step Given an empty chat history
    let (_script_tmp, provider) = build_provider();
    let messages: Vec<Message> = Vec::new();

    // @step When I invoke build_request
    let body = build_request_or_fail(&provider, &messages, &[], "empty_history").await;

    // @step Then the for-loop over request.messages is a no-op and the body is valid
    assert_valid_anthropic_body(&body, "empty_history");
    let msgs = body["messages"].as_array().expect("messages array");
    assert!(msgs.is_empty(), "empty_history: messages should be empty");
    // @step And body.system is not emitted (no system_parts collected)
    assert!(
        body.get("system").is_none() || body["system"].as_array().map(|a| a.is_empty()).unwrap_or(false),
        "empty_history: body.system should be absent or empty: {body}"
    );
}

// =========================================================================
// Scenario: build_request succeeds when a message carries structured
// `Parts` content. The `ContentPart` serialisation produces an array of
// tagged objects — the inner `for part in msg.content` loop (line 56
// in the production script) must see that as an iterable Rhai array.
// =========================================================================
#[tokio::test]
async fn build_request_succeeds_with_structured_content_parts() {
    // @step Given a system message whose content is a structured Parts array
    let (_script_tmp, provider) = build_provider();
    let messages = vec![
        Message {
            role: MessageRole::System,
            content: MessageContent::Parts(vec![
                ContentPart::Text {
                    text: "First system sentence.".to_string(),
                },
                ContentPart::Text {
                    text: "Second system sentence.".to_string(),
                },
            ]),
        },
        Message::user("what is 3 + 2?"),
    ];

    // @step When I invoke build_request
    let body = build_request_or_fail(&provider, &messages, &[], "structured_parts").await;

    // @step Then both inner for-loops (over request.messages AND over msg.content) succeed
    assert_valid_anthropic_body(&body, "structured_parts");
    // @step And body.system captures both text parts
    let system = body["system"]
        .as_array()
        .expect("structured_parts: body.system should be an array");
    assert_eq!(system.len(), 2);
    let texts: Vec<&str> = system
        .iter()
        .map(|p| p["text"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        texts,
        vec!["First system sentence.", "Second system sentence."]
    );
}

// =========================================================================
// Scenario: build_request succeeds when tools are present. The inner
// `for tool in request.tools` loop (line 77 in the production script)
// mirrors the messages loop — if the bridge regresses the tools array
// it fails with the identical error message for a different line.
// =========================================================================
#[tokio::test]
async fn build_request_succeeds_when_tools_are_present() {
    // @step Given a single user message and one tool definition
    let (_script_tmp, provider) = build_provider();
    let messages = vec![Message::user("what is 3 + 2?")];

    let mut properties = serde_json::Map::new();
    properties.insert(
        "expression".to_string(),
        json!({
            "type": "string",
            "description": "Arithmetic expression to evaluate."
        }),
    );
    let tool = ToolDefinition {
        name: "calculator".to_string(),
        description: "Evaluates arithmetic expressions.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": JsonValue::Object(properties),
            "required": ["expression"],
        }),
    };
    let tools = vec![tool];

    // @step When I invoke build_request
    let body = build_request_or_fail(&provider, &messages, &tools, "with_tools").await;

    // @step Then both the messages loop and the tools loop iterate successfully
    assert_valid_anthropic_body(&body, "with_tools");
    // @step And body.tools is populated with our single calculator tool
    let out_tools = body["tools"]
        .as_array()
        .expect("with_tools: body.tools should be an array");
    assert_eq!(out_tools.len(), 1);
    assert_eq!(out_tools[0]["name"].as_str(), Some("calculator"));
    assert_eq!(
        out_tools[0]["description"].as_str(),
        Some("Evaluates arithmetic expressions.")
    );
}
