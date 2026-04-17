# AST Research: Compaction Threshold System

## Current compaction_threshold.rs

### Constants
- `AUTOCOMPACT_BUFFER` = 50,000 (post-compaction headroom)
- `SESSION_OUTPUT_TOKEN_MAX` = 32,000 (output reservation cap)

### Functions
- `calculate_usable_context(context_window, model_max_output) -> u64`: threshold = context_window - min(max_output, 32k)
- `calculate_summarization_budget(context_window) -> u64`: post-compaction target = context_window - 50k

## Stream loop call site (stream_loop.rs:276-279)

```rust
let context_window = session.provider_manager().context_window() as u64;
let max_output_tokens = session.provider_manager().max_output_tokens() as u64;
let threshold = calculate_usable_context(context_window, max_output_tokens);
```

`threshold` flows to: pre-prompt check (line 315), CompactionHook::new (line 374, 1055, 1252, 1351), emit_context_fill_from_usage (lines 547, 819, 869), handle_gemini_continuation (938), handle_compaction_retry (1513).

## ProviderManager struct (manager.rs:81-99)

```rust
pub struct ProviderManager {
    credentials: ProviderCredentials,
    current_provider: ProviderType,
    model_registry: Option<ModelRegistry>,
    selected_model: Option<String>,
    pub(crate) model_context_window: Option<usize>,
    pub(crate) model_max_output_tokens: Option<usize>,
    facade_override: Option<String>,
}
```

## Model Family Detection

- `selected_model_info()` returns `Option<&ModelInfo>` with `family: Option<String>`
- For profile models: no registry → returns None → must fall back to model_id prefix matching
- Family values from models.dev: "claude-sonnet", "claude-opus", "gemini-pro", etc.

## Required Changes

1. **compaction_threshold.rs**: Add `CompactionThresholdConfig` enum (Tokens/Percentage) with `resolve(context_window)` method; add `builtin_compaction_threshold(model_id)` function
2. **manager.rs**: Add `compaction_threshold_config: Option<CompactionThresholdConfig>` field; add `compaction_threshold()` method with priority chain; wire into set_model_direct/select_model
3. **stream_loop.rs**: Replace `calculate_usable_context(context_window, max_output_tokens)` with `session.provider_manager().compaction_threshold()`
4. **session_manager.rs**: Expose resolved compaction_threshold in SessionModel (CTX-006 pattern)
