# CTX-007: Per-Model Configurable Compaction Threshold — Research Document

## Current Compaction Threshold System

### Single Formula (No Per-Model Override)

The compaction trigger point is calculated by a single function with no model-specific logic:

```rust
// codelet/cli/src/compaction_threshold.rs:90
pub fn calculate_usable_context(context_window: u64, model_max_output: u64) -> u64 {
    let output_reservation = model_max_output.min(SESSION_OUTPUT_TOKEN_MAX);  // cap at 32k
    let output_reservation = if output_reservation == 0 { SESSION_OUTPUT_TOKEN_MAX } else { output_reservation };
    context_window.saturating_sub(output_reservation)
}
```

**Formula:** `threshold = context_window - min(max_output, 32,000)`

### Call Site (The Only Production Consumer)

```rust
// codelet/cli/src/interactive/stream_loop.rs:276-279
let context_window = session.provider_manager().context_window() as u64;
let max_output_tokens = session.provider_manager().max_output_tokens() as u64;
let threshold = calculate_usable_context(context_window, max_output_tokens);
```

This `threshold` is then passed to:
1. Pre-prompt compaction check (line 315)
2. `CompactionHook::new(threshold)` (line 390)
3. `emit_context_fill_from_usage()` for fill percentage (lines 547, 819, 869)
4. Post-loop compaction retry (line 1497)

### Three Compaction Trigger Paths

| Path | Location | Trigger Condition |
|------|----------|-------------------|
| **Pre-prompt** | `stream_loop.rs:315` | `estimated_total > threshold` |
| **Hook-based** | `compaction_hook.rs:196` | `effective_total > threshold` |
| **Emergency** | `stream_loop.rs:1166` | API error "prompt is too long" |

All three paths use the same `threshold` value calculated at the start of `run_agent_stream_internal()`.

### Post-Compaction Budget

After compaction, the target size is:

```rust
// codelet/cli/src/compaction_threshold.rs:58
pub fn calculate_summarization_budget(context_window: u64) -> u64 {
    if context_window <= AUTOCOMPACT_BUFFER {  // 50,000
        (context_window as f64 * 0.8) as u64
    } else {
        context_window - AUTOCOMPACT_BUFFER  // e.g., 200k - 50k = 150k
    }
}
```

**Note:** This function has **zero production call sites** in the scoped Rust code. It's defined and tested but not directly invoked from the compaction execution path.

## Per-Model Threshold Examples

| Model | Context Window | Current Threshold | Desired Threshold | Config |
|-------|---------------|-------------------|-------------------|--------|
| Claude Sonnet 4 | 200,000 | 191,808 (200k-8k) | **184,000** (retain ~200k behavior) | `{ tokens: 200000 }` |
| Claude Opus 4.6 (API=200k) | 200,000 | 191,808 | **184,000** | `{ tokens: 200000 }` |
| Claude Opus 4.6 (API=1M, after CONFIG-007) | 1,000,000 | 968,000 | **200,000** (user preference) | `{ tokens: 200000 }` |
| Gemini 2.5 Pro | 1,000,000 | 968,000 | **800,000** (80%) | `{ percentage: 80 }` |
| GPT-4 | 128,000 | 123,904 | **102,400** (80%) | `{ percentage: 80 }` |
| Custom vLLM (32k) | 32,000 | 27,904 | **25,600** (80%) | `{ percentage: 80 }` |
| Custom vLLM (user override) | 32,000 | 27,904 | **24,000** (user-set) | `{ tokens: 24000 }` |

## Proposed Design

### 1. Threshold Configuration Type

```rust
/// Per-model compaction threshold configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompactionThresholdConfig {
    /// Absolute token count (e.g., 200000 for Claude)
    Tokens(u64),
    /// Percentage of context window (e.g., 80 for 80%)
    Percentage(u8),
}

impl CompactionThresholdConfig {
    /// Resolve to absolute token count given the context window
    pub fn resolve(&self, context_window: u64) -> u64 {
        match self {
            Self::Tokens(tokens) => *tokens,
            Self::Percentage(pct) => (context_window as f64 * (*pct as f64 / 100.0)) as u64,
        }
    }
}
```

### 2. Resolution Priority Chain

```
1. User-configured per-model threshold (from CustomModelDefinition or NAPI override)
2. Built-in model family defaults (Claude=200k tokens, others=80%)
3. Legacy calculation: calculate_usable_context(context_window, max_output)
```

### 3. Built-in Model Family Defaults

```rust
/// Get built-in compaction threshold for known model families
fn builtin_compaction_threshold(model_family: Option<&str>) -> Option<CompactionThresholdConfig> {
    match model_family {
        Some(f) if f.starts_with("claude-") => Some(CompactionThresholdConfig::Tokens(200_000)),
        Some(f) if f.starts_with("gemini-") => Some(CompactionThresholdConfig::Percentage(80)),
        Some(f) if f.starts_with("gpt-") => Some(CompactionThresholdConfig::Percentage(80)),
        Some(f) if f.starts_with("o1") || f.starts_with("o3") || f.starts_with("o4") => {
            Some(CompactionThresholdConfig::Percentage(80))
        }
        _ => None, // Fall through to legacy calculation
    }
}
```

### 4. ProviderManager Integration

```rust
impl ProviderManager {
    /// Model-specific compaction threshold override
    compaction_threshold_config: Option<CompactionThresholdConfig>,
    
    /// Resolve the effective compaction threshold for the current model
    pub fn compaction_threshold(&self) -> u64 {
        let context_window = self.context_window() as u64;
        let max_output = self.max_output_tokens() as u64;
        
        // 1. User-configured override (from NAPI/TUI)
        if let Some(config) = &self.compaction_threshold_config {
            return config.resolve(context_window);
        }
        
        // 2. Built-in model family default
        let family = self.selected_model_info()
            .and_then(|info| info.family.as_deref());
        if let Some(config) = builtin_compaction_threshold(family) {
            return config.resolve(context_window);
        }
        
        // 3. Legacy fallback
        calculate_usable_context(context_window, max_output)
    }
}
```

### 5. Stream Loop Integration

```rust
// codelet/cli/src/interactive/stream_loop.rs — CHANGE
// Before:
let threshold = calculate_usable_context(context_window, max_output_tokens);

// After:
let threshold = session.provider_manager().compaction_threshold();
```

This is a **one-line change** in the stream loop. All downstream consumers (CompactionHook, pre-prompt check, fill percentage, compaction retry) automatically use the new threshold.

### 6. Fill Percentage Recalibration

The fill percentage is currently: `(total_tokens / threshold) * 100`

With a configurable threshold, this becomes:
- **Green (0-50%)**: Plenty of room
- **Yellow (50-70%)**: Getting full
- **Magenta (70-85%)**: Nearly full  
- **Red (85-100%+)**: Compaction imminent

If the user configures a 200k threshold on a 1M context model, the percentage reflects proximity to the **compaction trigger**, not to the absolute context limit. This is the correct behavior — the user cares about "when will compaction fire?" not "how much of the theoretical context am I using?"

### 7. Post-Compaction Budget Adjustment

`calculate_summarization_budget()` currently uses `context_window - AUTOCOMPACT_BUFFER`. With a configurable threshold, the budget should be relative to the threshold, not the context window:

```rust
pub fn calculate_summarization_budget(threshold: u64) -> u64 {
    if threshold <= AUTOCOMPACT_BUFFER {
        (threshold as f64 * 0.8) as u64
    } else {
        threshold - AUTOCOMPACT_BUFFER
    }
}
```

For Claude with 200k threshold: budget = 200k - 50k = 150k (same as before).
For Gemini with 800k threshold: budget = 800k - 50k = 750k.

## Files to Modify

### Rust Side

| File | Change |
|------|--------|
| `codelet/cli/src/compaction_threshold.rs` | Add `CompactionThresholdConfig` enum, `resolve()`, `builtin_compaction_threshold()` |
| `codelet/providers/src/manager.rs` | Add `compaction_threshold_config: Option<CompactionThresholdConfig>` field, `compaction_threshold()` method, wire into `set_model_direct()` and `select_model()` |
| `codelet/cli/src/interactive/stream_loop.rs` | Replace `calculate_usable_context()` call with `provider_manager().compaction_threshold()` |
| `codelet/napi/src/session_manager.rs` | Accept optional `compaction_threshold` in `session_set_model` / `session_set_model_profile` |

### Unchanged (Automatically Correct)

| File | Why No Change |
|------|---------------|
| `codelet/core/src/compaction_hook.rs` | Receives `threshold` parameter — already uses whatever value the stream loop provides |
| `codelet/cli/src/interactive/compaction_retry.rs` | Receives `threshold` parameter — same reason |

## Test Strategy

1. **Unit tests for `CompactionThresholdConfig::resolve()`**: Tokens mode returns exact value; Percentage mode computes correctly
2. **Unit tests for `builtin_compaction_threshold()`**: Claude family → 200k tokens; Gemini → 80%; Unknown → None
3. **Unit tests for `ProviderManager::compaction_threshold()`**: Priority chain (user override > builtin > legacy)
4. **Integration test**: Stream loop uses the new threshold for CompactionHook
5. **Regression test**: Existing Claude behavior unchanged (200k threshold)
6. **Edge case**: Small context window (32k) with 80% threshold = 25,600
7. **Edge case**: User sets threshold > context_window (should be clamped to context_window)
