#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/rhai-reasoning-stream-chunks.feature
//!
//! This test file validates the acceptance criteria for PROV-089:
//! the StreamChunk::ReasoningDelta variant and the
//! stream_convert::handle_one dispatch arm that maps Rhai
//! `parse_stream_chunk` maps of kind "reasoning_delta" / "thinking_delta"
//! to a ReasoningDelta chunk.
//!
//! Red phase: these tests reference a `StreamChunk::ReasoningDelta`
//! variant that does not yet exist.

#[path = "custom_streaming_test_helpers.rs"]
mod helpers;

use codelet_common::Message;
use codelet_providers::custom::stream::StreamChunk;
use codelet_providers::StopReason;
use futures::StreamExt;
use helpers::{build_streaming_provider, process_events, streaming_config_with_script};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// A Rhai script that always returns a `reasoning_delta` with the given text,
// extracted from the raw SSE payload by a simple marker. The tests below
// feed handcrafted payloads so we can ignore real JSON parsing.
const REASONING_DELTA_SCRIPT: &str = r#"
fn build_request(request) { #{ messages: request.messages } }
fn build_headers(config) { #{ "Content-Type": "application/json" } }
fn build_url(config) { config.base_url + "/v1/chat/completions" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn build_stream_request(request) { #{ messages: request.messages, stream: true } }
fn map_error(status, body) { #{ type: "api", message: body } }
fn parse_stream_chunk(config, data) {
    #{ kind: "reasoning_delta", text: data }
}
"#;

const THINKING_DELTA_SCRIPT: &str = r#"
fn build_request(request) { #{ messages: request.messages } }
fn build_headers(config) { #{ "Content-Type": "application/json" } }
fn build_url(config) { config.base_url + "/v1/chat/completions" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn build_stream_request(request) { #{ messages: request.messages, stream: true } }
fn map_error(status, body) { #{ type: "api", message: body } }
fn parse_stream_chunk(config, data) {
    #{ kind: "thinking_delta", text: data }
}
"#;

// Returns reasoning_delta with empty text for any event.
const EMPTY_REASONING_SCRIPT: &str = r#"
fn build_request(request) { #{ messages: request.messages } }
fn build_headers(config) { #{ "Content-Type": "application/json" } }
fn build_url(config) { config.base_url + "/v1/chat/completions" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn build_stream_request(request) { #{ messages: request.messages, stream: true } }
fn map_error(status, body) { #{ type: "api", message: body } }
fn parse_stream_chunk(config, data) {
    #{ kind: "reasoning_delta", text: "" }
}
"#;

// Returns reasoning_delta with NO `text` field at all.
const MISSING_TEXT_REASONING_SCRIPT: &str = r#"
fn build_request(request) { #{ messages: request.messages } }
fn build_headers(config) { #{ "Content-Type": "application/json" } }
fn build_url(config) { config.base_url + "/v1/chat/completions" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn build_stream_request(request) { #{ messages: request.messages, stream: true } }
fn map_error(status, body) { #{ type: "api", message: body } }
fn parse_stream_chunk(config, data) {
    #{ kind: "reasoning_delta" }
}
"#;

// Dispatch based on an exact-match payload:
//   "REASON:Hel" → reasoning_delta("Hel")
//   "TEXT:answer" → text_delta("answer")
//   "STOP" → stop end_turn
const INTERLEAVED_SCRIPT: &str = r##"
fn build_request(request) { #{ messages: request.messages } }
fn build_headers(config) { #{ "Content-Type": "application/json" } }
fn build_url(config) { config.base_url + "/v1/chat/completions" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn build_stream_request(request) { #{ messages: request.messages, stream: true } }
fn map_error(status, body) { #{ type: "api", message: body } }
fn parse_stream_chunk(config, data) {
    if data == "REASON:Hel" { return #{ kind: "reasoning_delta", text: "Hel" }; }
    if data == "REASON:thinking done" { return #{ kind: "reasoning_delta", text: "thinking done" }; }
    if data == "TEXT:answer" { return #{ kind: "text_delta", text: "answer" }; }
    if data == "STOP" { return #{ kind: "stop", reason: "end_turn" }; }
    #{ kind: "ignore" }
}
"##;

// =========================================================================
// Scenario: Emit ReasoningDelta chunk for a reasoning_delta kind
// =========================================================================
#[tokio::test]
async fn emit_reasoning_delta_chunk_for_reasoning_delta_kind() {
    // @step Given a Rhai script whose parse_stream_chunk returns #{ kind: "reasoning_delta", text: "Let me think" }
    let script = REASONING_DELTA_SCRIPT;

    // @step When the bridge feeds any SSE data payload through process_event
    let (_tmp, results) = process_events(script, &["Let me think"]).await;

    // @step Then the bridge yields one StreamChunk::ReasoningDelta with value "Let me think"
    assert_eq!(
        results.len(),
        1,
        "expected exactly one chunk; got {results:?}"
    );
    match results.into_iter().next().unwrap() {
        Ok(StreamChunk::ReasoningDelta(text)) => assert_eq!(text, "Let me think"),
        other => panic!("expected ReasoningDelta(\"Let me think\"), got {other:?}"),
    }
}

// =========================================================================
// Scenario: Accept thinking_delta kind as an alias for reasoning_delta
// =========================================================================
#[tokio::test]
async fn accept_thinking_delta_kind_as_alias_for_reasoning_delta() {
    // @step Given a Rhai script whose parse_stream_chunk returns #{ kind: "thinking_delta", text: "computing..." }
    let script = THINKING_DELTA_SCRIPT;

    // @step When the bridge feeds any SSE data payload through process_event
    let (_tmp, results) = process_events(script, &["computing..."]).await;

    // @step Then the bridge yields one StreamChunk::ReasoningDelta with value "computing..."
    assert_eq!(
        results.len(),
        1,
        "expected exactly one chunk; got {results:?}"
    );
    match results.into_iter().next().unwrap() {
        Ok(StreamChunk::ReasoningDelta(text)) => assert_eq!(text, "computing..."),
        other => panic!("expected ReasoningDelta(\"computing...\"), got {other:?}"),
    }
}

// =========================================================================
// Scenario: Skip reasoning_delta with empty text
// =========================================================================
#[tokio::test]
async fn skip_reasoning_delta_with_empty_text() {
    // @step Given a Rhai script whose parse_stream_chunk returns #{ kind: "reasoning_delta", text: "" }
    let script = EMPTY_REASONING_SCRIPT;

    // @step When the bridge feeds any SSE data payload through process_event
    let (_tmp, results) = process_events(script, &["ignored"]).await;

    // @step Then no StreamChunk is yielded for that event
    assert!(
        results.is_empty(),
        "expected no chunks for empty reasoning_delta; got {results:?}"
    );
}

// =========================================================================
// Scenario: Skip reasoning_delta with missing text field
// =========================================================================
#[tokio::test]
async fn skip_reasoning_delta_with_missing_text_field() {
    // @step Given a Rhai script whose parse_stream_chunk returns #{ kind: "reasoning_delta" } with no text field
    let script = MISSING_TEXT_REASONING_SCRIPT;

    // @step When the bridge feeds any SSE data payload through process_event
    let (_tmp, results) = process_events(script, &["ignored"]).await;

    // @step Then no StreamChunk is yielded for that event
    assert!(
        results.is_empty(),
        "expected no chunks when text field is missing; got {results:?}"
    );
}

// =========================================================================
// Scenario: Preserve wire order for interleaved reasoning and text deltas
// =========================================================================
#[tokio::test]
async fn preserve_wire_order_for_interleaved_reasoning_and_text_deltas() {
    // @step Given a Rhai script that returns reasoning_delta or text_delta based on a marker in the event payload
    let script = INTERLEAVED_SCRIPT;

    // @step When the SSE stream emits a reasoning event then a text event then another reasoning event
    let (_tmp, results) = process_events(
        script,
        &["REASON:Hel", "TEXT:answer", "REASON:thinking done"],
    )
    .await;

    // @step Then the bridge yields ReasoningDelta("Hel") followed by TextDelta("answer") followed by ReasoningDelta("thinking done")
    assert_eq!(
        results.len(),
        3,
        "expected three chunks in order; got {results:?}"
    );
    match &results[0] {
        Ok(StreamChunk::ReasoningDelta(t)) => assert_eq!(t, "Hel"),
        other => panic!("expected ReasoningDelta(Hel), got {other:?}"),
    }
    match &results[1] {
        Ok(StreamChunk::TextDelta(t)) => assert_eq!(t, "answer"),
        other => panic!("expected TextDelta(answer), got {other:?}"),
    }
    match &results[2] {
        Ok(StreamChunk::ReasoningDelta(t)) => assert_eq!(t, "thinking done"),
        other => panic!("expected ReasoningDelta(thinking done), got {other:?}"),
    }
}

// =========================================================================
// Scenario: End-to-end stream through complete_with_tools_streaming yields ReasoningDelta
// =========================================================================
#[tokio::test]
async fn end_to_end_stream_through_complete_with_tools_streaming_yields_reasoning_delta() {
    // @step Given a wiremock SSE endpoint returning two thinking deltas followed by one text delta and a stop event
    let server = MockServer::start().await;
    let sse_body = concat!(
        "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"Let me\"}}\n\n",
        "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\" plan.\"}}\n\n",
        "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Done.\"}}\n\n",
        "data: {\"type\":\"message_stop\"}\n\n",
        "data: [DONE]\n\n",
    );
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body),
        )
        .mount(&server)
        .await;

    let script = format!(
        r#"
fn build_request(request) {{ #{{ messages: request.messages }} }}
fn build_headers(config) {{ #{{ "Content-Type": "application/json" }} }}
fn build_url(config) {{ "{base}/v1/chat/completions" }}
fn parse_response(raw) {{ #{{ content: "", stop_reason: "end_turn" }} }}
fn build_stream_request(request) {{ #{{ messages: request.messages, stream: true }} }}
fn map_error(status, body) {{ #{{ type: "api", message: body }} }}
fn parse_stream_chunk(config, data) {{
    let event = json::parse(data);
    if type_of(event["type"]) == "string" {{
        if event.type == "content_block_delta" && type_of(event["delta"]) != "()" {{
            let delta = event.delta;
            if type_of(delta["type"]) == "string" {{
                if delta.type == "thinking_delta" && type_of(delta["thinking"]) == "string" {{
                    return #{{ kind: "thinking_delta", text: delta.thinking }};
                }}
                if delta.type == "text_delta" && type_of(delta["text"]) == "string" {{
                    return #{{ kind: "text_delta", text: delta.text }};
                }}
            }}
        }}
        if event.type == "message_stop" {{
            return #{{ kind: "stop", reason: "end_turn" }};
        }}
    }}
    #{{ kind: "ignore" }}
}}
"#,
        base = server.uri()
    );
    let (_tmp, cfg) = streaming_config_with_script("my-llm", &server.uri(), &script);
    let provider = build_streaming_provider(cfg);

    // @step When RhaiCustomProvider performs a streaming completion
    let messages = vec![Message::user("hi")];
    let mut stream = provider.complete_with_tools_streaming(&messages, &[]).await;

    let mut collected: Vec<StreamChunk> = Vec::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => collected.push(chunk),
            Err(e) => panic!("unexpected streaming error: {e:?}"),
        }
    }

    // @step Then the collected chunks are ReasoningDelta, ReasoningDelta, TextDelta, StopReason in that exact order
    assert_eq!(
        collected.len(),
        4,
        "expected exactly 4 chunks; got {collected:?}"
    );
    match &collected[0] {
        StreamChunk::ReasoningDelta(t) => assert_eq!(t, "Let me"),
        other => panic!("expected ReasoningDelta(Let me), got {other:?}"),
    }
    match &collected[1] {
        StreamChunk::ReasoningDelta(t) => assert_eq!(t, " plan."),
        other => panic!("expected ReasoningDelta( plan.), got {other:?}"),
    }
    match &collected[2] {
        StreamChunk::TextDelta(t) => assert_eq!(t, "Done."),
        other => panic!("expected TextDelta(Done.), got {other:?}"),
    }
    match &collected[3] {
        StreamChunk::StopReason(sr) => assert_eq!(*sr, StopReason::EndTurn),
        other => panic!("expected StopReason(EndTurn), got {other:?}"),
    }
}
