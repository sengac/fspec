# PROV-064: Custom Provider Streaming SSE Bridge — Technical Research

## Table of Contents

1. [Existing Streaming Infrastructure](#1-existing-streaming-infrastructure)
2. [RhaiStreamProcessor Design](#2-rhaistreamprocessor-design)
3. [Tool Call Delta Accumulation](#3-tool-call-delta-accumulation)
4. [Stream Chunk Types](#4-stream-chunk-types)
5. [Performance Analysis](#5-performance-analysis)
6. [Integration with stream_loop](#6-integration-with-stream_loop)

---

## 1. Existing Streaming Infrastructure

### 1.1 Full Pipeline Overview

The streaming pipeline flows through four layers:

```
HTTP Response (bytes)
    │
    ▼
┌──────────────────────────────────────────┐
│  Layer 1: GenericEventSource (sse.rs)    │
│  eventsource_stream crate parses SSE    │
│  frames from raw HTTP byte stream       │
│  Output: Event::Open | Event::Message   │
└──────────────┬───────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────┐
│  Layer 2: Provider Streaming Module     │
│  (anthropic/streaming.rs or             │
│   openai/completion/streaming.rs)       │
│  Parses JSON from SSE data field        │
│  Output: RawStreamingChoice<R>          │
└──────────────┬───────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────┐
│  Layer 3: StreamingCompletionResponse   │
│  (streaming.rs — rig core)              │
│  Converts RawStreamingChoice → Stream   │
│  of StreamedAssistantContent<R>         │
│  Aggregates text + tool calls           │
└──────────────┬───────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────┐
│  Layer 4: Agent Multi-Turn Stream       │
│  (agent/prompt_request/streaming.rs)    │
│  Executes tools, manages multi-turn     │
│  Output: MultiTurnStreamItem<R>         │
└──────────────┬───────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────┐
│  Layer 5: stream_loop.rs                │
│  tokio::select! loop with interrupts    │
│  Updates UI, tracks tokens, handles     │
│  compaction, recovery, etc.             │
└──────────────────────────────────────────┘
```

### 1.2 Layer 1: SSE Event Parsing (`sse.rs`)

The SSE layer lives in `codelet/patches/rig-core/src/http_client/sse.rs`. It uses the `eventsource_stream` crate to parse raw HTTP byte streams into structured SSE events.

**Key struct — `GenericEventSource`:**

```rust
// From sse.rs — the generic SSE event source
pin_project! {
    pub struct GenericEventSource<HttpClient, RequestBody, ResponseBody>
    where
        HttpClient: HttpClientExt,
    {
        client: HttpClient,
        req: Request<RequestBody>,
        #[pin]
        next_response: Option<ResponseFuture<ResponseBody>>,
        #[pin]
        cur_stream: Option<EventStream>,
        #[pin]
        delay: Option<Delay>,
        is_closed: bool,
        retry_policy: BoxedRetry,
        last_event_id: String,
        last_retry: Option<(usize, Duration)>,
    }
}
```

**How it works:**

1. `GenericEventSource::new(client, req)` sends an HTTP request with `Accept: text/event-stream`
2. When the response arrives, `.handle_response()` wraps the body in `eventsource_stream::Eventsource`
3. The `Stream` impl polls `cur_stream` which yields `Event::Open` or `Event::Message(MessageEvent)`
4. `MessageEvent` contains `data: String`, `id: String`, `event: String`, and `retry: Option<Duration>`

```rust
// From sse.rs — the Event enum
pub enum Event {
    Open,
    Message(MessageEvent),  // MessageEvent { data, id, event, retry }
}
```

**Critical detail**: The `eventsource_stream` crate handles the SSE wire protocol (line-based parsing, multi-line data concatenation, event type routing). By the time our code sees an `Event::Message`, the `data` field is a complete string ready for JSON parsing.

### 1.3 Layer 2: Provider-Specific Streaming

Each provider implements a `stream()` method on its `CompletionModel` that:
1. Creates a `GenericEventSource` from an HTTP request
2. Iterates SSE events in an `async_stream::stream!` block
3. Parses JSON from `sse.data` into provider-specific event structs
4. Yields `RawStreamingChoice<R>` items

#### 1.3.1 Anthropic Streaming (`anthropic/streaming.rs`)

Anthropic uses a **block-based** protocol with explicit start/delta/stop events:

```rust
// From anthropic/streaming.rs — event types
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamingEvent {
    MessageStart { message: MessageStart },
    ContentBlockStart { index: usize, content_block: Content },
    ContentBlockDelta { index: usize, delta: ContentDelta },
    ContentBlockStop { index: usize },
    MessageDelta { delta: MessageDelta, usage: PartialUsage },
    MessageStop,
    Ping,
    #[serde(other)]
    Unknown,
}
```

**Tool call accumulation** uses a single `ToolCallState`:

```rust
// From anthropic/streaming.rs
#[derive(Default)]
struct ToolCallState {
    name: String,
    id: String,
    input_json: String,
}
```

The flow is:
1. `ContentBlockStart` with `Content::ToolUse { id, name }` → creates `ToolCallState`
2. `ContentBlockDelta` with `InputJsonDelta { partial_json }` → appends to `input_json`, yields `ToolCallDelta`
3. `ContentBlockStop` → parses accumulated JSON, yields complete `ToolCall`

#### 1.3.2 OpenAI Streaming (`openai/completion/streaming.rs`)

OpenAI uses an **index-based** delta protocol:

```rust
// From openai/completion/streaming.rs
struct StreamingToolCall {
    index: usize,           // Index identifies which tool call this delta belongs to
    id: Option<String>,
    function: StreamingFunction,
}

struct StreamingFunction {
    name: Option<String>,
    arguments: Option<String>,
}
```

**Tool call accumulation** uses a `HashMap<usize, ToolCall>`:

```rust
// From openai/completion/streaming.rs — accumulation pattern
let mut tool_calls: HashMap<usize, ToolCall> = HashMap::new();

// For each delta:
let existing_tool_call = tool_calls.entry(index).or_insert_with(|| ToolCall {
    id: String::new(),
    call_id: None,
    function: ToolFunction {
        name: String::new(),
        arguments: serde_json::Value::Null,
    },
    signature: None,
    additional_params: None,
});

// Update fields if present
if let Some(id) = &tool_call.id && !id.is_empty() {
    existing_tool_call.id = id.clone();
}
if let Some(name) = &tool_call.function.name && !name.is_empty() {
    existing_tool_call.function.name = name.clone();
    yield Ok(RawStreamingChoice::ToolCallDelta {
        id: existing_tool_call.id.clone(),
        content: ToolCallDeltaContent::Name(name.clone()),
    });
}
if let Some(chunk) = &tool_call.function.arguments && !chunk.is_empty() {
    // Concatenate chunks, try to parse when braces balance
    let combined = format!("{current_args}{chunk}");
    // ... parse logic ...
    yield Ok(RawStreamingChoice::ToolCallDelta {
        id: existing_tool_call.id.clone(),
        content: ToolCallDeltaContent::Delta(chunk.clone()),
    });
}
```

### 1.4 Layer 3: `RawStreamingChoice` → `StreamedAssistantContent`

The `StreamingCompletionResponse<R>` struct in `streaming.rs` implements `Stream` and converts `RawStreamingChoice<R>` items into `StreamedAssistantContent<R>`:

```rust
// From streaming.rs — the normalized stream item types
pub enum RawStreamingChoice<R> where R: Clone {
    Message(String),                              // Text chunk
    ToolCall(RawStreamingToolCall),                // Complete tool call
    ToolCallDelta { id: String, content: ToolCallDeltaContent },
    Reasoning { id: Option<String>, reasoning: String, signature: Option<String> },
    ReasoningDelta { id: Option<String>, reasoning: String },
    Usage(crate::completion::Usage),               // Token usage update
    FinalResponse(R),                              // Provider-specific final response
}

pub enum StreamedAssistantContent<R> {
    Text(Text),
    ToolCall(ToolCall),
    ToolCallDelta { id: String, content: ToolCallDeltaContent },
    Reasoning(Reasoning),
    ReasoningDelta { id: Option<String>, reasoning: String },
    Usage(crate::completion::Usage),
    Final(R),
}
```

### 1.5 Layer 4: Multi-Turn Stream (`agent/prompt_request/streaming.rs`)

The agent layer wraps provider streams in a multi-turn loop:

```rust
// From agent/prompt_request/streaming.rs
pub enum MultiTurnStreamItem<R> {
    StreamAssistantItem(StreamedAssistantContent<R>),
    StreamUserItem(StreamedUserContent),          // Tool results
    Usage(crate::completion::Usage),
    FinalResponse(FinalResponse),
}
```

This layer:
1. Receives `StreamedAssistantContent` from the provider stream
2. When a `ToolCall` arrives, executes it and yields `StreamUserItem(ToolResult)`
3. Then re-prompts the model with the tool result (multi-turn)
4. Yields `FinalResponse` when the model is done

### 1.6 Layer 5: `stream_loop.rs` — The `select!` Loop

The `stream_loop.rs` file (~1800 lines) is the outermost consumer. Its core is a `loop` with `tokio::select!`:

```rust
// From stream_loop.rs — the main select! loop (simplified)
loop {
    // Check interruption
    if is_interrupted.load(Acquire) { break; }

    let chunk = match (&mut event_stream, &mut status_interval, &status) {
        (Some(es), Some(si), Some(st)) => {
            // CLI mode
            tokio::select! {
                c = stream.next() => Some(c),
                event = es.next() => {
                    // Handle Esc key
                    None
                }
                _ = si.tick() => { None }  // Status update
                _ = tokio::time::sleep(effective_stall_timeout) => {
                    // Stall timeout
                    return Err(...)
                }
            }
        }
        _ => {
            // NAPI mode — uses interrupt_notify
            tokio::select! {
                c = stream.next() => Some(c),
                _ = interrupt_fut => None,
                _ = tokio::time::sleep(effective_stall_timeout) => { ... }
            }
        }
    };

    // Process chunk
    if let Some(chunk) = chunk {
        match chunk {
            Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::Text(text)))) => { ... }
            Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::ToolCall(tool_call)))) => { ... }
            Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::ReasoningDelta { reasoning, .. }))) => { ... }
            Some(Ok(MultiTurnStreamItem::StreamUserItem(
                StreamedUserContent::ToolResult(tool_result)))) => { ... }
            Some(Ok(MultiTurnStreamItem::Usage(usage))) => { ... }
            Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => { ... }
            Some(Err(e)) => { /* error recovery: compaction, truncation, network */ }
            None => { /* stream ended */ }
        }
    }
}
```

**Key observation for PROV-064**: The `stream_loop` is completely **provider-agnostic**. It only processes `MultiTurnStreamItem<R>` items. The custom Rhai provider needs to produce `RawStreamingChoice<R>` items at Layer 2, and the existing Layers 3–5 handle everything else.

---

## 2. RhaiStreamProcessor Design

### 2.1 Where It Sits in the Pipeline

The `RhaiStreamProcessor` replaces Layer 2 (provider-specific parsing) for custom providers. Instead of hardcoded `serde_json::from_str::<StreamingEvent>`, it calls a Rhai script function to interpret each SSE event:

```
GenericEventSource (Layer 1)
    │
    │  Event::Message { data, event_type }
    ▼
┌──────────────────────────────────────────┐
│  RhaiStreamProcessor (NEW — Layer 2)    │
│                                          │
│  For each SSE event:                     │
│    1. Call Rhai parse_stream_chunk()      │
│    2. Convert returned Map to            │
│       RawStreamingChoice                 │
│    3. Accumulate tool call deltas        │
│                                          │
│  Output: RawStreamingChoice<R>           │
└──────────────┬───────────────────────────┘
               │
               ▼
StreamingCompletionResponse (Layer 3) — unchanged
```

### 2.2 Struct Definition

```rust
use rhai::{AST, Dynamic, Engine, Map, Scope};
use std::collections::HashMap;
use std::sync::Arc;

/// Processes SSE events through a Rhai script's parse_stream_chunk function.
///
/// Holds the pre-compiled AST and engine, plus mutable state for
/// tool call delta accumulation across SSE events.
pub struct RhaiStreamProcessor {
    /// The Rhai engine (sandboxed, with safety limits)
    engine: Arc<Engine>,
    /// Pre-compiled AST of the provider script
    ast: AST,
    /// Provider config map passed to parse_stream_chunk as first arg
    config: Dynamic,
    /// Reusable Scope — cleared between calls but avoids reallocation
    scope: Scope<'static>,
    /// Tool call accumulator: index → (id, name, arguments_json)
    tool_calls: HashMap<usize, ToolCallAccumulator>,
    /// Thinking/reasoning accumulator
    thinking: Option<ThinkingAccumulator>,
}

struct ToolCallAccumulator {
    id: String,
    name: String,
    arguments_json: String,
}

struct ThinkingAccumulator {
    thinking: String,
    signature: String,
}
```

### 2.3 How It Receives SSE Events

The `RhaiStreamProcessor` is called from within an `async_stream::stream!` block, following the same pattern as Anthropic and OpenAI providers. The `stream()` method on the custom `CompletionModel` creates a `GenericEventSource` and iterates it:

```rust
// Inside the custom CompletionModel::stream() implementation
let stream: StreamingResult<RhaiStreamingResponse> = Box::pin(stream! {
    let mut processor = RhaiStreamProcessor::new(engine, ast, config);
    let mut sse_stream = Box::pin(GenericEventSource::new(client, req));

    while let Some(sse_result) = sse_stream.next().await {
        match sse_result {
            Ok(Event::Open) => {
                tracing::debug!("[rhai/streaming] SSE connection opened");
            }
            Ok(Event::Message(sse)) => {
                // sse.data is the complete SSE data field (already assembled
                // from multi-line SSE frames by eventsource_stream)
                // sse.event is the SSE event type (e.g. "message", "error")

                // Process through Rhai — returns zero or more RawStreamingChoice items
                match processor.process_event(&sse.event, &sse.data) {
                    Ok(chunks) => {
                        for chunk in chunks {
                            yield Ok(chunk);
                        }
                    }
                    Err(e) => {
                        yield Err(CompletionError::ResponseError(
                            format!("Rhai stream processing error: {e}")
                        ));
                    }
                }
            }
            Err(e) => {
                yield Err(CompletionError::ProviderError(
                    format!("SSE Error: {e}")
                ));
                break;
            }
        }
    }

    sse_stream.close();

    // Emit final response
    yield Ok(RawStreamingChoice::FinalResponse(
        processor.build_final_response()
    ));
});
```

### 2.4 Per-Event Rhai Call

The core of `process_event` calls the Rhai function synchronously (Rhai has no async runtime):

```rust
impl RhaiStreamProcessor {
    /// Process a single SSE event through the Rhai script.
    ///
    /// Calls `parse_stream_chunk(config, event_type, data)` which must return
    /// a Map (or Array of Maps) with a "type" field indicating the chunk kind.
    ///
    /// This is called per-SSE-event, NOT per-byte. A typical completion has
    /// 50–200 SSE events, so even at ~1μs per Rhai call, total overhead is <0.2ms.
    pub fn process_event(
        &mut self,
        event_type: &str,
        data: &str,
    ) -> Result<Vec<RawStreamingChoice<RhaiStreamingResponse>>, anyhow::Error> {
        // Skip [DONE] sentinel
        if data.trim() == "[DONE]" {
            return Ok(vec![]);
        }

        // Call Rhai: parse_stream_chunk(config, event_type, data) -> Map | Array
        let result: Dynamic = self.engine.call_fn(
            &mut self.scope,
            &self.ast,
            "parse_stream_chunk",
            (self.config.clone(), data.to_string(), event_type.to_string()),
        ).map_err(|e| anyhow::anyhow!("parse_stream_chunk failed: {e}"))?;

        // Handle single Map or Array of Maps
        let maps: Vec<Map> = if result.is_array() {
            result.into_array()
                .map_err(|_| anyhow::anyhow!("Expected array"))?
                .into_iter()
                .filter_map(|d| d.try_cast::<Map>())
                .collect()
        } else if result.is_map() {
            vec![result.try_cast::<Map>()
                .ok_or_else(|| anyhow::anyhow!("Expected Map"))?]
        } else if result.is_unit() {
            // Script returned () — ignore this event
            return Ok(vec![]);
        } else {
            return Err(anyhow::anyhow!(
                "parse_stream_chunk must return Map, Array, or () — got {:?}",
                result.type_name()
            ));
        };

        // Convert each Map to RawStreamingChoice
        let mut choices = Vec::with_capacity(maps.len());
        for map in maps {
            if let Some(choice) = self.convert_map_to_choice(map)? {
                choices.push(choice);
            }
        }

        Ok(choices)
    }
}
```

### 2.5 Converting Rhai Map to `RawStreamingChoice`

```rust
impl RhaiStreamProcessor {
    fn convert_map_to_choice(
        &mut self,
        map: Map,
    ) -> Result<Option<RawStreamingChoice<RhaiStreamingResponse>>, anyhow::Error> {
        let chunk_type = map.get("type")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();

        match chunk_type.as_str() {
            "text" => {
                let text = map.get("text")
                    .and_then(|v| v.clone().into_string().ok())
                    .unwrap_or_default();
                Ok(Some(RawStreamingChoice::Message(text)))
            }
            "tool_call_delta" => {
                self.handle_tool_call_delta(&map)
            }
            "thinking_delta" => {
                let content = map.get("content")
                    .and_then(|v| v.clone().into_string().ok())
                    .unwrap_or_default();
                // Accumulate thinking
                if self.thinking.is_none() {
                    self.thinking = Some(ThinkingAccumulator::default());
                }
                if let Some(ref mut state) = self.thinking {
                    state.thinking.push_str(&content);
                }
                Ok(Some(RawStreamingChoice::ReasoningDelta {
                    id: None,
                    reasoning: content,
                }))
            }
            "usage" => {
                let input = map.get("input_tokens")
                    .and_then(|v| v.as_int().ok())
                    .unwrap_or(0) as u64;
                let output = map.get("output_tokens")
                    .and_then(|v| v.as_int().ok())
                    .unwrap_or(0) as u64;
                let mut usage = crate::completion::Usage::new();
                usage.input_tokens = input;
                usage.output_tokens = output;
                usage.total_tokens = input + output;
                // Optional cache fields
                if let Some(v) = map.get("cache_read_input_tokens") {
                    usage.cache_read_input_tokens = v.as_int().ok().map(|n| n as u64);
                }
                Ok(Some(RawStreamingChoice::Usage(usage)))
            }
            "stop" => {
                // Flush any pending thinking block
                if let Some(thinking) = self.thinking.take() {
                    if !thinking.thinking.is_empty() {
                        return Ok(Some(RawStreamingChoice::Reasoning {
                            id: None,
                            reasoning: thinking.thinking,
                            signature: if thinking.signature.is_empty() {
                                None
                            } else {
                                Some(thinking.signature)
                            },
                        }));
                    }
                }
                Ok(None) // stop is handled by FinalResponse at stream end
            }
            "done" => {
                // Explicit done signal — flush pending tool calls
                let mut results = vec![];
                for (_idx, tc) in self.tool_calls.drain() {
                    let args = if tc.arguments_json.is_empty() {
                        serde_json::Value::Object(Default::default())
                    } else {
                        serde_json::from_str(&tc.arguments_json)
                            .unwrap_or(serde_json::Value::String(tc.arguments_json))
                    };
                    results.push(RawStreamingChoice::ToolCall(
                        RawStreamingToolCall::new(tc.id, tc.name, args)
                    ));
                }
                Ok(results.into_iter().next().map(|_| todo!("handle multiple")))
                // In practice, done is just a signal — tool calls flush on stop
            }
            "ignore" | "" => Ok(None),
            other => {
                tracing::debug!("[rhai/streaming] Unknown chunk type: {}", other);
                Ok(None)
            }
        }
    }
}
```

---

## 3. Tool Call Delta Accumulation

### 3.1 The Problem

Streaming tool calls never arrive as a single complete JSON payload. Both Anthropic and OpenAI send them as a sequence of small deltas that must be accumulated client-side before the complete tool call can be executed.

### 3.2 Real SSE Event Sequences

#### 3.2.1 Anthropic Tool Call SSE Sequence

A typical Anthropic tool call for `Read(file_path="/tmp/test.rs")`:

```
event: message_start
data: {"type":"message_start","message":{"id":"msg_01X","role":"assistant",
       "content":[],"model":"claude-sonnet-4-20250514","usage":{"input_tokens":1024,
       "output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":
       {"type":"tool_use","id":"toolu_01ABC","name":"Read"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":
       {"type":"input_json_delta","partial_json":"{\"file"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":
       {"type":"input_json_delta","partial_json":"_path\":"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":
       {"type":"input_json_delta","partial_json":"\"/tmp/test.rs\"}"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},
       "usage":{"output_tokens":42}}

event: message_stop
data: {"type":"message_stop"}
```

**Key observations:**
- `content_block_start` announces the tool name and ID upfront
- `input_json_delta` chunks arrive as partial JSON strings
- `content_block_stop` signals that all deltas have been received
- Anthropic tracks by **block index** (one tool at a time within a content block)

#### 3.2.2 OpenAI Tool Call SSE Sequence

A typical OpenAI tool call for `get_weather(location="Paris")`:

```
data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_abc123",
       "function":{"name":"get_weather","arguments":""}}]}}]}

data: {"choices":[{"delta":{"tool_calls":[{"index":0,
       "function":{"arguments":"{\"loc"}}]}}]}

data: {"choices":[{"delta":{"tool_calls":[{"index":0,
       "function":{"arguments":"ation"}}]}}]}

data: {"choices":[{"delta":{"tool_calls":[{"index":0,
       "function":{"arguments":"\":\"Paris\"}"}}]}}]}

data: {"choices":[{"delta":{},"finish_reason":"tool_calls"}]}

data: [DONE]
```

**Key observations:**
- First chunk contains `id` and `name`; subsequent chunks have `id: null`, `name: null`
- Arguments arrive as string fragments (not JSON objects)
- Multiple parallel tool calls use different `index` values
- `finish_reason: "tool_calls"` signals all tool calls are complete

### 3.3 How Existing Providers Accumulate Deltas

#### 3.3.1 Anthropic Pattern (Single Accumulator)

Anthropic streams one tool call at a time per content block:

```rust
// From anthropic/streaming.rs — simplified
let mut current_tool_call: Option<ToolCallState> = None;

// On ContentBlockStart with ToolUse:
*current_tool_call = Some(ToolCallState {
    name: name.clone(),
    id: id.clone(),
    input_json: String::new(),
});

// On ContentBlockDelta with InputJsonDelta:
if let Some(tool_call) = current_tool_call {
    tool_call.input_json.push_str(partial_json);
    // Emit delta for UI progress
    yield Ok(RawStreamingChoice::ToolCallDelta {
        id: tool_call.id.clone(),
        content: ToolCallDeltaContent::Delta(partial_json.clone()),
    });
}

// On ContentBlockStop:
if let Some(tool_call) = Option::take(current_tool_call) {
    let json_value = serde_json::from_str(&tool_call.input_json)?;
    yield Ok(RawStreamingChoice::ToolCall(
        RawStreamingToolCall::new(tool_call.id, tool_call.name, json_value)
    ));
}
```

#### 3.3.2 OpenAI Pattern (HashMap Accumulator)

OpenAI can stream multiple tool calls in parallel using index:

```rust
// From openai/completion/streaming.rs — simplified
let mut tool_calls: HashMap<usize, ToolCall> = HashMap::new();

for tool_call_delta in &delta.tool_calls {
    let index = tool_call_delta.index;
    let entry = tool_calls.entry(index).or_insert_with(|| ToolCall { ... });

    // Accumulate name, id, arguments across chunks
    if let Some(name) = &tool_call_delta.function.name {
        entry.function.name = name.clone();
    }
    if let Some(chunk) = &tool_call_delta.function.arguments {
        // String concatenation of argument fragments
        let combined = format!("{current_args}{chunk}");
        entry.function.arguments = serde_json::Value::String(combined);
    }
}

// On finish_reason == "tool_calls":
for (_idx, tool_call) in tool_calls.into_iter() {
    yield Ok(RawStreamingChoice::ToolCall(
        RawStreamingToolCall::new(tool_call.id, tool_call.function.name,
                                  tool_call.function.arguments)
    ));
}
```

### 3.4 Rust Accumulator Design for Rhai Custom Providers

The Rhai script returns `tool_call_delta` chunks with an index, id, name, and/or argument fragments. The Rust accumulator collects these and emits complete `ToolCall` items:

```rust
impl RhaiStreamProcessor {
    fn handle_tool_call_delta(
        &mut self,
        map: &Map,
    ) -> Result<Option<RawStreamingChoice<RhaiStreamingResponse>>, anyhow::Error> {
        let index = map.get("index")
            .and_then(|v| v.as_int().ok())
            .unwrap_or(0) as usize;

        let entry = self.tool_calls.entry(index).or_insert_with(|| {
            ToolCallAccumulator {
                id: String::new(),
                name: String::new(),
                arguments_json: String::new(),
            }
        });

        // Update ID if provided
        if let Some(id) = map.get("id").and_then(|v| v.clone().into_string().ok()) {
            if !id.is_empty() {
                entry.id = id;
            }
        }

        // Update name if provided — emit ToolCallDelta::Name
        if let Some(name) = map.get("name").and_then(|v| v.clone().into_string().ok()) {
            if !name.is_empty() {
                entry.name = name.clone();
                return Ok(Some(RawStreamingChoice::ToolCallDelta {
                    id: entry.id.clone(),
                    content: ToolCallDeltaContent::Name(name),
                }));
            }
        }

        // Accumulate argument fragments — emit ToolCallDelta::Delta
        if let Some(args) = map.get("arguments").and_then(|v| v.clone().into_string().ok()) {
            if !args.is_empty() {
                entry.arguments_json.push_str(&args);
                return Ok(Some(RawStreamingChoice::ToolCallDelta {
                    id: entry.id.clone(),
                    content: ToolCallDeltaContent::Delta(args),
                }));
            }
        }

        // "complete" flag signals this tool call is finished
        if map.get("complete").and_then(|v| v.as_bool().ok()).unwrap_or(false) {
            if let Some(tc) = self.tool_calls.remove(&index) {
                let args = if tc.arguments_json.is_empty() {
                    serde_json::json!({})
                } else {
                    serde_json::from_str(&tc.arguments_json)
                        .unwrap_or(serde_json::Value::String(tc.arguments_json))
                };
                return Ok(Some(RawStreamingChoice::ToolCall(
                    RawStreamingToolCall::new(tc.id, tc.name, args)
                )));
            }
        }

        Ok(None)
    }

    /// Flush all pending tool calls (called on stream end or "done" event)
    fn flush_tool_calls(&mut self) -> Vec<RawStreamingChoice<RhaiStreamingResponse>> {
        let mut results = Vec::new();
        for (_idx, tc) in self.tool_calls.drain() {
            let args = if tc.arguments_json.is_empty() {
                serde_json::json!({})
            } else {
                serde_json::from_str(&tc.arguments_json)
                    .unwrap_or(serde_json::Value::String(tc.arguments_json))
            };
            results.push(RawStreamingChoice::ToolCall(
                RawStreamingToolCall::new(tc.id, tc.name, args)
            ));
        }
        results
    }
}
```

### 3.5 Example Rhai Script: Anthropic-Compatible Parsing

```rhai
// parse_stream_chunk(config, data, event_type) -> Map | Array | ()
fn parse_stream_chunk(config, data, event_type) {
    let event = json::parse(data);
    let etype = event["type"];

    if etype == "message_start" {
        let usage = event["message"]["usage"];
        return #{
            type: "usage",
            input_tokens: usage["input_tokens"],
            output_tokens: 0
        };
    }

    if etype == "content_block_start" {
        let block = event["content_block"];
        if block["type"] == "tool_use" {
            return #{
                type: "tool_call_delta",
                index: event["index"],
                id: block["id"],
                name: block["name"]
            };
        }
        return ();  // ignore other block types
    }

    if etype == "content_block_delta" {
        let delta = event["delta"];
        if delta["type"] == "text_delta" {
            return #{ type: "text", text: delta["text"] };
        }
        if delta["type"] == "input_json_delta" {
            return #{
                type: "tool_call_delta",
                index: event["index"],
                arguments: delta["partial_json"]
            };
        }
        if delta["type"] == "thinking_delta" {
            return #{ type: "thinking_delta", content: delta["thinking"] };
        }
        return ();
    }

    if etype == "content_block_stop" {
        return #{
            type: "tool_call_delta",
            index: event["index"],
            complete: true
        };
    }

    if etype == "message_delta" {
        let usage = event["usage"];
        return #{
            type: "usage",
            input_tokens: 0,
            output_tokens: usage["output_tokens"]
        };
    }

    return ();  // ping, message_stop, unknown → ignore
}
```

---

## 4. Stream Chunk Types

### 4.1 Normalized Chunk Type Table

The Rhai `parse_stream_chunk` function returns a Map with a `type` field. Here are all supported types and their mapping to rig's `RawStreamingChoice`:

| Rhai `type` | Required Fields | Optional Fields | Maps To | Purpose |
|---|---|---|---|---|
| `text` | `text: String` | — | `RawStreamingChoice::Message(text)` | Text content delta |
| `tool_call_delta` | — | `index: i64`, `id: String`, `name: String`, `arguments: String`, `complete: bool` | `RawStreamingChoice::ToolCallDelta { id, content }` or `::ToolCall(...)` when `complete: true` | Progressive tool call construction |
| `thinking_delta` | `content: String` | — | `RawStreamingChoice::ReasoningDelta { id: None, reasoning }` | Extended thinking / chain-of-thought |
| `usage` | — | `input_tokens: i64`, `output_tokens: i64`, `cache_read_input_tokens: i64`, `cache_creation_input_tokens: i64` | `RawStreamingChoice::Usage(Usage)` | Token usage updates |
| `stop` | — | `reason: String` | Flushes pending thinking block, captures stop_reason | End-of-generation signal |
| `done` | — | — | Flushes all pending tool calls | Explicit stream termination |
| `ignore` or `""` | — | — | `None` (skipped) | Events to ignore (pings, unknown) |

### 4.2 How Each Type Flows Through the Pipeline

```
Rhai returns:            Rust converts to:              stream_loop sees:
─────────────            ─────────────────              ──────────────────
#{type:"text",      →  RawStreamingChoice::Message   →  MultiTurnStreamItem::
  text:"Hello"}         ("Hello")                        StreamAssistantItem(Text("Hello"))

#{type:"tool_call   →  RawStreamingChoice::           →  MultiTurnStreamItem::
  _delta",              ToolCallDelta{id,Name(n)}        StreamAssistantItem(ToolCallDelta{..})
  name:"Read"}

#{type:"tool_call   →  RawStreamingChoice::           →  MultiTurnStreamItem::
  _delta",              ToolCallDelta{id,Delta(a)}       StreamAssistantItem(ToolCallDelta{..})
  arguments:"..."}

#{type:"tool_call   →  RawStreamingChoice::           →  MultiTurnStreamItem::
  _delta",              ToolCall(RawStreamingToolCall)    StreamAssistantItem(ToolCall(..))
  complete:true}                                         → triggers tool execution

#{type:"thinking    →  RawStreamingChoice::           →  MultiTurnStreamItem::
  _delta",              ReasoningDelta{reasoning}        StreamAssistantItem(ReasoningDelta{..})
  content:"..."}

#{type:"usage",     →  RawStreamingChoice::           →  MultiTurnStreamItem::
  input_tokens:100}     Usage(Usage{...})                Usage(Usage{...})
```

### 4.3 Design Decision: Why Rust Accumulates, Not Rhai

Tool call accumulation happens in **Rust** (the `RhaiStreamProcessor`), not in Rhai, for three reasons:

1. **Statefulness across calls**: The Rhai function is called independently for each SSE event. Maintaining a mutable `HashMap` across calls would require persistent Scope variables, adding complexity and breaking the clean function-call pattern.

2. **Performance**: String concatenation of potentially large JSON arguments (e.g., multi-KB `Write` tool calls) is much faster in Rust than Rhai. The accumulator avoids creating Rhai `Dynamic` values for intermediate state.

3. **Consistency**: All existing rig providers accumulate in Rust. The Rhai script only needs to identify *what kind* of delta each SSE event represents — the mechanical work of collecting fragments is identical regardless of provider.

---

## 5. Performance Analysis

### 5.1 SSE Event Frequency

Based on the existing Anthropic and OpenAI provider implementations:

| Metric | Anthropic | OpenAI | Custom (expected) |
|---|---|---|---|
| Text-only completion (500 tokens) | ~500 SSE events (1 per token) | ~500 SSE events | ~500 SSE events |
| Tool call (medium args) | ~20–50 SSE events | ~10–30 SSE events | ~10–50 SSE events |
| Typical agent turn (text + 2 tools) | ~100–200 SSE events | ~50–150 SSE events | ~50–200 SSE events |
| Event arrival rate | ~50–200/sec | ~50–200/sec | ~50–200/sec |
| Time between events | 5–20ms | 5–20ms | 5–20ms |

### 5.2 Rhai `Engine::call_fn` Overhead

Examining `call_fn` in `/tmp/rhai/src/api/call_fn.rs`:

```rust
// From call_fn.rs — the public entry point
#[inline(always)]
pub fn call_fn<T: Variant + Clone>(
    &self,
    scope: &mut Scope,
    ast: &AST,
    fn_name: impl AsRef<str>,
    args: impl FuncArgs,
) -> RhaiResultOf<T> {
    self.call_fn_with_options(<_>::default(), scope, ast, fn_name, args)
}
```

The internal `_call_fn` does:
1. Save/restore global state (source, lib, tag, module resolver) — ~100ns
2. Optionally evaluate AST global statements (skipped with `eval_ast: false`) — 0ns if skipped
3. Look up function in `ast.shared_lib()` — hash lookup, ~50ns
4. Call `call_script_fn` which:
   - `track_operation` — counter increment + comparison, ~10ns
   - Stack overflow check — comparison, ~5ns
   - Push args into scope — 3 args × ~30ns = ~90ns
   - Execute function body — depends on script complexity
5. Restore global state — ~100ns

**For a typical `parse_stream_chunk` function** (10–20 Rhai operations: a JSON parse, 3–5 comparisons, 1 map construction):

| Component | Estimated Time |
|---|---|
| call_fn setup/teardown | ~200ns |
| Function lookup | ~50ns |
| Scope management (3 args) | ~90ns |
| Script execution (~15 operations) | ~500ns–1μs |
| Result extraction + Map creation | ~100ns |
| **Total per call** | **~1–1.5μs** |

### 5.3 Optimization: `eval_ast: false`

The default `CallFnOptions` has `eval_ast: true`, which evaluates all top-level AST statements before each function call. For streaming, we should use `call_fn_with_options` with `eval_ast: false`:

```rust
// From call_fn.rs — the options struct
pub struct CallFnOptions<'t> {
    pub this_ptr: Option<&'t mut Dynamic>,
    pub tag: Option<Dynamic>,
    pub eval_ast: bool,        // ← Set to false for repeated calls
    pub rewind_scope: bool,
    pub in_all_namespaces: bool,
}
```

With `eval_ast: false`, the engine skips `eval_global_statements` entirely, saving ~200ns per call on scripts with top-level statements. The first call should use `eval_ast: true` to initialize any global state, then subsequent calls use `false`.

### 5.4 Total Overhead Per Completion

```
Typical completion: 100–200 SSE events
Rhai call per event: ~1–1.5μs
────────────────────────────────────
Total Rhai overhead: 100–300μs  (0.1–0.3ms)

Compare to:
  - Network latency per SSE event: ~5–20ms
  - JSON parsing (serde_json) per event: ~1–5μs
  - Token generation time on server: ~10–30ms per token
  - Total completion wall time: 5–30 seconds

Rhai overhead as % of wall time: 0.001–0.006%
```

**Conclusion**: Per-event Rhai invocation is negligible. The ~1μs per call is dwarfed by the ~10ms between SSE events. Even at the extreme (500 events × 1.5μs = 0.75ms), it's under 1ms total overhead per completion.

### 5.5 Why Per-Event (Not Per-Byte) Is the Right Granularity

The `eventsource_stream` crate handles byte-level SSE frame assembly. By the time `Event::Message` arrives, the `data` field is a complete string. Calling Rhai per-byte would be:

1. **Unnecessary**: SSE frames are already assembled
2. **Incorrect**: Partial JSON can't be meaningfully parsed
3. **Wasteful**: Would multiply call count by ~100x (each SSE event is ~100 bytes)

Per-event is the natural boundary — it matches what all existing providers do with `serde_json::from_str`.

---

## 6. Integration with `stream_loop`

### 6.1 Zero Changes Required in `stream_loop.rs`

The `stream_loop` processes `MultiTurnStreamItem<R>` items and is completely agnostic to the provider that produced them. A custom Rhai provider needs to:

1. Implement `CompletionModel` trait (with `stream()` method)
2. Produce `RawStreamingChoice<R>` items from its `stream()` method
3. Let rig's existing `StreamingCompletionResponse` (Layer 3) and agent multi-turn (Layer 4) handle the rest

The `stream_loop` already handles all necessary chunk types:

```rust
// From stream_loop.rs — the match arms (already exist, no changes needed)
match chunk {
    // Text chunks — displayed to user
    Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
        StreamedAssistantContent::Text(text)))) => { handle_text_chunk(...) }

    // Tool calls — tool_execution_in_progress = true
    Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
        StreamedAssistantContent::ToolCall(tool_call)))) => { handle_tool_call(...) }

    // Reasoning/thinking — displayed in thinking panel
    Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
        StreamedAssistantContent::ReasoningDelta { reasoning, .. }))) => { emit_thinking(...) }

    // Tool results — tool_execution_in_progress = false
    Some(Ok(MultiTurnStreamItem::StreamUserItem(
        StreamedUserContent::ToolResult(tool_result)))) => { handle_tool_result(...) }

    // Usage — token tracking
    Some(Ok(MultiTurnStreamItem::Usage(usage))) => { update_token_display(...) }

    // Final response — captures stop_reason, triggers compaction check
    Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => { ... }

    // Errors — recovery (compaction, truncation, network, etc.)
    Some(Err(e)) => { error_recovery(...) }
}
```

### 6.2 The Custom CompletionModel Implementation

The Rhai custom provider implements rig's `CompletionModel` trait:

```rust
use rig::completion::{CompletionModel, CompletionRequest, CompletionResponse, CompletionError};
use rig::streaming::{self, RawStreamingChoice, StreamingCompletionResponse};

#[derive(Clone)]
pub struct RhaiCompletionModel<T: HttpClientExt> {
    client: T,
    model: String,
    engine: Arc<Engine>,
    ast: AST,
    config: Dynamic,
}

/// The streaming response type for Rhai providers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RhaiStreamingResponse {
    pub usage: rig::completion::Usage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
}

impl GetTokenUsage for RhaiStreamingResponse {
    fn token_usage(&self) -> Option<rig::completion::Usage> {
        Some(self.usage)
    }
    fn stop_reason(&self) -> Option<&str> {
        self.stop_reason.as_deref()
    }
}

impl<T: HttpClientExt + Clone + 'static> CompletionModel for RhaiCompletionModel<T> {
    type Response = serde_json::Value;
    type StreamingResponse = RhaiStreamingResponse;
    type Client = T;

    fn make(client: &Self::Client, model: impl Into<String>) -> Self {
        // Created by the provider manager, not make()
        todo!()
    }

    async fn completion(&self, request: CompletionRequest)
        -> Result<CompletionResponse<Self::Response>, CompletionError>
    {
        // Non-streaming — calls Rhai parse_response()
        todo!()
    }

    async fn stream(&self, request: CompletionRequest)
        -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError>
    {
        // 1. Call Rhai build_request(config, messages, tools) → request spec
        // 2. Build HTTP request from the spec
        // 3. Create GenericEventSource
        // 4. Create RhaiStreamProcessor
        // 5. Iterate SSE events through processor
        // 6. Yield RawStreamingChoice items

        let req = self.build_http_request(&request)?;
        let event_source = GenericEventSource::new(self.client.clone(), req);

        let engine = self.engine.clone();
        let ast = self.ast.clone();
        let config = self.config.clone();

        let stream = stream! {
            let mut processor = RhaiStreamProcessor::new(engine, ast, config);
            let mut sse_stream = Box::pin(event_source);

            while let Some(sse_result) = sse_stream.next().await {
                match sse_result {
                    Ok(Event::Open) => {}
                    Ok(Event::Message(sse)) => {
                        match processor.process_event(&sse.event, &sse.data) {
                            Ok(chunks) => {
                                for chunk in chunks {
                                    yield Ok(chunk);
                                }
                            }
                            Err(e) => {
                                yield Err(CompletionError::ResponseError(format!("{e}")));
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(CompletionError::ProviderError(format!("{e}")));
                        break;
                    }
                }
            }
            sse_stream.close();
            yield Ok(RawStreamingChoice::FinalResponse(
                processor.build_final_response()
            ));
        };

        Ok(StreamingCompletionResponse::stream(Box::pin(stream)))
    }
}
```

### 6.3 Connection to Provider Manager

The provider manager (in `codelet/providers/src/manager.rs`) creates the `RigAgent<M>` which is passed to `stream_loop`. For custom Rhai providers, it instantiates `RhaiCompletionModel<T>` with the pre-compiled engine and AST:

```
ProviderManager::create_agent()
    │
    ├── For "anthropic": creates RigAgent<anthropic::CompletionModel<T>>
    ├── For "openai":    creates RigAgent<openai::CompletionModel<T>>
    └── For "custom":    creates RigAgent<RhaiCompletionModel<T>>  ← NEW
            │
            │ All three produce the same stream type:
            │ impl Stream<Item = Result<MultiTurnStreamItem<_>, _>>
            │
            └──→ stream_loop processes identically
```

### 6.4 End-to-End Data Flow for a Custom Provider

```
1. User types prompt
2. stream_loop calls agent.prompt_streaming_with_history_and_hook(prompt, history, hook)
3. Agent calls RhaiCompletionModel::stream(request)
4. Rhai build_request() constructs HTTP request body
5. GenericEventSource sends HTTP request, receives SSE stream
6. For each SSE Event::Message:
   a. RhaiStreamProcessor.process_event() calls Rhai parse_stream_chunk()
   b. Rhai returns Map with {type: "text", text: "Hello"}
   c. Rust converts to RawStreamingChoice::Message("Hello")
   d. StreamingCompletionResponse converts to StreamedAssistantContent::Text
   e. Agent wraps in MultiTurnStreamItem::StreamAssistantItem
   f. stream_loop's select! receives it
   g. handle_text_chunk() emits to UI
7. For tool calls:
   a. Rhai returns {type: "tool_call_delta", index: 0, name: "Read"}
   b. Rust accumulator stores name for index 0
   c. Rhai returns {type: "tool_call_delta", index: 0, arguments: "{..."}
   d. Rust accumulator concatenates arguments
   e. Rhai returns {type: "tool_call_delta", index: 0, complete: true}
   f. Rust emits RawStreamingChoice::ToolCall(...)
   g. Agent executes tool, yields ToolResult
   h. Agent re-prompts model (multi-turn)
8. On stream end:
   a. RhaiStreamProcessor emits FinalResponse with accumulated usage
   b. stream_loop captures stop_reason, updates token tracker
```

### 6.5 Error Recovery Compatibility

The `stream_loop` error recovery handlers (compaction, truncation, network retry, stall timeout) work identically because they operate on the stream error types, not provider internals:

- **Compaction**: Triggered by `TokenState.compaction_needed` flag, provider-agnostic
- **Truncation**: Detected via `FinalResponse.stop_reason == "max_tokens"`, works if Rhai script returns `{type:"stop", reason:"max_tokens"}`
- **Network retry**: Triggered by SSE transport errors from `GenericEventSource`, provider-agnostic
- **Stall timeout**: tokio::select! timeout, provider-agnostic

---

## Summary

| Aspect | Design Decision | Rationale |
|---|---|---|
| **Where Rhai runs** | Layer 2 (provider SSE parsing) | Only SSE→chunk mapping is provider-specific |
| **Call granularity** | Per SSE event | Natural boundary; ~1μs overhead is negligible |
| **Tool call accumulation** | Rust `HashMap<usize, Accumulator>` | Performance; consistency with existing providers |
| **Rhai return type** | `Map` with `type` field | Simple, extensible; no Rhai struct definitions needed |
| **Stream integration** | Implements `CompletionModel::stream()` | Plugs into existing Layers 3–5 unchanged |
| **Performance impact** | <0.3ms per completion | 100–200 events × ~1.5μs = negligible vs. 5–30s wall time |
| **stream_loop changes** | None required | Already handles all `MultiTurnStreamItem` variants |
