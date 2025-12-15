# PROV-001 Research: Anthropic Claude Provider Implementation

## Overview

This document captures research from analyzing the TypeScript codelet project to inform the Rust implementation of ClaudeProvider.

## Source Analysis: codelet TypeScript Implementation

### Provider Architecture

The codelet project uses a ProviderManager class that wraps Vercel AI SDK providers. Key patterns:

1. **Provider Types**: `'claude' | 'openai' | 'codex' | 'gemini'`
2. **Credential Detection**: Environment variables checked in priority order
3. **Model Instantiation**: via `getModel()` method
4. **Provider Switching**: Runtime provider changes supported

### Claude Provider Specifics

**Files Analyzed**:
- `/home/rquast/projects/codelet/src/agent/provider-manager.ts`
- `/home/rquast/projects/codelet/src/agent/claude-provider.ts`
- `/home/rquast/projects/codelet/src/agent/claude-auth.ts`

**Authentication**:
- Primary: `ANTHROPIC_API_KEY` environment variable
- Secondary: `CLAUDE_CODE_OAUTH_TOKEN` (OAuth mode - out of scope for PROV-001)

**API Configuration**:
- Endpoint: `https://api.anthropic.com/v1/messages`
- Required header: `anthropic-version: 2023-06-01`
- Default model: `claude-sonnet-4-20250514`

**Beta Features Header**:
```
anthropic-beta: prompt-caching-2024-07-31,oauth-2025-04-20,interleaved-thinking-2025-05-14,tool-examples-2025-10-29
```

### Message Format

Anthropic API requires specific message structure:

```json
{
  "model": "claude-sonnet-4-20250514",
  "max_tokens": 8192,
  "system": "System prompt text here",
  "messages": [
    {"role": "user", "content": "User message"},
    {"role": "assistant", "content": "Assistant response"},
    {"role": "user", "content": "Follow-up"}
  ]
}
```

Key difference from OpenAI: `system` is a separate top-level field, not a message in the array.

### Model Limits

From `model-limits.ts`:
```typescript
'claude-sonnet-4-20250514': {
  contextWindow: 200000,
  maxOutputTokens: 8192,
  name: 'Claude Sonnet 4'
}
```

## Rust Implementation Plan

### Dependencies Required

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### LlmProvider Trait (existing)

From `src/providers/mod.rs`:
```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn context_window(&self) -> usize;
    fn max_output_tokens(&self) -> usize;
    fn supports_caching(&self) -> bool;
    fn supports_streaming(&self) -> bool;
    async fn complete(&self, messages: &[crate::agent::Message]) -> Result<String>;
}
```

### ClaudeProvider Structure

```rust
pub struct ClaudeProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl ClaudeProvider {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| anyhow!("ANTHROPIC_API_KEY not set"))?;

        Ok(Self {
            api_key,
            model: "claude-sonnet-4-20250514".to_string(),
            client: reqwest::Client::new(),
        })
    }
}
```

### API Request Structure

```rust
#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: usize,
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    // ... other fields
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}
```

### Error Handling

The implementation should handle:
1. Missing API key (at construction time)
2. Network errors (reqwest errors)
3. API errors (4xx/5xx responses with error body)
4. Rate limiting (429 responses)

## Scope Decisions

### In Scope (PROV-001)
- Basic ClaudeProvider implementing LlmProvider trait
- API key authentication via environment variable
- Non-streaming completions
- Message formatting (system separate from messages)
- Error handling for common cases
- Model limit reporting

### Out of Scope (Future Stories)
- Streaming responses (PROV-002)
- OAuth/Claude Code authentication (PROV-003)
- Prompt caching support (PROV-004)
- Tool use support (PROV-005)
- Multi-provider management (PROV-006)

## Estimation

Based on analysis:
- Well-defined trait to implement
- Straightforward HTTP client usage
- Clear API contract from Anthropic docs
- 6 scenarios to implement and test

**Recommended: 5 story points**

This accounts for:
- Provider struct and construction (1 point)
- Message formatting logic (1 point)
- HTTP client integration (1 point)
- Error handling (1 point)
- Testing with mocks (1 point)
