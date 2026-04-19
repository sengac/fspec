# PROV-064 AST Research — Streaming Integration Points

Conducted on 2026-04-17.

## Reuse targets

- `codelet/providers/src/custom/provider.rs` — extend `RhaiCustomProvider` with `complete_with_tools_streaming()` returning a pinned `Stream<Item = Result<StreamChunk, ProviderError>>`.
- `codelet/providers/src/custom/rhai_call.rs` — already provides `spawn_blocking` wrappers; add `call_fn2_string` specialization if needed.
- `codelet/providers/src/custom/response_bridge.rs` — reuse stop-reason mapping helper.
- `codelet/providers/src/custom/error_mapping.rs` — reuse for per-event Rhai error conversion and HTTP status mapping.
- `reqwest::Response::bytes_stream()` — drives raw HTTP byte stream.
- `eventsource-stream` or `async-sse` — parse SSE frames (research doc prefers `eventsource-stream`; add as workspace dep if not already present).

## New file `codelet/providers/src/custom/stream.rs` (under 300 lines)

- `pub enum StreamChunk { TextDelta(String), ToolCallStart { id, name }, ToolCallArgsDelta { id, chunk: String }, ToolCallComplete { id, name, input: serde_json::Value }, StopReason(StopReason) }`
- `pub async fn open_stream(...)` — returns `Pin<Box<dyn Stream<Item=Result<StreamChunk, ProviderError>> + Send>>`
- Internal `RhaiStreamProcessor` with `Mutex<HashMap<String, ToolCallAccumulator>>` keeping buffered arguments

## Tests use wiremock's streaming response (SSE): set Content-Type text/event-stream and body composed of `data: {...}\n\n` frames. Tool-call accumulation test exercises the "{\"pa" + "th\":\"a.txt\"}" split.
