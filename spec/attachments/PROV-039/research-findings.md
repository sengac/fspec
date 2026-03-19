# PROV-039: Token Output Limit Handling — Research Findings

## Executive Summary

When the LLM hits `max_tokens` during output generation, our streaming path **discards the stop_reason entirely**. The agent loop, CLI, and persistence layer all treat truncated responses identically to normal `end_turn` completions. This means truncated file writes, truncated tool calls, and truncated text responses are silently accepted with no user notification, no retry, and no continuation logic.

**Research methodology**: Four parallel research agents investigated the Anthropic API docs, our fspec/codelet codebase, vtcode (Rust-based AI coding agent), and opencode (TypeScript-based AI coding agent).

---

## 1. Anthropic API Behavior (Authoritative)

### `stop_reason` Values

The Anthropic Messages API returns **6 possible `stop_reason` values**:

| `stop_reason` | Meaning |
|---|---|
| `"end_turn"` | Claude finished naturally — most common |
| `"max_tokens"` | **Hit the `max_tokens` limit — response is TRUNCATED** |
| `"stop_sequence"` | Hit a custom stop sequence |
| `"tool_use"` | Claude wants to call a tool |
| `"pause_turn"` | Server-side tool loop hit iteration limit (default 10) |
| `"refusal"` | Safety policy violation |

### What Happens When `max_tokens` Is Hit

**During text generation:**
- Text is **abruptly truncated** mid-word/mid-sentence
- `content` array contains whatever was generated so far
- `stop_reason: "max_tokens"` (NOT `"end_turn"`)

**During tool calls — THE CRITICAL CASE:**
- The `tool_use` block's `input` JSON is **incomplete/unparseable** (missing closing braces, truncated strings)
- `stop_reason` will be **`"max_tokens"`**, NOT `"tool_use"` — this is the key signal
- The tool call **cannot be executed** — you must retry with higher `max_tokens`
- Anthropic docs: *"If Claude's response is cut off due to hitting the `max_tokens` limit, and the truncated response contains an incomplete tool use block, you'll need to retry the request with a higher `max_tokens` value to get the full tool use."*

### Extended Thinking & `max_tokens`

- `max_tokens` encompasses BOTH thinking AND text output
- `budget_tokens` must be strictly less than `max_tokens` (at least 1 token for output)
- If text output exceeds the remaining budget after thinking, it truncates with `stop_reason: "max_tokens"`
- **Adaptive thinking** (recommended for 4.6 models) lets Claude decide the budget dynamically

### Streaming Behavior

The `message_delta` event carries `stop_reason`:
```json
event: message_delta
data: {
  "type": "message_delta",
  "delta": { "stop_reason": "max_tokens" },
  "usage": { "output_tokens": 256 }
}
```
- `message_stop` always follows, even on truncation
- For text: `text_delta` events simply stop; accumulated text is incomplete
- For tool use: `input_json_delta` events stop; accumulated JSON is invalid
- For thinking: `thinking_delta` events may end early

### Token Limits by Model

| Model | Context | Max Output |
|---|---|---|
| Claude Opus 4.6 | 1M | 128K |
| Claude Sonnet 4.6 | 1M | 64K |
| Claude Haiku 4.5 | 200K | 64K |
| Claude Opus 4 | 200K | 32K |
| Claude Sonnet 4 | 200K | 64K |

---

## 2. Our fspec/codelet Codebase — THE GAP

### StopReason Enum Exists

In `codelet/providers/src/lib.rs:66-74`:
```rust
pub enum StopReason {
    EndTurn,     // Natural end
    ToolUse,     // Wants tools
    MaxTokens,   // Hit limit
}
```

Provider mappings are correct:
- **Claude**: `"max_tokens"` → `MaxTokens`
- **OpenAI**: `"length"` → `MaxTokens`
- **Gemini**: `FinishReason::MaxTokens` → `MaxTokens`

### THE GAP: `stop_reason` Is Lost in Streaming

This `StopReason` enum is only used in the **non-streaming** `complete_with_tools` path. The **streaming path** — which is what the agent loop actually uses — **discards the stop reason entirely**.

#### Layer 1: Anthropic streaming handler

**File**: `codelet/patches/rig-core/src/providers/anthropic/streaming.rs:346-374`

When a `MessageDelta` with `stop_reason` arrives:
1. It's logged at debug level (line 348)
2. A special warning is logged if it's `"compaction"` (lines 355-359)
3. The SSE loop `break`s — **regardless of what the stop_reason is**
4. A `FinalResponse` is yielded with only `usage` data — **no stop_reason field**

#### Layer 2: FinalResponse struct has no stop_reason

**File**: `codelet/patches/rig-core/src/agent/prompt_request/streaming.rs:185-188`
```rust
pub struct FinalResponse {
    response: String,
    aggregated_usage: crate::completion::Usage,
}
```
**No `stop_reason` field exists.**

#### Layer 3: Agent inner loop only checks `did_call_tool`

**File**: `codelet/patches/rig-core/src/agent/prompt_request/streaming.rs:612-626`
```rust
if !did_call_tool {
    yield Ok(MultiTurnStreamItem::final_response(&last_text_response, aggregated_usage));
    break;
}
```
When `max_tokens` fires during text, `did_call_tool` is `false` → **terminates identically to `end_turn`**.

#### Layer 4: CLI stream_loop has zero visibility

**File**: `codelet/cli/src/interactive/stream_loop.rs`

`stop_reason` / `StopReason` **never appear anywhere** in `stream_loop.rs` or `stream_handlers.rs` (confirmed by grep: zero matches).

#### Layer 5: Persistence hardcodes "end_turn"

**File**: `codelet/napi/src/persistence/message_envelope.rs:120, 299, 329`

The `AssistantMessage` envelope has `pub stop_reason: Option<String>`, but when constructing messages from streaming responses, the code **always hardcodes** `stop_reason: Some("end_turn".to_string())` at lines 299, 329, 537, 568. The actual stop reason is never propagated.

### Truncated Tool Calls: Partially Safe

When `max_tokens` fires mid-tool-call:
- **Anthropic**: Tool call JSON chunks accumulated via `InputJsonDelta`. On `ContentBlockStop`, `serde_json::from_str()` fails → `CompletionError`
- **rig `ToolDyn::call`**: Truncated JSON fails deserialization → `ToolError::JsonError`
- **rig agent loop**: Error string becomes the tool "result" sent back to the model. Loop **continues** since `did_call_tool` is true.
- **Safety**: No partial file writes occur — the Write tool never executes because deserialization fails

**BUT**: The error is generic — the model doesn't know it was truncated, just that JSON parsing failed.

### max_tokens Configuration Per Provider

| Provider | Constant | Value | Configurable? |
|---|---|---|---|
| Claude | `MAX_OUTPUT_TOKENS` | 8,192 | No (hardcoded) |
| OpenAI | `DEFAULT_MAX_OUTPUT_TOKENS` | 4,096 | ✅ `OPENAI_MAX_OUTPUT_TOKENS` env var |
| Gemini | `MAX_OUTPUT_TOKENS` | 8,192 | No |

**Bug**: `ProviderManager::max_output_tokens()` uses the module constant (always 4096) for OpenAI, not the runtime env var value — causing compaction threshold miscalculation.

### No Retry/Continuation for MaxTokens

Three continuation mechanisms exist, **none triggered by MaxTokens**:
1. **Gemini continuation** — Empty response after tool results
2. **Emergency compaction** — API "prompt too long" error
3. **CompactionHook cancel** — Token estimate exceeds threshold pre-API-call

---

## 3. vtcode (Rust) — Best-in-Class Reference Implementation

vtcode provides the most comprehensive handling of token output limits among the agents studied.

### 3.1 Unified `FinishReason` Enum

**File**: `vtcode-commons/src/llm.rs`

```rust
pub enum FinishReason {
    Stop,           // Normal completion
    Length,         // ← TRUNCATION: hit max_tokens limit
    ToolCalls,      // Model wants to call tools
    ContentFilter,  // Blocked by safety filter
    Pause,          // Compaction/pause_turn (Anthropic-specific)
    Refusal,        // Model refused to answer
    Error(String),  // Provider-specific error
}
```

Every `LLMResponse` carries a `finish_reason: FinishReason` field. `FinishReason::Length` is the universal truncation signal.

### 3.2 Provider-Specific Wire Format Mapping

| Provider | Wire Field | Truncation Value | Internal Mapping |
|---|---|---|---|
| **Anthropic** | `stop_reason` | `"max_tokens"` | `FinishReason::Length` |
| **OpenAI** | `finish_reason` | `"length"` | `FinishReason::Length` |
| **Gemini** | `finish_reason` | `"MAX_TOKENS"` | `FinishReason::Length` |
| **OpenRouter** | `finish_reason` | `"length"` | `FinishReason::Length` |
| **Ollama** | `done_reason` | `"length"` | `FinishReason::Length` |
| **OpenAI Responses** | `status` | `"incomplete"` | `FinishReason::Length` |
| **DeepSeek/Moonshot/ZAI/LiteLLM/HuggingFace** | `finish_reason` | `"length"` | `FinishReason::Length` |

Anthropic also maps: `"end_turn"` → `Stop`, `"tool_use"` → `ToolCalls`, `"compaction"` / `"pause_turn"` → `Pause`.

OpenAI-compatible providers share `map_finish_reason_common()` which also handles aliases like `"completed"`, `"done"`, `"finished"` → `Stop` and `"sensitive"` → `ContentFilter`.

Streaming providers accumulate the finish reason via a shared `ResponseAggregator` that stores it and places it into the final `LLMResponse` upon `finalize()`.

### 3.3 NO Automatic Re-prompt on `FinishReason::Length`

When the model runs out of output tokens, vtcode does **not** inject a "please continue" message. Instead:

**Skills Executor** — logs warning, returns partial content:
```rust
FinishReason::Length => {
    warn!("Skill '{}' hit token limit", skill.name());
    return Ok(content);  // Partial content accepted
}
```

**Session Controller** — converts to string `"length"` for metrics/events but takes no special action.

**Agent Runner Main Loop** — continuation check is generic (continues if pending tool calls or turns remaining, doesn't special-case `Length`).

### 3.4 Task-Level Continuation (Different Mechanism)

vtcode has a `ContinuationController` that re-prompts based on **task incompleteness**, not output truncation:
- Uses a **task tracker/checklist** system (analyze → change → verify phases)
- When the model signals completion but the checklist has incomplete items, it injects: `"Continue working. Do not stop yet. The task tracker still has incomplete steps: ..."`
- After verification commands fail (`cargo check`, `npm test`), it injects failure details
- Governed by `ContinuationPolicy` (Off / ExecOnly / All)

### 3.5 LLM Error Retry (Exponential Backoff)

Multiple retry layers for **transient API failures** (not truncation):
- `AgentRunner::execute_task_with_retry`: Exponential backoff (2s→30s, 2x multiplier)
- TUI-level retry: 3-6 attempts with category-aware backoff (rate limits: 1-30s; timeouts: 1-15s)
- **Streaming → non-streaming fallback** on stream timeouts
- **Orchestrator retry**: Primary model → fallback model with up to 5 retries
- **Circuit breaker**: Consecutive failures trigger backoff

### 3.6 Six-Layer Truncated Tool Call Recovery Pipeline

#### Layer 1: Streaming Accumulation (`ToolCallBuilder`)
During SSE streaming, tool call arguments arrive as incremental JSON fragments. `ToolCallBuilder` concatenates them with **no JSON validation** — raw string passed forward even if truncated. Empty args default to `"{}"`.

#### Layer 2: Two-Phase JSON Recovery (`parse_tool_arguments`)
```rust
fn parse_tool_arguments(raw: &str) -> Result<Value, Error> {
    match serde_json::from_str(trimmed) {
        Ok(parsed) => Ok(parsed),
        Err(primary_error) => {
            // Recovery: extract balanced JSON from text
            if let Some(candidate) = extract_balanced_json(trimmed) {
                if let Ok(parsed) = serde_json::from_str(candidate) {
                    return Ok(parsed);
                }
            }
            Err(primary_error)
        }
    }
}
```
`extract_balanced_json()` is a depth-tracking bracket matcher that handles:
- **Trailing text after JSON**: `{"path":"src/main.rs"} trailing text` → extracts just the JSON
- **Code-fenced JSON**: `` ```json\n{"path":"src/lib.rs"}\n``` `` → finds the balanced `{...}`
- **Incomplete/truncated JSON**: `{"path":"src/main.rs"` → unbalanced → returns `None`

Tests explicitly verify: `parsed_arguments_rejects_incomplete_json` ✅

#### Layer 3: `PreparedAssistantToolCall` with Error Recording
```rust
pub struct PreparedAssistantToolCall {
    raw_call: uni::ToolCall,
    parsed_args: Option<serde_json::Value>,  // None if parse failed
    args_error: Option<String>,              // Error message if parse failed
    is_parallel_safe: bool,
    is_command_execution: bool,
}
```

#### Layer 4: Dispatch Error Responses
```rust
if valid_calls == 0 {
    // ALL tool calls had invalid args
    for tool_call in tool_calls {
        if let Some(err) = tool_call.args_error() {
            push_invalid_tool_args_response(
                history, tool_call.call_id(), tool_call.tool_name(), err,
            );
        }
    }
    return Ok(None);  // No execution, continue turn loop
}
```
For mixed batches, invalid calls get error responses while valid ones execute normally.

#### Layer 5: Turn-Level Recovery
If a truncated response produces no usable tool calls or text (`TurnProcessingResult::Empty`), recovery mode activates:
- **With recent tool activity** → `RecoveryMode::ToolFreeSynthesis`: disables tools, asks model to synthesize
- **Without recent tool activity** → `RecoveryMode::ToolEnabledRetry`: retries with tools
- System message injected explaining the recovery
- If recovery also fails → turn is `Blocked`

#### Layer 6: Post-Tool LLM Failure Recovery
If tool execution succeeded but follow-up LLM request fails, `maybe_recover_after_post_tool_llm_failure()`:
- Checks if tool responses exist since turn started
- If yes: displays message that tool output is valid, injects `POST_TOOL_RESUME_DIRECTIVE`
- Categorizes error and provides recovery hints

#### Complete Flow for a Truncated Tool Call:
```
LLM generates: {"path":"src/main.rs   (cut off by max_tokens)

↓ FinishReason::Length detected by provider
↓ ToolCallBuilder.finalize() → raw args = '{"path":"src/main.rs'
↓ parse_tool_arguments() → extract_balanced_json() → None (unbalanced)
↓ PreparedAssistantToolCall { parsed_args: None, args_error: Some("...") }
↓ dispatch → push_invalid_tool_args_response() → error in conversation
↓ Model sees error → can retry with correct arguments on next turn
```

### 3.7 Truncated Thinking/Reasoning Handling

vtcode handles truncated thinking through **implicit graceful degradation**:

**Configuration:**
- Extended thinking enabled by default with 31,999-token budget
- Per-request overrides via `thinking_budget: Option<u32>` (min 1024)
- `reasoning_effort` levels: Minimal→1024, Low→4096, Medium→8192, High→16384, XHigh→32768
- Claude Opus 4.6 uses `ThinkingConfig::Adaptive`

**Budget clamping:**
```
effective_budget = budget.min(max_tokens - 100).max(1024)
```
If below 1024, thinking is disabled entirely rather than erroring.

**Signature-gated round-tripping (KEY SAFETY MEASURE):**
When building follow-up requests, thinking blocks are preserved **only if they have both non-empty text AND a cryptographic signature**. Truncated thinking blocks that lack valid signatures are **silently dropped**. This prevents API errors from sending malformed blocks back to Anthropic.

`RedactedThinking` blocks are preserved with their opaque `data` field.

### 3.8 max_tokens Configuration

**Canonical field**: `LLMRequest.max_tokens: Option<u32>` — single source of truth.

**Sources**: Agent runner sets 800 (simple) / 2000 (complex) dynamically. User prefs default to 4096. Environment and per-request overrides available.

**Provider API field mapping:**
- **Anthropic**: `max_tokens` (required), defaults 4096 / 16000 with thinking
- **OpenAI Chat (native)**: `max_completion_tokens`; (non-native): `max_tokens`
- **OpenAI Responses (native)**: `max_output_tokens`; (ChatGPT): omitted
- **Gemini**: `maxOutputTokens` (camelCase)
- **Ollama**: `num_predict`
- **All others**: `max_tokens`

---

## 4. opencode (TypeScript/Vercel AI SDK) — Clean but No Continuation

### 4.1 Normalized Finish Reasons

Normalizes through Vercel AI SDK's `LanguageModelV2FinishReason`:
- Anthropic `"max_tokens"` → `"length"`
- OpenAI `"length"` → `"length"`
- OpenAI Responses `"max_output_tokens"` → `"length"`
- Stored on `AssistantMessage.finish` field

### 4.2 NO Continuation on Length

The agentic loop (`prompt.ts:321-328`):
```typescript
if (
  lastAssistant?.finish &&
  !["tool-calls", "unknown"].includes(lastAssistant.finish) &&
  lastUser.id < lastAssistant.id
) {
  break; // EXIT THE LOOP
}
```
Since `"length"` is NOT in `["tool-calls", "unknown"]`, the loop **breaks** and saves truncated response as-is.

### 4.3 Four-Layer Truncated Tool Call Handling

1. **`experimental_repairToolCall`** (`session/llm.ts:179-198`): Fixes case-sensitivity issues; otherwise redirects to `InvalidTool` which returns error to model
2. **Zod validation** (`tool/tool.ts:59-69`): Every tool call validated against schema before execution
3. **Orphaned tool part cleanup** (`processor.ts:402-417`): After stream ends, pending/running tools marked as `status: "error", error: "Tool execution aborted"`
4. **Max steps limit**: After max steps, tools disabled, model forced to text-only summary

### 4.4 Truncated Thinking/Reasoning

Reasoning is a first-class `ReasoningPart` with full lifecycle tracking. If stream ends mid-reasoning (due to output token limits), the reasoning part is finalized with whatever text was received. **No special retry or continuation** for truncated thinking.

### 4.5 Retry System

Separate retry system (`session/retry.ts`) only for **transient API errors** (429 rate limits, `ECONNRESET`, `resource_exhausted`), NOT for output truncation. Uses exponential backoff (2s → 4s → 8s → 16s → 30s cap).

---

## 5. Comparison Matrix

| Capability | **fspec/codelet** | **vtcode** | **opencode** |
|---|---|---|---|
| Detects `max_tokens` stop | ✅ Enum exists | ✅ `FinishReason::Length` | ✅ `"length"` |
| Propagates to agent loop | ❌ Lost in streaming | ✅ On every `LLMResponse` | ✅ On `AssistantMessage.finish` |
| Auto-continues on truncation | ❌ No | ❌ No | ❌ No |
| Truncated tool call safety | ✅ JSON parse fails → error | ✅ 6-layer recovery | ✅ 4-layer recovery |
| User-visible truncation signal | ❌ Silent | ⚠️ Warning logged | ⚠️ `finish: "length"` stored |
| Persists real stop_reason | ❌ Hardcodes `"end_turn"` | ✅ Yes | ✅ Yes |
| Truncated thinking handling | ❌ Not handled | ✅ Signature-gated dropping | ⚠️ Accepts partial |

---

## 6. Recommended Fixes

### P0 — Propagate `stop_reason` Through Streaming

1. Add `stop_reason: Option<StopReason>` to `FinalResponse` struct
2. Capture `stop_reason` from `MessageDelta` in Anthropic streaming handler
3. Yield it through `MultiTurnStreamItem::FinalResponse`
4. Check it in the CLI `stream_loop`

### P1 — Signal Truncation to User

When `stop_reason == MaxTokens` + text (no tools):
- Display a warning in the TUI (e.g., "⚠️ Response was truncated due to output token limit")
- Allow user to type "continue" to re-prompt

### P2 — Persist Real `stop_reason`

Stop hardcoding `"end_turn"` in `message_envelope.rs`. Propagate the actual stop_reason to the `AssistantMessage` envelope.

### P3 — Consider Auto-Continuation

None of the studied agents auto-continue on truncation, but it would be a differentiator. When `MaxTokens` fires during text generation, inject a "Please continue from where you left off" message.

### P4 — Fix OpenAI max_output_tokens Bug

`ProviderManager::max_output_tokens()` should read the runtime value from the OpenAI provider instance, not the compile-time constant.

### P5 — Truncated Thinking Safety

Adopt vtcode's approach: only round-trip thinking blocks that have valid cryptographic signatures. Drop truncated thinking blocks silently to prevent API errors.
