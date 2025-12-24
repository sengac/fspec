# Context Compaction Limits: OpenCode vs fspec/codelet

This document analyzes OpenCode's sophisticated context compaction triggering algorithm and proposes implementing the same approach in fspec/codelet to improve compaction accuracy and reduce unnecessary triggers.

## Current State Analysis

### fspec/codelet Current Implementation

**Location**: `codelet/cli/src/compaction_threshold.rs`

```rust
pub const COMPACTION_THRESHOLD_RATIO: f64 = 0.9;

pub fn calculate_compaction_threshold(context_window: u64) -> u64 {
    (context_window as f64 * COMPACTION_THRESHOLD_RATIO) as u64
}
```

**Algorithm**: Simple percentage-based
- **Trigger**: `context_window * 0.9`
- **Example**: 200k context → triggers at 180k tokens
- **Pros**: Simple, predictable
- **Cons**: Doesn't account for output token reservation, may trigger too early

### OpenCode's Implementation

**Location**: `/tmp/opencode/packages/opencode/src/session/compaction.ts:30-38`

```typescript
export function isOverflow(input: { tokens: MessageV2.Assistant["tokens"]; model: Provider.Model }) {
  if (Flag.OPENCODE_DISABLE_AUTOCOMPACT) return false
  const context = input.model.limit.context
  if (context === 0) return false
  const count = input.tokens.input + input.tokens.cache.read + input.tokens.output
  const output = Math.min(input.model.limit.output, SessionPrompt.OUTPUT_TOKEN_MAX) || SessionPrompt.OUTPUT_TOKEN_MAX
  const usable = context - output
  return count > usable
}
```

**Algorithm**: Usable context calculation
- **Total tokens**: `input_tokens + cache_read_tokens + output_tokens`
- **Output limit**: `min(model_max_output, SESSION_OUTPUT_MAX)` where `SESSION_OUTPUT_MAX = 32,000`
- **Usable context**: `context_window - output_limit`
- **Trigger**: `total_tokens > usable_context`

## Detailed Calculation Comparison

### Example: Claude Sonnet 3.5 (200k context, 8k max output)

#### fspec/codelet Current
```rust
let threshold = 200_000 * 0.9;  // = 180,000 tokens
// Triggers when total tokens > 180,000
```

#### OpenCode Algorithm
```typescript
const context = 200_000;  // Context window
const output = Math.min(8_192, 32_000);  // = 8,192 (model limit is smaller)
const usable = 200_000 - 8_192;  // = 191,808 tokens
// Triggers when (input + cache + output) > 191,808
```

**Result**: OpenCode triggers ~12k tokens later (191.8k vs 180k), making better use of available context.

### Example: GPT-4 (128k context, 4k max output)

#### fspec/codelet Current
```rust
let threshold = 128_000 * 0.9;  // = 115,200 tokens
```

#### OpenCode Algorithm
```typescript
const context = 128_000;
const output = Math.min(4_096, 32_000);  // = 4,096
const usable = 128_000 - 4_096;  // = 123,904 tokens
// Triggers when total > 123,904
```

**Result**: OpenCode triggers ~8.7k tokens later (123.9k vs 115.2k).

### Example: High-Output Model (200k context, 64k max output)

#### fspec/codelet Current
```rust
let threshold = 200_000 * 0.9;  // = 180,000 tokens
```

#### OpenCode Algorithm
```typescript
const context = 200_000;
const output = Math.min(64_000, 32_000);  // = 32,000 (session limit applies)
const usable = 200_000 - 32_000;  // = 168,000 tokens
// Triggers when total > 168,000
```

**Result**: OpenCode triggers 12k tokens EARLIER (168k vs 180k) due to high output requirements.

## Key Insights

### OpenCode's Superior Approach

1. **Dynamic Output Reservation**: Reserves space based on actual model capabilities
2. **Session Output Limits**: Caps output reservation at 32k to prevent over-reservation
3. **Real-time Token Tracking**: Uses actual accumulated tokens (input + cache + output)
4. **Context-Aware**: Adapts to different model characteristics automatically

### fspec/codelet's Limitations

1. **Static Percentage**: Always 90% regardless of model output capabilities
2. **No Output Planning**: Doesn't reserve space for responses
3. **Potential Waste**: May trigger compaction when plenty of usable space remains
4. **Model-Agnostic**: Same threshold for all models regardless of output limits

## Implementation Plan for fspec/codelet

### 1. New Token Tracking Structure

```rust
// In codelet/core/src/session/tokens.rs
#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub input: u64,
    pub cache_read: u64,
    pub output: u64,
}

impl TokenUsage {
    pub fn total(&self) -> u64 {
        self.input + self.cache_read + self.output
    }
}
```

### 2. Enhanced Provider Interface

```rust
// In codelet/providers/src/lib.rs
pub trait ProviderLimits {
    fn context_window(&self) -> usize;
    fn max_output_tokens(&self) -> usize;  // New method
}
```

### 3. OpenCode-Style Compaction Check

```rust
// In codelet/cli/src/compaction_threshold.rs

/// Session output token maximum (matches OpenCode)
pub const SESSION_OUTPUT_TOKEN_MAX: u64 = 32_000;

/// Check if compaction should trigger using OpenCode algorithm
pub fn should_trigger_compaction(
    token_usage: &TokenUsage,
    context_window: u64,
    model_max_output: u64,
) -> bool {
    // Calculate effective output limit (match OpenCode logic)
    let output_limit = model_max_output.min(SESSION_OUTPUT_TOKEN_MAX);
    
    // Calculate usable context (reserve space for output)
    let usable_context = context_window.saturating_sub(output_limit);
    
    // Check if current usage exceeds usable context
    token_usage.total() > usable_context
}

/// Legacy function for backward compatibility
#[deprecated(note = "Use should_trigger_compaction instead")]
pub fn calculate_compaction_threshold(context_window: u64) -> u64 {
    (context_window as f64 * COMPACTION_THRESHOLD_RATIO) as u64
}
```

### 4. Provider Updates

```rust
// In codelet/providers/src/claude.rs
impl ProviderLimits for ClaudeProvider {
    fn context_window(&self) -> usize { 200_000 }
    fn max_output_tokens(&self) -> usize { 8_192 }  // Claude's actual limit
}

// In codelet/providers/src/openai.rs  
impl ProviderLimits for OpenAIProvider {
    fn context_window(&self) -> usize { 128_000 }
    fn max_output_tokens(&self) -> usize { 4_096 }  // GPT-4's limit
}
```

### 5. Integration Points

```rust
// In codelet/cli/src/interactive/stream_loop.rs
use crate::compaction_threshold::should_trigger_compaction;

// Replace current threshold check with:
if should_trigger_compaction(&token_usage, context_window, provider.max_output_tokens()) {
    // Trigger compaction
}
```

## Migration Strategy

### Phase 1: Add New API (Non-Breaking)
- Add `should_trigger_compaction()` function
- Add `max_output_tokens()` to provider trait
- Keep existing `calculate_compaction_threshold()` with deprecation warning

### Phase 2: Update Callers
- Replace threshold checks in stream loop
- Update token tracking to use `TokenUsage` struct
- Add comprehensive tests comparing old vs new behavior

### Phase 3: Remove Legacy
- Remove deprecated `calculate_compaction_threshold()`
- Clean up old percentage-based constants
- Update documentation

## Benefits of OpenCode Algorithm

### More Accurate Triggering
- **Claude 3.5**: Gains 11.8k tokens (191.8k vs 180k trigger)
- **GPT-4**: Gains 8.7k tokens (123.9k vs 115.2k trigger) 
- **High-output models**: Appropriately reserves space for large responses

### Model-Specific Optimization
- Adapts automatically to each model's output characteristics
- No manual tuning needed for new models
- Consistent behavior across different providers

### Real-World Usage Patterns
- Accounts for actual conversation flow (input → thinking → output)
- Prevents compaction when plenty of space remains for responses
- Reduces unnecessary compaction cycles

## Test Cases for Validation

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_compaction_vs_opencode() {
        let usage = TokenUsage { input: 190_000, cache_read: 0, output: 1_000 };
        let should_compact = should_trigger_compaction(&usage, 200_000, 8_192);
        // OpenCode: 191,000 > (200,000 - 8,192) = false
        assert!(!should_compact);
        
        // But legacy would trigger: 191,000 > 180,000 = true
        let legacy_threshold = calculate_compaction_threshold(200_000);
        assert!(usage.total() > legacy_threshold);
    }
    
    #[test]
    fn test_high_output_model_compaction() {
        let usage = TokenUsage { input: 170_000, cache_read: 0, output: 1_000 };
        let should_compact = should_trigger_compaction(&usage, 200_000, 64_000);
        // OpenCode: 171,000 > (200,000 - 32,000) = true (session limit applies)
        assert!(should_compact);
    }
}
```

## Conclusion

OpenCode's compaction algorithm is significantly more sophisticated and accurate than fspec/codelet's current percentage-based approach. By implementing the same algorithm, we can:

1. **Reduce unnecessary compaction** by 6-12k tokens in typical scenarios
2. **Improve model compatibility** through dynamic output reservation  
3. **Match industry best practices** used by OpenCode's production system
4. **Maintain backward compatibility** during migration

The implementation should be straightforward and can be done incrementally without breaking existing functionality.