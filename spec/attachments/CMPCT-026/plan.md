# CMPCT-026 — Eliminate Fragile `&& compaction_triggered` Guard

**Parent:** CMPCT-022
**Bug:** BUG 4
**Depends on:** CMPCT-025

## The Problem

In `codelet/cli/src/interactive/stream_loop.rs:1141-1161`:

```rust
let is_compaction_cancel = is_compaction_cancelled(&e);

let compaction_triggered = token_state
    .lock()
    .map(|state| state.compaction_needed)
    .unwrap_or(false);

if is_compaction_cancel && compaction_triggered {
    break;
}
```

This guard requires TWO independent pieces of evidence to agree:
1. The error carries the `PromptCancelled` marker.
2. The shared `TokenState::compaction_needed` flag is `true`.

## Why This Is Fragile

### 1. Only one cancel site sets the flag

`CompactionHook::on_completion_call` (`codelet/core/src/compaction_hook.rs:209-216`):
```rust
if effective_total > self.threshold {
    state.compaction_needed = true;    // ← ONLY set here
    cancel_sig.cancel();
}
```

Rig checks `cancel_sig.is_cancelled()` at **6** sites (`streaming.rs:412, 460, 486, 509, 542, 586`). Today only site 412 (`on_completion_call`) triggers the flag. If the hook is ever extended to cancel from `on_text_delta`, `on_tool_call`, `on_tool_result`, `on_tool_call_delta`, or `on_stream_completion_response_finish` (e.g., for a future rate-limit hook, timeout hook, or user-initiated cancel), PromptCancelled will fire but `compaction_needed` will be false.

Result: the `&&` guard fails, the error falls through to `is_prompt_too_long_error("PromptCancelled")` → returns `false` → cascade continues down to `stream_loop.rs:1447` which returns `Err("Agent error: PromptCancelled")`.

**The session terminates** instead of recovering.

### 2. `token_state` reference may be stale

Lines 1062, 1254, 1359 of `stream_loop.rs` create fresh `retry_token_state` instances when recovery paths restart the stream. The outer `token_state: Arc<Mutex<TokenState>>` variable still points to the ORIGINAL state. If a cancel fires on the retry stream, its `compaction_needed` flag is on the retry state, not the original — the guard reads the wrong mutex.

### 3. The flag and the error are redundant

By design, whenever the hook sets `compaction_needed = true`, it ALSO calls `cancel_sig.cancel()` (atomically, inside the mutex). Conversely, `cancel_sig.cancel()` is only called by code that ALSO sets the flag. So the two signals are supposed to be equivalent — the `&&` adds zero information when the system is healthy and causes false negatives when the system is degraded.

## The Fix

After CMPCT-025 lands (structural `PromptCancelled` detection), we can collapse to a single source of truth. There are two design options:

### Option A — Error is authoritative

```rust
if let Some(chat_history) = extract_prompt_cancelled(&e) {
    // Check flag is a warning, not a gate
    if !token_state.lock().map(|s| s.compaction_needed).unwrap_or(false) {
        warn!("PromptCancelled without compaction_needed=true; recovering anyway");
        // Defensively set the flag so post-loop handler runs
        signal_compaction_needed(&token_state);
    }
    break;
}
```

### Option B — Flag is authoritative

```rust
let compaction_triggered = token_state.lock()
    .map(|s| s.compaction_needed)
    .unwrap_or(false);
if compaction_triggered {
    // Error type doesn't matter — flag says to compact
    if !is_compaction_cancelled(&e) {
        warn!("compaction_needed=true but error is not PromptCancelled: {}", e);
    }
    break;
}
```

### Recommendation: Option A

The error is the more specific signal (comes from rig, carries chat_history), and the structural downcast post-CMPCT-025 is robust. The flag check becomes a defense-in-depth assertion rather than a gate.

## Acceptance Criteria

1. When `PromptCancelled` fires AND `compaction_needed == true` → recovery runs (unchanged).
2. When `PromptCancelled` fires AND `compaction_needed == false` → recovery STILL runs (with a warning). This was a silent session-termination before.
3. When an unrelated error fires AND `compaction_needed == true` → something is wrong; log loudly, but do NOT run recovery (there's no PromptCancelled to recover from).
4. A new integration test verifies case 2 produces a successful session, not an error.

## Files to Modify

- `codelet/cli/src/interactive/stream_loop.rs` (lines 1141-1161)
- Possibly `codelet/cli/src/interactive/gemini_continuation.rs:331-346` for symmetry

## Testing

```rust
#[test]
fn compaction_cancel_with_flag_false_still_recovers() {
    // Simulate a rig extension that cancels without setting flag
    // Assert recovery runs and session continues
}

#[test]
fn unrelated_error_with_flag_true_does_not_trigger_recovery() {
    // Set compaction_needed=true manually
    // Yield a non-PromptCancelled error
    // Assert recovery does NOT run (error propagates)
}
```

## Risk

This changes the behavior of edge cases that were previously silent failures. There's a small risk that something upstream was relying on the `&&` gate as a safety net. Mitigate by logging loudly when the two signals disagree.
