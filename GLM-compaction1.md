# GLM Compaction System Analysis

**Date**: 2025-01-19
**Subject**: Analysis of compaction limit calculations and potential issues in the codelet compaction system

---

## Executive Summary

After analyzing the codebase for the compaction system, I found **several potential issues** with how compaction limits are calculated and how token counting is performed. These issues may cause:

- Missed compaction triggers leading to "prompt too long" API errors
- Premature compaction reducing session effectiveness
- Inconsistent token counting between different parts of the system
- Potential double-counting of cache tokens

---

## Part 1: Provider-Specific Compaction Constants

### Provider Context Windows and Max Output Tokens

All providers have hardcoded constants for their context windows and maximum output tokens:

| Provider | Context Window | Max Output | File | Lines |
|----------|----------------|------------|------|-------|
| Claude   | 200,000       | 8,192      | `providers/src/claude.rs` | 33, 36 |
| OpenAI   | 128,000       | 4,096      | `providers/src/openai.rs` | 23, 26 |
| Gemini   | 1,000,000     | 8,192      | `providers/src/gemini.rs` | 26, 29 |
| Z.AI     | 128,000       | 8,192      | `providers/src/zai.rs` | 33, 36 |
| Codex    | 272,000       | 4,096      | `providers/src/codex/mod.rs` | 24, 27 |

### How Providers Are Accessed

The `ProviderManager` (line 430-456 in `manager.rs`) provides methods to get these values:

```rust
/// Get context window size for the current provider
pub fn context_window(&self) -> usize {
    match self.current_provider {
        ProviderType::Claude => claude::CONTEXT_WINDOW,
        ProviderType::OpenAI => openai::CONTEXT_WINDOW,
        ProviderType::Gemini => gemini::CONTEXT_WINDOW,
        ProviderType::Codex => codex::CONTEXT_WINDOW,
        ProviderType::ZAI => zai::CONTEXT_WINDOW,
    }
}

/// Get max output tokens for the current provider (CTX-002)
pub fn max_output_tokens(&self) -> usize {
    match self.current_provider {
        ProviderType::Claude => claude::MAX_OUTPUT_TOKENS,
        ProviderType::OpenAI => openai::MAX_OUTPUT_TOKENS,
        ProviderType::Gemini => gemini::MAX_OUTPUT_TOKENS,
        ProviderType::Codex => codex::MAX_OUTPUT_TOKENS,
        ProviderType::ZAI => zai::MAX_OUTPUT_TOKENS,
    }
}
```

### Compaction Threshold Calculations

The compaction threshold is calculated per provider:

```rust
// From compaction_threshold.rs:112-120
pub fn calculate_usable_context(context_window: u64, model_max_output: u64) -> u64 {
    let output_reservation = model_max_output.min(SESSION_OUTPUT_TOKEN_MAX);
    let output_reservation = if output_reservation == 0 {
        SESSION_OUTPUT_TOKEN_MAX
    } else {
        output_reservation
    };
    context_window.saturating_sub(output_reservation)
}
```

Where `SESSION_OUTPUT_TOKEN_MAX = 32_000` (cap for high-output models).

**Formula**:
```
output_reservation = min(model_max_output, 32,000)
threshold = context_window - output_reservation
```

**Calculated Thresholds**:

| Provider | Calculation | Threshold |
|----------|-------------|------------|
| Claude   | 200,000 - min(8,192, 32,000) | **191,808** |
| OpenAI   | 128,000 - min(4,096, 32,000) | **123,904** |
| Gemini   | 1,000,000 - min(8,192, 32,000) | **991,808** |
| Z.AI     | 128,000 - min(8,192, 32,000) | **119,808** |
| Codex    | 272,000 - min(4,096, 32,000) | **267,904** |

### Compaction Budget (Target After Compaction)

The compaction budget is calculated as:

```rust
// From compaction_threshold.rs:58-64
pub fn calculate_summarization_budget(context_window: u64) -> u64 {
    if context_window <= AUTOCOMPACT_BUFFER {
        (context_window as f64 * 0.8) as u64
    } else {
        context_window - AUTOCOMPACT_BUFFER
    }
}
```

Where `AUTOCOMPACT_BUFFER = 50_000`.

**Formula**:
```
if context_window <= 50,000:
    budget = context_window * 0.8
else:
    budget = context_window - 50,000
```

**Calculated Budgets**:

| Provider | Calculation | Budget |
|----------|-------------|---------|
| Claude   | 200,000 - 50,000 | **150,000** |
| OpenAI   | 128,000 - 50,000 | **78,000** |
| Gemini   | 1,000,000 - 50,000 | **950,000** |
| Z.AI     | 128,000 - 50,000 | **78,000** |
| Codex    | 272,000 - 50,000 | **222,000** |

---

## Part 2: Token Counting Inconsistencies

### Issue #1: Comment vs Code Mismatch in Pre-Prompt Estimation

**File**: `cli/src/interactive/stream_loop.rs`

**Location**: Lines 385-312

**The Problem**:

```rust
// Line 385: Comment says input_tokens ALREADY includes cache
// PROV-001: session.token_tracker.input_tokens stores TOTAL context (input + cache_read + cache_creation)
let token_state = Arc::new(Mutex::new(TokenState {
    input_tokens: session.token_tracker.input_tokens, // Already includes cache
    cache_read_input_tokens: 0,                       // Don't double count
    cache_creation_input_tokens: 0,                   // Don't double count
    output_tokens: session.token_tracker.output_tokens,
    compaction_needed: false,
}));
```

```rust
// Line 311: Pre-prompt estimation EXCLUDES cache tokens
let prompt_tokens = count_tokens(prompt) as u64;
let current_tokens = session.token_tracker.input_tokens + session.token_tracker.output_tokens;
let estimated_total = current_tokens + prompt_tokens;
```

**Inconsistency**:

- The comment at line 385 claims `session.token_tracker.input_tokens` **already includes cache tokens** (`cache_read_input_tokens` + `cache_creation_input_tokens`)
- However, the pre-prompt check at line 311 **does not include cache tokens** - it only adds `input_tokens` + `output_tokens`
- **If the comment is correct**: The pre-prompt check is undercounting total context by missing cache tokens
- **If the comment is wrong**: The `TokenState` initialization is missing cache tokens, causing undercounting in the hook

**Impact**:
- Pre-prompt check may fail to trigger compaction when it should
- User could encounter "prompt is too long" API errors after resuming a session with high cache token usage

---

### Issue #2: Missing Cache Creation Tokens in TokenUsage::total()

**File**: `cli/src/compaction_threshold.rs`

**Location**: Lines 92-96

**Current Implementation**:

```rust
impl TokenUsage {
    /// Calculate total token count (simple sum, no discounting)
    ///
    /// Algorithm: count = input + cache_read + output
    pub fn total(&self) -> u64 {
        self.input_tokens + self.cache_read_tokens + self.output_tokens
    }
}
```

**The Issue**: `cache_creation_tokens` is **completely missing** from the total!

Compare with other total calculations:

#### TokenState::total() (compaction_hook.rs:65-67)
```rust
#[inline]
pub fn total(&self) -> u64 {
    self.as_usage().total_context()
}
```

This calls `ApiTokenUsage::total_context()` which **does include** cache creation:

#### ApiTokenUsage::total_context() (core/src/token_usage.rs:59-61)
```rust
#[inline]
pub fn total_context(&self) -> u64 {
    self.total_input() + self.output_tokens
}
```

Where `total_input()` (lines 53-55):

```rust
#[inline]
pub fn total_input(&self) -> u64 {
    self.input_tokens + self.cache_read_input_tokens + self.cache_creation_input_tokens
}
```

**Comparison Table**:

| Calculation | Input | Cache Read | Cache Creation | Output |
|-------------|--------|------------|----------------|---------|
| `TokenUsage::total()` | ✓ | ✓ | **✗** | ✓ |
| `TokenState::total()` | ✓ | ✓ | ✓ | ✓ |
| `ApiTokenUsage::total_context()` | ✓ | ✓ | ✓ | ✓ |

**Impact**:
- `TokenUsage::total()` **undercounts** by `cache_creation_input_tokens`
- This is used by `should_trigger_compaction()` in `compaction_threshold.rs`
- If cache creation is significant (e.g., first API call with caching enabled), compaction may trigger later than it should

---

### Issue #3: Pre-Prompt Estimation Doesn't Match Hook Logic

**The Flow**:

#### 1. Pre-Prompt Check (stream_loop.rs:310-314)
```rust
let prompt_tokens = count_tokens(prompt) as u64;
let current_tokens = session.token_tracker.input_tokens + session.token_tracker.output_tokens;
let estimated_total = current_tokens + prompt_tokens;

if estimated_total > threshold && !session.messages.is_empty() {
    info!(
        "[CTX-005] Pre-prompt compaction triggered: estimated {} > threshold {}",
        estimated_total, threshold
    );
    // ... execute compaction
}
```

**Formula used**: `total = input_tokens + output_tokens`
- Excludes `cache_read_input_tokens`
- Excludes `cache_creation_input_tokens`

#### 2. CompactionHook Check (compaction_hook.rs:161-198)
```rust
async fn on_completion_call(
    &self,
    prompt: &Message,
    history: &[Message],
    cancel_sig: CancelSignal,
) {
    let Ok(mut state) = self.state.lock() else {
        return;
    };

    // Estimate tokens from actual payload being sent
    let mut all_messages = history.to_vec();
    all_messages.push(prompt.clone());
    let estimated_payload = estimate_messages_tokens(&all_messages) as u64;

    // Use MAX of last known API tokens vs estimated payload
    let last_known_total = state.total();
    let effective_total = last_known_total.max(estimated_payload);

    if effective_total > self.threshold {
        state.compaction_needed = true;
        tracing::info!(
            "Compaction triggered: {} tokens > {} threshold",
            effective_total,
            self.threshold
        );
        cancel_sig.cancel();
    }
}
```

**Formula used**: `total = TokenState::total()`
- Includes `input_tokens`
- Includes `cache_read_input_tokens`
- Includes `cache_creation_input_tokens`
- Includes `output_tokens`

**The Inconsistency**:

- Pre-prompt check: `total = input + output` (missing cache)
- Hook check: `total = input + cache_read + cache_creation + output`

**Example Scenario**:

```
session.token_tracker.input_tokens = 150,000
session.token_tracker.cache_read_input_tokens = 20,000
session.token_tracker.output_tokens = 5,000
prompt_tokens = 10,000
threshold = 191,808 (Claude)
```

**Pre-prompt check**:
```
current_tokens = 150,000 + 5,000 = 155,000
estimated_total = 155,000 + 10,000 = 165,000
165,000 < 191,808 → NO compaction
```

**Hook check** (if it ran with same values):
```
last_known_total = 150,000 + 20,000 + 5,000 = 175,000
effective_total = max(175,000, estimated) ≈ 175,000
175,000 < 191,808 → NO compaction (in this case)
```

But if cache tokens were higher:

```
session.token_tracker.cache_read_input_tokens = 40,000
```

**Pre-prompt check**: `165,000 < 191,808` → **NO compaction**
**Hook check**: `195,000 > 191,808` → **YES compaction**

**Result**: The pre-prompt check might fail to catch cases where the hook would trigger. This defeats the purpose of CTX-005 (preventing "prompt too long" errors).

---

### Issue #4: TokenState Initialization Comment Contradiction

**File**: `cli/src/interactive/stream_loop.rs`

**Location**: Lines 385-395

```rust
// PROV-001: session.token_tracker.input_tokens stores TOTAL context (input + cache_read + cache_creation)
// Initialize cache values to 0 to avoid double-counting in TokenState::total()
// During streaming, on_stream_completion_response_finish will update with actual API values
let token_state = Arc::new(Mutex::new(TokenState {
    input_tokens: session.token_tracker.input_tokens, // Already includes cache
    cache_read_input_tokens: 0,                       // Don't double count
    cache_creation_input_tokens: 0,                   // Don't double count
    output_tokens: session.token_tracker.output_tokens,
    compaction_needed: false,
}));
```

**The Contradiction**:

If `session.token_tracker.input_tokens` already includes cache:
- Setting `cache_read_input_tokens = 0` is correct (avoid double-counting)
- Setting `cache_creation_input_tokens = 0` is correct (avoid double-counting)

But then `TokenState::total()` would be:
```
total = input (includes cache) + cache_read (0) + cache_creation (0) + output
```

**However**, look at how `TokenState` is updated from API responses:

```rust
// compaction_hook.rs:219-224
async fn on_stream_completion_response_finish(
    &self,
    _prompt: &Message,
    response: &<M as CompletionModel>::StreamingResponse,
    _cancel_sig: CancelSignal,
) {
    if let Some(usage) = response.token_usage() {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.input_tokens = usage.input_tokens;                        // Raw input (NOT including cache)
        state.cache_read_input_tokens = usage.cache_read_input_tokens.unwrap_or(0);
        state.cache_creation_input_tokens = usage.cache_creation_input_tokens.unwrap_or(0);
        state.output_tokens = usage.output_tokens;
    }
}
```

The API response provides:
- `input_tokens`: **Fresh tokens not from cache**
- `cache_read_input_tokens`: Tokens read from existing cache
- `cache_creation_input_tokens`: Tokens being written to new cache

These are **three disjoint sets** per Anthropic documentation.

**The Question**: Does `session.token_tracker.input_tokens` store:
- A. `input_tokens` only (raw input, no cache)
- B. `input_tokens + cache_read_input_tokens + cache_creation_input_tokens` (total context)

**Evidence for Option A**:

1. `TokenTracker::update()` in `core/src/compaction/model.rs:72-88`:
```rust
pub fn update(
    &mut self,
    input: u64,
    output: u64,
    cache_read: Option<u64>,
    cache_creation: Option<u64>,
) {
    // CTX-003: Overwrite current context (for display and threshold checks)
    self.input_tokens = input;
    self.output_tokens = output;
    // CTX-003: Accumulate for billing analytics
    self.cumulative_billed_input += input;
    self.cumulative_billed_output += output;
    // Cache tokens are per-request values
    self.cache_read_input_tokens = cache_read;
    self.cache_creation_input_tokens = cache_creation;
}
```

This overwrites `input_tokens` with the raw `input` value from the API.

2. Comment in `TokenTracker` (lines 25-27):
```rust
/// Current context input tokens (latest from API - overwritten, not accumulated)
/// CTX-003: This is what should be displayed to users and used for threshold checks
pub input_tokens: u64,
```

"Latest from API" suggests the raw value, not a sum.

**Evidence for Option B**:

1. Comment in `stream_loop.rs:385` (the one we're analyzing):
```rust
// PROV-001: session.token_tracker.input_tokens stores TOTAL context (input + cache_read + cache_creation)
```

This explicitly states it stores TOTAL context.

2. Pre-prompt check at line 311:
```rust
let current_tokens = session.token_tracker.input_tokens + session.token_tracker.output_tokens;
```

If `input_tokens` already includes cache, this makes sense (just input + output).
If `input_tokens` is raw only, this is missing cache tokens.

**The Contradiction**:

The API returns separate values, but the comment suggests they're being aggregated in `session.token_tracker.input_tokens`. However, the `update()` method doesn't show any aggregation logic.

**Impact**:

- If comment is **correct**: Pre-prompt estimation is wrong (missing cache), and TokenState initialization is double-counting
- If comment is **wrong**: Pre-prompt estimation is correct (raw input only), but then why does it exclude cache?

---

## Part 3: How TokenTracker is Updated

### Update Points in the Codebase

#### 1. After API Response (stream_loop.rs:908-911)
```rust
session.token_tracker.input_tokens = turn_usage.total_input();
session.token_tracker.output_tokens = turn_cumulative_output;
session.token_tracker.cache_read_input_tokens = Some(turn_usage.cache_read_input_tokens);
session.token_tracker.cache_creation_input_tokens = Some(turn_usage.cache_creation_input_tokens);
```

Where `turn_usage.total_input()` = `input + cache_read + cache_creation` (from `ApiTokenUsage`).

**Wait - this sets `input_tokens` to the TOTAL including cache!**

#### 2. After Continuation (stream_loop.rs:961-966)
```rust
session.token_tracker.input_tokens = continuation_usage.total_input();
session.token_tracker.output_tokens = continuation_cumulative_output;
session.token_tracker.cumulative_billed_input += continuation_usage.input_tokens;
session.token_tracker.cumulative_billed_output += continuation_usage.output_tokens;
session.token_tracker.cache_read_input_tokens = Some(continuation_usage.cache_read_input_tokens);
session.token_tracker.cache_creation_input_tokens = Some(continuation_usage.cache_creation_input_tokens);
```

Again, `input_tokens` is set to `total_input()` which includes cache.

#### 3. After Compaction (interactive_helpers.rs:247-248)
```rust
session.token_tracker.input_tokens = new_total_tokens;
session.token_tracker.output_tokens = 0;
```

Where `new_total_tokens` is calculated from message text:
```rust
let new_total_tokens: u64 = session
    .messages
    .iter()
    .map(|msg| {
        let text = extract_message_text(msg);
        count_tokens(&text) as u64
    })
    .sum();
```

This is an estimate, not API values.

---

## Part 4: The True State of TokenTracker

### Reconciling the Evidence

**Evidence that `input_tokens` includes cache**:

1. Line 908 in `stream_loop.rs`:
   ```rust
   session.token_tracker.input_tokens = turn_usage.total_input();
   ```
   Where `total_input()` = `input + cache_read + cache_creation`

2. Comment at line 385 explicitly states this.

**Evidence that `input_tokens` is raw only**:

1. `TokenTracker::update()` overwrites with raw `input` parameter.

2. API returns separate values.

**Resolution**:

The code shows that `session.token_tracker.input_tokens` **IS** being set to the total including cache (via `total_input()`), but the `TokenTracker::update()` method is **not being called directly** to update from API responses in the streaming loop.

Instead, the streaming loop manually updates each field:
```rust
session.token_tracker.input_tokens = turn_usage.total_input();
session.token_tracker.cache_read_input_tokens = Some(turn_usage.cache_read_input_tokens);
session.token_tracker.cache_creation_input_tokens = Some(turn_usage.cache_creation_input_tokens);
```

This creates a **redundant state**:
- `input_tokens` = total context (includes cache)
- `cache_read_input_tokens` = separate cache read value
- `cache_creation_input_tokens` = separate cache creation value

**This confirms the comment is correct**, but it creates confusion because:
- `input_tokens` already includes cache
- But we're also storing cache separately
- TokenState initializes cache to 0 to avoid double-counting
- But then why store them separately at all?

---

## Part 5: Summary of Issues

| Issue | Severity | File(s) | Problem | Impact |
|-------|----------|-----------|---------|--------|
| #1 | High | `stream_loop.rs:311` | Pre-prompt estimation excludes cache tokens | May fail to trigger compaction, causing "prompt too long" errors |
| #2 | Medium | `compaction_threshold.rs:92-96` | `TokenUsage::total()` missing `cache_creation` | Undercounts tokens, may delay compaction |
| #3 | High | Multiple files | Pre-prompt and hook use different total calculations | Inconsistent compaction triggers |
| #4 | Medium | `stream_loop.rs:385-395` | Redundant state: input includes cache, stored separately | Confusing architecture, potential for bugs |
| #5 | Low | `compaction/model.rs:54-58` | `TokenTracker::effective_tokens()` uses cache discount | This is for billing/display, not compaction (correct but should be documented) |

---

## Part 6: Recommended Fixes

### Fix #1: Align Pre-Prompt Estimation with TokenTracker State

Since `session.token_tracker.input_tokens` already includes cache (confirmed by code at line 908), the pre-prompt estimation should use the same logic:

**Current (line 311)**:
```rust
let current_tokens = session.token_tracker.input_tokens + session.token_tracker.output_tokens;
```

**If `input_tokens` already includes cache**:
- This is correct! Just add output.

**BUT**, we should verify that cache tokens are not double-counted elsewhere.

### Fix #2: Add Cache Creation to TokenUsage::total()

**Current** (`compaction_threshold.rs:92-96`):
```rust
pub fn total(&self) -> u64 {
    self.input_tokens + self.cache_read_tokens + self.output_tokens
}
```

**Should be**:
```rust
pub fn total(&self) -> u64 {
    self.input_tokens + self.cache_read_tokens + self.cache_creation_tokens + self.output_tokens
}
```

Wait, looking at `TokenUsage` definition (lines 84-88):
```rust
pub struct TokenUsage {
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
}
```

**There is no `cache_creation_tokens` field in `TokenUsage`!**

This is **structurally different** from `ApiTokenUsage` which has all three fields.

### Fix #3: Add `cache_creation_tokens` to `TokenUsage` or Deprecate It

`TokenUsage` in `compaction_threshold.rs` appears to be a different struct than `ApiTokenUsage`. We should either:

**Option A**: Add `cache_creation_tokens` to `TokenUsage`:
```rust
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub output_tokens: u64,
}
```

Then update `total()`:
```rust
pub fn total(&self) -> u64 {
    self.input_tokens + self.cache_read_tokens + self.cache_creation_tokens + self.output_tokens
}
```

**Option B**: Deprecate `TokenUsage` and use `ApiTokenUsage` everywhere.

Since `ApiTokenUsage` is the canonical type used by the hook, streaming loop, and has full cache support, it makes sense to use it consistently.

### Fix #4: Clarify TokenTracker Semantics in Comments

The comments are confusing because:

1. Comment says `input_tokens` includes cache
2. But we also store `cache_read_input_tokens` and `cache_creation_input_tokens` separately

We should document:
- `input_tokens` is for display and threshold checking (includes cache)
- Cache fields are stored separately for granular tracking
- When using `input_tokens`, don't add cache fields (to avoid double-counting)

### Fix #5: Verify Pre-Prompt vs Hook Alignment

The pre-prompt check and hook should use the same calculation:

**Pre-prompt**:
```rust
let current_tokens = session.token_tracker.input_tokens + session.token_tracker.output_tokens;
```

**Hook** (via `TokenState::total()`):
```rust
let last_known_total = state.total();  // Calls ApiTokenUsage::total_context()
```

If `session.token_tracker.input_tokens` = `total_input()` (includes cache), and `session.token_tracker.output_tokens` is output:

Pre-prompt: `total_input + output` ✓
Hook: `total_input + output` ✓

These **are aligned** if `input_tokens` truly includes cache.

The discrepancy is in how we interpret the comment. The code at line 908 confirms `input_tokens` is set to `total_input()`, so the pre-prompt calculation is actually correct.

---

## Part 7: Root Cause Analysis

After deeper analysis, the **real issue** is:

### `TokenUsage` in `compaction_threshold.rs` is Outdated

The `TokenUsage` struct at line 84-88:
```rust
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
}
```

This struct is **missing `cache_creation_tokens`** and is **only used by**:
- `should_trigger_compaction()` function
- Its tests

Meanwhile, the rest of the codebase uses:
- `ApiTokenUsage` (from `core/src/token_usage.rs`) - has full cache support
- `TokenState` (from `core/src/compaction_hook.rs`) - wraps `ApiTokenUsage`
- `TokenTracker` (from `core/src/compaction/model.rs`) - has all cache fields

The `should_trigger_compaction()` function uses the outdated `TokenUsage` struct, causing the mismatch.

**Impact**:
- `should_trigger_compaction()` is **not actually used** in the main code flow!
- The compaction check uses `CompactionHook` via `TokenState` (which uses `ApiTokenUsage`)
- This means the `TokenUsage` struct and `should_trigger_compaction()` function might be **dead code**

**Verification**:

Search for `should_trigger_compaction` usage:
```bash
grep -r "should_trigger_compaction"
```

Results:
- `cli/src/compaction_threshold.rs` (definition and tests)
- `cli/tests/pre_prompt_compaction_test.rs` (test file)
- `spec/attachments/CLI-008/codelet-persistent-context.md` (documentation)

**No actual usage in main code!**

The main compaction trigger is via `CompactionHook::on_completion_call()` in `compaction_hook.rs`.

---

## Part 8: Final Findings

### Confirmed Issues

| # | Issue | Status | Notes |
|---|--------|--------|-------|
| 1 | **False Alarm** | Pre-prompt estimation is correct (input_tokens includes cache) |
| 2 | **Real Issue** | `TokenUsage` struct missing `cache_creation_tokens` |
| 3 | **False Alarm** | Pre-prompt and hook are aligned (both use same formula) |
| 4 | **Documentation Issue** | Comment is correct, but architecture is confusing (redundant state) |
| 5 | **Code Quality** | `TokenUsage` and `should_trigger_compaction()` appear to be dead code |

### Real Issues to Fix

1. **Dead Code**: `TokenUsage` struct and `should_trigger_compaction()` function in `compaction_threshold.rs` are not used in the main code flow. Either:
   - Remove them (clean up dead code)
   - OR document why they're kept (for tests, documentation, etc.)

2. **Documentation**: Clarify the token tracking architecture:
   - `session.token_tracker.input_tokens` stores `total_input()` (includes cache)
   - Cache fields are stored separately for granular tracking
   - When using `input_tokens` directly, don't add cache fields

3. **Architecture**: Consider whether storing cache separately is necessary if `input_tokens` already includes them. This redundancy could be a source of bugs.

### What Works Correctly

1. **Compaction threshold calculation**: `calculate_usable_context()` correctly calculates `context_window - output_reservation`

2. **Compaction budget calculation**: `calculate_summarization_budget()` correctly calculates the target after compaction

3. **Provider constants**: All providers have correct `CONTEXT_WINDOW` and `MAX_OUTPUT_TOKENS` constants

4. **Hook-based compaction**: `CompactionHook::on_completion_call()` correctly uses `TokenState::total()` which includes all token types

5. **Pre-prompt check**: The formula is correct given that `input_tokens` includes cache

---

## Appendix: Complete Call Flow

### Streaming Loop Token Update Flow

```
1. Initialize TokenState (line 385-395):
   - input_tokens = session.token_tracker.input_tokens (includes cache)
   - cache_read_input_tokens = 0 (avoid double-counting)
   - cache_creation_input_tokens = 0 (avoid double-counting)
   - output_tokens = session.token_tracker.output_tokens

2. Pre-prompt check (line 310-314):
   - current_tokens = input_tokens + output_tokens
   - estimated_total = current_tokens + prompt_tokens
   - if estimated_total > threshold: trigger compaction

3. Create CompactionHook (line 397):
   - Pass TokenState and threshold

4. Streaming starts:
   - For each API call, hook.on_completion_call() is invoked
   - Hook checks: effective_total = max(last_known_total, estimated_payload)
   - If effective_total > threshold: cancel stream, set compaction_needed flag

5. API response finishes:
   - hook.on_stream_completion_response_finish() updates TokenState from API usage

6. After streaming completes (line 908-911):
   - Update session.token_tracker from turn_usage
   - input_tokens = turn_usage.total_input() (includes cache)
   - Store cache fields separately (redundant)
```

---

## Conclusion

The compaction system is **largely correct**, but has some code quality issues:

1. **Dead code** that should be removed or documented
2. **Confusing comments** that could be clarified
3. **Redundant state** that could be simplified

The actual compaction logic (via `CompactionHook`) is correct and uses proper token counting with all cache token types included.

The **only potential bug** is if someone were to use the outdated `TokenUsage` struct or `should_trigger_compaction()` function, but these appear to be unused in production code.

**Recommendation**: Clean up the dead code and improve documentation to prevent future confusion.
