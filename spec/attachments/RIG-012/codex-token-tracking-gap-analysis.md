# Codex Token Tracking Gap Analysis

**Date**: 2026-03-04  
**Related**: RIG-011 (rig-core level fixes), upstream Codex CLI at `/tmp/codex`  
**Scope**: End-to-end reasoning token pipeline from rig-core → NAPI → TypeScript TUI

---

## Executive Summary

RIG-011 correctly fixed the **bottom layers** of the token tracking pipeline:
- ✅ `rig::completion::Usage` struct now has `reasoning_tokens: Option<u64>`
- ✅ OpenAI Responses/Completions API propagation
- ✅ `ApiTokenUsage` in codelet-core
- ✅ Debug capture events include `reasoningTokens`
- ✅ Debug metadata uses `current_model_id()`

However, reasoning tokens are **still invisible in the TUI** because the **middle and upper layers** of the pipeline were never updated. The data correctly reaches `stream_loop.rs` debug events but is **dropped** during the conversion chain:

```
ApiTokenUsage (has reasoning_tokens ✅)
    ↓ creates TokenDisplayUpdate (MISSING reasoning_tokens ❌)
        ↓ converts to TokenInfo (MISSING reasoning_tokens ❌)
            ↓ emits StreamEvent::Tokens
                ↓ converts to NAPI TokenTracker (MISSING reasoning_tokens ❌)
                    ↓ exposed to TypeScript TUI (MISSING reasoningTokens ❌)
                        ↓ SessionHeader display (never shows reasoning ❌)
```

Additionally, our codebase is missing the **server_reasoning_included** logic that Codex uses to prevent double-counting reasoning tokens in context window calculations.

---

## How Upstream Codex CLI Does It Right

### Data Flow (Working)

```
SSE response
    ↓ ResponseCompletedUsage (has output_tokens_details.reasoning_tokens)
    ↓ From<ResponseCompletedUsage> for TokenUsage (maps reasoning_output_tokens)
    ↓ codex_protocol::protocol::TokenUsage (has reasoning_output_tokens: i64)
    ↓ TokenUsageInfo (aggregates via add_assign, preserves reasoning)
    ↓ EventMsg::TokenCount (includes full TokenUsageInfo)
    ↓ TUI chatwidget.rs reads TokenUsageInfo
    ↓ status/card.rs displays reasoning in /status output
    ↓ FinalOutput::fmt() shows "output=N (reasoning M)"
```

### Key Codex Structs

**`codex_protocol::protocol::TokenUsage`** (protocol/src/protocol.rs:1527):
```rust
pub struct TokenUsage {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_output_tokens: i64,  // ← First-class field
    pub total_tokens: i64,
}
```

**`add_assign`** sums `reasoning_output_tokens` (protocol.rs:1695).

**`FinalOutput::fmt()`** displays reasoning conditionally (protocol.rs:1729):
```rust
if token_usage.reasoning_output_tokens > 0 {
    format!(" (reasoning {})", format_with_separators(token_usage.reasoning_output_tokens))
}
```

### SSE Parsing (codex-api/src/sse/responses.rs:130-145)
```rust
impl From<ResponseCompletedUsage> for TokenUsage {
    fn from(val: ResponseCompletedUsage) -> Self {
        TokenUsage {
            input_tokens: val.input_tokens,
            cached_input_tokens: val.input_tokens_details
                .map(|d| d.cached_tokens).unwrap_or(0),
            output_tokens: val.output_tokens,
            reasoning_output_tokens: val.output_tokens_details
                .map(|d| d.reasoning_tokens).unwrap_or(0),  // ← Extracted here
            total_tokens: val.total_tokens,
        }
    }
}
```

### Server Reasoning Included (codex-api/src/common.rs + core/src/codex.rs)

Codex has a `ServerReasoningIncluded(bool)` event from the API. When the server tells the client that reasoning tokens are already included in `total_tokens`, the context manager adjusts its `get_total_token_usage()` calculation:

```rust
// core/src/context_manager/history.rs:280
pub(crate) fn get_total_token_usage(&self, server_reasoning_included: bool) -> i64 {
    let last_tokens = self.token_info
        .as_ref().map(|info| info.last_token_usage.total_tokens).unwrap_or(0);
    let items_after = ...;
    if server_reasoning_included {
        // Server already counted reasoning — don't re-estimate
        last_tokens.saturating_add(items_after)
    } else {
        // Client must add reasoning estimates
        last_tokens
            .saturating_add(self.get_non_last_reasoning_items_tokens())
            .saturating_add(items_after)
    }
}
```

We have NO equivalent logic — our `total_context()` always adds `reasoning_tokens`, which could lead to double-counting.

### Reasoning Item Token Estimation

Codex also estimates token cost of reasoning items in history (encrypted reasoning content):
```rust
// core/src/context_manager/history.rs:414
fn estimate_reasoning_length(encoded_len: usize) -> usize {
    encoded_len.saturating_mul(3).checked_div(4).unwrap_or(0).saturating_sub(650)
}
```

This is used for tracking non-last reasoning items' token consumption. We have no equivalent.

### OTEL Integration

Codex logs reasoning tokens to OpenTelemetry:
```rust
// otel/src/traces/otel_manager.rs:502
pub fn sse_event_completed(
    &self,
    input_token_count: i64,
    output_token_count: i64,
    cached_input_token_count: Option<i64>,
    reasoning_token_count: Option<i64>,  // ← tracked in telemetry
    total_token_count: i64,
)
```

---

## Specific Gaps in Our Code

### Gap 1: `TokenDisplayUpdate` missing `reasoning_tokens`

**File**: `codelet/core/src/streaming_display/streaming_token_display.rs`

```rust
// CURRENT (line 11)
pub struct TokenDisplayUpdate {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub tokens_per_second: Option<f64>,
    // ❌ MISSING: pub reasoning_tokens: u64,
}
```

The `current_display()` method at line 233 constructs this struct but never includes reasoning tokens. The `total_context()` method at line 48 returns `total_input() + output_tokens` — missing reasoning.

### Gap 2: `TokenInfo` missing `reasoning_tokens`

**File**: `codelet/cli/src/interactive/output.rs`

```rust
// CURRENT (line 20)
pub struct TokenInfo {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    pub tokens_per_second: Option<f64>,
    // ❌ MISSING: pub reasoning_tokens: Option<u64>,
}
```

The `From<TokenDisplayUpdate>` impl at line 45 doesn't map reasoning tokens. The `from_usage()` factory at line 34 doesn't either.

### Gap 3: NAPI `TokenTracker` missing `reasoning_tokens`

**File**: `codelet/napi/src/types.rs`

```rust
// CURRENT (line 123)
#[napi(object)]
pub struct TokenTracker {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_input_tokens: Option<u32>,
    pub cache_creation_input_tokens: Option<u32>,
    pub tokens_per_second: Option<f64>,
    pub cumulative_billed_input: Option<u32>,
    pub cumulative_billed_output: Option<u32>,
    // ❌ MISSING: pub reasoning_tokens: Option<u32>,
}
```

The NAPI index.d.ts TypeScript type definition also lacks `reasoningTokens`.

### Gap 4: StreamEvent conversion drops reasoning tokens

**File**: `codelet/napi/src/session_manager.rs` (lines 5778-5789)

```rust
StreamEvent::Tokens(info) => {
    self.session.update_tokens(info.input_tokens as u32, info.output_tokens as u32);
    StreamChunk::token_update(TokenTracker {
        input_tokens: info.input_tokens as u32,
        output_tokens: info.output_tokens as u32,
        cache_read_input_tokens: info.cache_read_input_tokens.map(|v| v as u32),
        cache_creation_input_tokens: info.cache_creation_input_tokens.map(|v| v as u32),
        tokens_per_second: info.tokens_per_second,
        cumulative_billed_input: None,
        cumulative_billed_output: None,
        // ❌ MISSING: reasoning_tokens
    })
}
```

### Gap 5: TypeScript `TokenTracker` interface missing `reasoningTokens`

**File**: `src/tui/utils/sessionHeaderUtils.ts`

```typescript
// CURRENT (line 39)
export interface TokenTracker {
  inputTokens: number;
  outputTokens: number;
  cacheReadInputTokens?: number;
  cacheCreationInputTokens?: number;
  // ❌ MISSING: reasoningTokens?: number;
}
```

### Gap 6: SessionHeader doesn't display reasoning tokens

**File**: `src/tui/components/SessionHeader.tsx` (line 210)

```tsx
<Text dimColor>tokens: {inputTokens}↓ {outputTokens}↑  </Text>
```

No reasoning token display. Compare with Codex CLI which shows reasoning separately.

### Gap 7: Compaction `TokenTracker` missing reasoning tokens

**File**: `codelet/core/src/compaction/model.rs` (line 57)

```rust
pub struct TokenTracker {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cumulative_billed_input: u64,
    pub cumulative_billed_output: u64,
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    // ❌ MISSING: pub reasoning_tokens: u64,
}
```

`total_tokens()` returns `input + output` only. `effective_tokens()` doesn't account for reasoning.

### Gap 8: No `server_reasoning_included` logic

Our code naively adds reasoning tokens in `total_context()`:
```rust
// codelet/core/src/token_usage.rs:70
pub fn total_context(&self) -> u64 {
    self.total_input() + self.output_tokens + self.reasoning_tokens
}
```

If the server's `total_tokens` already includes reasoning (as Codex's `ServerReasoningIncluded` event indicates), we're **double-counting** reasoning tokens. This inflates context fill percentage and may trigger premature compaction.

### Gap 9: Context fill calculation ignores reasoning

**File**: `src/tui/utils/tokenStateUtils.ts` (line 103)

```typescript
export function calculateContextFillPercentage(
  inputTokens: number,
  contextWindow: number,
  maxOutput: number
): number {
  const threshold = contextWindow - Math.min(maxOutput, MAX_OUTPUT_RESERVATION);
  return Math.round((inputTokens / threshold) * 100);
  // ❌ inputTokens doesn't include reasoning - context fill is under-reported
}
```

### Gap 10: Token persistence doesn't include reasoning

**File**: `src/tui/utils/tokenStateUtils.ts` (line 127)

```typescript
export function persistTokenState(sessionId: string | null): void {
  const tokens = sessionGetTokens(sessionId);
  persistenceSetSessionTokens(
    sessionId,
    tokens.inputTokens,
    tokens.outputTokens,
    0, // cacheRead
    0, // cacheCreate
    tokens.inputTokens,  // cumulativeInput
    tokens.outputTokens  // cumulativeOutput
    // ❌ No reasoning tokens persisted
  );
}
```

On session restore, reasoning token count is lost.

---

## Summary of Required Changes

### Rust Layer

| File | Change |
|------|--------|
| `codelet/core/src/streaming_display/streaming_token_display.rs` | Add `reasoning_tokens: u64` to `TokenDisplayUpdate`, update `current_display()`, `update_from_usage()`, `update_from_final_response()`, `total_context()` |
| `codelet/cli/src/interactive/output.rs` | Add `reasoning_tokens: Option<u64>` to `TokenInfo`, update `From<TokenDisplayUpdate>` and `from_usage()` |
| `codelet/napi/src/types.rs` | Add `reasoning_tokens: Option<u32>` to NAPI `TokenTracker` |
| `codelet/napi/src/session_manager.rs` | Map reasoning_tokens in `StreamEvent::Tokens` → `TokenTracker` conversion |
| `codelet/core/src/compaction/model.rs` | Add `reasoning_tokens: u64` to compaction `TokenTracker`, update `total_tokens()` |

### TypeScript Layer

| File | Change |
|------|--------|
| `src/tui/utils/sessionHeaderUtils.ts` | Add `reasoningTokens?: number` to `TokenTracker` interface |
| `src/tui/components/SessionHeader.tsx` | Display reasoning tokens (e.g., `{reasoningTokens}🧠`) |
| `src/tui/utils/tokenStateUtils.ts` | Include reasoning in `calculateContextFillPercentage()` and `persistTokenState()` |

### Future Considerations

| Area | Description |
|------|-------------|
| `server_reasoning_included` | Implement API event detection to prevent double-counting reasoning in context window calculations (like Codex's `ServerReasoningIncluded` event) |
| Reasoning item estimation | Track encrypted reasoning items in history and estimate their token cost for context tracking (like Codex's `estimate_reasoning_length()`) |
| Session summary | Show reasoning breakdown on session end (like Codex's `FinalOutput::fmt()`: `"output=N (reasoning M)"`) |

---

## Verification Method

After fix, a Codex session with `T:High` should:
1. Show non-zero `reasoningTokens` in `TokenTracker` NAPI events
2. Display reasoning tokens in SessionHeader (TUI)
3. Include reasoning in context fill percentage
4. Persist reasoning tokens across session restore
5. Not double-count reasoning if server already includes them in `total_tokens`
