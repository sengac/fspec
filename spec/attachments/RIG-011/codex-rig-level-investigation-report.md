# Investigation Report: Codex rig-core Integration Issues

**Date**: 2026-03-04
**Context**: GPT-5.3 Codex session with `[DEBUG]` mode enabled
**Debug Session**: `/Users/rquast/.fspec/debug/session-2026-03-04T00-27-28.jsonl`
**Screenshot**: `/Users/rquast/Desktop/codex-bad.png`

---

## Executive Summary

Investigation of the debug log from a GPT-5.3 Codex session reveals **multiple critical issues** at the rig-core integration layer in codelet. The model was asked to investigate its own rig-core integration but produced only a 135-token text response with zero tool calls — effectively promising to investigate but never actually doing so. The debug capture itself also fails to properly record key metadata.

---

## Issue 1: Model Name Incorrectly Set to Provider Name (CRITICAL)

### Location
`codelet/cli/src/interactive/repl_loop.rs` line 46

### Evidence
```jsonl
// session.start event:
"model": "unknown", "provider": "unknown"

// api.request event:
"model": "codex", "provider": "codex"
```

### Root Cause
```rust
manager.set_session_metadata(SessionMetadata {
    provider: Some(session.current_provider_name().to_string()),
    model: Some(session.current_provider_name().to_string()),  // BUG: uses provider name, not model name
    context_window: Some(session.provider_manager().context_window()),
    max_output_tokens: None,
});
```

The `model` field is set to `session.current_provider_name()` which returns `"codex"` (the provider name), NOT the actual model ID (e.g., `"gpt-5.3-codex"` or `"codex"`). At session start time, the metadata hasn't been set yet so it falls back to `"unknown"`.

### Impact
- Debug logs show wrong model identity
- Cannot distinguish between GPT-5.1-codex, GPT-5.2-codex, GPT-5.3-codex sessions
- Analytics and metrics are unreliable

### Fix Required
Use `session.current_model_id()` instead of `session.current_provider_name()` for the model field.

---

## Issue 2: Context Window Mismatch Between Session Start and Runtime (MEDIUM)

### Evidence
```jsonl
// session.start: contextWindow = 200000
// compaction.check: contextWindow = 272000
```

### Root Cause
`codelet/common/src/debug_capture/session_lifecycle.rs` line 10:
```rust
const DEFAULT_CONTEXT_WINDOW: usize = 200000;
```

The session metadata defaults to 200,000 because `set_session_metadata` is called AFTER the debug capture is started. The compaction check correctly uses the provider's `CONTEXT_WINDOW = 272_000`.

### Additional Note
The screenshot title bar shows `[400k]` which doesn't match either value — suggesting the TUI displays a configured/overridden value that never reaches the debug capture system.

### Impact
- Misleading session metadata in debug logs
- Context window percentage calculations based on wrong denominator at session start

---

## Issue 3: Missing Thinking/Reasoning Token Tracking (CRITICAL)

### Evidence from debug log
```jsonl
"aggregatedUsage": {
    "inputTokens": 10309,
    "outputTokens": 135,
    "totalInputTokens": 10309
}
```

**Zero thinking/reasoning tokens reported** despite `[T:High]` (thinking level: High) being shown in the title bar.

### Root Cause Chain

**Layer 1 — rig-core `Usage` struct** (`patches/rig-core/src/completion/request.rs:295`):
```rust
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    // ❌ MISSING: pub reasoning_tokens: Option<u64>
}
```

**Layer 2 — OpenAI Responses API has the data** (`patches/rig-core/src/providers/openai/responses_api/mod.rs:535-539`):
```rust
pub struct OutputTokensDetails {
    pub reasoning_tokens: u64,  // ✅ Present at provider level
}
```

**Layer 3 — Conversion drops reasoning tokens** (same file, line 1136-1143):
```rust
let usage = response.usage.as_ref()
    .map(|usage| completion::Usage {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        total_tokens: usage.total_tokens,
        ..Default::default()  // ❌ reasoning_tokens DROPPED here
    })
```

**Layer 4 — Streaming also drops reasoning tokens** (`responses_api/streaming.rs:42-48`):
```rust
impl GetTokenUsage for StreamingCompletionResponse {
    fn token_usage(&self) -> Option<crate::completion::Usage> {
        let mut usage = crate::completion::Usage::new();
        usage.input_tokens = self.usage.input_tokens;
        usage.output_tokens = self.usage.output_tokens;
        usage.total_tokens = self.usage.total_tokens;
        Some(usage)
        // ❌ reasoning_tokens from self.usage.output_tokens_details.reasoning_tokens NOT propagated
    }
}
```

**Layer 5 — `ApiTokenUsage` in codelet-core** (`core/src/token_usage.rs:20`):
```rust
pub struct ApiTokenUsage {
    pub input_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub output_tokens: u64,
    // ❌ MISSING: pub reasoning_tokens: u64
}
```

**Layer 6 — Debug capture events** (`cli/src/interactive/stream_loop.rs:700-746`):
```json
"aggregatedUsage": {
    "inputTokens": ...,
    "outputTokens": ...,
    // ❌ MISSING: "reasoningTokens": ...
}
```

### Impact
- Thinking/reasoning token consumption is invisible
- Cannot measure cost of extended thinking
- Cannot verify thinking is actually being used (as seen in this session)
- Context window calculations may be wrong (reasoning tokens consume context)
- The `total_tokens` in the API response likely INCLUDES reasoning tokens, but they aren't broken out

---

## Issue 4: Codex MAX_OUTPUT_TOKENS Too Low (LOW)

### Location
`codelet/providers/src/codex/mod.rs` line 45

```rust
pub const MAX_OUTPUT_TOKENS: usize = 4096;
```

### Issue
GPT-5.3 Codex supports significantly higher output token limits. The 4096 limit may be artificially constraining model responses, especially for complex tool-use scenarios.

### Note
The code has a comment about not setting `max_tokens` because the Codex API rejects `max_output_tokens`. This constant may only be used for display/estimation purposes.

---

## Issue 5: Model Produced No Tool Calls (BEHAVIORAL)

### Evidence
The debug log shows only text content chunks (135 output tokens total). The model said "I'll inspect the rig-core integration" but never invoked any tools.

### Possible Causes
1. **System prompt not properly including tool definitions**: The Codex agent builder sets up tools via `create_rig_agent()` but the API may not be receiving them correctly
2. **Thinking tokens consuming output budget**: If thinking is active but not tracked, the actual output budget for visible text + tool calls may be severely reduced
3. **Model-specific behavior**: GPT-5.3 Codex with T:High may prefer to outline plans before acting, and the session was stopped before the model could complete

---

## Issue 6: OpenAI Completions API Streaming — Reasoning Tokens Also Lost

### Location
`patches/rig-core/src/providers/openai/completion/streaming.rs` lines 195-210

The OpenAI Chat Completions streaming path also constructs `crate::completion::Usage` without reasoning tokens:
```rust
let crate_usage = crate::completion::Usage {
    input_tokens: usage.prompt_tokens as u64,
    output_tokens: usage.output_tokens(),
    total_tokens: usage.total_tokens as u64,
    cache_read_input_tokens: if cached_tokens > 0 { Some(cached_tokens) } else { None },
    // ❌ No reasoning_tokens propagation
    ..Default::default()
};
```

---

## Comparison with Upstream Codex (OpenAI)

The upstream OpenAI Codex codebase (`/tmp/codex/codex-rs`) **properly tracks reasoning tokens**:

1. **`TokenUsage` struct** includes `reasoning_output_tokens: i64`
2. **SSE response parsing** extracts `output_tokens_details.reasoning_tokens`
3. **Display** shows reasoning tokens separately in the summary
4. **Context window calculations** account for reasoning tokens

Our patched rig-core is **missing this entire data flow**.

---

## Recommended Fix Plan

### Priority 1: Add `reasoning_tokens` to rig-core `Usage` struct
- Add `pub reasoning_tokens: Option<u64>` to `completion::Usage`
- Update `Add` and `AddAssign` impls
- Update `Default` impl

### Priority 2: Propagate reasoning tokens in all providers
- **OpenAI Responses API**: Map `output_tokens_details.reasoning_tokens` during Usage conversion
- **OpenAI Completions API**: Same for streaming and non-streaming paths
- **Anthropic**: Map thinking tokens (if reported in usage) 

### Priority 3: Update codelet-core `ApiTokenUsage`
- Add `reasoning_tokens: u64` field
- Update `update_from_usage()` to extract it
- Update `total_context()` if reasoning tokens affect context accounting

### Priority 4: Fix debug capture metadata
- Use `session.current_model_id()` for model field
- Ensure metadata is set before `capture_session_start()`
- Add `reasoningTokens` to debug capture events

### Priority 5: Add reasoning tokens to debug events
- Include in `api.response.end` aggregatedUsage and displayUsage
- Include in `token.update` events
- Include in `session.end` summary

---

## Files Requiring Changes

| File | Change |
|------|--------|
| `patches/rig-core/src/completion/request.rs` | Add `reasoning_tokens` to `Usage` |
| `patches/rig-core/src/providers/openai/responses_api/mod.rs` | Propagate reasoning tokens in conversion |
| `patches/rig-core/src/providers/openai/responses_api/streaming.rs` | Same for streaming |
| `patches/rig-core/src/providers/openai/completion/mod.rs` | Same for completions API |
| `patches/rig-core/src/providers/openai/completion/streaming.rs` | Same for completions streaming |
| `codelet/core/src/token_usage.rs` | Add `reasoning_tokens` to `ApiTokenUsage` |
| `codelet/cli/src/interactive/repl_loop.rs` | Fix model name in debug metadata |
| `codelet/cli/src/interactive/stream_loop.rs` | Add reasoning tokens to debug events |
| `codelet/common/src/debug_capture/session_lifecycle.rs` | Fix default context window |
