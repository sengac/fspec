# Claude Opus 4.6 & Sonnet 4.6 Implementation Plan for fspec/codelet

## Overview

This document provides the implementation plan for supporting Claude Opus 4.6 and Sonnet 4.6 with adaptive thinking, based on the **official Anthropic documentation** at platform.claude.com/docs.

**Key Principle**: Explicit model constants + exact equality checks. No pattern matching.

---

## Official Anthropic Specification

Source: https://platform.claude.com/docs/en/build-with-claude/adaptive-thinking

### Adaptive Thinking Models

| Model | ID | Thinking Type |
|-------|-----|---------------|
| Claude Opus 4.6 | `claude-opus-4-6` | `{"type": "adaptive"}` |
| Claude Sonnet 4.6 | `claude-sonnet-4-6` | `{"type": "adaptive"}` |

> `thinking.type: "enabled"` and `budget_tokens` are **deprecated** on Opus 4.6 and Sonnet 4.6 and will be removed in a future model release. Use `thinking.type: "adaptive"` instead.

### Budgeted Thinking Models (Older)

| Model | ID | Thinking Type |
|-------|-----|---------------|
| Claude Opus 4.5 | `claude-opus-4-5` | `{"type": "enabled", "budget_tokens": N}` |
| Claude Sonnet 4.5 | `claude-sonnet-4-5` | `{"type": "enabled", "budget_tokens": N}` |
| Older models | Various | `{"type": "enabled", "budget_tokens": N}` |

### 1M Context Window Support

Source: https://platform.claude.com/docs/en/build-with-claude/context-windows

> Claude Opus 4.6, Sonnet 4.6, Sonnet 4.5, and Sonnet 4 support a 1-million token context window.

Requires `context-1m-2025-08-07` beta header.

### Beta Headers Summary

| Header | Opus 4.6 | Sonnet 4.6 | Sonnet 4.5 | Opus 4.5 | Unknown |
|--------|----------|------------|------------|----------|---------|
| `prompt-caching-2024-07-31` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `output-64k-2025-02-19` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `interleaved-thinking-2025-05-14` | ✗ | ✗ | ✓ | ✓ | ✓ |
| `context-1m-2025-08-07` | ✓ | ✓ | ✓ | ✗ | ✗ |

**Key insight**: Adaptive thinking models (4.6) get interleaved thinking **automatically** - no beta header needed.

---

## Implementation

### Model Constants (Explicit, Not Pattern Matching)

```rust
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

### Thinking Configuration

```rust
impl ClaudeProvider {
    /// Build thinking configuration for the current model.
    /// 
    /// For Opus/Sonnet 4.6: Returns adaptive thinking config, ignoring any user budget.
    /// For other models: Returns user config as-is (budgeted thinking).
    /// 
    /// THINKING LEVEL HANDLING:
    /// - "off" → No thinking configuration (disabled)
    /// - "low", "med", "high", "adaptive" → type: "adaptive" (budget levels ignored)
    fn build_thinking_config(
        &self,
        user_budget: Option<u32>,
        thinking_level: Option<&str>,
    ) -> Option<serde_json::Value> {
        // Check for explicit "off" - respects user intent to disable thinking
        if thinking_level == Some("off") {
            return None;
        }
        
        // EXACT EQUALITY CHECK - not pattern matching
        if is_adaptive_thinking_model(&self.model_name) {
            // Opus/Sonnet 4.6: Always use adaptive thinking
            // User-provided budget_tokens is intentionally ignored
            // thinking levels (low/med/high) default to adaptive
            return Some(json!({
                "thinking": {
                    "type": "adaptive"
                }
            }));
        }
        
        // Other Claude models: Use budgeted thinking
        if let Some(budget) = user_budget {
            return Some(json!({
                "thinking": {
                    "type": "enabled",
                    "budget_tokens": budget
                }
            }));
        }
        
        None
    }
}
```

### Beta Header Construction

```rust
mod beta_headers {
    pub const PROMPT_CACHING: &str = "prompt-caching-2024-07-31";
    pub const INTERLEAVED_THINKING: &str = "interleaved-thinking-2025-05-14";
    pub const OUTPUT_64K: &str = "output-64k-2025-02-19";
    pub const CONTEXT_1M: &str = "context-1m-2025-08-07";
}

impl ClaudeProvider {
    /// Get beta headers appropriate for the current model.
    /// Based on official Anthropic documentation.
    fn get_beta_headers(&self) -> Vec<&'static str> {
        let mut headers = Vec::new();
        
        // Always include prompt caching and output-64k
        headers.push(beta_headers::PROMPT_CACHING);
        headers.push(beta_headers::OUTPUT_64K);
        
        // Adaptive thinking models do NOT need interleaved-thinking header
        // (it's automatic in adaptive mode)
        if !is_adaptive_thinking_model(&self.model_name) {
            headers.push(beta_headers::INTERLEAVED_THINKING);
        }
        
        // 1M context for specific models
        if supports_1m_context(&self.model_name) {
            headers.push(beta_headers::CONTEXT_1M);
        }
        
        headers
    }
}
```

---

## Test Cases

### Adaptive Thinking Tests

```rust
#[test]
fn test_opus_4_6_uses_adaptive_thinking() {
    assert!(is_adaptive_thinking_model("claude-opus-4-6"));
    let config = build_thinking_config("claude-opus-4-6", Some(16000), None);
    assert_eq!(config["thinking"]["type"], "adaptive");
    assert!(config["thinking"].get("budget_tokens").is_none());
}

#[test]
fn test_sonnet_4_6_uses_adaptive_thinking() {
    assert!(is_adaptive_thinking_model("claude-sonnet-4-6"));
    let config = build_thinking_config("claude-sonnet-4-6", Some(16000), None);
    assert_eq!(config["thinking"]["type"], "adaptive");
}

#[test]
fn test_opus_4_5_uses_budgeted_thinking() {
    assert!(!is_adaptive_thinking_model("claude-opus-4-5"));
    let config = build_thinking_config("claude-opus-4-5", Some(16000), None);
    assert_eq!(config["thinking"]["type"], "enabled");
    assert_eq!(config["thinking"]["budget_tokens"], 16000);
}

#[test]
fn test_thinking_level_high_defaults_to_adaptive_for_opus_4_6() {
    let config = build_thinking_config("claude-opus-4-6", None, Some("high"));
    assert_eq!(config["thinking"]["type"], "adaptive");
    assert!(config["thinking"].get("budget_tokens").is_none());
}

#[test]
fn test_thinking_level_low_defaults_to_adaptive_for_sonnet_4_6() {
    let config = build_thinking_config("claude-sonnet-4-6", None, Some("low"));
    assert_eq!(config["thinking"]["type"], "adaptive");
}

#[test]
fn test_thinking_level_off_disables_thinking_for_opus_4_6() {
    let config = build_thinking_config("claude-opus-4-6", None, Some("off"));
    assert!(config.is_none());
}

#[test]
fn test_thinking_level_off_disables_thinking_for_sonnet_4_6() {
    let config = build_thinking_config("claude-sonnet-4-6", None, Some("off"));
    assert!(config.is_none());
}
```

### Beta Header Tests

```rust
#[test]
fn test_opus_4_6_headers() {
    let headers = get_beta_headers("claude-opus-4-6");
    assert!(headers.contains(&PROMPT_CACHING));
    assert!(headers.contains(&OUTPUT_64K));
    assert!(headers.contains(&CONTEXT_1M));
    assert!(!headers.contains(&INTERLEAVED_THINKING)); // Not needed for adaptive
}

#[test]
fn test_sonnet_4_6_headers() {
    let headers = get_beta_headers("claude-sonnet-4-6");
    assert!(headers.contains(&CONTEXT_1M));
    assert!(!headers.contains(&INTERLEAVED_THINKING)); // Not needed for adaptive
}

#[test]
fn test_sonnet_4_5_headers() {
    let headers = get_beta_headers("claude-sonnet-4-5");
    assert!(headers.contains(&CONTEXT_1M)); // Sonnet 4.5 DOES support 1M
    assert!(headers.contains(&INTERLEAVED_THINKING)); // Needed for budgeted
}

#[test]
fn test_opus_4_5_headers() {
    let headers = get_beta_headers("claude-opus-4-5");
    assert!(!headers.contains(&CONTEXT_1M)); // Opus 4.5 does NOT support 1M
    assert!(headers.contains(&INTERLEAVED_THINKING)); // Needed for budgeted
}
```

### Explicit Constant Tests

```rust
#[test]
fn test_unknown_model_uses_defaults() {
    assert!(!is_adaptive_thinking_model("claude-opus-4-7"));
    assert!(!supports_1m_context("claude-opus-4-7"));
}

#[test]
fn test_partial_match_does_not_work() {
    // Explicit constants mean partial matches don't work
    assert!(!is_adaptive_thinking_model("claude-opus-4-6-preview"));
    assert!(!is_adaptive_thinking_model("my-claude-opus-4-6"));
}
```

---

## Checklist

### Model Constants
- [ ] Add `CLAUDE_OPUS_4_6` constant
- [ ] Add `CLAUDE_SONNET_4_6` constant
- [ ] Add `ADAPTIVE_THINKING_MODELS` list (Opus 4.6, Sonnet 4.6)
- [ ] Add `CONTEXT_1M_MODELS` list (Opus 4.6, Sonnet 4.6, Sonnet 4.5)
- [ ] Implement `is_adaptive_thinking_model()` with exact equality
- [ ] Implement `supports_1m_context()` with exact equality

### Thinking Configuration
- [ ] Check for "off" thinking level first - return None
- [ ] Return `{"type": "adaptive"}` for Opus 4.6 AND Sonnet 4.6
- [ ] Ignore user-provided `budget_tokens` for adaptive models
- [ ] Ignore thinking levels (low/med/high) for adaptive models - default to adaptive
- [ ] Return `{"type": "enabled", "budget_tokens": N}` for older models

### Beta Headers
- [ ] Exclude `interleaved-thinking` for adaptive models (Opus 4.6, Sonnet 4.6)
- [ ] Include `context-1m` for Opus 4.6, Sonnet 4.6, Sonnet 4.5
- [ ] Exclude `context-1m` for Opus 4.5 and unknown models

### Testing
- [ ] Test adaptive thinking for Opus 4.6
- [ ] Test adaptive thinking for Sonnet 4.6
- [ ] Test budgeted thinking for Opus 4.5
- [ ] Test budgeted thinking for Sonnet 4.5
- [ ] Test thinking level "high" defaults to adaptive for Opus 4.6
- [ ] Test thinking level "low" defaults to adaptive for Sonnet 4.6
- [ ] Test thinking level "off" disables thinking for Opus 4.6
- [ ] Test thinking level "off" disables thinking for Sonnet 4.6
- [ ] Test beta headers for all model types
- [ ] Test explicit constant matching (no pattern matching)

---

## Story Points: 3

| Phase | Estimate |
|-------|----------|
| Model constants | 10 min |
| Thinking config | 20 min |
| Beta headers | 30 min |
| Provider construction | 20 min |
| Tests | 30 min |
| **Total** | **~2 hrs** |

---

## References

- Official Anthropic docs: https://platform.claude.com/docs/en/build-with-claude/adaptive-thinking
- Context windows: https://platform.claude.com/docs/en/build-with-claude/context-windows
- VTCode implementation: `/tmp/VTCode/vtcode-core/src/llm/providers/anthropic/`
- OpenCode implementation: `/tmp/opencode/packages/opencode/src/provider/transform.ts`
