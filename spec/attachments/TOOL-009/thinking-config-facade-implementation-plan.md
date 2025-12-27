# ThinkingConfigFacade Implementation Plan

## Overview

This document outlines the implementation plan for a provider-agnostic thinking/reasoning configuration facade in Rust, with TypeScript bindings via NAPI-RS.

## Problem Statement

Different LLM providers have incompatible APIs for configuring "thinking" or "reasoning" tokens:

| Provider | API Format | Configuration Method |
|----------|------------|---------------------|
| **Gemini 3** | `thinkingConfig: { thinkingLevel: "high" }` | Enum-based level |
| **Gemini 2.5** | `thinkingConfig: { thinkingBudget: 8192 }` | Token count |
| **Claude** | `thinking: { type: "enabled", budget_tokens: 32000 }` | Type + token count |
| **OpenAI o-series** | `reasoning: { effort: "medium" }` | Effort level |

Response parsing also differs:
- **Gemini**: Parts with `thought: true` are thinking content
- **Claude**: Interleaved `<thinking>` blocks or separate content blocks
- **OpenAI**: `reasoning_tokens` in usage stats

## Research Summary

### Gemini CLI (packages/core/src/config/defaultModelConfigs.ts)
```typescript
// Gemini 3 specific
'chat-base-3': {
  thinkingConfig: {
    thinkingLevel: ThinkingLevel.HIGH,
  }
}
// Gemini 2.5 specific
'chat-base-2.5': {
  thinkingConfig: {
    thinkingBudget: 8192,
  }
}
```

### OpenCode (packages/opencode/src/provider/transform.ts)
```typescript
if (model.api.npm === "@ai-sdk/google" || model.api.npm === "@ai-sdk/google-vertex") {
  result["thinkingConfig"] = {
    includeThoughts: true,
  }
  if (model.api.id.includes("gemini-3")) {
    result["thinkingConfig"]["thinkingLevel"] = "high"
  }
}
```

### Aider (aider/models.py)
```python
def set_thinking_tokens(self, value):
    if self.name.startswith("openrouter/"):
        self.extra_params["extra_body"]["reasoning"] = {"max_tokens": num_tokens}
    else:
        self.extra_params["thinking"] = {"type": "enabled", "budget_tokens": num_tokens}
```

### Aider Model Settings (model-settings.yml)
```yaml
- name: gemini/gemini-3-pro-preview
  overeager: true
  edit_format: diff-fenced
  use_repo_map: true
  weak_model_name: gemini/gemini-2.5-flash
  use_temperature: false
  accepts_settings: ["thinking_tokens"]
```

## Design

### Core Abstraction: ThinkingLevel Enum

```rust
/// Provider-agnostic thinking intensity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingLevel {
    /// Disable thinking/reasoning entirely
    Off,
    /// Minimal thinking (fast responses)
    Low,
    /// Balanced thinking (default for most tasks)
    Medium,
    /// Maximum thinking (complex reasoning tasks)
    High,
}
```

### Trait Definition: ThinkingConfigFacade

```rust
pub trait ThinkingConfigFacade {
    /// Returns the provider identifier (e.g., "gemini-3", "claude", "openai-o1")
    fn provider(&self) -> &'static str;

    /// Generates the request configuration JSON for the specified thinking level
    fn request_config(&self, level: ThinkingLevel) -> Value;

    /// Checks if a response part contains thinking content
    fn is_thinking_part(&self, part: &Value) -> bool;

    /// Extracts thinking text from a response part (if it's a thinking part)
    fn extract_thinking_text(&self, part: &Value) -> Option<String>;
}
```

### Provider Implementations

#### Gemini 3 (thinkingLevel enum)

```rust
pub struct Gemini3ThinkingFacade;

impl ThinkingConfigFacade for Gemini3ThinkingFacade {
    fn provider(&self) -> &'static str {
        "gemini-3"
    }

    fn request_config(&self, level: ThinkingLevel) -> Value {
        match level {
            ThinkingLevel::Off => json!({}),
            ThinkingLevel::Low => json!({
                "thinkingConfig": {
                    "includeThoughts": true,
                    "thinkingLevel": "low"
                }
            }),
            ThinkingLevel::Medium => json!({
                "thinkingConfig": {
                    "includeThoughts": true,
                    "thinkingLevel": "medium"
                }
            }),
            ThinkingLevel::High => json!({
                "thinkingConfig": {
                    "includeThoughts": true,
                    "thinkingLevel": "high"
                }
            }),
        }
    }

    fn is_thinking_part(&self, part: &Value) -> bool {
        part.get("thought").and_then(|v| v.as_bool()).unwrap_or(false)
    }

    fn extract_thinking_text(&self, part: &Value) -> Option<String> {
        if self.is_thinking_part(part) {
            part.get("text").and_then(|v| v.as_str()).map(String::from)
        } else {
            None
        }
    }
}
```

#### Gemini 2.5 (thinkingBudget token count)

```rust
pub struct Gemini25ThinkingFacade;

impl ThinkingConfigFacade for Gemini25ThinkingFacade {
    fn provider(&self) -> &'static str {
        "gemini-2.5"
    }

    fn request_config(&self, level: ThinkingLevel) -> Value {
        match level {
            ThinkingLevel::Off => json!({}),
            ThinkingLevel::Low => json!({
                "thinkingConfig": {
                    "includeThoughts": true,
                    "thinkingBudget": 2048
                }
            }),
            ThinkingLevel::Medium => json!({
                "thinkingConfig": {
                    "includeThoughts": true,
                    "thinkingBudget": 8192
                }
            }),
            ThinkingLevel::High => json!({
                "thinkingConfig": {
                    "includeThoughts": true,
                    "thinkingBudget": 32768
                }
            }),
        }
    }

    fn is_thinking_part(&self, part: &Value) -> bool {
        part.get("thought").and_then(|v| v.as_bool()).unwrap_or(false)
    }

    fn extract_thinking_text(&self, part: &Value) -> Option<String> {
        if self.is_thinking_part(part) {
            part.get("text").and_then(|v| v.as_str()).map(String::from)
        } else {
            None
        }
    }
}
```

#### Claude (thinking type + budget_tokens)

```rust
pub struct ClaudeThinkingFacade;

impl ThinkingConfigFacade for ClaudeThinkingFacade {
    fn provider(&self) -> &'static str {
        "claude"
    }

    fn request_config(&self, level: ThinkingLevel) -> Value {
        match level {
            ThinkingLevel::Off => json!({}),
            ThinkingLevel::Low => json!({
                "thinking": {
                    "type": "enabled",
                    "budget_tokens": 4096
                }
            }),
            ThinkingLevel::Medium => json!({
                "thinking": {
                    "type": "enabled",
                    "budget_tokens": 16000
                }
            }),
            ThinkingLevel::High => json!({
                "thinking": {
                    "type": "enabled",
                    "budget_tokens": 32000
                }
            }),
        }
    }

    fn is_thinking_part(&self, part: &Value) -> bool {
        // Claude uses content blocks with type "thinking"
        part.get("type").and_then(|v| v.as_str()) == Some("thinking")
    }

    fn extract_thinking_text(&self, part: &Value) -> Option<String> {
        if self.is_thinking_part(part) {
            part.get("thinking").and_then(|v| v.as_str()).map(String::from)
        } else {
            None
        }
    }
}
```

### File Location

`codelet/tools/src/facade/thinking_config.rs`

This follows the existing facade pattern used by:
- `codelet/tools/src/facade/web_search.rs` - Web search facades
- `codelet/tools/src/facade/system_prompt.rs` - System prompt facades

### NAPI Bindings for TypeScript

TypeScript needs to configure thinking settings from the Node.js layer. We expose:

```rust
// In codelet/napi/src/lib.rs or new file

use napi_derive::napi;

#[napi]
pub enum JsThinkingLevel {
    Off,
    Low,
    Medium,
    High,
}

#[napi]
pub fn get_thinking_config(provider: String, level: JsThinkingLevel) -> napi::Result<String> {
    let level = match level {
        JsThinkingLevel::Off => ThinkingLevel::Off,
        JsThinkingLevel::Low => ThinkingLevel::Low,
        JsThinkingLevel::Medium => ThinkingLevel::Medium,
        JsThinkingLevel::High => ThinkingLevel::High,
    };

    let config = match provider.as_str() {
        "gemini-3" | "gemini-3-pro" | "gemini-3-flash" => {
            Gemini3ThinkingFacade.request_config(level)
        }
        "gemini-2.5" | "gemini-2.5-pro" | "gemini-2.5-flash" => {
            Gemini25ThinkingFacade.request_config(level)
        }
        "claude" | "claude-3" | "claude-opus" | "claude-sonnet" => {
            ClaudeThinkingFacade.request_config(level)
        }
        _ => serde_json::json!({})
    };

    Ok(serde_json::to_string(&config)?)
}

#[napi]
pub fn is_thinking_content(provider: String, part_json: String) -> napi::Result<bool> {
    let part: serde_json::Value = serde_json::from_str(&part_json)?;

    let is_thinking = match provider.as_str() {
        "gemini-3" | "gemini-3-pro" | "gemini-3-flash" => {
            Gemini3ThinkingFacade.is_thinking_part(&part)
        }
        "gemini-2.5" | "gemini-2.5-pro" | "gemini-2.5-flash" => {
            Gemini25ThinkingFacade.is_thinking_part(&part)
        }
        "claude" | "claude-3" | "claude-opus" | "claude-sonnet" => {
            ClaudeThinkingFacade.is_thinking_part(&part)
        }
        _ => false
    };

    Ok(is_thinking)
}
```

### TypeScript Usage

```typescript
import { ThinkingLevel, getThinkingConfig, isThinkingContent } from '@anthropic/codelet-napi';

// Get request configuration
const config = JSON.parse(getThinkingConfig('gemini-3', ThinkingLevel.High));
// Returns: { thinkingConfig: { includeThoughts: true, thinkingLevel: "high" } }

// Merge into API request
const request = {
  model: 'gemini-3-pro',
  contents: [...],
  ...config
};

// Parse response
for (const part of response.candidates[0].content.parts) {
  if (isThinkingContent('gemini-3', JSON.stringify(part))) {
    console.log('Thinking:', part.text);
  } else {
    console.log('Response:', part.text);
  }
}
```

## Implementation Steps

### Phase 1: Core Rust Implementation
1. Create `codelet/tools/src/facade/thinking_config.rs`
2. Define `ThinkingLevel` enum
3. Define `ThinkingConfigFacade` trait
4. Implement `Gemini3ThinkingFacade`
5. Implement `Gemini25ThinkingFacade`
6. Implement `ClaudeThinkingFacade`
7. Add module to `codelet/tools/src/facade/mod.rs`
8. Write unit tests

### Phase 2: NAPI Bindings
1. Add NAPI exports in `codelet/napi/src/lib.rs`
2. Export `JsThinkingLevel` enum
3. Export `getThinkingConfig()` function
4. Export `isThinkingContent()` function
5. Generate TypeScript definitions
6. Write integration tests

### Phase 3: TypeScript Integration
1. Update TypeScript type definitions in `codelet/napi/index.d.ts`
2. Create wrapper module for ergonomic API (optional)
3. Update provider implementations to use thinking config
4. Add documentation

## Testing Strategy

### Unit Tests (Rust)
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini3_high_level() {
        let facade = Gemini3ThinkingFacade;
        let config = facade.request_config(ThinkingLevel::High);

        assert_eq!(
            config["thinkingConfig"]["thinkingLevel"],
            "high"
        );
        assert_eq!(
            config["thinkingConfig"]["includeThoughts"],
            true
        );
    }

    #[test]
    fn test_gemini3_off_returns_empty() {
        let facade = Gemini3ThinkingFacade;
        let config = facade.request_config(ThinkingLevel::Off);

        assert_eq!(config, json!({}));
    }

    #[test]
    fn test_is_thinking_part_gemini() {
        let facade = Gemini3ThinkingFacade;

        let thinking_part = json!({
            "thought": true,
            "text": "Let me think about this..."
        });

        let response_part = json!({
            "text": "The answer is 42."
        });

        assert!(facade.is_thinking_part(&thinking_part));
        assert!(!facade.is_thinking_part(&response_part));
    }
}
```

### Integration Tests (TypeScript)
```typescript
import { ThinkingLevel, getThinkingConfig, isThinkingContent } from '@anthropic/codelet-napi';

describe('ThinkingConfig NAPI Bindings', () => {
  test('Gemini 3 high level config', () => {
    const config = JSON.parse(getThinkingConfig('gemini-3', ThinkingLevel.High));

    expect(config.thinkingConfig.thinkingLevel).toBe('high');
    expect(config.thinkingConfig.includeThoughts).toBe(true);
  });

  test('Claude high level config', () => {
    const config = JSON.parse(getThinkingConfig('claude', ThinkingLevel.High));

    expect(config.thinking.type).toBe('enabled');
    expect(config.thinking.budget_tokens).toBe(32000);
  });

  test('isThinkingContent identifies Gemini thinking parts', () => {
    const thinkingPart = JSON.stringify({ thought: true, text: 'Thinking...' });
    const responsePart = JSON.stringify({ text: 'Answer' });

    expect(isThinkingContent('gemini-3', thinkingPart)).toBe(true);
    expect(isThinkingContent('gemini-3', responsePart)).toBe(false);
  });
});
```

## Open Questions / Decisions

1. **Token budget defaults**: The proposed values (Low=2048/4096, Medium=8192/16000, High=32768/32000) are reasonable starting points. Should these be configurable?

2. **OpenAI o-series support**: Not included in initial scope. Add later if needed.

3. **OpenRouter routing**: Aider handles OpenRouter specially. Consider if codelet needs similar handling.

4. **Streaming thinking**: Both Gemini and Claude support streaming thinking content. The facade currently handles complete responses. Streaming support could be added later.

## References

- [Gemini API - Thinking Config](https://ai.google.dev/gemini-api/docs/thinking)
- [Claude Extended Thinking](https://docs.anthropic.com/en/docs/build-with-claude/extended-thinking)
- [OpenAI Reasoning Tokens](https://platform.openai.com/docs/guides/reasoning)
- Existing facade patterns: `codelet/tools/src/facade/web_search.rs`
