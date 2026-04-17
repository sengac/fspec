# CMPCT-023 — Unify Compaction Entry Paths

**Parent:** CMPCT-022
**Bug:** BUG 1 — Asymmetric message handling between pre-prompt, hook-cancel, API-error, and Gemini-continuation compaction paths

## The Problem

There are currently **four independent implementations** of "compact and retry", each with a different sequence of operations on `session.messages`:

### Path A — Pre-prompt compaction
**File:** `codelet/cli/src/interactive/stream_loop.rs:301-455`

```rust
// Runs BEFORE pushing user prompt
execute_compaction(session, flag.clone(), Some(prompt)).await
// → session.messages rewritten; prompt embedded in instruction
compaction_just_ran = true;
// Later at line 451:
let effective_prompt = if compaction_just_ran { "Continue" } else { prompt };
// Push User(effective_prompt) AFTER rig gets it (line 466)
```

### Path B — API-returned "prompt is too long"
**File:** `codelet/cli/src/interactive/stream_loop.rs:1181-1202`

```rust
// Runs AFTER user prompt was pushed at line 466
if let Some(last_msg) = session.messages.last() {
    if matches!(last_msg, rig::message::Message::User { .. }) {
        session.messages.pop();  // ← EXPLICIT pop
    }
}
signal_compaction_needed(&token_state);
break;
// Post-loop: handle_compaction_retry → execute_compaction → retry with "Continue"
```

### Path C — Hook-triggered cancel (PromptCancelled)
**File:** `codelet/cli/src/interactive/stream_loop.rs:1156-1161`

```rust
if is_compaction_cancel && compaction_triggered {
    break;   // ← NO POP, NO SAVE, NO TRACKER UPDATE
}
// Post-loop: handle_compaction_retry → execute_compaction → retry with "Continue"
```

### Path D — Gemini continuation cancel
**File:** `codelet/cli/src/interactive/gemini_continuation.rs:329-351`

```rust
if is_compaction_cancelled(&e) {
    if !text.is_empty() {
        handle_final_response(text, &mut session.messages)?;  // ← SAVES partial text
    }
    update_token_tracker(session, display);  // ← UPDATES tracker
    super::stream_loop::signal_compaction_needed(parent_token_state);
    output.emit_compaction_started();
    output.emit_compaction_progress("Context limit reached", 0, total_turns.max(1));
    set_tool_progress_callback(uuid::Uuid::nil(), None);
    return Ok(GeminiContinuationResult::CompactionNeeded);
}
```

### Comparison Matrix

| Step                           | Path A | Path B | Path C | Path D |
|--------------------------------|--------|--------|--------|--------|
| Pop last user message          | N/A    | ✅     | ❌     | N/A    |
| Save partial assistant text    | N/A    | ❌     | ❌     | ✅     |
| Update token tracker           | N/A    | ❌     | ❌     | ✅     |
| Emit compaction-started event  | ❌†    | ❌‡    | ❌‡    | ✅     |
| Clear tool progress callback   | N/A    | ❌     | ❌     | ✅     |
| Signal compaction_needed flag  | ✅§    | ✅     | ✅§    | ✅     |

† Path A runs BEFORE the stream even starts; emission happens elsewhere.
‡ Emitted later by `handle_compaction_retry`.
§ Path A sets it via `execute_compaction`; Path C relies on the hook having set it.

## Proposed Fix

Introduce a single helper in `stream_loop.rs` (or a new module `compaction_entry.rs`):

```rust
/// Unified entry point for all compaction recovery paths.
///
/// Guarantees the following invariants regardless of entry point:
/// 1. Partial assistant_text is saved to session.messages via handle_final_response
/// 2. Token tracker is updated with cumulative billing from streaming_display
/// 3. The last user message is popped IF pop_user_prompt=true (indicates the
///    prompt has been added to session.messages but never delivered to the LLM)
/// 4. compaction_needed flag is set on the shared token_state
/// 5. Tool progress callback is cleared
/// 6. compaction_started + compaction_progress events are emitted
pub(super) fn begin_compaction_recovery<O: StreamOutput>(
    session: &mut Session,
    token_state: &Arc<Mutex<TokenState>>,
    streaming_display: &StreamingTokenDisplay,
    assistant_text: &mut String,
    output: &O,
    pop_user_prompt: bool,
) -> Result<()>;
```

All four paths should call this helper. The behavior matrix then becomes uniform.

## Implementation Steps

1. Add `begin_compaction_recovery` to `stream_loop.rs`.
2. Refactor Path C (hook-cancel, line 1156-1161) to call the helper with `pop_user_prompt=false` (prompt hasn't been consumed by the API yet — it's in session.messages, but compaction instruction will re-embed it).
   - **Wait — this is counter-intuitive.** Actually, when `on_completion_call` cancels BEFORE the API request, the prompt has been pushed to `session.messages` but NOT sent to the LLM. For `execute_compaction` to correctly embed the prompt, it should be popped first so it's passed as `Some(prompt)` argument instead of being in the compactable turn list.
   - Re-examine `execute_compaction` — it calls `reset_session_to_reminders` which wipes all compactable turns anyway. So the push-then-wipe is harmless. The question is purely cosmetic/defensive.
   - **Recommendation:** `pop_user_prompt=true` for Paths B and C (both have the prompt in session.messages but it hasn't been delivered in a completed turn). `pop_user_prompt=false` for Path D (Gemini continuation — prompt is mid-flight, not at tail).

3. Refactor Path B (line 1181-1202) to call the helper.
4. Refactor Path D (Gemini continuation) to call the helper.
5. Path A is structurally different (runs pre-prompt), so it can stay — but extract any shared logic (e.g., `output.emit_compaction_started()`).

## Acceptance Criteria

- All four paths call a single unified helper (or Path A is documented as intentionally pre-prompt).
- The comparison matrix above collapses to a single column: ✅ for every invariant except Path A's pre-prompt case.
- No existing behavior changes for the pre-prompt path.
- Gemini continuation tests still pass.
- A new integration test verifies Path B, C, D all produce identical `session.messages` state after compaction completion.

## Dependencies

- **CMPCT-024** should land first (it establishes the correct invariants for partial-text and token-tracker preservation).
- **CMPCT-026** (single source of truth for cancel detection) is orthogonal but compatible.

## Files to Modify

- `codelet/cli/src/interactive/stream_loop.rs` — lines 1156-1202 (Path C, B)
- `codelet/cli/src/interactive/gemini_continuation.rs` — lines 329-351 (Path D)
- `codelet/cli/src/interactive/compaction_retry.rs` — entry point may need adjustment

## Testing

- New integration test: for each of Path A, B, C, D, assert `session.messages` + `token_tracker` end states are identical given identical starting conditions.
- Existing tests in `gemini_continuation_compaction_test.rs` should be replaced per CMPCT-030.
