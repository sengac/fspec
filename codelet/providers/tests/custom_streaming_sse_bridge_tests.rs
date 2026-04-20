#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/custom-provider-streaming-sse-bridge.feature
//!
//! This test file validates the acceptance criteria for PROV-064:
//! the streaming SSE bridge that parses SSE events via the Rhai
//! `parse_stream_chunk` function and surfaces the resulting
//! `StreamChunk` items through `RhaiCustomProvider::complete_with_tools_streaming`.
//!
//! The tests import symbols that do not yet exist (red phase):
//!   - `codelet_providers::custom::stream::{StreamChunk, RhaiStreamProcessor, open_stream}`
//!   - `RhaiCustomProvider::complete_with_tools_streaming`
//!   - `RhaiCustomProvider::invoke_build_stream_request`

#[path = "custom_streaming_test_helpers.rs"]
mod helpers;

use codelet_common::Message;
use codelet_providers::custom::stream::StreamChunk;
use codelet_providers::{ProviderError, StopReason};
use futures::StreamExt;
use helpers::{
    build_streaming_provider, process_events, streaming_config_with_script,
    FAIL_IF_CALLED_SCRIPT, OPENAI_TEXT_DELTA_SCRIPT,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// =========================================================================
// Scenario 1: Emit TextDelta chunk for single content delta
// =========================================================================
#[tokio::test]
async fn emit_text_delta_chunk_for_single_content_delta() {
    // @step Given a Rhai script whose parse_stream_chunk extracts delta.content as text_delta
    let script = OPENAI_TEXT_DELTA_SCRIPT;

    // @step When the SSE stream emits data '{"choices":[{"delta":{"content":"Hel"}}]}'
    let (_tmp, results) =
        process_events(script, &[r#"{"choices":[{"delta":{"content":"Hel"}}]}"#]).await;

    // @step Then the bridge yields one StreamChunk::TextDelta with value "Hel"
    assert_eq!(results.len(), 1, "expected exactly one chunk");
    match results.into_iter().next().unwrap() {
        Ok(StreamChunk::TextDelta(text)) => assert_eq!(text, "Hel"),
        other => panic!("expected TextDelta(\"Hel\"), got {other:?}"),
    }
}

// =========================================================================
// Scenario 2: Emit TextDelta chunks in order for consecutive content deltas
// =========================================================================
#[tokio::test]
async fn emit_text_delta_chunks_in_order_for_consecutive_content_deltas() {
    // @step Given a Rhai script extracting text_delta from delta.content
    let script = OPENAI_TEXT_DELTA_SCRIPT;

    // @step When the SSE stream emits two content deltas "Hel" then "lo"
    let (_tmp, results) = process_events(
        script,
        &[
            r#"{"choices":[{"delta":{"content":"Hel"}}]}"#,
            r#"{"choices":[{"delta":{"content":"lo"}}]}"#,
        ],
    ).await;

    // @step Then the bridge yields TextDelta("Hel") followed by TextDelta("lo")
    assert_eq!(results.len(), 2, "expected two chunks in order");
    match (&results[0], &results[1]) {
        (Ok(StreamChunk::TextDelta(a)), Ok(StreamChunk::TextDelta(b))) => {
            assert_eq!(a, "Hel");
            assert_eq!(b, "lo");
        }
        other => panic!("expected TextDelta(Hel), TextDelta(lo), got {other:?}"),
    }
}

// =========================================================================
// Scenario 3: Terminate stream on DONE marker without invoking parse_stream_chunk
// =========================================================================
#[tokio::test]
async fn terminate_stream_on_done_marker_without_invoking_parse_stream_chunk() {
    // @step Given any valid streaming provider configuration
    // Use the "fail-if-called" script so the test fails if [DONE] accidentally
    // invokes parse_stream_chunk.
    let script = FAIL_IF_CALLED_SCRIPT;

    // @step When the SSE stream emits data "[DONE]"
    let (_tmp, results) = process_events(script, &["[DONE]"]).await;

    // @step Then the bridge completes without yielding further chunks
    //        and parse_stream_chunk is not invoked for that event
    assert!(
        results.is_empty(),
        "expected no chunks on [DONE]; got {results:?}"
    );
}

// =========================================================================
// Scenario 4: Accumulate partial tool call arguments into a single ToolCallComplete
// =========================================================================
#[tokio::test]
async fn accumulate_partial_tool_call_arguments_into_single_tool_call_complete() {
    // @step Given a Rhai script emitting tool_call_delta with incremental arguments
    let script = r#"
fn build_request(request) { #{ messages: request.messages } }
fn build_headers(config) { #{ "Content-Type": "application/json" } }
fn build_url(config) { config.base_url + "/v1/chat/completions" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn build_stream_request(request) { #{ stream: true } }
fn map_error(status, body) { #{ type: "api", message: body } }
fn has(m, k) { type_of(m[k]) != "()" }

fn parse_stream_chunk(config, data) {
    let event = json::parse(data);
    let choices = event["choices"];
    if type_of(choices) == "()" { return #{ kind: "ignore" }; }
    let ch = choices[0];
    let delta = ch["delta"];
    if type_of(delta) != "()" && has(delta, "tool_calls") {
        let tc = delta["tool_calls"][0];
        let out = #{ kind: "tool_call_delta", index: tc.index };
        if has(tc, "id") { out.id = tc.id; }
        if has(tc, "function") {
            let f = tc.function;
            if has(f, "name") { out.name = f.name; }
            if has(f, "arguments") { out.arguments = f.arguments; }
        }
        return out;
    }
    let fr = ch["finish_reason"];
    if type_of(fr) == "string" {
        if fr == "tool_calls" { return #{ kind: "stop", reason: "tool_use" }; }
        if fr == "stop" { return #{ kind: "stop", reason: "end_turn" }; }
    }
    #{ kind: "ignore" }
}
"#;

    // @step When the stream emits arguments "{\"pa" then "th\":\"a.txt\"}" followed by finish_reason "tool_calls"
    let (_tmp, results) = process_events(
        script,
        &[
            r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"read_file","arguments":""}}]}}]}"#,
            r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"pa"}}]}}]}"#,
            r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"th\":\"a.txt\"}"}}]}}]}"#,
            r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
        ],
    ).await;

    // @step Then the bridge yields one ToolCallComplete whose input equals {"path":"a.txt"}
    let completes: Vec<_> = results
        .iter()
        .filter_map(|r| match r {
            Ok(StreamChunk::ToolCallComplete { id, name, input }) => {
                Some((id.clone(), name.clone(), input.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        completes.len(),
        1,
        "expected exactly one ToolCallComplete; got {results:?}"
    );
    let (id, name, input) = &completes[0];
    assert_eq!(id, "call_1");
    assert_eq!(name, "read_file");
    let obj = input.as_object().expect("input is an object");
    assert_eq!(
        obj.get("path").and_then(|v| v.as_str()),
        Some("a.txt"),
        "expected input == {{\"path\":\"a.txt\"}}, got {input:?}"
    );
}

// =========================================================================
// Scenario 5: Emit StopReason EndTurn for stop finish_reason
// =========================================================================
#[tokio::test]
async fn emit_stop_reason_end_turn_for_stop_finish_reason() {
    // @step Given a Rhai script that maps finish_reason to the stop kind
    let script = r#"
fn build_request(request) { #{} }
fn build_headers(config) { #{} }
fn build_url(config) { "" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn build_stream_request(request) { #{ stream: true } }
fn map_error(status, body) { #{ type: "api", message: body } }
fn parse_stream_chunk(config, data) {
    let event = json::parse(data);
    let ch = event.choices[0];
    if (type_of(ch["finish_reason"]) != "()") && type_of(ch.finish_reason) == "string" {
        if ch.finish_reason == "stop" {
            return #{ kind: "stop", reason: "end_turn" };
        }
        if ch.finish_reason == "tool_calls" {
            return #{ kind: "stop", reason: "tool_use" };
        }
    }
    #{ kind: "ignore" }
}
"#;

    // @step When the stream emits finish_reason "stop"
    let (_tmp, results) = process_events(
        script,
        &[r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#],
    ).await;

    // @step Then the bridge yields StreamChunk::StopReason(EndTurn)
    let stops: Vec<_> = results
        .iter()
        .filter_map(|r| match r {
            Ok(StreamChunk::StopReason(sr)) => Some(*sr),
            _ => None,
        })
        .collect();
    assert_eq!(stops.len(), 1, "expected 1 StopReason, got {results:?}");
    assert_eq!(stops[0], StopReason::EndTurn);
}

// =========================================================================
// Scenario 6: Emit StopReason ToolUse for tool_calls finish_reason
// =========================================================================
#[tokio::test]
async fn emit_stop_reason_tool_use_for_tool_calls_finish_reason() {
    // @step Given a Rhai script mapping finish_reason "tool_calls" to tool_use
    let script = r#"
fn build_request(request) { #{} }
fn build_headers(config) { #{} }
fn build_url(config) { "" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn build_stream_request(request) { #{ stream: true } }
fn map_error(status, body) { #{ type: "api", message: body } }
fn parse_stream_chunk(config, data) {
    let event = json::parse(data);
    let ch = event.choices[0];
    if (type_of(ch["finish_reason"]) != "()") && type_of(ch.finish_reason) == "string" {
        if ch.finish_reason == "tool_calls" {
            return #{ kind: "stop", reason: "tool_use" };
        }
        if ch.finish_reason == "stop" {
            return #{ kind: "stop", reason: "end_turn" };
        }
    }
    #{ kind: "ignore" }
}
"#;

    // @step When the stream emits finish_reason "tool_calls"
    let (_tmp, results) = process_events(
        script,
        &[r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#],
    ).await;

    // @step Then the bridge yields StreamChunk::StopReason(ToolUse)
    let stops: Vec<_> = results
        .iter()
        .filter_map(|r| match r {
            Ok(StreamChunk::StopReason(sr)) => Some(*sr),
            _ => None,
        })
        .collect();
    assert_eq!(stops.len(), 1, "expected 1 StopReason, got {results:?}");
    assert_eq!(stops[0], StopReason::ToolUse);
}

// =========================================================================
// Scenario 7: Skip events when parse_stream_chunk returns ignore
// =========================================================================
#[tokio::test]
async fn skip_events_when_parse_stream_chunk_returns_ignore() {
    // @step Given a Rhai script that returns kind "ignore" for keepalive events
    let script = r#"
fn build_request(request) { #{} }
fn build_headers(config) { #{} }
fn build_url(config) { "" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn build_stream_request(request) { #{ stream: true } }
fn map_error(status, body) { #{ type: "api", message: body } }
fn parse_stream_chunk(config, data) {
    #{ kind: "ignore" }
}
"#;

    // @step When the SSE stream emits a keepalive event
    let (_tmp, results) = process_events(script, &[r#"{"keepalive":true}"#]).await;

    // @step Then no StreamChunk is yielded for that event
    assert!(
        results.is_empty(),
        "expected no chunks for keepalive; got {results:?}"
    );
}

// =========================================================================
// Scenario 8: Yield error and terminate on Rhai runtime error
// =========================================================================
#[tokio::test]
async fn yield_error_and_terminate_on_rhai_runtime_error() {
    // @step Given a Rhai script whose parse_stream_chunk throws a runtime error
    let script = r#"
fn build_request(request) { #{} }
fn build_headers(config) { #{} }
fn build_url(config) { "" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn build_stream_request(request) { #{ stream: true } }
fn map_error(status, body) { #{ type: "api", message: body } }
fn parse_stream_chunk(config, data) {
    throw "boom";
}
"#;

    // @step When the SSE stream emits any event the script throws on
    // Feed TWO events; after the first throw the second must NOT be processed.
    let (_tmp, results) = process_events(
        script,
        &[r#"{"event":"first"}"#, r#"{"event":"second"}"#],
    ).await;

    // @step Then the bridge yields a single Err(ProviderError::Api) and then terminates
    assert_eq!(
        results.len(),
        1,
        "expected exactly one Err after throw; got {results:?}"
    );
    match &results[0] {
        Err(ProviderError::Api { message, .. }) => {
            assert!(
                message.contains("boom") || message.contains("runtime"),
                "expected api error message to reference the throw; got {message}"
            );
        }
        other => panic!("expected Err(ProviderError::Api), got {other:?}"),
    }
}

// =========================================================================
// Scenario 9: Yield auth error before any chunk on 401 streaming response
// =========================================================================
#[tokio::test]
async fn yield_auth_error_before_any_chunk_on_401_streaming_response() {
    // @step Given a wiremock server responding with 401 to the streaming endpoint
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(401)
                .insert_header("content-type", "application/json")
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
fn build_stream_request(request) {{ #{{ messages: request.messages, stream: true }} }}
fn map_error(status, body) {{
    if status == 401 {{
        #{{ type: "auth", message: "unauthorized" }}
    }} else {{
        #{{ type: "api", message: body }}
    }}
}}
fn parse_stream_chunk(config, data) {{
    let event = json::parse(data);
    if (type_of(event["choices"]) != "()") {{
        let d = event.choices[0].delta;
        if (type_of(d["content"]) != "()") && type_of(d.content) == "string" {{
            return #{{ kind: "text_delta", text: d.content }};
        }}
    }}
    #{{ kind: "ignore" }}
}}
"#,
        base = server.uri()
    );
    let (_tmp, cfg) = streaming_config_with_script("my-llm", &server.uri(), &script);
    let provider = build_streaming_provider(cfg);

    // @step When RhaiCustomProvider starts a streaming completion
    let messages = vec![Message::user("hi")];
    let mut stream = provider
        .complete_with_tools_streaming(&messages, &[])
        .await;

    // @step Then the stream yields one Err(ProviderError::Auth) and no TextDelta chunks
    let mut got_auth = false;
    let mut text_count = 0;
    while let Some(item) = stream.next().await {
        match item {
            Err(ProviderError::Authentication { message, .. }) => {
                assert!(
                    message.contains("unauthorized"),
                    "expected auth message to contain 'unauthorized', got {message}"
                );
                got_auth = true;
            }
            Ok(StreamChunk::TextDelta(_)) => {
                text_count += 1;
            }
            other => panic!("expected Err(Authentication), got {other:?}"),
        }
    }
    assert!(got_auth, "expected an Authentication error to be yielded");
    assert_eq!(text_count, 0, "no TextDelta chunks should appear before auth error");
}

// =========================================================================
// Scenario 10: build_stream_request produces streaming body
// =========================================================================
#[tokio::test]
async fn build_stream_request_produces_streaming_body() {
    // @step Given a Rhai script whose build_stream_request clones build_request
    //        and sets "stream": true
    let script = r#"
fn build_request(request) { #{ messages: request.messages } }
fn build_headers(config) { #{} }
fn build_url(config) { "" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn build_stream_request(request) {
    let base = build_request(request);
    base.stream = true;
    base
}
fn map_error(status, body) { #{ type: "api", message: body } }
fn parse_stream_chunk(config, data) { #{ kind: "ignore" } }
"#;
    let (_tmp, cfg) = streaming_config_with_script("my-llm", "https://api.example.com", script);
    let provider = build_streaming_provider(cfg);

    // @step When the provider invokes build_stream_request with a user message
    let messages = vec![Message::user("hi")];
    let body = provider
        .invoke_build_stream_request(&messages, &[], None)
        .await
        .expect("build_stream_request returns JSON");

    // @step Then the returned JSON body has stream equal to true
    assert_eq!(
        body.get("stream").and_then(serde_json::Value::as_bool),
        Some(true),
        "expected body.stream == true; got {body}"
    );
    // Messages should still round-trip through build_request.
    let messages_arr = body
        .get("messages")
        .and_then(|v| v.as_array())
        .expect("messages array present");
    assert_eq!(messages_arr.len(), 1);
    assert_eq!(
        messages_arr[0].get("role").and_then(|v| v.as_str()),
        Some("user")
    );
}

// =========================================================================
// Scenario 11: End-to-end stream against mock SSE server
// =========================================================================
#[tokio::test]
async fn end_to_end_stream_against_mock_sse_server() {
    // @step Given a wiremock SSE endpoint returning three content deltas and one stop event
    let server = MockServer::start().await;
    let sse_body = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"lo, \"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"world\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
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
    if (type_of(event["choices"]) != "()") {{
        let ch = event.choices[0];
        if (type_of(ch["delta"]) != "()") && (type_of(ch.delta["content"]) != "()") && type_of(ch.delta.content) == "string" {{
            return #{{ kind: "text_delta", text: ch.delta.content }};
        }}
        if (type_of(ch["finish_reason"]) != "()") && type_of(ch.finish_reason) == "string" {{
            if ch.finish_reason == "stop" {{
                return #{{ kind: "stop", reason: "end_turn" }};
            }}
            if ch.finish_reason == "tool_calls" {{
                return #{{ kind: "stop", reason: "tool_use" }};
            }}
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
    let mut stream = provider
        .complete_with_tools_streaming(&messages, &[])
        .await;

    let mut collected: Vec<StreamChunk> = Vec::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => collected.push(chunk),
            Err(e) => panic!("unexpected streaming error: {e:?}"),
        }
    }

    // @step Then the collected chunks are TextDelta, TextDelta, TextDelta, StopReason
    //        in that exact order
    assert_eq!(
        collected.len(),
        4,
        "expected exactly 4 chunks; got {collected:?}"
    );
    match &collected[0] {
        StreamChunk::TextDelta(t) => assert_eq!(t, "Hel"),
        other => panic!("expected TextDelta(Hel), got {other:?}"),
    }
    match &collected[1] {
        StreamChunk::TextDelta(t) => assert_eq!(t, "lo, "),
        other => panic!("expected TextDelta(lo, ), got {other:?}"),
    }
    match &collected[2] {
        StreamChunk::TextDelta(t) => assert_eq!(t, "world"),
        other => panic!("expected TextDelta(world), got {other:?}"),
    }
    match &collected[3] {
        StreamChunk::StopReason(sr) => assert_eq!(*sr, StopReason::EndTurn),
        other => panic!("expected StopReason(EndTurn), got {other:?}"),
    }
}
