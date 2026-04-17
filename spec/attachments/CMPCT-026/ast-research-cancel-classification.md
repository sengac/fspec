# CMPCT-026 — AST Research: Single Source of Truth for Compaction-Cancel Classification

## Scope
Replace the fragile `is_compaction_cancel && compaction_triggered` conjunction in `stream_loop.rs` (and mirror the same fix in `gemini_continuation.rs`) with a single authoritative check: **presence of `PromptCancelled` in the error chain**. The `TokenState.compaction_needed` flag becomes defense-in-depth: when the error says "cancelled" but the flag is false, we log a warning and set the flag before routing to recovery.

## Fragile Sites Located

### Site 1 — stream_loop.rs (PRIMARY)
```
/home/rquast/projects/fspec/codelet/cli/src/interactive/stream_loop.rs:1156:21
if is_compaction_cancel && compaction_triggered {
```
Lines 1141–1183. The `&&` requires both (a) the error to be `PromptCancelled` AND (b) `token_state.compaction_needed == true`. Only `compaction_hook.rs:216` sets the flag today. Three other `cancel_sig.cancel()` sites (`compaction_hook.rs:113`, `:143`, and `gemini_history_hook.rs:192`) never set the flag and thus fail the guard.

### Site 2 — gemini_continuation.rs (MIRROR)
```
/home/rquast/projects/fspec/codelet/cli/src/interactive/gemini_continuation.rs:331:17
if is_compaction_cancelled(&e) { ... }
```
Lines 329–346. Currently only uses the error-side check (no flag guard), but the post-branch flow unconditionally calls `signal_compaction_needed`, so this site is already closer to the target. The fix here is mostly **confirmation** — add a `warn!` when the flag was already false, to surface cross-site drift for diagnostics.

## Flag-Setter Sites (`cancel_sig.cancel()`)

| File:Line | Sets `state.compaction_needed`? | Comment |
|-----------|--------------------------------|---------|
| `core/src/compaction_hook.rs:113` | NO | Image-sanitize cancel path |
| `core/src/compaction_hook.rs:143` | NO | Another image path |
| `core/src/compaction_hook.rs:216` | **YES** | Threshold exceeded (the canonical path) |
| `core/src/gemini_history_hook.rs:192` | NO | Gemini history hook cancel |

Only one of four cancel-emitters updates the flag. The `&&` guard therefore fails for 75% of cancel sites, silently terminating recoverable sessions.

## Structural Detector (from CMPCT-025)
`codelet/cli/src/interactive/error_classifiers.rs:149` exposes:
```rust
pub(super) fn extract_prompt_cancelled(
    error: &anyhow::Error,
) -> Option<&Vec<rig::message::Message>>
```
Walks the full `anyhow::Error::chain()` and matches both bare `PromptError::PromptCancelled` and `Box<PromptError>` (thiserror's `#[from]` boxed variant). This is the authoritative structural gate for Option A.

## Planned Fix Shape (Option A)
```rust
let extracted = extract_prompt_cancelled(&e);
let compaction_triggered = token_state
    .lock()
    .map(|state| state.compaction_needed)
    .unwrap_or(false);

match (extracted.is_some(), compaction_triggered) {
    (true, true) => { /* normal recovery */ }
    (true, false) => {
        warn!(error = %e, "PromptCancelled without compaction_needed=true; recovering anyway");
        signal_compaction_needed(&token_state);
        /* normal recovery */
    }
    (false, true) => {
        warn!(error = %e, "compaction_needed=true but error is not PromptCancelled; falling through");
        /* fall through to classifier cascade */
    }
    (false, false) => { /* fall through */ }
}
```

## Test Surface
New file: `codelet/cli/tests/compaction_single_source_of_truth_test.rs`.
Tests target a small extractable classifier helper (to be added as `classify_compaction_branch` or similar) so the matrix above can be exercised without spinning up a real rig stream. The integration test harness in `compaction_cancel_preservation_test.rs` is the model.
