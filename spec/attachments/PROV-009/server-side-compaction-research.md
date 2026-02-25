# Server-Side Compaction Research

## Problem Statement

With Claude Opus 4.6, Anthropic introduced server-side compaction as a beta feature. When testing Opus 4.6 in codelet, we see:

```
2026-02-25T08:22:34.868Z [error]: Stream error: Compaction failed: Cannot compact empty turn history
2026-02-25T08:22:34.885Z [warn]: [signal_compaction_needed] CALLED - setting compaction_needed=true
```

This happens on the **first message** with an empty context, which is impossible for our client-side compaction to trigger (the CompactionHook explicitly says "compaction NOT triggered" on line 17).

### Log Analysis

```
Line 14: DIAG pre-prompt check: has_turns=false, will_compact=false
Line 15: [CompactionHook] on_completion_call ENTERED - history_len=2
Line 17: [CompactionHook] compaction NOT triggered: 2880 tokens <= 191808 threshold
Line 18: Stream error: Compaction failed: Cannot compact empty turn history
Line 19: [signal_compaction_needed] CALLED - setting compaction_needed=true
```

The error on line 18 occurs BEFORE line 19's signal is set, meaning something else is triggering compaction - likely related to server-side compaction signals from the API.

---

## Anthropic Server-Side Compaction API

**Documentation**: https://platform.claude.com/docs/en/build-with-claude/compaction

### Key Points

1. **Beta Feature** - Requires header: `anthropic-beta: compact-2026-01-12`
2. **Must be explicitly enabled** via `context_management.edits` parameter
3. **Supported models**: `claude-opus-4-6`, `claude-sonnet-4-6`
4. **New content block type**: `compaction`
5. **New stop_reason**: `"compaction"`
6. **New delta type**: `compaction_delta`

### API Request Structure

```json
{
  "model": "claude-opus-4-6",
  "max_tokens": 4096,
  "messages": [...],
  "context_management": {
    "edits": [
      {
        "type": "compact_20260112",
        "trigger": { "type": "input_tokens", "value": 150000 },
        "pause_after_compaction": false,
        "instructions": null
      }
    ]
  }
}
```

### API Response Structure (when compaction triggers)

```json
{
  "content": [
    {
      "type": "compaction",
      "content": "Summary of the conversation: The user requested help building..."
    },
    {
      "type": "text",
      "text": "Based on our conversation so far..."
    }
  ],
  "stop_reason": "compaction"
}
```

---

## vtcode Implementation Analysis

vtcode has fully implemented server-side compaction support. Here's how:

### 1. Type Definitions (`vtcode-core/src/llm/providers/anthropic_types.rs`)

```rust
// Content block types
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum AnthropicContentBlock {
    // ... other variants ...
    
    #[serde(rename = "compaction")]
    Compaction {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
}

// Stream delta types
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicStreamDelta {
    // ... other variants ...
    
    #[serde(rename = "compaction_delta")]
    CompactionDelta { content: String },
}

// Request structure
#[derive(Debug, Serialize, Deserialize)]
pub struct AnthropicRequest {
    // ... other fields ...
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_management: Option<Value>,
}
```

### 2. Beta Header Logic (`vtcode-core/src/llm/providers/anthropic/provider.rs`)

**CRITICAL**: vtcode only adds the compaction beta header when `context_management` is explicitly set:

```rust
fn effective_betas(&self, request: &LLMRequest) -> Option<Vec<String>> {
    let mut betas = request.betas.clone().unwrap_or_default();
    
    // Only add compaction beta when context_management is explicitly requested
    if request.context_management.is_some()
        && !betas.iter().any(|beta| beta == "compact-2026-01-12")
    {
        betas.push("compact-2026-01-12".to_string());
    }

    if betas.is_empty() { None } else { Some(betas) }
}
```

### 3. Stop Reason Parsing (`vtcode-core/src/llm/providers/anthropic/response_parser.rs`)

```rust
pub fn parse_finish_reason(stop_reason: &str) -> FinishReason {
    match stop_reason {
        "end_turn" => FinishReason::Stop,
        "max_tokens" => FinishReason::Length,
        "stop_sequence" => FinishReason::Stop,
        "tool_use" => FinishReason::ToolCalls,
        "compaction" => FinishReason::Pause,  // <-- Maps to Pause
        "pause_turn" => FinishReason::Pause,
        "refusal" => FinishReason::Refusal,
        other => FinishReason::Error(other.to_string()),
    }
}
```

### 4. Client-Side Compaction Module (`vtcode-core/src/compaction/mod.rs`)

vtcode has a separate client-side compaction module that can be used as fallback:

```rust
pub async fn compact_history(
    provider_client: &dyn LLMClient,
    model: &str,
    history: &mut Vec<Message>,
    config: &CompactionConfig,
) -> Result<()> {
    // ... client-side compaction logic ...
}
```

---

## Current codelet Implementation

### What We Have

1. **Content block type** (`completion.rs:240`):
   ```rust
   Compaction {
       content: String,
   },
   ```

2. **Delta type** (`streaming.rs:67`):
   ```rust
   CompactionDelta { content: String },
   ```

3. **Streaming handler** (`streaming.rs:451-459`):
   ```rust
   ContentDelta::CompactionDelta { content } => {
       tracing::warn!(
           "[anthropic/streaming] Server-side COMPACTION delta received - content_len={}",
           content.len()
       );
       // For now, just log - we may need to emit this as a special event
       None
   }
   ```

4. **Content block start handler** (`streaming.rs:490-498`):
   ```rust
   Content::Compaction { content } => {
       tracing::warn!(
           "[anthropic/streaming] Server-side COMPACTION block received - content_len={}",
           content.len()
       );
       // For now, just log - we may need to handle this specially
       None
   }
   ```

### What's Missing

1. **No `context_management` parameter in request** - We don't have a way to enable server-side compaction
2. **No beta header logic** - We're not conditionally adding `compact-2026-01-12` 
3. **No stop_reason handling** - We don't handle `"compaction"` stop reason
4. **No compaction block passthrough** - We log but don't emit compaction content to the client
5. **No pause handling** - When `stop_reason: "compaction"`, client needs to pass compaction block back

---

## Root Cause Hypothesis

Looking at the log more carefully:

```
Line 39: ContentBlockStart index=0 type=Discriminant(5)
```

`Discriminant(5)` in our `Content` enum is:
- 0: Text
- 1: Image
- 2: ToolUse
- 3: ToolResult
- 4: Document
- 5: **Thinking**
- 6: Compaction
- 7: Unknown

So Opus 4.6 is sending a **Thinking** block first (which is expected with adaptive thinking). The issue is likely:

1. Opus 4.6 with adaptive thinking sends a thinking block first
2. Something in how we handle this or the subsequent content is triggering our client-side compaction code path incorrectly
3. The "Cannot compact empty turn history" error suggests we're trying to run client-side compaction when we shouldn't

**Alternative hypothesis**: Opus 4.6 may be sending server-side compaction signals even without the beta header enabled (perhaps a default behavior change), and we're misinterpreting those signals.

---

## Implementation Plan

### Phase 1: Diagnostic (Current Issue)

1. Add more detailed logging to understand exactly what's being received
2. Check if Opus 4.6 sends any compaction-related signals without beta header
3. Verify the thinking block handling isn't triggering false positives

### Phase 2: Server-Side Compaction Support

1. **Request Layer** (`LLMRequest` / `CompletionRequest`):
   - Add `context_management: Option<Value>` field
   - Add helper methods to configure compaction settings

2. **Provider Layer** (`anthropic/provider.rs` or `claude.rs`):
   - Implement `effective_betas()` logic to conditionally add header
   - Pass `context_management` through to request builder

3. **Response Layer** (`streaming.rs`):
   - Emit `Compaction` content blocks as a new stream item type
   - Handle `"compaction"` stop_reason properly
   - Don't trigger client-side compaction when receiving server-side compaction

4. **Agent Layer** (`stream_loop.rs`):
   - Detect `stop_reason: "compaction"` 
   - Pass compaction block back to API on subsequent requests
   - Option to pause after compaction for user intervention

5. **Session Layer**:
   - Store compaction blocks in message history
   - Ensure compaction blocks are sent back correctly

### Phase 3: Configuration

1. Add config option to enable/disable server-side compaction
2. Add config for compaction trigger threshold
3. Add config for pause_after_compaction behavior
4. Add config for custom summarization instructions

---

## Files to Modify

### codelet/patches/rig-core/
- `src/completion/request.rs` - Add `context_management` field
- `src/providers/anthropic/completion.rs` - Handle compaction in Content enum
- `src/providers/anthropic/streaming.rs` - Emit compaction events, handle stop_reason
- `src/agent/prompt_request/streaming.rs` - Handle compaction stop reason

### codelet/
- `providers/src/claude.rs` - Add beta header logic
- `core/src/compaction_hook.rs` - Don't trigger client-side when server-side active
- `cli/src/interactive/stream_loop.rs` - Handle compaction flow
- `napi/src/session_manager.rs` - Pass compaction blocks back

---

## Testing Strategy

1. **Unit tests** for compaction block parsing
2. **Integration test** with mock server returning compaction blocks
3. **E2E test** with real Opus 4.6 API (requires API key)
4. **Regression test** to ensure client-side compaction still works for non-Opus models

---

## References

- Anthropic Compaction Docs: https://platform.claude.com/docs/en/build-with-claude/compaction
- vtcode anthropic_types.rs: `/tmp/vtcode/vtcode-core/src/llm/providers/anthropic_types.rs`
- vtcode provider.rs: `/tmp/vtcode/vtcode-core/src/llm/providers/anthropic/provider.rs`
- vtcode response_parser.rs: `/tmp/vtcode/vtcode-core/src/llm/providers/anthropic/response_parser.rs`
