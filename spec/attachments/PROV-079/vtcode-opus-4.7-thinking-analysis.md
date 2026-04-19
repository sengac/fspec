# VTCode Opus 4.7 Thinking Mode Analysis

## Context

Thinking mode stopped working for Claude Opus 4.7 in our codebase (it works for Opus 4.6).
Analysis of the vtcode repository (`https://github.com/vinhnx/vtcode`) reveals the changes
needed to support Opus 4.7's adaptive-only thinking model.

## Root Cause

Our `ADAPTIVE_THINKING_MODELS` array in `codelet/tools/src/facade/thinking_config.rs` only
contains `claude-opus-4-6` and `claude-sonnet-4-6`. When `claude-opus-4-7` is used, it falls
through to **budgeted thinking** (sending `{"type": "enabled", "budget_tokens": N}`), which
the Opus 4.7 API **rejects** because Opus 4.7 is adaptive-only with no manual budget support.

## VTCode's Fix (Commits Analysed)

### Key Commits
| Commit | Description |
|--------|-------------|
| `c08245318` | Update Anthropic provider to support Claude Opus 4.7 with new task budget and reasoning effort features |
| `0dd616488` | Add `thinking_display` option for Anthropic provider to control API response format |
| `01b3b1b52` | Refactor Anthropic provider to support adaptive thinking and update reasoning effort handling |
| `9049a09aa` | Add handling for reasoning signature in UI and ACP streams |

### VTCode's Opus 4.7 Thinking Profile

VTCode defines a per-model `ClaudeThinkingProfile` struct. For Opus 4.7:

```rust
ClaudeThinkingProfile {
    mode: ClaudeThinkingMode::Adaptive,
    supports_manual_budget: false,      // Rejects budget_tokens
    adaptive_only: true,                // Cannot disable thinking
    default_thinking_enabled: false,    // Thinking requires extended_thinking_enabled=true
    manual_interleaved_beta: false,     // No interleaved-thinking beta header
    supports_effort: true,              // Supports output_config.effort
    supports_task_budget: true,         // Supports output_config.task_budget
    default_display: ThinkingDisplay::Omitted,  // Thinking text omitted by default
    default_effort: "xhigh",            // Default effort level
    supports_xhigh_effort: true,        // Unique to Opus 4.7
    supports_max_effort: true,
}
```

### Comparison: Opus 4.6 vs 4.7

| Feature | Opus 4.6 | Opus 4.7 |
|---------|----------|----------|
| Thinking mode | Adaptive (default) | Adaptive-only |
| Manual budget | Supported | **Rejected** |
| Default effort | `high` | `xhigh` |
| `xhigh` effort | Not supported | Supported |
| Task budget | Not supported | Supported (min 20,000 tokens) |
| Thinking display | `summarized` (default) | `omitted` (default) |
| Interleaved-thinking beta | Not needed | Not needed |
| Temperature/top_p/top_k | Allowed (except when manual thinking active) | **Always rejected** |
| Prefill | Allowed (when thinking disabled) | **Always rejected** |

## Three Layers Affected in Our Codebase

### Layer 1: `codelet/tools/src/facade/thinking_config.rs`

**Problem:** `ADAPTIVE_THINKING_MODELS` doesn't include `claude-opus-4-7`.
**Fix:** Add `claude-opus-4-7` to the list.

```rust
pub const CLAUDE_OPUS_4_7: &str = "claude-opus-4-7";

pub const ADAPTIVE_THINKING_MODELS: &[&str] = &[
    CLAUDE_OPUS_4_6,
    CLAUDE_SONNET_4_6,
    CLAUDE_OPUS_4_7,  // ← ADD
];
```

### Layer 2: `codelet/providers/src/claude.rs`

**Problem:** `build_beta_headers()` sends `interleaved-thinking-2025-05-14` for Opus 4.7
because `is_adaptive_thinking_model()` returns false for it.
**Fix:** Automatic once Layer 1 is fixed — `is_adaptive_thinking_model()` will return true.

### Layer 3: `codelet/napi/src/thinking_config.rs`

**Problem:** `get_thinking_config()` routes through `ClaudeThinkingFacade.request_config_for_model()`
which returns budgeted thinking for unknown models.
**Fix:** Automatic once Layer 1 is fixed.

## Additional VTCode Changes (Optional Enhancements)

### 1. `thinking_display` field
VTCode added a `display` field to `ThinkingConfig::Adaptive`:
```json
{"type": "adaptive", "display": "summarized"}
```
Without this, Opus 4.7 defaults to `"omitted"` and thinking blocks come back empty.
Setting `"summarized"` restores visible thinking summaries.

### 2. `output_config.effort`
VTCode sends `{"output_config": {"effort": "xhigh"}}` for Opus 4.7 to control thinking intensity.
Available levels for Opus 4.7: `low`, `medium`, `high`, `xhigh`, `max`.

### 3. `output_config.task_budget`
VTCode supports task-level token budgets for Opus 4.7:
```json
{"output_config": {"task_budget": {"type": "tokens", "total": 128000}}}
```
Minimum 20,000 tokens. Requires `task-budgets-2026-03-13` beta header.

### 4. Validation guards
- Temperature, top_p, top_k are rejected for Opus 4.7
- Prefilling assistant responses is rejected
- Manual `thinking_budget`/`budget_tokens` is rejected

### 5. `ReasoningSignature` stream event
Opus 4.7 sends `signature_delta` events in the SSE stream. Without handling,
stream parsing may fail. VTCode added a `ReasoningSignature` event variant.

## Minimum Viable Fix

Add `"claude-opus-4-7"` to `ADAPTIVE_THINKING_MODELS` in `thinking_config.rs`.
This single change propagates through `is_adaptive_thinking_model()` to fix:
- Thinking config (adaptive instead of budgeted) ✓
- Beta headers (no interleaved-thinking header) ✓
- NAPI routing (correct JSON config) ✓

## Files Modified in VTCode (52 files in main commit)

Key files relevant to our codebase:
- `vtcode-config/src/constants/models/anthropic.rs` — model constants
- `vtcode-core/src/llm/providers/anthropic/capabilities.rs` — thinking profiles
- `vtcode-core/src/llm/providers/anthropic/request_builder/thinking.rs` — request building
- `vtcode-core/src/llm/providers/anthropic/validation.rs` — request validation
- `vtcode-core/src/llm/providers/anthropic/headers.rs` — beta header logic
- `vtcode-core/src/llm/providers/anthropic_types.rs` — ThinkingConfig enum with display field
- `vtcode-core/src/llm/providers/anthropic/stream_decoder.rs` — signature event handling
