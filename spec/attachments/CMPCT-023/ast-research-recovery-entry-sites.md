# AST Research — Compaction Recovery Entry Sites (CMPCT-023)

## Goal

Enumerate every compaction-recovery entry path and the invariants they currently guarantee, so a single unified helper `begin_compaction_recovery<O: StreamOutput>` can subsume them.

## AST scan: `output.emit_compaction_started()`

```
codelet/cli/src/interactive/gemini_continuation.rs:355   Path D — Gemini continuation cancel
codelet/cli/src/interactive/stream_loop.rs:333           Path A — pre-prompt compaction (structurally separate)
codelet/cli/src/interactive/stream_loop.rs:1215          Path B — prompt-too-long recovery
codelet/cli/src/interactive/compaction_retry.rs:59       POST-LOOP — handle_compaction_retry
```

Double-emit: Path D emits at 355, then `compaction_retry.rs:59` emits again inside `handle_compaction_retry`. Path B emits at 1215, then `compaction_retry.rs:59` emits again. Path C (hook-cancel, stream_loop.rs:1156) does NOT emit at the break site — only `compaction_retry.rs:59` emits.

**Strategy:** keep emission at the call site (B, C, D — via helper) and REMOVE the emission in `compaction_retry.rs::handle_compaction_retry`. Path A is structurally distinct (pre-prompt) and keeps its own emission.

## AST scan: `signal_compaction_needed($TOKEN_STATE)`

```
codelet/cli/src/interactive/stream_loop.rs:1230   Path B — explicit set
codelet/cli/src/interactive/gemini_continuation.rs:353  Path D — explicit set (post-CMPCT-026 also defensive)
```

Path C (hook-cancel) does NOT call this — the hook itself already set `compaction_needed=true`, and CMPCT-026's `classify_compaction_branch` defensively sets it if somehow missed. The unified helper will also set the flag (warn if already set) for uniform defense-in-depth.

## AST scan: `set_tool_progress_callback(uuid::Uuid::nil(), None)`

```
gemini_continuation.rs:203, 325, 358, 362, 381   Various exit points
compaction_retry.rs:374                            Post-retry cleanup
stream_loop.rs:1177                                Path C (CMPCT-024 addition)
stream_loop.rs:1528                                POST-LOOP top-level
```

Path B currently does NOT clear the callback at the break site; it relies on the post-loop at line 1528 OR the compaction_retry cleanup at compaction_retry.rs:374. The unified helper will clear at every entry for uniformity.

## Path Invariant Matrix (pre-CMPCT-023)

| Step                           | Path A | Path B | Path C | Path D |
|--------------------------------|--------|--------|--------|--------|
| Pop last user message          | N/A    | ✅     | ❌     | N/A    |
| Save partial assistant text    | N/A    | ❌     | ✅¹    | ✅     |
| Update token tracker           | N/A    | ❌     | ✅¹    | ✅     |
| Emit compaction-started event  | ✅     | ✅     | ❌²    | ✅     |
| Clear tool progress callback   | N/A    | ❌     | ✅¹    | ✅     |
| Signal compaction_needed flag  | ✅     | ✅     | ✅³    | ✅     |

¹ Added by CMPCT-024 (`flush_partial_state_before_compaction` + explicit progress-cb clear).
² Emitted LATER by `handle_compaction_retry`.
³ Set by hook itself (pre-existing); CMPCT-026 `classify_compaction_branch` defensively sets if missing.

## Target Invariant Matrix (post-CMPCT-023)

All rows ✅ for Paths B, C, D after unified helper. `handle_compaction_retry` no longer emits the lifecycle events (caller guarantees emission). Path A stays separate.

## Signature

```rust
pub(super) fn begin_compaction_recovery<O: StreamOutput>(
    session: &mut Session,
    token_state: &Arc<Mutex<TokenState>>,
    streaming_display: &StreamingTokenDisplay,
    assistant_text: &mut String,
    output: &O,
    pop_user_prompt: bool,
) -> Result<()>;
```

## Internal step order (chosen for safety)

1. `flush_partial_state_before_compaction(session, assistant_text, display)?`
   — saves partial text (if any), flushes token tracker. Reuses CMPCT-024 helper.
2. Pop last user message IFF `pop_user_prompt && matches!(last, Message::User { .. })`.
3. Lock `token_state`, warn if `compaction_needed` was already true (disagreement), then set to true.
4. `set_tool_progress_callback(Uuid::nil(), None);`
5. `output.emit_compaction_started();`
   `output.emit_compaction_progress("Context limit reached", 0, total_turns.max(1));`
   where `total_turns = session.messages.len() as u32 / 2`.

Note: popping happens BEFORE the total_turns calculation, so the popped prompt is not counted.

## Files to Modify

| File | Change |
|------|--------|
| `codelet/cli/src/interactive/recovery_compaction.rs` | Add `begin_compaction_recovery<O>`; keep `flush_partial_state_before_compaction` as internal step |
| `codelet/cli/src/interactive/stream_loop.rs` | Path B (≈1213-1232) and Path C (≈1156-1188) both call helper |
| `codelet/cli/src/interactive/gemini_continuation.rs` | Path D (≈340-359) calls helper |
| `codelet/cli/src/interactive/compaction_retry.rs` | REMOVE the `emit_compaction_started` + first `emit_compaction_progress` at lines 59-61 |
| `codelet/cli/src/interactive/mod.rs` | `pub use ...::begin_compaction_recovery;` for integration test visibility |

## Out of Scope

- CMPCT-027 Option B restructuring of compaction_retry.rs into the primary loop
- Path A pre-prompt consolidation
