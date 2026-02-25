# AST Research: Claude Thinking Configuration Implementation

## Overview

This document analyzes the current Claude provider implementation in the codelet codebase
to identify what changes are needed for Claude Opus 4.6 and Sonnet 4.6 adaptive thinking support.

---

## Files Analyzed

### 1. `codelet/providers/src/claude.rs`

**Current State:**
- Uses `model_name: String` for model storage (line 85)
- Beta headers are hardcoded constants (lines 37-43):
  - `ANTHROPIC_BETA_HEADER_API_KEY` = `prompt-caching-2024-07-31,interleaved-thinking-2025-05-14`
  - `ANTHROPIC_BETA_HEADER_OAUTH` includes same headers plus OAuth-specific ones
- No model-specific header logic exists
- Thinking config passed as `Option<serde_json::Value>` (line 306)

**Changes Needed:**
- Add model constants: `CLAUDE_OPUS_4_6`, `CLAUDE_SONNET_4_6`
- Add capability lists: `ADAPTIVE_THINKING_MODELS`, `CONTEXT_1M_MODELS`
- Add helper functions: `is_adaptive_thinking_model()`, `supports_1m_context()`
- Modify `create_rig_agent` to check model for adaptive thinking
- Modify beta header construction based on model capabilities

### 2. `codelet/tools/src/facade/thinking_config.rs`

**Current State:**
- `ClaudeThinkingFacade` returns budget-based thinking for all levels (lines 147-169):
  ```rust
  ThinkingLevel::Low => json!({ "thinking": { "type": "enabled", "budget_tokens": 4096 } })
  ThinkingLevel::Medium => json!({ "thinking": { "type": "enabled", "budget_tokens": 16000 } })
  ThinkingLevel::High => json!({ "thinking": { "type": "enabled", "budget_tokens": 32000 } })
  ThinkingLevel::Off => json!({})
  ```
- No model-awareness - same config for all Claude models

**Changes Needed:**
- Add model parameter to `ClaudeThinkingFacade` or create model-aware variant
- Return `{"thinking": {"type": "adaptive"}}` for Opus 4.6 / Sonnet 4.6
- Ignore budget_tokens entirely for adaptive models
- Handle `/thinking off` to disable thinking (return empty config)

### 3. `codelet/napi/src/thinking_config.rs`

**Current State:**
- NAPI bindings call `ClaudeThinkingFacade.request_config(level)` directly (lines 68-71)
- Provider matching includes old Claude models only
- No model version awareness

**Changes Needed:**
- Add `claude-opus-4-6` and `claude-sonnet-4-6` to provider matching
- Pass model identifier to facade for model-specific config generation

### 4. `codelet/napi/src/thinking_level_detection.rs`

**Current State:**
- Detects thinking level from prompt keywords (Off, Low, Medium, High)
- `JsThinkingLevel` enum with 4 variants (lines 13-24)
- Disable keywords force Off (lines 26-35)

**Status:** ✅ No changes needed - this correctly handles `/thinking off`

---

## Key Implementation Points

### Model Constants (New)

```rust
// In codelet/providers/src/claude.rs or new model_constants.rs

/// Claude Opus 4.6 model identifier
pub const CLAUDE_OPUS_4_6: &str = "claude-opus-4-6";

/// Claude Sonnet 4.6 model identifier
pub const CLAUDE_SONNET_4_6: &str = "claude-sonnet-4-6";

/// Claude Sonnet 4.5 model identifier
pub const CLAUDE_SONNET_4_5: &str = "claude-sonnet-4-5";

/// Models that use adaptive thinking (exact equality checks)
pub const ADAPTIVE_THINKING_MODELS: &[&str] = &[
    CLAUDE_OPUS_4_6,
    CLAUDE_SONNET_4_6,
];

/// Models that support 1M context window (exact equality checks)
pub const CONTEXT_1M_MODELS: &[&str] = &[
    CLAUDE_OPUS_4_6,
    CLAUDE_SONNET_4_6,
    CLAUDE_SONNET_4_5,
    "claude-sonnet-4-5-20250929",
];
```

### Capability Check Functions (New)

```rust
/// Check if a model uses adaptive thinking
/// Uses exact equality, NOT pattern matching
pub fn is_adaptive_thinking_model(model: &str) -> bool {
    ADAPTIVE_THINKING_MODELS.contains(&model)
}

/// Check if a model supports 1M context window
/// Uses exact equality, NOT pattern matching
pub fn supports_1m_context(model: &str) -> bool {
    CONTEXT_1M_MODELS.contains(&model)
}
```

### Beta Header Construction (Modified)

Current header is monolithic. Need to make it model-specific:

```rust
fn get_beta_headers(model: &str, auth_mode: AuthMode) -> String {
    let mut headers = vec![
        "prompt-caching-2024-07-31",
        "output-64k-2025-02-19",
    ];
    
    // Add OAuth-specific headers if OAuth mode
    if auth_mode == AuthMode::OAuth {
        headers.insert(0, "claude-code-20250219");
        headers.insert(1, "oauth-2025-04-20");
    }
    
    // Adaptive thinking models do NOT need interleaved-thinking header
    if !is_adaptive_thinking_model(model) {
        headers.push("interleaved-thinking-2025-05-14");
    }
    
    // 1M context for specific models
    if supports_1m_context(model) {
        headers.push("context-1m-2025-08-07");
    }
    
    headers.join(",")
}
```

### Thinking Config Generation (Modified)

```rust
impl ClaudeThinkingFacade {
    /// Generate thinking config based on model and level.
    /// 
    /// For Opus/Sonnet 4.6: Returns adaptive thinking, ignores level (except Off)
    /// For other models: Returns budgeted thinking based on level
    pub fn request_config_for_model(&self, model: &str, level: ThinkingLevel) -> Value {
        // Off always disables thinking
        if level == ThinkingLevel::Off {
            return json!({});
        }
        
        // Adaptive thinking models (Opus 4.6, Sonnet 4.6)
        if is_adaptive_thinking_model(model) {
            return json!({
                "thinking": {
                    "type": "adaptive"
                }
            });
        }
        
        // Budget-based thinking for other models
        match level {
            ThinkingLevel::Off => json!({}),
            ThinkingLevel::Low => json!({
                "thinking": { "type": "enabled", "budget_tokens": 4096 }
            }),
            ThinkingLevel::Medium => json!({
                "thinking": { "type": "enabled", "budget_tokens": 16000 }
            }),
            ThinkingLevel::High => json!({
                "thinking": { "type": "enabled", "budget_tokens": 32000 }
            }),
        }
    }
}
```

---

## Test Coverage Required

Based on feature file scenarios:

1. **Adaptive Thinking Tests** (4 scenarios)
   - Opus 4.6 uses adaptive thinking automatically
   - Sonnet 4.6 uses adaptive thinking automatically
   - User-provided budget_tokens ignored for Opus 4.6
   - User-provided budget_tokens ignored for Sonnet 4.6

2. **Budgeted Thinking Tests** (2 scenarios)
   - Opus 4.5 uses budget-based thinking
   - Sonnet 4.5 uses budget-based thinking

3. **Beta Header Tests** (4 scenarios)
   - Opus 4.6 headers: prompt-caching, output-64k, context-1m (NO interleaved-thinking)
   - Sonnet 4.6 headers: prompt-caching, output-64k, context-1m (NO interleaved-thinking)
   - Opus 4.5 headers: prompt-caching, output-64k, interleaved-thinking (NO context-1m)
   - Sonnet 4.5 headers: prompt-caching, output-64k, interleaved-thinking, context-1m

4. **Model Detection Tests** (2 scenarios)
   - Unknown model uses default behavior
   - Partial model name does not match adaptive thinking models

5. **Thinking Level Tests** (4 scenarios) - NEW
   - Thinking level 'high' defaults to adaptive for Opus 4.6
   - Thinking level 'low' defaults to adaptive for Sonnet 4.6
   - Thinking disabled with 'off' for Opus 4.6
   - Thinking disabled with 'off' for Sonnet 4.6

---

## Summary

The current codebase:
- ✅ Has thinking level detection infrastructure
- ✅ Has thinking config facade pattern
- ✅ Has `/thinking off` support via disable keywords
- ❌ Lacks model-specific thinking config generation
- ❌ Lacks model constants and capability lists
- ❌ Lacks model-aware beta header construction
- ❌ Lacks adaptive thinking support

All changes follow the VTCode pattern:
- Explicit string constants (not pattern matching)
- Exact equality checks (==)
- Capability-based grouping (ADAPTIVE_THINKING_MODELS, CONTEXT_1M_MODELS)
