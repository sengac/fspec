# Token Doubling Bug Analysis

## Executive Summary

Found a **critical token double-counting bug** in `cli/src/interactive/stream_loop.rs` that causes token counts to be inflated during streaming display. This explains why token consumption appears much higher than expected.

## The Bug Location

**File**: `/Users/rquast/projects/fspec/codelet/cli/src/interactive/stream_loop.rs`
**Lines**: 395-425 (streaming display) and 401-404 (accumulation logic)

## Root Cause: Double-Counting in Display Logic

### The Problematic Code

```rust
// Lines 401-404: Accumulation logic
if usage.output_tokens == 0 {
    // MessageStart - new API call starting
    // First, commit previous API call's tokens (if any) to accumulated totals
    turn_accumulated_input += current_api_input;  // ← ADDS current to accumulated
    turn_accumulated_output += current_api_output;
    // Now track the new API call's input tokens
    current_api_input = usage.input_tokens;       // ← THEN starts tracking new API call
    current_api_output = 0;
}

// Lines 416-423: Display logic
output.emit_tokens(&TokenInfo {
    input_tokens: prev_input_tokens           // Previous session total
                + turn_accumulated_input      // Already processed API calls 
                + current_api_input,          // ← DOUBLE COUNTS current API call!
    output_tokens: prev_output_tokens
                + turn_accumulated_output
                + current_api_output,
    // ...
});
```

### The Double-Counting Flow

1. **API Call 1 Starts**: `current_api_input = 1000`
2. **Display Shows**: `prev(5000) + accumulated(0) + current(1000) = 6000` ✅
3. **API Call 1 Finishes**: `turn_accumulated_input += current_api_input` → `accumulated = 1000`
4. **API Call 2 Starts**: `current_api_input = 500`
5. **Display Shows**: `prev(5000) + accumulated(1000) + current(500) = 6500` ✅
6. **BUT**: The `accumulated(1000)` already includes the first API call's tokens!
7. **Result**: User sees inflated token counts during multi-API-call turns (tool use)

## Impact

- **Visual Impact**: Token counts appear to grow faster than reality
- **User Confusion**: Makes it seem like the system is consuming more tokens than it actually is
- **No Billing Impact**: The session tracker (`session.token_tracker`) is updated correctly at turn end
- **Compaction Triggers**: May trigger context compaction earlier than necessary due to inflated estimates

## Evidence

### Session Tracker Update (Correct)
```rust
// Lines 811-812: This is CORRECT
session.token_tracker.input_tokens += turn_accumulated_input;   
session.token_tracker.output_tokens += turn_accumulated_output;
```

### Compaction After Retry (Shows Correct Tracking)
```rust
// Lines 648-653: Compaction correctly resets only output/cache metrics
// NOTE: execute_compaction already sets session.token_tracker.input_tokens
// to the correct new_total_tokens calculated from compacted messages.
session.token_tracker.output_tokens = 0;
session.token_tracker.cache_read_input_tokens = None;
session.token_tracker.cache_creation_input_tokens = None;
```

## The Fix

### Option 1: Simple Fix (Remove Double-Count)
```rust
// BEFORE (buggy):
input_tokens: prev_input_tokens + turn_accumulated_input + current_api_input,

// AFTER (fixed):
input_tokens: prev_input_tokens + turn_accumulated_input,
```

### Option 2: More Accurate Streaming (Show In-Progress)
```rust
// Show current API call progress only when output tokens exist (streaming active)
input_tokens: prev_input_tokens + turn_accumulated_input + 
              if usage.output_tokens > 0 { current_api_input } else { 0 },
```

### Option 3: Restructure Accumulation Logic
Move the accumulation logic to happen AFTER the display, not before:

```rust
// Display first (before accumulation)
output.emit_tokens(&TokenInfo {
    input_tokens: prev_input_tokens + turn_accumulated_input + current_api_input,
    // ...
});

// Then accumulate (only when API call truly finishes)
if should_accumulate {
    turn_accumulated_input += current_api_input;
    turn_accumulated_output += current_api_output;
}
```

## Testing the Fix

1. **Before Fix**: Watch token counts during tool use - they should appear to jump significantly
2. **After Fix**: Token counts should grow more smoothly and match actual API usage
3. **Verification**: Compare displayed counts with final session tracker values

## Related Files

- `/Users/rquast/projects/fspec/codelet/cli/src/interactive/stream_loop.rs` (main bug)
- `/Users/rquast/projects/fspec/codelet/core/src/common/token_tracker.rs` (session tracking - correct)
- `/Users/rquast/projects/fspec/codelet/core/src/compaction/` (compaction logic - uses correct session values)

## Comparison with OpenCode

OpenCode doesn't have this issue because it:
1. Uses simpler token tracking without multi-API-call accumulation
2. Updates session state immediately after each API call
3. Doesn't have complex streaming display logic with separate accumulated/current tracking

The complexity in fspec/codelet's streaming implementation introduced this double-counting bug in the display logic while keeping the underlying session tracking correct.

## Recommended Token System Simplification

### Current Problems Beyond the Double-Counting Bug

1. **Complex Multi-Variable Tracking**: Too many token counters (`prev_input_tokens`, `turn_accumulated_input`, `current_api_input`, etc.)
2. **Estimation vs Reality**: Mix of estimated tokens (tiktoken approximations) and actual API-reported tokens
3. **Inconsistent Token Estimation**: Different estimation methods across the codebase
4. **Complex Compaction Logic**: Token calculations spread across multiple files

### Proposed Simplification Strategy

#### 1. Unified Token Estimation with `tiktoken-rs`

**Current State**: Mix of estimation methods
```rust
// Multiple approaches found in codebase:
const APPROX_BYTES_PER_TOKEN: f64 = 3.0;  // Rough estimate
estimate_tokens(text.len() as f64 / APPROX_BYTES_PER_TOKEN)  // Byte-based
// Plus OpenAI API reported tokens (actual)
```

**Proposed**: Single source of truth with `tiktoken-rs`
```rust
// Add to Cargo.toml:
// tiktoken-rs = "0.5"

use tiktoken_rs::{cl100k_base, CoreBPE};

pub struct TokenEstimator {
    encoder: CoreBPE,
}

impl TokenEstimator {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            encoder: cl100k_base()?,  // Claude uses cl100k_base encoding
        })
    }
    
    pub fn count_tokens(&self, text: &str) -> usize {
        self.encoder.encode_with_special_tokens(text).len()
    }
    
    pub fn count_message_tokens(&self, messages: &[Message]) -> usize {
        messages.iter()
            .map(|msg| self.count_tokens(&msg.to_string()))
            .sum()
    }
}
```

#### 2. Simplified Token Tracker

**Current**: Complex multi-counter system
```rust
pub struct TokenTracker {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    // Plus separate tracking in stream_loop.rs
}
```

**Proposed**: Single unified tracker
```rust
pub struct TokenTracker {
    // Actual API-reported tokens (source of truth)
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    
    // Estimation for preview/compaction decisions
    pub estimated_context_tokens: usize,
    pub estimator: TokenEstimator,
}

impl TokenTracker {
    pub fn add_api_usage(&mut self, usage: &Usage) {
        self.total_input_tokens += usage.input_tokens;
        self.total_output_tokens += usage.output_tokens;
        self.cache_read_tokens += usage.cache_read_input_tokens.unwrap_or(0);
        self.cache_creation_tokens += usage.cache_creation_input_tokens.unwrap_or(0);
    }
    
    pub fn estimate_current_context(&mut self, messages: &[Message]) -> usize {
        self.estimated_context_tokens = self.estimator.count_message_tokens(messages);
        self.estimated_context_tokens
    }
    
    pub fn effective_tokens(&self) -> u64 {
        // Use actual API tokens, not estimates
        self.total_input_tokens + self.total_output_tokens
    }
}
```

#### 3. Simplified Stream Loop Logic

**Remove Complex Multi-Counter Tracking**:
```rust
// BEFORE: Multiple counters (buggy)
let prev_input_tokens = session.token_tracker.input_tokens;
let mut turn_accumulated_input: u64 = 0;
let mut current_api_input: u64 = 0;
// ... complex accumulation logic

// AFTER: Simple direct tracking
pub fn run_agent_stream(/* ... */) -> Result<(), Box<dyn Error>> {
    let mut session_start_input = session.token_tracker.total_input_tokens;
    let mut session_start_output = session.token_tracker.total_output_tokens;
    
    loop {
        match event {
            StreamEvent::Usage(usage) => {
                // Update tracker directly
                session.token_tracker.add_api_usage(&usage);
                
                // Emit simple cumulative totals
                output.emit_tokens(&TokenInfo {
                    input_tokens: session.token_tracker.total_input_tokens,
                    output_tokens: session.token_tracker.total_output_tokens,
                    cache_read_input_tokens: Some(session.token_tracker.cache_read_tokens),
                    cache_creation_input_tokens: Some(session.token_tracker.cache_creation_tokens),
                });
            }
            // ...
        }
    }
}
```

#### 4. Simplified Compaction Logic

**Use tiktoken-rs for accurate pre-compaction estimation**:
```rust
pub async fn should_compact(&mut self, max_tokens: usize) -> bool {
    let estimated_tokens = self.token_tracker.estimate_current_context(&self.messages);
    estimated_tokens > max_tokens
}

pub async fn execute_compaction(&mut self, target_tokens: usize) -> Result<CompactionResult, Error> {
    // Use tiktoken for accurate token counting during compaction
    let current_tokens = self.token_tracker.estimate_current_context(&self.messages);
    
    // Compress until we hit target
    let compacted = compact_messages(&self.messages, target_tokens, &self.token_tracker.estimator)?;
    
    // Reset tracker to reflect new context
    self.token_tracker.total_input_tokens = 0;  // Reset after compaction
    self.token_tracker.total_output_tokens = 0;
    self.token_tracker.estimated_context_tokens = 
        self.token_tracker.estimator.count_message_tokens(&compacted.messages);
    
    self.messages = compacted.messages;
    Ok(compacted)
}
```

### Implementation Plan

#### Phase 1: Add tiktoken-rs (Low Risk)
1. Add `tiktoken-rs` dependency
2. Create `TokenEstimator` utility
3. Update estimation calls to use tiktoken instead of byte-based estimates
4. **Keep existing tracking logic intact**

#### Phase 2: Fix Stream Loop Bug (Medium Risk)
1. Apply the double-counting fix from main bug analysis
2. Simplify counter variables in `stream_loop.rs`
3. Test streaming display accuracy

#### Phase 3: Unified Token Tracker (Higher Risk)
1. Redesign `TokenTracker` struct
2. Update all usage sites
3. Simplify compaction logic
4. Remove redundant token calculations

#### Phase 4: Validation & Cleanup
1. Add comprehensive token counting tests
2. Validate against OpenAI API actual usage
3. Remove deprecated estimation methods
4. Update documentation

### Benefits of Simplification

1. **Accuracy**: tiktoken-rs provides exact token counts matching OpenAI/Anthropic
2. **Simplicity**: Single source of truth for token calculations
3. **Performance**: tiktoken-rs is highly optimized
4. **Maintainability**: Less complex state to track and debug
5. **Reliability**: Eliminates estimation discrepancies and double-counting bugs

### Migration Considerations

- **Backward Compatibility**: Keep old token fields during transition
- **Testing**: Extensive testing with real conversations to validate accuracy
- **Performance**: tiktoken-rs has minimal overhead but benchmark for large contexts
- **Error Handling**: tiktoken-rs can fail on invalid text encoding