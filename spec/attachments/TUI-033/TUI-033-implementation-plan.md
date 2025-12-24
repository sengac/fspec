# TUI-033: Context Window Fill Percentage Indicator - Implementation Plan

## Overview

Add a color-coded context window fill percentage indicator to the AgentModal header, positioned to the right of the token count display (`tokens: X↓ Y↑`) but to the left of the `[Tab] Switch` component.

## Current UI Layout (AgentModal.tsx lines 1892-1924)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Agent: claude (streaming...) [DEBUG]  12.3 tok/s  tokens: 1234↓ 567↑  [Tab] │
└─────────────────────────────────────────────────────────────────────────────┘
│← LEFT (flexGrow=1) →│← MIDDLE →│←── RIGHT ──→│← TAB →│
```

## Proposed UI Layout

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│ Agent: claude (streaming...)  12.3 tok/s  tokens: 1234↓ 567↑  [43% CTX]  [Tab]  │
└──────────────────────────────────────────────────────────────────────────────────┘
│← LEFT (flexGrow=1) →│← MIDDLE →│←── RIGHT ──→│← NEW →│← TAB →│
```

## Technical Analysis

### Data Sources Required

**From Session/TokenTracker (Rust side):**
- `input_tokens: u64` - Total input tokens consumed
- `cache_read_input_tokens: Option<u64>` - Cached tokens (90% discount)
- `cache_creation_input_tokens: Option<u64>` - Cache creation tokens

**From Provider Configuration:**
- `context_window: u64` - Provider's context window size (e.g., 200,000 for Claude)

**Calculation (from compaction_threshold.rs):**
```rust
const COMPACTION_THRESHOLD_RATIO: f64 = 0.9;

// Threshold = when compaction triggers
let threshold = context_window * COMPACTION_THRESHOLD_RATIO;  // e.g., 180,000

// Effective tokens = actual usage (cache-aware)
let effective_tokens = input_tokens - (cache_read_input_tokens * 0.9);

// Fill percentage
let fill_percentage = (effective_tokens / threshold) * 100.0;
```

### Files to Modify

#### 1. Rust Side: Add Context Fill Data to Stream Events

**File: `codelet/core/src/stream/types.rs`**

Add a new event type or extend `TokenUpdate` to include context fill data:

```rust
pub struct ContextFillUpdate {
    pub fill_percentage: f64,      // 0.0 - 100.0+
    pub effective_tokens: u64,
    pub threshold: u64,
    pub context_window: u64,
}
```

**File: `codelet/cli/src/interactive/stream_loop.rs`**

After token tracking updates (around line 728-732), emit context fill data:

```rust
// Calculate and emit context fill percentage
let context_window = session.provider_manager().context_window() as u64;
let threshold = calculate_compaction_threshold(context_window);
let effective = session.token_tracker.effective_tokens();
let fill_pct = (effective as f64 / threshold as f64) * 100.0;

// Emit to frontend via existing chunk mechanism
sender.send(StreamChunk::ContextFillUpdate {
    fill_percentage: fill_pct,
    effective_tokens: effective,
    threshold,
    context_window,
});
```

#### 2. NAPI Bindings: Expose Context Fill to TypeScript

**File: `codelet/napi/src/streaming.rs`**

Add handling for new `ContextFillUpdate` event in the streaming event emitter.

**File: `codelet/napi/index.d.ts`**

Add TypeScript types:

```typescript
interface ContextFillUpdate {
  fillPercentage: number;
  effectiveTokens: number;
  threshold: number;
  contextWindow: number;
}

// In StreamChunk union type
type StreamChunk =
  | { type: 'Text'; text: string }
  | { type: 'TokenUpdate'; tokens: TokenUsage }
  | { type: 'ContextFillUpdate'; contextFill: ContextFillUpdate }
  // ... other variants
```

#### 3. TypeScript Side: Handle Context Fill Events

**File: `src/tui/components/AgentModal.tsx`**

**A. Add State (near line 275):**
```typescript
const [contextFill, setContextFill] = useState<{
  fillPercentage: number;
  effectiveTokens: number;
  threshold: number;
  contextWindow: number;
} | null>(null);
```

**B. Handle Event (in stream chunk handler, around line 1035):**
```typescript
else if (chunk.type === 'ContextFillUpdate' && chunk.contextFill) {
  setContextFill(chunk.contextFill);
}
```

**C. Add UI Component (between token count and Tab Switch, around line 1918):**
```typescript
{/* TUI-033: Context window fill percentage */}
{contextFill !== null && (
  <Box marginLeft={2}>
    <Text color={getContextFillColor(contextFill.fillPercentage)}>
      [{Math.round(contextFill.fillPercentage)}% CTX]
    </Text>
  </Box>
)}
```

**D. Color Coding Function:**
```typescript
function getContextFillColor(percentage: number): string {
  if (percentage < 50) return 'green';       // Plenty of room
  if (percentage < 70) return 'yellow';      // Getting full
  if (percentage < 85) return 'magenta';     // Warning zone
  return 'red';                               // Critical - compaction imminent
}
```

## Color Coding Rationale

| Percentage | Color   | Meaning                                    |
|------------|---------|---------------------------------------------|
| 0-49%      | Green   | Plenty of headroom, normal operation        |
| 50-69%     | Yellow  | Context getting full, awareness needed      |
| 70-84%     | Magenta | Warning zone, compaction approaching        |
| 85-100%+   | Red     | Critical, compaction imminent or triggered  |

**Note:** Values can exceed 100% briefly if tokens spike between threshold check and UI update.

## Implementation Order

### Phase 1: Backend Data Flow
1. Add `ContextFillUpdate` to stream types (`codelet/core/src/stream/types.rs`)
2. Calculate and emit context fill in stream loop (`codelet/cli/src/interactive/stream_loop.rs`)
3. Test backend emits correct data

### Phase 2: NAPI Bridge
4. Add NAPI bindings for new event (`codelet/napi/src/streaming.rs`)
5. Update TypeScript types (`codelet/napi/index.d.ts`)
6. Build and verify NAPI layer

### Phase 3: Frontend Display
7. Add React state for context fill (`AgentModal.tsx`)
8. Handle `ContextFillUpdate` events in stream handler
9. Add color-coded UI component between token count and Tab Switch
10. Test end-to-end display

### Phase 4: Edge Cases & Polish
11. Handle null/initial state (don't display until first update)
12. Handle provider switch (reset context fill state)
13. Handle compaction (context fill resets after compaction)
14. Consider adding tooltip with detailed breakdown

## Testing Strategy

### Unit Tests
- Color function returns correct colors for boundary values (0, 49, 50, 69, 70, 84, 85, 100, 150)
- Context fill calculation matches expected formula

### Integration Tests
- Stream loop emits `ContextFillUpdate` after token updates
- NAPI correctly translates Rust event to TypeScript event
- React component renders correct color at each threshold

### Manual Testing
- Start conversation, watch percentage grow with each turn
- Verify color transitions at 50%, 70%, 85%
- Trigger compaction, verify percentage resets
- Switch providers, verify percentage resets

## Considerations

### Performance
- Context fill calculation is lightweight (simple math)
- Only emit when tokens change (not every frame)
- Reuse existing token tracking infrastructure

### UX Decisions
- Display format: `[43% CTX]` - brief but clear
- Alternative formats considered:
  - `CTX: 43%` - more verbose
  - `43%` - too ambiguous without label
  - `█████░░░░░ 43%` - progress bar (too wide)

### Cache Awareness
The display shows **effective** tokens, not raw tokens:
- User with 150k raw tokens but 80k cached sees ~43% (not 83%)
- This accurately reflects actual compaction risk
- Consider adding tooltip showing raw vs effective breakdown

## Open Questions for User

1. **Display format preference?**
   - `[43% CTX]` (current proposal)
   - `[CTX 43%]`
   - `[43%]` with tooltip
   - Other?

2. **Color thresholds acceptable?**
   - Green: 0-49%
   - Yellow: 50-69%
   - Magenta: 70-84%
   - Red: 85%+

3. **Show when empty?**
   - Always show (even at 0%)
   - Hide until first token update
   - Show after N tokens threshold?

4. **Tooltip/hover info?**
   - Just percentage (current)
   - Add effective/threshold breakdown
   - Add cache savings info

## Dependencies

- Existing `TokenTracker` infrastructure (already in place)
- Existing `calculate_compaction_threshold()` function (already in place)
- Existing NAPI streaming event system (already in place)
- No new external dependencies required

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| NAPI binding issues | Low | Medium | Follow existing TokenUpdate pattern |
| Color visibility on terminals | Medium | Low | Use standard terminal colors |
| Performance overhead | Very Low | Low | Lightweight calculation |
| Context window unknown | Low | Medium | Default to not showing if unavailable |

## Summary

This feature adds visibility into context window usage, helping users understand:
- How close they are to triggering compaction
- When to start a new conversation vs continue
- The effectiveness of prompt caching (effective vs raw tokens)

The implementation leverages existing infrastructure (TokenTracker, NAPI streaming, Ink components) with minimal new code required.
