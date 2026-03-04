# PROV-038: Codex Token Tracking Investigation

## Summary

The Codex provider's token tracking shows `tokens: 0↓ 130↑ [0%]` — input tokens permanently stuck at 0, context fill percentage always 0%. Output tokens work (estimated from chunk lengths), but input tokens are never populated with authoritative data from the API.

## Root Cause

**The rig-core OpenAI Responses API streaming implementation never yields `RawStreamingChoice::Usage` events.** This is a gap compared to both the Anthropic and OpenAI Chat Completions API providers.

### Provider Comparison

| Provider | Streaming File | Emits `RawStreamingChoice::Usage`? |
|---|---|---|
| **Anthropic** | `anthropic/streaming.rs` | ✅ Yes — on `message_start` (line 336) and `message_delta` (line 344) |
| **OpenAI Chat Completions** | `openai/completion/streaming.rs` | ✅ Yes — when chunk contains `usage` field (line 218). Also requests usage via `stream_options: { include_usage: true }` (line 113) |
| **OpenAI Responses API** (Codex) | `openai/responses_api/streaming.rs` | ❌ **NEVER** — no `RawStreamingChoice::Usage` yield anywhere in the file |

### Impact Chain

1. `responses_api/streaming.rs` never yields `RawStreamingChoice::Usage(...)` 
2. → `rig-core/src/streaming.rs` never converts it to `StreamedAssistantContent::Usage(...)` 
3. → `agent/prompt_request/streaming.rs` never yields `MultiTurnStreamItem::Usage(...)` 
4. → `stream_loop.rs` line 633 (`MultiTurnStreamItem::Usage`) never triggers for Codex
5. → `streaming_display` never receives authoritative input/output token data
6. → Display stays at 0 input tokens

### FinalResponse Path Also Fails

The fallback path in `stream_loop.rs` (lines 675-686) for OpenAI-compatible providers:

```rust
let final_update = if !streaming_display.has_authoritative_output() && usage.input_tokens > 0 {
    streaming_display.update_from_final_response(&usage)  // Only if input_tokens > 0
} else {
    streaming_display.current()  // Falls through here because input_tokens == 0
};
```

The `response.completed` event from the Codex backend API (`chatgpt.com/backend-api/codex/responses`) **does include usage data** — the official Codex CLI (`/tmp/codex`) parses it successfully (see Reference section). However, in rig-core's implementation, the usage is captured into `final_usage` but only emitted as a `FinalResponse`, not as a separate `Usage` event. Since `FinalResponse.token_usage()` returns the usage from `StreamingCompletionResponse.usage`, the data IS available at the rig level but the `stream_loop.rs` condition `usage.input_tokens > 0` gates whether it gets applied.

## Debug Log Evidence

From `~/.fspec/debug/session-2026-03-04T06-36-08.jsonl`:

```json
// Sequence 1: inputTokens already 0 at start
{"eventType":"compaction.check","data":{"inputTokens":0,"contextWindow":272000}}

// Sequence 2: Provider correctly identified
{"eventType":"api.request","data":{"model":"gpt-5.3-codex","provider":"codex"}}

// Sequences 4-52: Only text chunks — NO Usage events
{"eventType":"api.response.chunk","data":{"chunkLength":3}}
// ... (50 more chunk events)

// Session ended by user — no FinalResponse ever arrived
{"eventType":"session.end"}
```

Key observation: There are **zero** `token.update` or `api.response.end` events, confirming no token data was ever received.

## Reference: Official Codex CLI (`/tmp/codex`)

The official Codex CLI properly handles token usage from `response.completed`:

**File: `/tmp/codex/codex-rs/codex-api/src/sse/responses.rs`** (lines 113-145):

```rust
#[derive(Debug, Deserialize)]
struct ResponseCompleted {
    id: String,
    #[serde(default)]
    usage: Option<ResponseCompletedUsage>,
}

#[derive(Debug, Deserialize)]
struct ResponseCompletedUsage {
    input_tokens: i64,
    input_tokens_details: Option<ResponseCompletedInputTokensDetails>,
    output_tokens: i64,
    output_tokens_details: Option<ResponseCompletedOutputTokensDetails>,
    total_tokens: i64,
}

impl From<ResponseCompletedUsage> for TokenUsage {
    fn from(val: ResponseCompletedUsage) -> Self {
        TokenUsage {
            input_tokens: val.input_tokens,
            cached_input_tokens: val.input_tokens_details
                .map(|d| d.cached_tokens).unwrap_or(0),
            output_tokens: val.output_tokens,
            reasoning_output_tokens: val.output_tokens_details
                .map(|d| d.reasoning_tokens).unwrap_or(0),
            total_tokens: val.total_tokens,
        }
    }
}
```

**File: `/tmp/codex/codex-rs/core/src/client.rs`** (lines 1127-1149):

```rust
Ok(ResponseEvent::Completed { response_id, token_usage }) => {
    if let Some(usage) = &token_usage {
        otel_manager.sse_event_completed(
            usage.input_tokens,
            usage.output_tokens,
            Some(usage.cached_input_tokens),
            Some(usage.reasoning_output_tokens),
            usage.total_tokens,
        );
    }
    // ... forwards token_usage to event consumers
}
```

This confirms:
1. The Codex backend API **does** return `usage` in `response.completed`
2. The official CLI extracts `input_tokens`, `output_tokens`, `cached_input_tokens`, `reasoning_output_tokens`, and `total_tokens`
3. It uses the same structure as the standard OpenAI Responses API

## OpenAI API Documentation Reference

From the [OpenAI Streaming Responses guide](https://developers.openai.com/api/docs/guides/streaming-responses):

> The Responses API uses semantic events for streaming. Each event is typed with a predefined schema.

Key lifecycle events:
- `response.created` — Response created
- `response.output_text.delta` — Text chunk
- **`response.completed`** — Final event with full response including **`usage`** field

The `response.completed` event includes a full `Response` object with a `usage` field containing:
- `input_tokens` — Total input tokens
- `input_tokens_details.cached_tokens` — Cached input tokens
- `output_tokens` — Total output tokens  
- `output_tokens_details.reasoning_tokens` — Reasoning output tokens
- `total_tokens` — Sum of input + output

## Fix Required

In `codelet/patches/rig-core/src/providers/openai/responses_api/streaming.rs`, when `response.completed` is received and `final_usage` is populated, **yield a `RawStreamingChoice::Usage` event** before yielding `FinalResponse`. This matches what the Chat Completions API streaming already does.

### Location

**File:** `codelet/patches/rig-core/src/providers/openai/responses_api/streaming.rs`

**Lines 349-358** (current code that captures usage but doesn't emit it):

```rust
if let StreamingCompletionChunk::Response(chunk) = data {
    if let ResponseChunk { kind: ResponseChunkKind::ResponseCompleted, response, .. } = *chunk {
        span.record("gen_ai.response.id", response.id);
        span.record("gen_ai.response.model", response.model);
        if let Some(usage) = response.usage {
            final_usage = usage;  // ← Captured but never emitted as Usage event
        }
    }
}
```

### Fix

After capturing `final_usage`, yield a `RawStreamingChoice::Usage` event:

```rust
if let Some(usage) = response.usage {
    final_usage = usage.clone();
    // Emit Usage event so stream_loop gets real-time token data
    let crate_usage = crate::completion::Usage {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        total_tokens: usage.total_tokens,
        cache_read_input_tokens: usage.input_tokens_details
            .as_ref().map(|d| d.cached_tokens),
        reasoning_tokens: Some(usage.output_tokens_details.reasoning_tokens),
        ..Default::default()
    };
    yield Ok(RawStreamingChoice::Usage(crate_usage));
}
```

This ensures:
1. `stream_loop.rs` receives `MultiTurnStreamItem::Usage(...)` with actual token counts
2. `streaming_display` gets authoritative input/output values
3. The TUI displays real token counts and context fill percentage
4. The compaction hook (`CompactionHook`) receives correct values for threshold checks
