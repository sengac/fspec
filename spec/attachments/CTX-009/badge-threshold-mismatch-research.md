# CTX-009: SessionHeader Badge vs Fill Percentage Mismatch — Research & Fix Plan

## The Problem

The SessionHeader component displays two context-related indicators that disagree about their denominator:

| Indicator | What it shows | Denominator | Example (Gemini 1M) |
|-----------|---------------|-------------|---------------------|
| Badge `[1M]` | Raw context window | `context_window` (1,000,000) | `[1M]` |
| Fill `[50%]` | Tokens / threshold | `compaction_threshold` (800,000) | `[50%]` = 400k tokens |

A user reading `[1M] [50%]` intuitively thinks "I'm at 500k of 1M" — but they're at 400k of an 800k compaction threshold. This mismatch is invisible for Claude (200k context, ~192k threshold ≈ 4% difference) but breaks down for:

- **Gemini** (1M context, 800k threshold = 20% gap)
- **OpenAI** (128k context, 102k threshold = 20% gap)
- **Custom models** with user-configured thresholds

## Root Cause — Specification Conflict

**CTX-006 Rule [5] / Scenario line 94-100:**
> "The ContextFillUpdate event already sends context_window from Rust — the SessionHeader badge and fill percentage must both ultimately derive from the same Rust authority"
> Scenario says: "fill percentage should use 200000 as the context window"

**CTX-007 Rule [4]:**
> "Context fill percentage must be computed relative to the compaction threshold, not the raw context window"

These two rules contradict each other. CTX-007 correctly changed the fill percentage denominator to the compaction threshold, but nobody updated the badge to match. The badge still shows `context_window` while the percentage uses `threshold`.

## Data Flow Trace

### Rust Side (stream_loop.rs:98-116)
```rust
pub(super) fn emit_context_fill_from_usage<O: StreamOutput>(
    output: &O,
    usage: &ApiTokenUsage,
    threshold: u64,      // ← compaction threshold from resolve_compaction_threshold()
    context_window: u64,  // ← raw context window
) {
    let total_tokens = usage.total_context();
    let fill_percentage = if threshold > 0 {
        ((total_tokens as f64 / threshold as f64) * 100.0) as u32  // ← computed against threshold ✅
    } else { 0 };
    output.emit_context_fill(&ContextFillInfo {
        fill_percentage,      // ← correct: relative to threshold
        effective_tokens: total_tokens,
        threshold,            // ← the compaction threshold
        context_window,       // ← the raw context window (DIFFERENT from threshold)
    });
}
```

### NAPI Bridge (session_manager.rs:5745-5749, types.rs:640-646)
```rust
StreamEvent::ContextFill(info) => StreamChunk::context_fill_update(ContextFillInfo {
    fill_percentage: info.fill_percentage,  // pre-computed against threshold
    effective_tokens: info.effective_tokens,
    threshold: info.threshold,              // compaction threshold available
    context_window: info.context_window,    // raw context window available
})
```

JSON output:
```json
{
  "type": "contextFillUpdate",
  "contextFill": {
    "fillPercentage": 50,
    "effectiveTokens": 400000,
    "threshold": 800000,       // ← threshold IS available in the event
    "contextWindow": 1000000   // ← context window also available
  }
}
```

### TypeScript TUI (AgentView.tsx:1133-1134)
```typescript
} else if (chunk.type === 'ContextFillUpdate' && chunk.contextFill) {
  setContextFillPercentage(chunk.contextFill.fillPercentage); // ← reads pre-computed %
  // NOTE: chunk.contextFill.threshold is AVAILABLE but UNUSED
}
```

### SessionHeader Badge (SessionHeader.tsx:160-162)
```typescript
if (contextWindow > 0) {
  leftContent += chalk.dim(` [${formatContextWindow(contextWindow)}]`);
  // contextWindow comes from rustModelInfo.contextWindow (AgentView.tsx:5255)
  // This is the RAW context window, NOT the compaction threshold
}
```

## The Fix

The badge should show the **compaction threshold** (what the fill% is relative to), not the raw context window.

### Option Chosen: Badge shows compaction threshold

Change the badge from `[200k]` (context window) to `[192k]` (compaction threshold).

Reasoning:
- The percentage is relative to the threshold → the badge should show the same number
- Users care about "when does compaction fire?" not "what's the theoretical max?"
- The context window is already visible in the model selector — it doesn't need to be in the header too

### Implementation Points

1. **AgentView.tsx** — Extract `compactionThreshold` from the Rust snapshot model data (already exposed via CTX-007: `rustSnapshot.model.compactionThreshold`) and pass it to SessionHeader instead of (or alongside) `contextWindow`.

2. **SessionHeader.tsx** — Change the badge to display the compaction threshold:
   ```typescript
   // Before:
   if (contextWindow > 0) {
     leftContent += chalk.dim(` [${formatContextWindow(contextWindow)}]`);
   }
   // After:
   const badgeValue = compactionThreshold ?? contextWindow;
   if (badgeValue > 0) {
     leftContent += chalk.dim(` [${formatContextWindow(badgeValue)}]`);
   }
   ```

3. **SessionHeader props** — Add `compactionThreshold?: number` prop, falling back to `contextWindow` if not yet available (pre-model-selection).

4. **sessionHeaderUtils.ts** — `formatContextWindow()` already works for any token count, no change needed.

### Files to Modify

| File | Change |
|------|--------|
| `src/tui/components/SessionHeader.tsx` | Add `compactionThreshold` prop, use for badge display |
| `src/tui/components/AgentView.tsx` | Read `rustSnapshot.model.compactionThreshold`, pass to SessionHeader |
| Test files | Update to verify badge shows threshold, not context window |

### Already Available Data

The compaction threshold is **already exposed** via the NAPI boundary:
- `SessionModel.compaction_threshold: Option<u32>` (added by CTX-007)
- `useRustSessionState` hook already reads this field
- `ContextFillUpdate` event already carries `threshold` field

No Rust changes needed. This is a TypeScript-only fix.
