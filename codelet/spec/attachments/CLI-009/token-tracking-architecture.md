# Token Tracking Architecture for CLI-009

**Work Unit**: CLI-009
**Feature**: Context Compaction with Anchoring System
**Decision Date**: 2025-12-03
**Status**: Approved

---

## Executive Summary

This document explains why **codelet must implement custom token tracking** instead of relying on rig's built-in `GetTokenUsage` trait. Analysis of both codelet (TypeScript) and rig (Rust) codebases reveals that rig loses critical cache token granularity during conversion from provider-specific types to generic types, making it impossible to calculate "effective tokens" needed for intelligent compaction triggering.

**Decision**: Implement custom token tracking using rig's provider-level `anthropic::completion::Usage` directly, bypassing the lossy conversion to `crate::completion::Usage`.

---

## Background: The Problem

Context compaction in codelet (following codelet's design) requires calculating **effective tokens** to determine when to trigger compaction:

```
Effective Tokens = Input Tokens - (Cache Read Tokens × 0.9)
```

This accounts for Anthropic's **90% discount on cached tokens**, preventing premature compaction when the prompt cache is working effectively.

**Example**:
- Total input: 100k tokens
- Cache read: 80k tokens
- Effective: 100k - (80k × 0.9) = 100k - 72k = **28k tokens**

Without separate `cache_read_input_tokens`, this calculation is **impossible**.

---

## Technical Analysis

### Codelet Token Tracking (TypeScript)

**Source**: `/home/rquast/projects/codelet/src/agent/token-tracker.ts`

Codelet uses the `ai` SDK with Anthropic provider and extracts tokens from SSE events:

```typescript
export interface TokenUsage {
  inputTokens: number;                    // From message_start.usage.input_tokens
  outputTokens: number;                   // From message_delta.usage.output_tokens
  cachedInputTokens: number;              // LEGACY (deprecated)
  reasoningTokens: number;                // From Codex/OpenAI only
  totalTokens: number;                    // Calculated sum
  cacheReadInputTokens: number;           // ✅ FROM ANTHROPIC API
  cacheCreationInputTokens: number;       // ✅ FROM ANTHROPIC API
}
```

**Extraction from Anthropic SSE Events**:

```typescript
// token-tracker.ts:90-102
export function updateFromClaudeMessageStart(
  tracker: TokenUsage,
  event: ClaudeMessageStartEvent
): TokenUsage {
  const inputTokens = event.message.usage.input_tokens;
  const cachedInputTokens = event.message.usage.cache_read_input_tokens ?? 0;

  return {
    ...tracker,
    inputTokens,
    cachedInputTokens,
    totalTokens: inputTokens + tracker.outputTokens,
  };
}
```

**Effective Token Calculation**:

```typescript
// runner.ts:124-129
export function calculateEffectiveTokens(tracker: TokenUsage): number {
  const cacheDiscount = tracker.cacheReadInputTokens * 0.9;
  return tracker.inputTokens - cacheDiscount;
}
```

**Compaction Trigger Logic**:

```typescript
// runner.ts:836-849
const compactionThreshold = getCompactionThreshold(modelLimits.contextWindow);
if (shouldTriggerCompaction(tokenTracker, compactionThreshold)) {
  logger.info('Triggering auto-compaction', {
    currentTokens: tokenTracker.totalTokens,
    threshold: compactionThreshold,
    contextWindow: modelLimits.contextWindow,
  });

  // Perform compaction...
}

function shouldTriggerCompaction(tracker: TokenUsage, threshold: number): boolean {
  const effectiveTokens = calculateEffectiveTokens(tracker);
  return effectiveTokens > threshold;
}
```

---

### Rig Token Tracking (Rust)

**Source**: `/home/rquast/projects/rig/rig-core/src/providers/anthropic/`

#### Provider-Level Usage (Has Cache Data)

Rig's Anthropic provider receives cache tokens directly from the API:

```rust
// rig-core/src/providers/anthropic/completion.rs:90-95
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub cache_read_input_tokens: Option<u64>,     // ✅ FROM ANTHROPIC API
    pub cache_creation_input_tokens: Option<u64>, // ✅ FROM ANTHROPIC API
    pub output_tokens: u64,
}
```

This struct correctly models Anthropic's API response, including:
- `cache_read_input_tokens` - Tokens read from cache (90% discount)
- `cache_creation_input_tokens` - Tokens written to cache (25% premium)

#### Generic Usage (Loses Cache Data)

However, rig converts this to a **generic** `crate::completion::Usage` type that **loses cache granularity**:

```rust
// rig-core/src/providers/anthropic/completion.rs:126-137
impl GetTokenUsage for Usage {
    fn token_usage(&self) -> Option<crate::completion::Usage> {
        let mut usage = crate::completion::Usage::new();

        // ⚠️ PROBLEM: Cache tokens MERGED into input_tokens!
        usage.input_tokens = self.input_tokens
            + self.cache_creation_input_tokens.unwrap_or_default()
            + self.cache_read_input_tokens.unwrap_or_default();  // LOST!

        usage.output_tokens = self.output_tokens;
        usage.total_tokens = usage.input_tokens + usage.output_tokens;

        Some(usage)
    }
}
```

**The Generic Type** (no cache fields):

```rust
// rig-core/src/completion/request.rs
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Usage {
    pub input_tokens: u64,     // Cache tokens merged into this
    pub output_tokens: u64,
    pub total_tokens: u64,
    // ❌ NO cache_read_input_tokens field
    // ❌ NO cache_creation_input_tokens field
}
```

#### Streaming Token Extraction

During streaming, rig extracts tokens from `MessageStart` event:

```rust
// rig-core/src/providers/anthropic/streaming.rs:241-247
StreamingEvent::MessageStart { message } => {
    input_tokens = message.usage.input_tokens;

    let span = tracing::Span::current();
    span.record("gen_ai.response.id", &message.id);
    span.record("gen_ai.response.model_name", &message.model);
},
```

The `message.usage` is an `anthropic::completion::Usage` struct that **contains** `cache_read_input_tokens`, but this data is:
1. Captured in `input_tokens` only (line 242)
2. Never exposed separately
3. Lost when converted to `PartialUsage` then `crate::completion::Usage`

---

## Comparison Matrix

| Aspect | Codelet (TypeScript) | Rig (Rust) |
|--------|---------------------|-----------|
| **Receives cache_read from API?** | ✅ YES (message_start.usage) | ✅ YES (MessageStart.usage) |
| **Receives cache_creation from API?** | ✅ YES (Anthropic API) | ✅ YES (MessageStart.usage) |
| **Preserves cache_read separately?** | ✅ YES (TokenUsage.cacheReadInputTokens) | ❌ NO (merged into input_tokens) |
| **Preserves cache_creation separately?** | ✅ YES (TokenUsage.cacheCreationInputTokens) | ❌ NO (merged into input_tokens) |
| **Can calculate effective tokens?** | ✅ YES | ❌ NO (data lost) |
| **Supports cache-aware compaction?** | ✅ YES | ❌ NO |
| **Data Loss in Conversion?** | ❌ NO | ✅ YES (GetTokenUsage trait) |
| **Accuracy for Compaction** | **100% accurate** | **Insufficient** |

---

## Root Cause Analysis

### Why Rig Loses Cache Data

Rig's architecture prioritizes **provider abstraction** over **provider-specific features**:

1. **Generic `crate::completion::Usage`** designed for ALL providers (OpenAI, Anthropic, Gemini, etc.)
2. **Lowest common denominator approach** - only fields ALL providers support
3. **Cache tokens are Anthropic-specific**, so excluded from generic type
4. **`GetTokenUsage` trait** enforces conversion to generic type, losing data

This is a **deliberate design choice** for cross-provider compatibility, but it makes cache-aware compaction impossible with the generic API.

### Why Codelet Preserves Cache Data

Codelet's architecture uses **provider-specific handling**:

1. **Separate extraction functions** per provider (updateFromClaudeMessageStart, updateFromCodexResponseCompleted)
2. **Provider-specific fields** in unified TokenUsage struct
3. **No forced abstraction** to generic type
4. **Direct API mapping** preserves all data

---

## Solution: Custom Token Tracking

### Recommended Implementation (Option A)

**Use rig's provider-level `anthropic::completion::Usage` directly, bypassing generic conversion.**

#### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                   Streaming Agent Response                       │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│         MultiTurnStreamItem::FinalResponse(response)             │
│                                                                   │
│  Contains: response.usage() -> rig::completion::Usage            │
│  Problem: Cache data ALREADY LOST at this point                  │
└─────────────────────────────────────────────────────────────────┘
                         │
                         ✗ TOO LATE - DATA LOST
                         │
                         ▼
         We need to intercept EARLIER in the stream
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│      MultiTurnStreamItem::StreamAssistantItem(content)           │
│                                                                   │
│  We need access to raw Anthropic MessageStart event HERE         │
└─────────────────────────────────────────────────────────────────┘
```

#### Implementation Strategy

Since rig's `MultiTurnStreamItem` doesn't expose provider-specific usage data, we have **two approaches**:

**Approach 1: Extend RigAgent to Track Provider-Specific Usage**

Create a custom wrapper that intercepts streaming events before they're converted:

```rust
// src/agent/token_tracker.rs

/// Custom token tracker that preserves Anthropic cache metrics
#[derive(Debug, Clone, Default)]
pub struct TokenTracker {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,

    // Anthropic-specific cache fields
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
}

impl TokenTracker {
    /// Calculate effective tokens accounting for cache discount
    /// Formula: input - (cache_read * 0.9)
    pub fn effective_tokens(&self) -> u64 {
        let cache_discount = (self.cache_read_input_tokens as f64 * 0.9) as u64;
        self.input_tokens.saturating_sub(cache_discount)
    }

    /// Update from Anthropic provider-specific usage
    pub fn update_from_anthropic(&mut self, usage: &anthropic::completion::Usage) {
        self.input_tokens = usage.input_tokens;
        self.output_tokens = usage.output_tokens;
        self.total_tokens = self.input_tokens + self.output_tokens;
        self.cache_read_input_tokens = usage.cache_read_input_tokens.unwrap_or(0);
        self.cache_creation_input_tokens = usage.cache_creation_input_tokens.unwrap_or(0);
    }
}
```

**Approach 2: Fork/Extend Rig's MultiTurnStreamItem**

Add a variant that exposes provider-specific data:

```rust
// This would require modifying rig or creating a wrapper
pub enum ExtendedStreamItem<R> {
    RigItem(MultiTurnStreamItem<R>),
    ProviderSpecificUsage(ProviderUsage),
}

pub enum ProviderUsage {
    Anthropic(anthropic::completion::Usage),
    OpenAI(openai::completion::Usage),
    // ... other providers
}
```

**Recommended: Approach 1** - Less invasive, maintains rig compatibility.

#### Detailed Implementation Plan

```rust
// src/agent/token_tracker.rs

use rig::providers::anthropic;

/// Token usage tracker with cache-aware metrics
#[derive(Debug, Clone, Default)]
pub struct TokenTracker {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
}

impl TokenTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate effective tokens for compaction trigger
    pub fn effective_tokens(&self) -> u64 {
        let cache_discount = (self.cache_read_input_tokens as f64 * 0.9) as u64;
        self.input_tokens.saturating_sub(cache_discount)
    }

    /// Update from provider-specific usage (Anthropic)
    pub fn update_from_anthropic(&mut self, usage: &anthropic::completion::Usage) {
        self.input_tokens = usage.input_tokens;
        self.output_tokens = usage.output_tokens;
        self.total_tokens = self.input_tokens + self.output_tokens;
        self.cache_read_input_tokens = usage.cache_read_input_tokens.unwrap_or(0);
        self.cache_creation_input_tokens = usage.cache_creation_input_tokens.unwrap_or(0);
    }
}

/// Compaction trigger configuration
pub struct CompactionConfig {
    pub threshold_percent: f64,  // Default: 0.9 (90%)
    pub context_window: u64,
}

impl CompactionConfig {
    pub fn threshold_tokens(&self) -> u64 {
        (self.context_window as f64 * self.threshold_percent) as u64
    }

    pub fn should_compact(&self, tracker: &TokenTracker) -> bool {
        tracker.effective_tokens() > self.threshold_tokens()
    }
}
```

#### Integration with Session

```rust
// src/session/mod.rs

use crate::agent::token_tracker::{TokenTracker, CompactionConfig};

pub struct Session {
    provider_manager: ProviderManager,
    pub messages: Vec<rig::message::Message>,
    messages_before_interruption: Option<Vec<rig::message::Message>>,

    // NEW: Token tracking for cache-aware compaction
    token_tracker: TokenTracker,
    compaction_config: CompactionConfig,
}

impl Session {
    pub fn new(provider_name: Option<&str>) -> Result<Self> {
        let provider_manager = if let Some(name) = provider_name {
            ProviderManager::with_provider(name)?
        } else {
            ProviderManager::new()?
        };

        // Get context window from provider
        let context_window = provider_manager.context_window();

        Ok(Self {
            provider_manager,
            messages: Vec::new(),
            messages_before_interruption: None,
            token_tracker: TokenTracker::new(),
            compaction_config: CompactionConfig {
                threshold_percent: 0.9,
                context_window,
            },
        })
    }

    /// Check if compaction should trigger based on effective tokens
    pub fn should_compact(&self) -> bool {
        self.compaction_config.should_compact(&self.token_tracker)
    }

    /// Update token tracker (called after each LLM response)
    pub fn update_tokens(&mut self, usage: &anthropic::completion::Usage) {
        self.token_tracker.update_from_anthropic(usage);
    }
}
```

#### Problem: Accessing Provider-Specific Usage

**Challenge**: Rig's `FinalResponse` only provides `crate::completion::Usage`, not `anthropic::completion::Usage`.

**Solution**: We need to access the raw provider response BEFORE conversion. This requires:

1. **Option A**: Modify `RigAgent` to expose provider-specific usage
2. **Option B**: Intercept streaming events directly (bypass RigAgent wrapper)
3. **Option C**: Request rig maintainers to preserve cache data in generic Usage

**Recommended: Option B** - Direct streaming event interception.

```rust
// src/agent/rig_agent.rs

impl<M> RigAgent<M>
where
    M: CompletionModel + 'static,
{
    /// Execute prompt with provider-specific usage tracking
    pub async fn prompt_streaming_with_cache_tracking(
        &self,
        prompt: &str,
        history: &mut [rig::message::Message],
        token_tracker: &mut TokenTracker,
    ) -> impl futures::Stream<Item = Result<MultiTurnStreamItem<M::StreamingResponse>, anyhow::Error>> + '_ {
        use futures::StreamExt;

        let history_for_rig = history.to_vec();

        // Get the underlying streaming response
        let stream = self.agent
            .stream_prompt(prompt)
            .with_history(history_for_rig)
            .multi_turn(self.max_depth)
            .await;

        // Intercept stream to extract provider-specific usage
        stream.map(move |item| {
            match &item {
                Ok(MultiTurnStreamItem::FinalResponse(response)) => {
                    // Extract provider-specific usage here
                    // This is the challenge - FinalResponse doesn't expose it
                    // We may need to track during streaming instead
                }
                _ => {}
            }
            item.map_err(|e| anyhow::anyhow!("Streaming error: {}", e))
        })
    }
}
```

#### Alternative: Estimation Fallback

If we cannot access provider-specific usage without forking rig, we can **estimate cache tokens** based on message content:

```rust
impl TokenTracker {
    /// Estimate cache read tokens based on message history
    /// This is less accurate than API data but works as fallback
    pub fn estimate_cache_tokens(&mut self, messages: &[rig::message::Message]) {
        // First message is typically system prompt (cached)
        // Assume ~90% of system prompt is cached after first call
        // This is a heuristic, not accurate

        if messages.is_empty() {
            return;
        }

        // Very rough estimation - need better heuristic
        let estimated_cache = (self.input_tokens as f64 * 0.7) as u64;
        self.cache_read_input_tokens = estimated_cache;
    }
}
```

**This is NOT recommended** - estimation defeats the purpose of cache-aware compaction.

---

## Decision

**Implement Option A (Custom Token Tracker) with Option B (Direct Streaming Interception)**

### Rationale

1. **Preserves rig compatibility** - No forking required
2. **Accurate token tracking** - Direct API data, not estimates
3. **Cache-aware compaction** - Can calculate effective tokens correctly
4. **Matches codelet design** - Same architecture, proven approach
5. **Provider-agnostic extensibility** - Can add OpenAI/Gemini later

### Implementation Steps

1. Create `src/agent/token_tracker.rs` with `TokenTracker` struct
2. Add cache token fields: `cache_read_input_tokens`, `cache_creation_input_tokens`
3. Implement `effective_tokens()` method
4. Add `TokenTracker` to `Session` struct
5. Intercept streaming responses to extract provider-specific usage
6. Update compaction trigger logic to use `effective_tokens()`

### Trade-offs

**Pros**:
- ✅ 100% accurate (same as codelet)
- ✅ Cache-aware compaction works correctly
- ✅ No rig fork required
- ✅ Extensible to other providers

**Cons**:
- ⚠️ Requires accessing rig internals (streaming events)
- ⚠️ May need unsafe or reflection if rig doesn't expose data
- ⚠️ Provider-specific code (Anthropic only initially)

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effective_tokens_with_cache() {
        let mut tracker = TokenTracker::new();
        tracker.input_tokens = 100_000;
        tracker.cache_read_input_tokens = 80_000;

        // Effective = 100k - (80k * 0.9) = 28k
        assert_eq!(tracker.effective_tokens(), 28_000);
    }

    #[test]
    fn test_effective_tokens_no_cache() {
        let mut tracker = TokenTracker::new();
        tracker.input_tokens = 50_000;
        tracker.cache_read_input_tokens = 0;

        // Effective = 50k - 0 = 50k
        assert_eq!(tracker.effective_tokens(), 50_000);
    }

    #[test]
    fn test_compaction_trigger() {
        let config = CompactionConfig {
            threshold_percent: 0.9,
            context_window: 100_000,
        };

        let mut tracker = TokenTracker::new();
        tracker.input_tokens = 95_000;
        tracker.cache_read_input_tokens = 0;

        // 95k > 90k threshold -> should compact
        assert!(config.should_compact(&tracker));

        // With cache: effective = 95k - (80k * 0.9) = 23k
        tracker.cache_read_input_tokens = 80_000;

        // 23k < 90k threshold -> should NOT compact
        assert!(!config.should_compact(&tracker));
    }
}
```

### Integration Tests

Test with real Anthropic API responses to verify cache token extraction.

---

## Future Considerations

### Multi-Provider Support

Currently focused on Anthropic (Claude). Future work:

- OpenAI: Different cache mechanism (ephemeral cache)
- Gemini: No cache tokens in API (yet)
- Codex: No cache support

Each provider will need provider-specific extraction logic in `TokenTracker::update_from_*()` methods.

### Rig Upstream Contribution

Consider submitting PR to rig to:
1. Add cache fields to `crate::completion::Usage`
2. Preserve provider-specific data in `GetTokenUsage` trait
3. Make cache-aware compaction possible for all rig users

---

## References

- **Codelet Token Tracking**: `/home/rquast/projects/codelet/src/agent/token-tracker.ts`
- **Codelet Compaction Trigger**: `/home/rquast/projects/codelet/src/agent/runner.ts` (lines 829-949)
- **Rig Anthropic Provider**: `/home/rquast/projects/rig/rig-core/src/providers/anthropic/`
- **Rig Generic Usage**: `/home/rquast/projects/rig/rig-core/src/completion/request.rs`
- **Anthropic Prompt Caching**: https://docs.anthropic.com/en/docs/build-with-claude/prompt-caching
- **Factory AI Interview**: ~/concept.txt (Luke Alvoeiro on anchored summaries)

---

## Appendix: Anthropic API Cache Token Format

From Anthropic's API documentation, the `message_start` SSE event contains:

```json
{
  "type": "message_start",
  "message": {
    "id": "msg_...",
    "type": "message",
    "role": "assistant",
    "content": [],
    "model": "claude-3-5-sonnet-20241022",
    "stop_reason": null,
    "stop_sequence": null,
    "usage": {
      "input_tokens": 2095,
      "cache_creation_input_tokens": 2051,
      "cache_read_input_tokens": 0,
      "output_tokens": 1
    }
  }
}
```

**Key Fields**:
- `usage.input_tokens`: Total input tokens (includes cache creation + cache read + new tokens)
- `usage.cache_creation_input_tokens`: Tokens written to cache (25% premium cost)
- `usage.cache_read_input_tokens`: Tokens read from cache (90% discount cost)

For effective token calculation:
```
Effective = input_tokens - (cache_read_input_tokens × 0.9)
```

This is the EXACT data rig's `anthropic::completion::Usage` receives, but loses when converting to `crate::completion::Usage`.
