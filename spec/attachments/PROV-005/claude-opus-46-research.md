# Claude Opus 4.6 Research - VTCode & OpenCode Reference

## Overview

Research into how VTCode and OpenCode implement Claude Opus 4.6 support with adaptive thinking.

---

## Key Differences: Opus 4.6 vs Older Models

| Aspect | Opus 4.5 / Sonnet 4.5 | Opus 4.6 / Sonnet 4.6 |
|--------|----------------------|----------------------|
| Thinking Type | `enabled` with `budget_tokens` | `adaptive` (model decides depth) |
| User Budget | Respected | **Ignored** |
| Beta: `interleaved-thinking-2025-05-14` | ✓ Included | ✗ Excluded |
| Beta: `effort-2025-11-24` | ✓ Included | ✗ Excluded |
| Beta: `context-1m-2025-08-07` | ✗ Not supported | ✓ Included (1M context) |
| Beta: `prompt-caching-2024-07-31` | ✓ Included | ✓ Included |
| Beta: `output-64k-2025-02-19` | ✓ Included | ✓ Included |

---

## CRITICAL: Explicit Model Constants (Not Pattern Matching)

VTCode uses **explicit model constants and exact string equality** - NOT substring/pattern matching.

### Why Explicit Constants?

1. **Safety**: Pattern matching like `model.contains("opus-4-6")` could match unintended models
2. **Predictability**: Each model's capabilities are explicitly defined
3. **Maintainability**: When new models release, add them explicitly to the appropriate lists
4. **No false positives**: Exact equality prevents accidental matches

### VTCode's Approach

**File**: `vtcode-config/src/constants/models/anthropic.rs`

```rust
// Explicit constants - NOT pattern matching
pub const CLAUDE_OPUS_4_6: &str = "claude-opus-4-6";
pub const CLAUDE_SONNET_4_6: &str = "claude-sonnet-4-6";
pub const CLAUDE_SONNET_4_5: &str = "claude-sonnet-4-5";
pub const CLAUDE_SONNET_4_5_20250929: &str = "claude-sonnet-4-5-20250929";

// Supported models list - explicit entries
pub const SUPPORTED_MODELS: &[&str] = &[
    "claude-sonnet-4-6",
    "claude-opus-4-6",
    "claude-sonnet-4-5",
    "claude-sonnet-4-5-20250929",
    // ... etc
];
```

**File**: `vtcode-core/src/llm/providers/anthropic/headers.rs`

```rust
// EXACT equality checks - NOT contains/starts_with
if config.model == models::anthropic::CLAUDE_OPUS_4_6 {
    // Opus 4.6 specific behavior
}

// Multiple exact matches for 1M context support
if config.model == models::anthropic::CLAUDE_SONNET_4_5
    || config.model == models::anthropic::CLAUDE_SONNET_4_5_20250929
    || config.model == models::anthropic::CLAUDE_OPUS_4_6
{
    pieces.push("context-1m-2025-08-07".to_owned());
}
```

### What This Means for Codelet

1. Add explicit constants: `CLAUDE_OPUS_4_6`, `CLAUDE_SONNET_4_6`
2. Use `==` equality, not `.contains()` or `.starts_with()`
3. When versioned models release (e.g., `claude-opus-4-6-20260201`), add them explicitly
4. Group models by capability in explicit lists (ADAPTIVE_THINKING_MODELS, CONTEXT_1M_MODELS, etc.)

---

## VTCode Implementation Details

### 1. ThinkingConfig Enum

**File**: `vtcode-core/src/llm/providers/anthropic_types.rs`

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ThinkingConfig {
    Enabled { budget_tokens: u32 },
    Adaptive,
    Disabled,
}
```

**Serialization behavior:**
- `ThinkingConfig::Adaptive` → `{"type": "adaptive"}`
- `ThinkingConfig::Enabled { budget_tokens: 16000 }` → `{"type": "enabled", "budget_tokens": 16000}`
- `ThinkingConfig::Disabled` → `{"type": "disabled"}`

### 2. Model Constants

**File**: `vtcode-config/src/constants/models/anthropic.rs`

```rust
pub const CLAUDE_OPUS_4_6: &str = "claude-opus-4-6";
pub const CLAUDE_SONNET_4_6: &str = "claude-sonnet-4-6";

// Also in supported models list
pub const SUPPORTED_MODELS: &[&str] = &[
    // ...
    "claude-sonnet-4-6",          // Alias for Claude Sonnet 4.6
    "claude-opus-4-6",            // Alias for Claude Opus 4.6
    // ...
];
```

### 3. Request Builder Logic (EXACT EQUALITY)

**File**: `vtcode-core/src/llm/providers/anthropic/request_builder/thinking.rs`

```rust
pub(crate) fn build_thinking_config(
    request: &LLMRequest,
    anthropic_config: &AnthropicConfig,
    default_model: &str,
) -> (Option<ThinkingConfig>, Option<Value>) {
    let resolved_model = if request.model.trim().is_empty() {
        default_model
    } else {
        request.model.as_str()
    };
    
    let thinking_enabled = anthropic_config.extended_thinking_enabled
        && supports_reasoning_effort(&request.model, default_model);

    if thinking_enabled {
        // EXACT EQUALITY CHECK - not pattern matching
        if resolved_model == models::anthropic::CLAUDE_OPUS_4_6 {
            return (Some(ThinkingConfig::Adaptive), None);
        }

        // Other models: Use budgeted thinking
        let budget = /* ... calculate budget ... */;
        if budget >= 1024 {
            return (
                Some(ThinkingConfig::Enabled { budget_tokens: effective_budget }),
                None,
            );
        }
    }

    (None, None)
}
```

### 4. Beta Header Management (EXACT EQUALITY)

**File**: `vtcode-core/src/llm/providers/anthropic/headers.rs`

```rust
pub fn combined_beta_header_value(
    cache_enabled: bool,
    settings: &AnthropicPromptCacheSettings,
    config: &BetaHeaderConfig,
) -> Option<String> {
    let mut pieces: Vec<String> = Vec::new();

    // Prompt caching - always included if enabled
    if let Some(pc) = prompt_cache_beta_header_value(cache_enabled, settings) {
        for p in pc.split(',').map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()) {
            pieces.push(p);
        }
    }

    // EXACT EQUALITY: Opus 4.6 does NOT get interleaved-thinking header
    if config.config.extended_thinking_enabled 
        && config.model != models::anthropic::CLAUDE_OPUS_4_6 
    {
        pieces.push(config.config.interleaved_thinking_beta.clone());
    }

    // Structured outputs
    if config.include_structured {
        pieces.push("structured-outputs-2025-11-13".to_owned());
    }

    // Tool search
    if config.include_tool_search {
        pieces.push("advanced-tool-use-2025-11-20".to_owned());
    }

    // EXACT EQUALITY: Opus 4.6 does NOT get effort header
    if config.include_effort && config.model != models::anthropic::CLAUDE_OPUS_4_6 {
        pieces.push("effort-2025-11-24".to_owned());
    }

    // Output 64k - always included
    pieces.push("output-64k-2025-02-19".to_owned());

    // EXACT EQUALITY: Specific models get 1M context header
    if config.model == models::anthropic::CLAUDE_SONNET_4_5
        || config.model == models::anthropic::CLAUDE_SONNET_4_5_20250929
        || config.model == models::anthropic::CLAUDE_OPUS_4_6
    {
        pieces.push("context-1m-2025-08-07".to_owned());
    }

    // Request-specific betas
    if let Some(betas) = config.request_betas {
        for b in betas {
            if !pieces.contains(b) {
                pieces.push(b.clone());
            }
        }
    }

    if pieces.is_empty() {
        None
    } else {
        Some(pieces.join(", "))
    }
}
```

### 5. VTCode Tests

```rust
#[test]
fn uses_adaptive_thinking_for_opus_4_6_by_default() {
    let request = LLMRequest {
        model: models::anthropic::CLAUDE_OPUS_4_6.to_string(),
        ..Default::default()
    };
    let config = AnthropicConfig::default();
    let (thinking, reasoning) =
        build_thinking_config(&request, &config, models::anthropic::DEFAULT_MODEL);

    assert!(matches!(thinking, Some(ThinkingConfig::Adaptive)));
    assert!(reasoning.is_none());
}

#[test]
fn ignores_explicit_budget_for_opus_4_6() {
    let request = LLMRequest {
        model: models::anthropic::CLAUDE_OPUS_4_6.to_string(),
        thinking_budget: Some(2048),
        ..Default::default()
    };
    let config = AnthropicConfig::default();
    let (thinking, _) =
        build_thinking_config(&request, &config, models::anthropic::DEFAULT_MODEL);

    assert!(matches!(thinking, Some(ThinkingConfig::Adaptive)));
}
```

---

## OpenCode Implementation

### Thinking Variants

**File**: `packages/opencode/test/provider/transform.test.ts`

OpenCode returns effort levels alongside adaptive thinking for 4.6 models:

```typescript
test("anthropic sonnet 4.6 models return adaptive thinking options", () => {
  const model = createMockModel({
    id: "anthropic/claude-sonnet-4-6",
    providerID: "gateway",
    api: {
      id: "anthropic/claude-sonnet-4-6",
      url: "https://gateway.ai",
      npm: "@ai-sdk/gateway",
    },
  })
  const result = ProviderTransform.variants(model)
  expect(Object.keys(result)).toEqual(["low", "medium", "high", "max"])
  expect(result.medium).toEqual({
    thinking: {
      type: "adaptive",
    },
    effort: "medium",
  })
})

test("anthropic opus 4.6 dot-format models return adaptive thinking options", () => {
  const model = createMockModel({
    id: "anthropic/claude-opus-4-6",
    providerID: "gateway",
    api: {
      id: "anthropic/claude-opus-4.6",  // Note: dot format
      url: "https://gateway.ai",
      npm: "@ai-sdk/gateway",
    },
  })
  const result = ProviderTransform.variants(model)
  expect(Object.keys(result)).toEqual(["low", "medium", "high", "max"])
  expect(result.high).toEqual({
    thinking: {
      type: "adaptive",
    },
    effort: "high",
  })
})

// Contrast with older models using budgeted thinking
test("anthropic models return anthropic thinking options", () => {
  const model = createMockModel({
    id: "anthropic/claude-sonnet-4",
    // ...
  })
  const result = ProviderTransform.variants(model)
  expect(Object.keys(result)).toEqual(["high", "max"])
  expect(result.high).toEqual({
    thinking: {
      type: "enabled",
      budgetTokens: 16000,
    },
  })
  expect(result.max).toEqual({
    thinking: {
      type: "enabled",
      budgetTokens: 31999,
    },
  })
})
```

---

## API Request Format

### Adaptive Thinking Request (Opus 4.6)

```json
{
  "model": "claude-opus-4-6",
  "messages": [...],
  "thinking": {
    "type": "adaptive"
  }
}
```

### Budgeted Thinking Request (Opus 4.5 and older)

```json
{
  "model": "claude-opus-4-5",
  "messages": [...],
  "thinking": {
    "type": "enabled",
    "budget_tokens": 16000
  }
}
```

---

## Beta Header Examples

### Opus 4.6 Request Headers

```
anthropic-beta: prompt-caching-2024-07-31, output-64k-2025-02-19, context-1m-2025-08-07
```

### Opus 4.5 Request Headers

```
anthropic-beta: prompt-caching-2024-07-31, interleaved-thinking-2025-05-14, output-64k-2025-02-19
```

---

## Summary

Both VTCode and OpenCode follow the same pattern:

1. **Explicit model constants**: Define `CLAUDE_OPUS_4_6`, `CLAUDE_SONNET_4_6` as string constants
2. **Exact equality checks**: Use `model == CLAUDE_OPUS_4_6`, NOT `model.contains("opus-4-6")`
3. **Ignore user budget**: For 4.6 models, always use adaptive regardless of user-provided `budget_tokens`
4. **Model-specific beta headers**: 
   - Exclude `interleaved-thinking` for 4.6 (exact equality check)
   - Exclude `effort` for 4.6 (exact equality check)
   - Include `context-1m` for 4.6 (exact equality check)

**No pattern matching, no complex effort mapping.** The model handles thinking depth automatically with adaptive mode.
