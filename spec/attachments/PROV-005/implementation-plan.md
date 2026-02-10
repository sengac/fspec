# Claude Opus 4.6 Implementation Plan for fspec/codelet

## Overview

This document outlines the specific changes needed in the fspec/codelet codebase to support Claude Opus 4.6 with adaptive thinking.

---

## Current Codebase Analysis

### Relevant Files

| File | Purpose | Changes Needed |
|------|---------|----------------|
| `codelet/providers/src/claude.rs` | Claude provider implementation | Add model detection, skip beta headers |
| `codelet/patches/rig-core/src/providers/anthropic/streaming.rs` | Streaming SSE handling | Add AdaptiveDelta handling (if needed) |
| `codelet/patches/rig-core/src/providers/anthropic/completion.rs` | Completion API types | Add Adaptive thinking type |
| `codelet/patches/rig-core.patch` | Upstream rig-core modifications | Update patch with Adaptive support |
| `codelet/core/src/` | Core types and traits | May need ThinkingConfig updates |

---

## Detailed Implementation Plan

### Phase 1: Model Registration and Detection

#### 1.1 Add Model Constant

**File:** `codelet/providers/src/claude.rs` (or create constants module)

```rust
/// Claude Opus 4.6 model identifier
pub const CLAUDE_OPUS_4_6: &str = "claude-opus-4-6";

/// Check if a model is Claude Opus 4.6
pub fn is_opus_4_6(model: &str) -> bool {
    model == CLAUDE_OPUS_4_6 
        || model == "claude-opus-4.6"
        || model.starts_with("claude-opus-4-6-")
}
```

### Phase 2: rig-core Patch Updates

#### 2.1 Add Adaptive Thinking Type

**File:** `codelet/patches/rig-core/src/providers/anthropic/completion.rs`

Current thinking-related types need updating. Look for `Content` enum and add `Adaptive` handling:

```rust
// In the Content enum or similar structure
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ThinkingConfig {
    Enabled { budget_tokens: u32 },
    #[serde(rename = "adaptive")]
    Adaptive,
    Disabled,
}
```

#### 2.2 Update Streaming Handler

**File:** `codelet/patches/rig-core/src/providers/anthropic/streaming.rs`

The streaming handler already handles `ThinkingDelta` and `SignatureDelta`. No changes should be needed here since adaptive thinking uses the same stream format - the API just decides internally how much to think.

#### 2.3 Update Request Building

**File:** `codelet/patches/rig-core/src/providers/anthropic/streaming.rs` (in the `stream` function)

When building the request body, detect Opus 4.6 and use adaptive thinking:

```rust
// In the stream() function where additional_params are merged
if let Some(ref params) = completion_request.additional_params {
    // Check if this is Opus 4.6 and modify thinking config
    let mut params = params.clone();
    if self.model.contains("opus-4-6") {
        // Force adaptive thinking for Opus 4.6
        if let Some(thinking) = params.get_mut("thinking") {
            *thinking = json!({"type": "adaptive"});
        }
    }
    merge_inplace(&mut body, params)
}
```

### Phase 3: Beta Header Management

#### 3.1 Update Beta Headers for Opus 4.6

**File:** `codelet/providers/src/claude.rs`

Current headers:
```rust
const ANTHROPIC_BETA_HEADER_API_KEY: &str =
    "prompt-caching-2024-07-31,interleaved-thinking-2025-05-14";
```

Need to add a function to get model-specific headers:

```rust
/// Get the appropriate beta header for a model
pub fn get_anthropic_beta_header(model: &str, auth_mode: AuthMode) -> String {
    let mut features = vec!["prompt-caching-2024-07-31"];
    
    // Opus 4.6 doesn't need extended thinking beta - it uses adaptive
    // Also doesn't need effort beta (effort is GA for 4.6)
    if !is_opus_4_6(model) {
        features.push("interleaved-thinking-2025-05-14");
    }
    
    // Opus 4.6 and Sonnet 4.5 support 1M context
    if is_opus_4_6(model) || model.contains("sonnet-4-5") {
        features.push("context-1m-2025-08-07");
    }
    
    if auth_mode == AuthMode::OAuth {
        // OAuth mode requires additional headers
        features.insert(0, "claude-code-20250219");
        features.insert(1, "oauth-2025-04-20");
    }
    
    features.join(",")
}
```

#### 3.2 Update ClaudeProvider Construction

**File:** `codelet/providers/src/claude.rs`

In `from_api_key_with_mode_and_model`:

```rust
// Current code builds beta features statically
// Change to dynamic based on model:
let beta_header = get_anthropic_beta_header(model, auth_mode);
let beta_features: Vec<&str> = beta_header.split(',').collect();

anthropic::Client::builder()
    .api_key(api_key)
    .anthropic_betas(&beta_features)
    // ... rest of builder
```

### Phase 4: Agent Creation Updates

#### 4.1 Update create_rig_agent

**File:** `codelet/providers/src/claude.rs`

The `create_rig_agent` method accepts `thinking_config: Option<serde_json::Value>`. Need to handle Opus 4.6:

```rust
pub fn create_rig_agent(
    &self,
    preamble: Option<&str>,
    thinking_config: Option<serde_json::Value>,
) -> rig::agent::Agent<anthropic::completion::CompletionModel> {
    // ... existing code ...

    // TOOL-010: Merge thinking config with system prompt in additional_params
    let mut additional = json!({
        "system": cached_system
    });

    // Handle thinking config - force adaptive for Opus 4.6
    if let Some(thinking) = thinking_config {
        if is_opus_4_6(&self.model_name) {
            // Override to adaptive for Opus 4.6
            if let Some(obj) = additional.as_object_mut() {
                obj.insert("thinking".to_string(), json!({"type": "adaptive"}));
            }
        } else if let Some(obj) = additional.as_object_mut() {
            if let Some(thinking_obj) = thinking.as_object() {
                for (key, value) in thinking_obj {
                    obj.insert(key.clone(), value.clone());
                }
            }
        }
    } else if is_opus_4_6(&self.model_name) {
        // Default to adaptive for Opus 4.6 even without explicit config
        if let Some(obj) = additional.as_object_mut() {
            obj.insert("thinking".to_string(), json!({"type": "adaptive"}));
        }
    }

    // ... rest of method
}
```

### Phase 5: Effort Parameter Support

#### 5.1 Add Effort Mapping

**File:** `codelet/providers/src/claude.rs` (or new module)

```rust
/// Reasoning effort levels
#[derive(Debug, Clone, Copy)]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
    Max,
}

impl ReasoningEffort {
    /// Convert to API effort string
    pub fn as_str(&self) -> &'static str {
        match self {
            ReasoningEffort::Low => "low",
            ReasoningEffort::Medium => "medium",
            ReasoningEffort::High => "high",
            ReasoningEffort::Max => "max",
        }
    }
}
```

#### 5.2 Add Effort to Request

For Opus 4.6, effort can be included in the request without a beta header:

```rust
// In request building for Opus 4.6
if is_opus_4_6(model) {
    if let Some(effort) = config.effort {
        merge_inplace(&mut body, json!({
            "output_config": {
                "effort": effort.as_str()
            }
        }));
    }
}
```

### Phase 6: Update rig-core Patch

#### 6.1 Regenerate Patch

After making changes to `codelet/patches/rig-core/`, regenerate the patch:

```bash
cd codelet
diff -ruN /tmp/rig-upstream/rig/rig-core patches/rig-core > patches/rig-core.patch
```

Changes to include in the patch:
1. `ThinkingConfig::Adaptive` variant in completion types
2. Any streaming handler updates
3. Header handling if done at rig level

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_opus_4_6_detection() {
    assert!(is_opus_4_6("claude-opus-4-6"));
    assert!(is_opus_4_6("claude-opus-4.6"));
    assert!(is_opus_4_6("claude-opus-4-6-20260205"));
    assert!(!is_opus_4_6("claude-opus-4-5"));
    assert!(!is_opus_4_6("claude-sonnet-4-5"));
}

#[test]
fn test_opus_4_6_beta_headers() {
    let headers = get_anthropic_beta_header("claude-opus-4-6", AuthMode::ApiKey);
    
    // Should include 1M context
    assert!(headers.contains("context-1m-2025-08-07"));
    
    // Should NOT include extended thinking beta
    assert!(!headers.contains("interleaved-thinking-2025-05-14"));
    
    // Should include prompt caching
    assert!(headers.contains("prompt-caching-2024-07-31"));
}

#[test]
fn test_opus_4_6_uses_adaptive_thinking() {
    let provider = ClaudeProvider::from_api_key_with_model(
        "test-key", 
        "claude-opus-4-6"
    ).unwrap();
    
    let agent = provider.create_rig_agent(None, None);
    // Verify additional_params contains {"thinking": {"type": "adaptive"}}
}
```

### Integration Tests

1. Test streaming with Opus 4.6 model
2. Verify thinking blocks are still captured correctly
3. Test effort parameter with different levels
4. Verify OAuth mode works with Opus 4.6

---

## Files to Modify Summary

| Priority | File | Change Type |
|----------|------|-------------|
| High | `codelet/providers/src/claude.rs` | Add model detection, update headers |
| High | `codelet/patches/rig-core/src/providers/anthropic/completion.rs` | Add Adaptive type |
| Medium | `codelet/patches/rig-core/src/providers/anthropic/streaming.rs` | Verify no changes needed |
| Medium | `codelet/patches/rig-core.patch` | Regenerate patch |
| Low | `codelet/core/tests/` | Add tests for Opus 4.6 |

---

## Migration Notes

### Backward Compatibility

- Existing models (Sonnet 4.5, Opus 4.5, etc.) continue to work unchanged
- Extended thinking with budgets still works for non-Opus-4.6 models
- No breaking changes to public APIs

### Configuration

Users can use Opus 4.6 by simply specifying the model:
```
ANTHROPIC_MODEL=claude-opus-4-6
```

No additional configuration needed - adaptive thinking is automatic.

---

## Open Questions

1. **Effort Parameter Exposure**: Should we expose effort levels in the TUI/CLI for Opus 4.6?
2. **1M Context Default**: Should we enable 1M context by default for Opus 4.6, or make it opt-in?
3. **Model Alias**: Should `claude-opus-4.6` (with dot) be supported in addition to `claude-opus-4-6`?

---

## Estimated Effort

- **Phase 1-2**: 2 hours (Model detection and types)
- **Phase 3-4**: 3 hours (Header and agent updates)
- **Phase 5**: 1 hour (Effort parameter)
- **Phase 6**: 1 hour (Patch regeneration)
- **Testing**: 2 hours

**Total**: ~9 hours (estimate: 8 story points)
