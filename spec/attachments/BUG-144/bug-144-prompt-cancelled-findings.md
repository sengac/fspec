# BUG-144: PromptCancelled Errors Not Caught During Compaction

## Cross-Pollination Consensus Report

**Date:** 2026-04-23  
**Status:** Confirmed by 3 independent agents  
**Severity:** Critical — causes session termination on user-initiated cancellation  
**Epic:** provider-resilience  

---

## Executive Summary

When a user cancels a prompt during a compaction operation, the session terminates instead of gracefully handling the cancellation. The root cause is that `anyhow::anyhow!("Streaming error: {e}")` in `rig_agent.rs` destroys the typed error chain by formatting `StreamingError::Prompt(Box<PromptError::PromptCancelled>)` into a bare string. Downstream error classifiers cannot downcast the bare string back to the original typed error, so they fail to recognize the cancellation and allow it to fall through to the generic error cascade, terminating the session.

---

## Root Cause Analysis

### Primary Location

**File:** `codelet/core/src/rig_agent.rs`  
**Lines:** 68, 103, 137, 177  

All four locations use:
```rust
anyhow::anyhow!("Streaming error: {e}")
```

**Only line 177 matters in production** — it is the CompactionHook path. The other three are in non-compaction streaming paths that follow different error handling flows.

### What Happens

1. User cancels a prompt during compaction
2. The streaming loop raises `StreamingError::Prompt(Box<PromptError::PromptCancelled>)`
3. Line 177 converts it to: `anyhow::anyhow!("Streaming error: Streaming error: PromptCancelled")`
4. This creates a bare-string `anyhow::Error` — the original type information is **destroyed**
5. The error propagates to `extract_prompt_cancelled()` in `error_classifiers.rs`

### Why Downstream Fails

**File:** `codelet/core/src/error_classifiers.rs` (lines 160-184)

`extract_prompt_cancelled()` attempts two downcasts:
- `downcast_ref::<PromptError>()` → returns `None` (bare string is not a `PromptError`)
- `downcast_ref::<Box<PromptError>>()` → returns `None` (bare string is not a `Box<PromptError>`)

Both fail because `anyhow::anyhow!(...)` formats the error via `Display` into a string, destroying the type chain.

### Cascade to Session Termination

1. `extract_prompt_cancelled()` returns `None`
2. `classify_compaction_branch()` returns `NotCompaction`
3. Error falls through the generic error cascade
4. Session terminates

---

## Why Existing Safety Nets Fail

### Safety Net #1: `compaction_needed` Flag

The `compaction_needed` flag does NOT save you. The `FlagExtraneous` branch just logs a warning and falls through to termination. It does not intercept or recover from the error.

### Safety Net #2: CMPCT-032 Post-Loop Safety Net

CMPCT-032's post-loop safety net is completely bypassed. The error path uses `return Err()` (not `break`), so the safety net loop never runs. The function exits immediately with the unclassifiable error.

---

## False Positive Test

**Existing test:** `detects_streaming_error_wrapped_prompt_cancelled`

This test is a **FALSE POSITIVE**. It uses `.into()` to convert the error:
```rust
// Test path — preserves type chain
let error: anyhow::Error = streaming_error.into();
```

But production code uses `anyhow::anyhow!(...)`:
```rust
// Production path — destroys type chain
let error = anyhow::anyhow!("Streaming error: {e}");
```

The test passes because `.into()` preserves the typed error chain (allowing `downcast_ref` to work), but production uses `anyhow::anyhow!(...)` which destroys it. **The test passes but does not test the actual production code path.**

---

## Proposed Fix

### Consensus: Use `anyhow::Error::from(e)` Instead of `anyhow::anyhow!(...)`

Change all four locations in `rig_agent.rs` from:
```rust
// BEFORE — destroys typed error chain
Err(e) => return Err(anyhow::anyhow!("Streaming error: {e}")),
```

To:
```rust
// AFTER — preserves typed error chain
Err(e) => return Err(anyhow::Error::from(e)),
```

### Why `Error::from(e)` (not `.context()`)

There was disagreement among the investigating agents:

| Approach | Supporters | Pros | Cons |
|----------|-----------|------|------|
| `anyhow::Error::from(e)` | Agent A (kimi-k2p6) | Preserves typed chain; no string format changes that could break string-based classifiers | Loses "Streaming error" prefix in Display output |
| `Error::from(e).context("Streaming error")` | Agent B (glm-5p1), Agent C (qwen3p6-plus) | Preserves typed chain AND adds context string | `.context()` changes the string format which could break string-based classifiers that search for "Streaming error" patterns |

**Recommended safe fix:** `anyhow::Error::from(e)` — it preserves the typed chain without any risk of changing string-based classifier behavior. The "Streaming error" prefix is already present in the `StreamingError` Display impl, so it will appear in the error message regardless.

### Required Changes

1. **`codelet/core/src/rig_agent.rs`** — Lines 68, 103, 137, 177:
   - Replace `anyhow::anyhow!("Streaming error: {e}")` with `anyhow::Error::from(e)`

2. **`codelet/core/src/error_classifiers.rs`** — No changes needed:
   - `extract_prompt_cancelled()` already handles the typed path correctly
   - The existing `downcast_ref` calls will work once the type chain is preserved

3. **New test required:**
   - Write a test that exercises the `anyhow::anyhow!(...)` conversion path (currently untested)
   - Test should verify that `extract_prompt_cancelled()` correctly identifies `PromptCancelled` when the error flows through the actual production conversion path
   - The existing false positive test should also be updated to match the production code path

---

## Verification Checklist

- [ ] Change `anyhow::anyhow!("Streaming error: {e}")` → `anyhow::Error::from(e)` at all 4 sites in `rig_agent.rs`
- [ ] Verify `extract_prompt_cancelled()` works with the new conversion (should work — no changes needed)
- [ ] Verify string-based classifiers still work (search for any code matching on "Streaming error" string patterns)
- [ ] Fix the false positive test `detects_streaming_error_wrapped_prompt_cancelled` to use the production conversion path
- [ ] Add new test that specifically exercises the `Error::from(e)` path and validates `extract_prompt_cancelled()` returns `Some`
- [ ] Add test verifying compaction loop continues after PromptCancelled (safety net path)
- [ ] Run full test suite to ensure no regressions

---

## Agent Sessions Used

| Session ID | Model | Verdict | Fix Preference |
|-----------|-------|---------|---------------|
| 528ebb00 | kimi-k2p6 | CONFIRMED | `Error::from(e)` without context |
| b8e60941 | glm-5p1 | CONFIRMED | `Error::from(e).context(...)` — discovered false positive test |
| 92387760 | qwen3p6-plus | CONFIRMED | Supports `.context()` version |
| 3709c956 | minimax-m2p7 | N/A | Never responded (model issue) |

---

## Related Work Units

- **CMPCT-032** — Compaction safety net (bypassed by this bug)
- **CTX prefix** — Context management and compaction work units
- **PROV prefix** — Provider and API client functionality

---

## Technical Deep Dive

### anyhow Error Chain Mechanics

When you write `anyhow::anyhow!("Streaming error: {e}")`:
1. The macro calls `format!("Streaming error: {e}")` to create a `String`
2. This string is wrapped in `anyhow::Error`
3. The original error `e` is **NOT** stored — only its `Display` output is captured
4. `downcast_ref::<OriginalType>()` will always return `None`

When you write `anyhow::Error::from(e)`:
1. The `From` impl checks if `e` is already an `anyhow::Error` — if so, returns it as-is
2. If `e` is a different error type, it wraps it preserving the original type via `TypeId`
3. `downcast_ref::<OriginalType>()` will return `Some(&original)`
4. The original error's `Display` impl is preserved for error messages

### The StreamingError Type Chain

```
StreamingError::Prompt(Box<PromptError>)
  └── PromptError::PromptCancelled
```

With `Error::from(e)`:
- `downcast_ref::<StreamingError>()` → ✅ Some
- `downcast_ref::<Box<PromptError>>()` → ✅ Some (via anyhow's inner chain)
- `downcast_ref::<PromptError>()` → Depends on anyhow version's chain traversal

The `extract_prompt_cancelled()` function already handles both `PromptError` and `Box<PromptError>` downcasts, so preserving the chain via `Error::from(e)` makes both paths available.
