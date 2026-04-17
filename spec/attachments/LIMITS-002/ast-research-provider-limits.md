# AST Research: Provider Limits Architecture

**Work Unit:** LIMITS-002
**Date:** 2026-04-16
**Purpose:** Understand current provider limits constants, traits, and resolution paths

---

## 1. Existing Traits in codelet-providers

### LlmProvider trait (lib.rs:85)
```rust
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn context_window(&self) -> usize;
    fn max_output_tokens(&self) -> usize;
    fn supports_caching(&self) -> bool;
    fn supports_streaming(&self) -> bool;
    // ... async methods
}
```

### CopilotBehaviorFacade trait (copilot/behavior_facade.rs:24)
- Separate behavior facade for Copilot-specific concerns

---

## 2. Provider Constants — CONTEXT_WINDOW

| Provider | File | Value |
|----------|------|-------|
| Claude | claude.rs:42 | 200,000 |
| OpenAI | openai.rs:31 | DEFAULT_CONTEXT_WINDOW (128,000) |
| Gemini | gemini.rs:20 | 1,000,000 |
| Codex | codex/mod.rs:42 | 272,000 |
| Z.AI | zai.rs:30 | 128,000 |
| Copilot | copilot/mod.rs:64 | 200,000 |

## 3. Provider Constants — MAX_OUTPUT_TOKENS

| Provider | File | Value |
|----------|------|-------|
| Claude | claude.rs:45 | 8,192 |
| OpenAI | openai.rs:34 | DEFAULT_MAX_OUTPUT_TOKENS (4,096) |
| Gemini | gemini.rs:23 | 8,192 |
| Codex | codex/mod.rs:45 | 4,096 |
| Z.AI | zai.rs:33 | 8,192 |
| Copilot | copilot/mod.rs:72 | 4,096 |

## 4. Current Resolution (manager.rs)

### context_window() (line 684)
```rust
pub fn context_window(&self) -> usize {
    self.model_context_window
        .unwrap_or_else(|| self.provider_constant_context_window())
}
```

### max_output_tokens() (line 715)
```rust
pub fn max_output_tokens(&self) -> usize {
    self.model_max_output_tokens
        .unwrap_or_else(|| self.provider_constant_max_output_tokens())
}
```

**Problem:** `model_context_window` is set from models.dev registry in `select_model()`, unconditionally overriding provider constants. The fallback is dead code for registry-selected models.

## 5. Design Decision for LIMITS-002

The new `ModelLimitsResolver` trait will:
- Be a standalone trait (not extending LlmProvider) in `model_limits.rs`
- Provide `max_context_window()` / `max_output_tokens_limit()` returning `Option<usize>` for clamping
- Provide `default_context_window()` / `default_max_output_tokens()` as fallbacks
- Provide `should_send_max_output_tokens()` defaulting to `true` (Codex returns `false`)
- Have a standalone `resolve_model_limits()` pure function for the priority chain

The trait does NOT modify any existing provider code — that's LIMITS-003/004.
