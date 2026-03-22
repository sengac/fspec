# VTCode Thinking Token Research — PROV-041

## Executive Summary

VTCode implements a **multi-layered preventive system** for managing thinking/reasoning tokens across all providers, but has **NO explicit recovery mechanism** when thinking tokens are actually exhausted. This research documents VTCode's full architecture to inform PROV-041's design, which aims to go beyond VTCode by adding active recovery.

---

## 1. Universal Abstraction Layer

### ReasoningEffortLevel Enum (`vtcode-config/src/types/mod.rs`)

VTCode abstracts thinking across all providers through a 6-level enum:

| Level | Anthropic (tokens) | OpenAI (effort) | Gemini Flash | Gemini Pro |
|-------|-------------------|-----------------|--------------|------------|
| None | 0 | — | — | — |
| Minimal | 1,024 | "minimal"→"low" | "minimal" | "low" |
| Low | 4,096 | "low" | "low" | "low" |
| Medium (default) | 8,192 | "medium" | "medium" | "high" (fallback) |
| High | 16,384 | "high" | "high" | "high" |
| XHigh | 32,768 | "xhigh" | "high" (capped) | "high" (capped) |

### LLMRequest Fields
- `reasoning_effort: Option<ReasoningEffortLevel>` — Universal reasoning level
- `thinking_budget: Option<u32>` — Explicit Anthropic thinking budget (minimum 1024)
- `effort: Option<String>` — Anthropic Opus 4.5/4.6 token efficiency ("low"/"medium"/"high"/"max")

### LLMResponse Fields
- `reasoning: Option<String>` — Captured reasoning/thinking text from any provider
- `reasoning_details: Option<Vec<String>>` — Detailed reasoning traces

### FinishReason Enum
- `Stop`, `Length`, `ToolCalls`, `ContentFilter`, `Pause`, `Refusal`, `Error(String)`
- When thinking tokens are exhausted together with output tokens, the response terminates with `FinishReason::Length`

---

## 2. Provider-Specific Implementations

### 2.1 Anthropic Provider (Most Sophisticated)

#### Configuration (`vtcode-config/src/core/provider.rs`)

```rust
pub extended_thinking_enabled: bool,                    // default: true
pub interleaved_thinking_beta: String,                  // "interleaved-thinking-2025-05-14"
pub interleaved_thinking_budget_tokens: u32,            // default: 31,999
pub effort: String,                                     // default: "low"
```

**Constants:**
- `RECOMMENDED_THINKING_BUDGET = 10,000`
- `DEFAULT_THINKING_BUDGET = 31,999`
- `MAX_THINKING_BUDGET_64K = 63,999` (Opus 4.5, Sonnet 4.5, Haiku 4.5)
- `MAX_THINKING_BUDGET_32K = 31,999` (Opus 4)
- Environment override: `MAX_THINKING_TOKENS`

#### Budget Resolution Priority Cascade (`anthropic/request_builder/thinking.rs`)

1. **Model-specific override**: Claude Opus 4.6 → always `ThinkingConfig::Adaptive`
2. **Explicit `thinking_budget`** from the request → used directly
3. **`MAX_THINKING_TOKENS` env var** → parsed as override
4. **`reasoning_effort` mapped to tokens** → None=0, Minimal=1024, Low=4096, etc.
5. **Config default** → `interleaved_thinking_budget_tokens` (31,999)

#### Budget Clamping (PRIMARY PREVENTION)

```rust
let effective_budget = budget.min(max_tokens.saturating_sub(100)).max(1024);
```

Budget is clamped to `max_tokens - 100` (leaving room for output) and at least 1024.

#### ThinkingConfig Wire Types

```rust
pub enum ThinkingConfig {
    Enabled { budget_tokens: u32 },   // Fixed budget
    Adaptive,                          // Claude Opus 4.6 only
    Disabled,
}
```

#### Validation Guards (`anthropic/validation.rs`)
- `thinking_budget < 1024` → error
- `budget >= max_tokens` (without interleaved support) → error
- Extended thinking + temperature/top_k → error
- Extended thinking + prefill → error
- Extended thinking + forced tool choice → error
- `top_p` must be 0.95-1.0 when thinking is enabled

#### Streaming Events
- `ThinkingDelta { thinking }` → `LLMStreamEvent::Reasoning { delta }`
- `SignatureDelta { signature }` → silently ignored
- `RedactedThinking { data }` → opaque safety-filtered thinking

#### Exhaustion Signal
- `stop_reason: "max_tokens"` → `FinishReason::Length`
- **No separate "thinking_exhausted" signal**

### 2.2 OpenAI Provider

#### Reasoning Effort
- Uses `reasoning` JSON object with `effort` field + optional `summary: "auto"` for GPT-5
- Default effort per model: Some default to `None`, others to `Medium`
- `Minimal` remapped to `"low"` for GPT-5 Codex
- `XHigh` sent as `"xhigh"`

#### Model-Family Gating
- `supports_reasoning_summaries` flag per model family gates reasoning forwarding
- o3/o4 (o-series): ✅, gemini-3: ✅, gpt-5/codex: ❌
- If `false`, reasoning deltas **silently dropped** AND `strip_reasoning_for_model()` nullifies reasoning on final response

#### Streaming
- Chat Completions: `delta.reasoning_content` field
- Responses API: `response.reasoning_text.delta` / `response.reasoning_summary_text.delta` events

#### Exhaustion Signal
- Chat Completions: `finish_reason: "length"` → `FinishReason::Length`
- Responses API: `response.incomplete` → **hard error** (not FinishReason::Length)

### 2.3 Gemini Provider

#### Thinking Configuration
```rust
pub struct ThinkingConfig {
    pub thinking_level: Option<String>,  // "minimal", "low", "medium", "high"
}
```

#### Model-Specific Mapping
- **Gemini 3 Flash**: supports `minimal`, `low`, `medium`, `high`
- **Gemini 3 Pro**: supports only `low`, `high` (Medium → "high" fallback)

#### Unique: Tag-Based Extraction (No Native Reasoning Field)
Gemini has **no dedicated reasoning field** in its SSE stream. Instead:
1. Streams content **cumulatively** (each chunk contains ALL previous text)
2. `apply_stream_delta()` computes incremental diff
3. Delta passes through `TagStreamSanitizer` extracting `<think>`, `<thought>`, `<reasoning>`, `<analysis>` tags

#### Unique: Thought Signatures
- `thoughtSignature` on `Part` variants for maintaining reasoning context
- Fallback: `"skip_thought_signature_validator"` (Google API escape hatch)

#### Exhaustion Signal
- `"MAX_TOKENS"` → `FinishReason::Length`

### 2.4 LM Studio Provider

#### Pure Delegation
- Thin wrapper around `OpenAIProvider` with zero own reasoning logic
- Primary mechanism: `TagStreamSanitizer` for `<think>...</think>` tag extraction
- Model families typically don't match known families → `supports_reasoning_summaries: false`

### 2.5 Other Providers

| Provider | Reasoning Mechanism |
|----------|-------------------|
| **DeepSeek** | `reasoning_content` or `reasoning` fields in responses |
| **OpenRouter** | Routes to provider-specific formats; handles interleaved thinking models |
| **ZAI/GLM-5** | `thinking: { type: "enabled" }` + `thinking_effort` field |
| **HuggingFace** | Simple `reasoning_effort` pass-through |
| **Minimax, Ollama** | No reasoning parameters |

---

## 3. What Happens When Thinking Tokens Are Exhausted

### VTCode's Approach: Prevention Only, No Recovery

1. **Budget clamping**: `effective_budget = min(budget, max_tokens - 100)` — primary prevention
2. **FinishReason::Length**: Response returns with `stop_reason = "max_tokens"` — signals truncation but VTCode does NOT differentiate thinking exhaustion from regular output truncation
3. **Adaptive thinking (Opus 4.6)**: Model self-manages budget — most sophisticated prevention
4. **No automatic retry**: No mechanism detects "thinking budget exhausted" and re-issues with higher budget
5. **Simple task optimization**: Tasks < 240 chars → forces `Minimal` reasoning + `max_tokens = 800`

### How VTCode Handles FinishReason::Length Generally

| Surface | Behavior |
|---------|----------|
| **Skills executor** | Logs warning, returns partial content |
| **Session controller** | Maps to "length" string, normalizes response back to Stop |
| **Exec runner** | Doesn't check finish_reason — uses ContinuationController |
| **TUI turn loop** | Relies on content/tool_calls examination, not finish_reason |

---

## 4. Context Preservation Architecture

### 4.1 Automatic Compaction

**Trigger**: Token usage > 90% of context window (checked before every turn)

**Two strategies:**
1. **Server-side**: For providers supporting it (OpenAI Responses API)
2. **Local LLM-based**: Sends conversation to LLM for summarization, keeps last 10 messages

### 4.2 Session Memory Envelope (The "inject_summary" Pattern)

```rust
struct SessionMemoryEnvelope {
    session_id: String,
    summary: String,
    task_summary: Option<String>,      // from .vtcode/tasks/current_task.md
    grounded_facts: Vec<GroundedFactRecord>,
    touched_files: Vec<String>,
    history_artifact_path: Option<String>,
    generated_at: String,
}
```

**Process:**
1. Writes full pre-compaction history to JSONL (`.vtcode/history/{session_id}.jsonl`)
2. Extracts grounded facts from tool outputs and user assertions
3. Reads task tracker for task context
4. Records touched files from SessionStats
5. Saves `.memory.json` alongside history
6. Injects envelope as system message at position 0

**Grounded facts extraction:**
- From tool outputs: `summary`, `message`, `result`, `output`, `stdout` fields
- From user assertions: "remember", "note that", "important:", "I am", "my"
- Deduplicates, keeps latest 5 facts

### 4.3 Session Resume with Memory Rehydration

On `/resume`:
1. Scans `.vtcode/history/` for `{session_id}*.memory.json`
2. Loads latest matching envelope
3. Inserts as position-0 system message in restored history

### 4.4 Crash Recovery

`recover_history_from_crash()`:
- Creates synthetic tool responses for missing outputs
- Removes orphan tool outputs without matching calls
- Ensures valid call/output pairing invariants

### 4.5 Session Archive

- Full conversation persisted to `~/.vtcode/sessions/` after every turn
- Throttled to avoid excessive writes
- Retention: max 50 files, 30 days, 100MB
- Searchable via `search_sessions()`

---

## 5. Retry and Recovery Infrastructure

### 5.1 Task-Level Retry (`retry.rs`)
- Exponential backoff, max 3 retries
- Only retries transient errors (timeouts, 5xx)

### 5.2 Per-Turn LLM Request Retry (`llm_request/mod.rs`)
- Default 3 attempts (max 6)
- Category-aware backoff
- Streaming fallback on timeout
- Tool context compaction on post-tool failure
- Permanent error fast-fail

### 5.3 ContinuationController
- **Not triggered by FinishReason::Length**
- Operates at task level — checks completion indicators vs TaskTracker checklist
- Returns: Accept / Continue { prompt } / Verify { commands } / SkipAccept

### 5.4 Loop Detection
- **Response loop detection**: Identical assistant responses
- **Idle turn limit**: N consecutive turns with no tool calls
- **Tool loop limit**: Configurable per-turn tool call limit (120/240)
- **Tool repeat tracker**: Detects repeated identical tool calls

---

## 6. Gap Analysis: What VTCode Doesn't Do (PROV-041 Opportunities)

| Gap | VTCode Behavior | PROV-041 Opportunity |
|-----|-----------------|---------------------|
| **Thinking exhaustion detection** | No differentiation from regular truncation | Detect via heuristic: `FinishReason::Length + has_reasoning + empty_output` |
| **Automatic budget adjustment** | Static budget, no retry with different budget | Retry with halved budget or downgraded effort level |
| **Thinking content preservation** | Reasoning captured but not reused on retry | Inject thinking summary into retry prompt |
| **Progressive degradation** | Manual effort selection only | Auto-downgrade effort across turns on repeated exhaustion |
| **Context window awareness during thinking recovery** | Compaction independent of thinking recovery | Combine: persist memory envelope before thinking-retry near limits |
| **Provider-specific retry strategies** | N/A (no retry on thinking exhaustion) | Anthropic: reduce budget_tokens; OpenAI: lower effort; Gemini: lower thinking_level; Adaptive: inject hint |

---

## 7. Recommended Implementation Strategy

### Layer 1: Prevention (Mirror VTCode)
- Budget clamping: `effective_budget = min(budget, max_tokens - 100)`
- Simple task optimization: Low effort for short prompts

### Layer 2: Detection (New)
- Heuristic: `finish_reason == Length && reasoning_tokens > 0 && output_tokens < 50`
- Provider-agnostic — works on the normalized `LLMResponse`

### Layer 3: Turn-Level Recovery (New)
- Retry budget: max 2 retries per turn
- Strategy per retry:
  1. First retry: Halve thinking budget / drop one effort level
  2. Second retry: Disable thinking entirely
- Preserve thinking content from failed attempt as context

### Layer 4: Session-Level Degradation (New)
- Track thinking exhaustion events across turns
- After 3 exhaustion events: auto-downgrade session reasoning effort
- Notify user of the downgrade

### Layer 5: Context Preservation (Leverage Existing)
- Use existing compaction infrastructure
- On thinking exhaustion near context limits: persist memory envelope first
- Ensure SessionSearch can find the pre-compaction state
