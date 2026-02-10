# Claude Opus 4.6 Research Findings

## Sources

Research gathered from analyzing:
- **opencode** (https://github.com/anomalyco/opencode) - cloned to /tmp/opencode-analysis
- **vtcode** (https://github.com/vinhnx/vtcode) - cloned to /tmp/vtcode-analysis

---

## Summary

Claude Opus 4.6 introduces **adaptive thinking** - a new approach to reasoning that differs from the previous "extended thinking" with explicit token budgets used by earlier models.

---

## Key Differences from Previous Models

| Feature | Opus 4.5 / Earlier | Opus 4.6 |
|---------|-------------------|----------|
| Thinking Mode | Extended thinking with explicit `budget_tokens` | Adaptive thinking (auto) |
| Effort Parameter | Beta header required (`effort-2025-11-24`) | GA (no beta header needed) |
| Extended Thinking Beta | Required (`interleaved-thinking-2025-05-14`) | Not needed |
| API Thinking Payload | `{"type": "enabled", "budget_tokens": N}` | `{"type": "adaptive"}` |
| Context Window | 200K | 200K base, 1M with beta |
| Pricing | $5/$25 per million tokens | $5/$25 per million tokens (same) |

---

## opencode Changes

### Key Files Modified

#### 1. `packages/console/app/src/routes/zen/util/provider/anthropic.ts` (line 23)

Added support for 1M context window beta for Opus 4.6:

```typescript
const supports1m = reqModel.includes("sonnet") || reqModel.includes("opus-4-6")
```

When `supports1m` is true, the following header is set:
```typescript
headers.set("anthropic-beta", "context-1m-2025-08-07")
```

#### 2. Documentation Updates (`packages/web/src/content/docs/zen.mdx`)

Added Claude Opus 4.6 to the available models table with pricing information.

---

## vtcode Changes (More Comprehensive)

vtcode implemented full support across multiple files:

### 1. Model Registration (`vtcode-config/src/constants/models/anthropic.rs`)

```rust
// Added model constant
pub const CLAUDE_OPUS_4_6: &str = "claude-opus-4-6";

// Added to SUPPORTED_MODELS array
pub const SUPPORTED_MODELS: &[&str] = &[
    // ...
    "claude-opus-4-6",            // Alias for Claude Opus 4.6
    // ...
];

// Added to REASONING_MODELS array
pub const REASONING_MODELS: &[&str] = &[
    // ...
    CLAUDE_OPUS_4_6,
    // ...
];
```

### 2. Adaptive Thinking Configuration (`vtcode-core/src/llm/providers/anthropic_types.rs`)

New enum variant for adaptive thinking:

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ThinkingConfig {
    Enabled { budget_tokens: u32 },
    Adaptive,  // NEW - for Opus 4.6
    Disabled,
}
```

### 3. Thinking Builder Logic (`vtcode-core/src/llm/providers/anthropic/request_builder/thinking.rs`)

Opus 4.6 automatically uses adaptive thinking - no explicit budget needed:

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
        // Opus 4.6 always uses adaptive thinking
        if resolved_model == models::anthropic::CLAUDE_OPUS_4_6 {
            return (Some(ThinkingConfig::Adaptive), None);
        }
        // ... budget-based thinking for other models
    }
    // ...
}
```

**Key behavior:**
- Opus 4.6 **always uses adaptive thinking** (not token-budget-based extended thinking)
- Explicit budget settings are **ignored** for Opus 4.6

### 4. Effort Parameter Mapping (`vtcode-core/src/llm/providers/anthropic/request_builder/mod.rs`)

Maps reasoning effort levels to effort parameter values for adaptive thinking:

```rust
fn effort_from_reasoning_for_adaptive(effort: ReasoningEffortLevel) -> &'static str {
    match effort {
        ReasoningEffortLevel::None | ReasoningEffortLevel::Minimal | ReasoningEffortLevel::Low => "low",
        ReasoningEffortLevel::Medium => "medium",
        ReasoningEffortLevel::High => "high",
        ReasoningEffortLevel::XHigh => "max",
    }
}
```

Usage in request building:
```rust
let adaptive_effort =
    if resolved_model == models::anthropic::CLAUDE_OPUS_4_6 && request.effort.is_none() {
        request
            .reasoning_effort
            .map(|effort| effort_from_reasoning_for_adaptive(effort).to_string())
    } else {
        None
    };
```

### 5. Beta Headers (`vtcode-core/src/llm/providers/anthropic/headers.rs`)

Opus 4.6 doesn't need certain beta headers:

```rust
// Opus 4.6 doesn't need the extended thinking beta header
if config.extended_thinking_enabled && model != models::anthropic::CLAUDE_OPUS_4_6 {
    pieces.push(config.interleaved_thinking_beta.clone());
}

// Opus 4.6 doesn't need the effort beta header (effort is GA)
if include_effort && model != models::anthropic::CLAUDE_OPUS_4_6 {
    pieces.push("effort-2025-11-24".to_owned());
}

// Opus 4.6 supports 1M context beta
if model == models::anthropic::CLAUDE_SONNET_4_5
    || model == models::anthropic::CLAUDE_SONNET_4_5_20250929
    || model == models::anthropic::CLAUDE_OPUS_4_6
{
    pieces.push("context-1m-2025-08-07".to_owned());
}
```

### 6. Model Presets (`vtcode-core/src/models_manager/model_presets.rs`)

```rust
ModelPreset {
    id: "claude-opus-4.6".to_string(),
    model: "claude-opus-4.6".to_string(),
    display_name: "Claude Opus 4.6".to_string(),
    description: "Next-gen flagship with adaptive thinking".to_string(),
    provider: Provider::Anthropic,
    default_reasoning_effort: ReasoningEffortLevel::Medium,
    supported_reasoning_efforts: vec![
        ReasoningEffortPreset {
            effort: ReasoningEffortLevel::Medium,
            description: "Balanced".to_string(),
        },
        ReasoningEffortPreset {
            effort: ReasoningEffortLevel::High,
            description: "Deep reasoning".to_string(),
        },
    ],
    is_default: false,
    upgrade: None,
    show_in_picker: true,
    supported_in_api: true,
    context_window: Some(200_000),
}
```

### 7. Model Documentation (`docs/models.json`)

```json
"claude-opus-4-6": {
  "id": "claude-opus-4-6",
  "name": "Claude Opus 4.6",
  "description": "Next-gen flagship model with extended and adaptive thinking. 200K context with 1M beta.",
  "reasoning": true,
  "effort": true,
  "tool_call": true,
  "modalities": {
    "input": ["text", "image"],
    "output": ["text"]
  },
  "context": 200000,
  "output_tokens": 128000,
  "status": "current"
}
```

---

## API Request Format Differences

### Opus 4.5 and Earlier (Extended Thinking)

```json
{
  "model": "claude-opus-4-5",
  "thinking": {
    "type": "enabled",
    "budget_tokens": 16384
  },
  "messages": [...]
}
```

Headers required:
```
anthropic-beta: interleaved-thinking-2025-05-14,effort-2025-11-24
```

### Opus 4.6 (Adaptive Thinking)

```json
{
  "model": "claude-opus-4-6",
  "thinking": {
    "type": "adaptive"
  },
  "messages": [...]
}
```

Headers (simplified):
```
anthropic-beta: context-1m-2025-08-07
```

Note: `interleaved-thinking-2025-05-14` and `effort-2025-11-24` are NOT needed for Opus 4.6.

---

## Effort Parameter (Output Control)

The effort parameter controls how many tokens Claude uses when responding:

| Effort Level | Description |
|--------------|-------------|
| `low` | Minimal tokens, fast responses |
| `medium` | Balanced (default) |
| `high` | More thorough responses |
| `max` | Maximum thoroughness |

For Opus 4.6, effort is GA (generally available) - no beta header needed.

---

## Documentation References from vtcode

From `docs/project/TODO.md`:
- https://platform.claude.com/docs/en/build-with-claude/adaptive-thinking
- https://platform.claude.com/docs/en/build-with-claude/effort
- https://platform.claude.com/docs/en/about-claude/models/migration-guide

---

## Test Cases Added (vtcode)

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
        thinking_budget: Some(2048),  // Should be ignored
        ..Default::default()
    };
    let config = AnthropicConfig::default();
    let (thinking, _) =
        build_thinking_config(&request, &config, models::anthropic::DEFAULT_MODEL);

    // Should still use adaptive, not budgeted
    assert!(matches!(thinking, Some(ThinkingConfig::Adaptive)));
}
```

---

## Summary of Required Changes

To support Claude Opus 4.6:

1. **Add the model identifier** `claude-opus-4-6` to supported models list
2. **Add `Adaptive` variant** to thinking configuration enum
3. **Use adaptive thinking** for Opus 4.6 - send `{"type": "adaptive"}` in thinking config
4. **Skip extended thinking beta header** (`interleaved-thinking-2025-05-14`) for Opus 4.6
5. **Skip effort beta header** (`effort-2025-11-24`) for Opus 4.6 - effort is GA
6. **Optionally enable 1M context** with beta header `context-1m-2025-08-07`
7. **Map reasoning effort to effort parameter**: low/medium/high/max
8. **Ignore explicit budget settings** for Opus 4.6 - adaptive doesn't use budgets
