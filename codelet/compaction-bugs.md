# Compaction Implementation Bugs - TypeScript vs Rust Comparison

## CRITICAL BUGS

### 1. Summary Generation Method - COMPLETELY DIFFERENT

**TypeScript** (`WeightedSummaryProvider.generateWeightedSummary`):
```typescript
async generateWeightedSummary(turns, anchors, preservationContext): Promise<string> {
  const outcomes = turns.map(turn => this.turnToOutcome(turn, anchors));
  const contextSummary = this.preserveContext(preservationContext);
  return `${contextSummary}\n\nKey outcomes:\n${outcomes.join('\n')}`;
}
```
- **NO LLM CALL** - just template formatting
- Fast, free, never fails
- Produces structured output with context preservation

**Rust** (`ContextCompactor::generate_summary`):
```rust
async fn generate_summary(..., llm_prompt: &F) -> Result<String> {
    let prompt = "Summarize the following conversation turns...";
    // CALLS LLM!
    llm_prompt(prompt.clone()).await
}
```
- **CALLS LLM** - makes actual API request
- Slow, costs money, can fail (has retry logic)
- Can timeout, rate limit, or produce unexpected output

**Impact**: Rust compaction is slower, more expensive, and less reliable.

---

### 2. Message Ordering After Compaction - DIFFERENT ORDER

**TypeScript order**:
1. System messages
2. Kept turn messages (user/assistant pairs)
3. Summary message (user)
4. Session continuation (user)

**Rust order**:
1. System messages (empty!)
2. Summary message (user)
3. Continuation message (user)
4. Kept turn messages (user/assistant pairs)

**Impact**: Rust puts summary BEFORE kept turns. This changes conversation context and could confuse the model about what happened when.

---

### 3. Compression Ratio Handling - FAIL vs WARN

**TypeScript**:
```typescript
if (compressionRatio < 0.6) {
  warnings.push('Compression ratio below 60%...');
}
// Continues anyway!
```

**Rust**:
```rust
if !metrics.meets_threshold(self.min_compression_ratio) {
    anyhow::bail!("Compaction did not meet minimum compression ratio...");
}
// FAILS and returns error!
```

**Impact**: Rust compaction can fail and leave the session in the same state, potentially causing repeated failed compaction attempts.

---

### 4. Turn Selection Strategy - SUBTLE DIFFERENCE

**TypeScript**:
1. ALWAYS split into `recentTurns` (last 2-3) and `olderTurns`
2. Only look for anchors in `olderTurns`
3. If anchor found: keep anchor→end of older + all recent
4. If no anchor: keep only recent turns

**Rust**:
1. Look for anchors in ALL turns
2. If anchor found: keep anchor→end (might overlap with recent)
3. If no anchor: keep last 2-3 turns

**Impact**: TypeScript guarantees recent context is always kept as a separate block. Rust may handle edge cases differently.

---

### 5. Token Tracker Update After Compaction

**TypeScript**:
```typescript
const newTotalTokens = messages.reduce(calculateMessageTokens, 0);
tokenTracker = {
  ...tokenTracker,
  inputTokens: newTotalTokens,
  totalTokens: newTotalTokens,
};
```
- Recalculates from ACTUAL messages array
- Preserves other tracker fields (cache metrics)

**Rust**:
```rust
session.token_tracker.input_tokens = result.metrics.compacted_tokens;
session.token_tracker.output_tokens = 0;
session.token_tracker.cache_read_input_tokens = None;
session.token_tracker.cache_creation_input_tokens = None;
```
- Uses pre-calculated metric (may not match actual messages)
- Resets cache metrics to None/0

**Impact**: Token count mismatch could cause unexpected compaction timing.

---

## MEDIUM BUGS

### 6. ToolResult Missing Error Field

**TypeScript**:
```typescript
interface ToolResult {
  success: boolean;
  output: string;
  error?: string;  // Optional error field
}
```

**Rust**:
```rust
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    // Missing error field!
}
```

---

### 7. ToolCall Structure Difference

**TypeScript**:
```typescript
interface ToolCall {
  tool: string;
  parameters: Record<string, unknown>;
}
```

**Rust**:
```rust
pub struct ToolCall {
    pub tool: String,
    pub id: String,  // Extra field
    pub input: serde_json::Value,  // Different name: 'input' vs 'parameters'
}
```

**Impact**: File path extraction for anchor descriptions uses wrong field name.

---

### 8. Token Estimation Mismatch

**TypeScript**:
```typescript
Math.ceil(text.length / APPROX_BYTES_PER_TOKEN)  // Rounds UP
```

**Rust** (now fixed):
```rust
((text.len() + APPROX_BYTES_PER_TOKEN - 1) / APPROX_BYTES_PER_TOKEN) as u64  // Ceiling division
```

---

## ALREADY FIXED

### 9. Compaction Threshold Ratio
- Was 0.8 (80%) in Rust, should be 0.9 (90%) - **FIXED**

### 10. Empty Content Extraction
- Was filtering out tool calls to empty strings - **FIXED** (now JSON.stringify)

---

## RECOMMENDED FIXES (Priority Order)

1. **Change Rust summary to template-based** (match TypeScript exactly)
2. **Fix message ordering** to match TypeScript
3. **Change compression ratio to warning** instead of failure
4. **Fix token tracker update** to recalculate from actual messages
5. **Add error field to ToolResult**
6. **Rename ToolCall.input to ToolCall.parameters**
