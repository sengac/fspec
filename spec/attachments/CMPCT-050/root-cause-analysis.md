# CMPCT-050: Orphan tool_call defeats execute_compaction — Root Cause Analysis

## Symptom (reported in sub-agent session, 2026-09-21)

```
Compaction failed: execute_compaction refuses to proceed: 1 orphan tool_call(s)
without matching tool_result: [call_66016913094b485a8b5b189d] - will retry on next turn
```

The error is surfaced by `rust/cli/src/interactive/recovery_compaction.rs:478` (UI
emission) and `:495` (error propagation). The inner error is the CMPCT-029
defensive guard inside `execute_compaction`
(`rust/cli/src/interactive_helpers.rs:607-618`), which refuses to run when
`session.messages` contains a tool_call without a matching tool_result — because
post-compaction the next API request would violate the provider's tool-pair
invariant (Anthropic: "tool_use block must be followed by tool_result"; OpenAI:
silently inconsistent history).

**The guard is behaving as designed. The bug is upstream: something re-orphaned
a tool_call AFTER the CMPCT-029 cleanup ran.**

## The message path (where the user sees it)

1. `execute_compaction` (`interactive_helpers.rs:586`) — guard at `:607-618`:
   `validate_no_orphan_tool_calls(&session.messages)` returns
   `Err(vec!["call_66016913094b485a8b5b189d"])` → returns
   `anyhow!("execute_compaction refuses to proceed: ...")`.
2. `execute_compaction_and_capture_events` (`recovery_compaction.rs:412-497`):
   - `:477` `Err(e) =>` arm
   - `:478` `output.emit_compaction_failed(&format!("{e} - will retry on next turn"));`
   - `:495` `Err(anyhow::anyhow!("Compaction failed: {e}"))`
3. In the sub-agent (agent-loop) path, `StreamEvent::CompactionFailed { reason }`
   is rendered as `"Compaction failed: {reason}"` at
   `rust/agent-loop/src/background_output.rs:559-568` (warning-severity
   notification). This is exactly the string the user pasted.

Note the NAPI/manual twins produce the same string:
- `rust/sessions/src/handle_impl.rs:454` (manual `/compact`, RPC-418)
- `rust/napi/src/session_bindings.rs:3177` (NAPI `session_compact`)
- `rust/napi/src/agent_loop.rs:1786`, `rust/agent-loop/src/background_output.rs:565`

The "will retry on next turn" suffix is **misleading**: the failure aborts the
turn, and the orphan is *persisted into the session file*. On the next turn the
raw API request itself violates the tool-pair invariant (no recovery path
re-sanitizes before a fresh turn's first request), and `/compact` hits the same
guard — so the session is effectively stuck in a retry-forever loop until the
orphan is manually healed.

## Root cause: the trailing-User pop in `begin_compaction_recovery`

### The recovery order on Path C (PromptCancelled)

`rust/cli/src/interactive/stream_loop.rs:1709-1807` (the
`Some(Err(e)) =>` arm, `CompactionBranch::Recover`):

```
:1707  let branch = classify_compaction_branch(&e, &token_state);
:1711  if matches!(branch, CompactionBranch::Recover { .. }) {
:1730      if let Some(rig_chat_history) = extract_prompt_cancelled(&e) {
:1735          reconcile_session_messages(&mut session.messages, rig_chat_history);   // step 1
:1742      if !tool_calls_buffer.is_empty() {
:1747          add_assistant_tool_calls_message(&mut session.messages, ...)          // step 2
:1751          tool_calls_buffer.clear();
:1754      let injected =
:1755          inject_synthetic_tool_results_for_orphans(&mut session.messages);    // step 3
:1792      let policy = super::recovery_compaction::begin_compaction_recovery(
:1793          session, ..., true, /* pop_user_prompt=true */ )?;                    // step 4 ← POP
:1805      in_loop_compaction_restart!(policy);                                     // step 5 → guard
:1806      continue;
```

Steps 1-3 are the CMPCT-029 cleanup: they prove `session.messages` has **no
orphans** at that point.

### The pop bug

`begin_compaction_recovery` (`recovery_compaction.rs:266-356`), step 1 of its
own (lines `:279-292`):

```rust
if pop_user_prompt {
    if let Some(last) = session.messages.last() {
        if matches!(last, rig::message::Message::User { .. }) {
            session.messages.pop();   // ← removes ANY trailing User message
            ...
```

The intent (documented at `:214-217` and `:274-278`) is to discard the trailing
**user prompt** text message that rig pushed but the API never consumed. But
`matches!(last, Message::User { .. })` matches **any** User message — including
a `User(ToolResult)`.

### The trigger sequence

How a `User(ToolResult)` ends up at the tail when the cancel fires:

1. `handle_tool_call` (`stream_handlers.rs:106-154`) buffers the call into
   `tool_calls_buffer` (NOT yet in `session.messages`).
2. rig executes the tool inside the multi-turn stream; the `ToolResult` chunk is
   delivered as `MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult)`
   and `handle_tool_result` (`stream_handlers.rs:157-242`) appends BOTH the
   buffered call (as Assistant) and the result (as `Message::User {
   content: ToolResult }`) to `session.messages`.
3. The `Usage` item for that segment (`stream_loop.rs` Usage arm) updates
   `display`/`token_state`. If the cumulative input tokens now cross
   `threshold`, the `CompactionHook` (wired via
   `prompt_streaming_with_history_and_hook`) sets
   `token_state.compaction_needed = true` and cancels the in-flight prompt,
   making the stream yield `Err(PromptError::PromptCancelled)`.
4. The next loop iteration's `Some(Err(e))` arm classifies `Recover` and runs
   the Path C sequence above. At step 3 (`inject_synthetic_tool_results_for_orphans`)
   everything is paired — the list is clean.
5. At step 4, the trailing message is that `User(ToolResult)` (not a text
   prompt), so `pop_user_prompt=true` removes it → the tool_call at
   `call_66016913094b485a8b5b189d` is now **orphaned, after cleanup ran**.
6. Step 5's `execute_compaction` guard detects the orphan and refuses.

Sub-agents hit this far more than interactive sessions because they are
tool-call-dense: every tool result is a `User(ToolResult)` at the tail, and a
single usage update after a big tool result (e.g. a large file read or a
long research dump) is enough to cross the threshold. That is why the user saw
it "in the other agent."

## Secondary issues (same bug family, other entry paths)

The CMPCT-029 cleanup block (reconcile + drain + inject) exists **only** in the
Path C arm. Every other compaction entry path goes straight from
`begin_compaction_recovery` into `in_loop_compaction_restart!` /
`execute_compaction` with no orphan cleanup:

| Path | Location | Orphan cleanup? |
|------|----------|-----------------|
| C (PromptCancelled) | `stream_loop.rs:1711-1807` | ✅ yes (but defeated by the pop, see above) |
| B ("prompt too long") | `stream_loop.rs:1832-1876` | ❌ none |
| CMPCT-044 overflow | `stream_loop.rs:1894-1931` | ❌ none |
| CMPCT-032 FinalResponse clean-exit | `stream_loop.rs:1465-1491` | ❌ none |
| D (Gemini continuation) | `gemini_continuation.rs:403-431` → `stream_loop.rs:1235-1267` | ❌ none (continuation loop's `handle_tool_call`/`handle_tool_result` share the same buffering; a PromptCancelled mid-continuation carries the same risk) |
| Manual `/compact` (RPC) | `sessions/src/handle_impl.rs:408-497` (calls `execute_compaction` directly, `None` prompt) | ❌ none — guard fires on `compact_session` |
| Manual `/compact` (NAPI) | `napi/src/session_bindings.rs:~3177` | ❌ none — same |

Any orphan that ever reaches `session.messages` through a non-Path-C trigger
will hit the guard with the same user-visible error.

Additional latent orphan sources to keep in mind (do not need fixing for this
card, but relevant context):

- The interrupt path (`stream_loop.rs:761-778`) breaks out mid-tool-call with
  `tool_calls_buffer` still holding calls that were never flushed to
  `session.messages` — the calls are dropped, so no orphan, but the turn state
  is inconsistent.
- The PROV-040 truncated-tool-call recovery (`stream_loop.rs:1964+`) saves
  partial assistant text but does NOT drain `tool_calls_buffer` before the
  recovery prompt is pushed as a `User` message — if a truncated tool_call was
  buffered, the next `handle_tool_result`/turn may leave an orphan behind.
  (Worth a follow-up audit, not in scope here.)

## Evidence: the guard and the helpers (CMPCT-029)

- `validate_no_orphan_tool_calls` — `interactive_helpers.rs:326-361`.
  Correlates via `tool_call_correlation_key(id, call_id)` (`:303-308`):
  `call_id` if present, else `id`.
- `reconcile_session_messages` — `interactive_helpers.rs:382-486`.
  Drains tool pairs from rig's `PromptCancelled` chat_history (the rig patch
  at streaming.rs cancel-site 508 flushes pending pairs there).
- `inject_synthetic_tool_results_for_orphans` — `interactive_helpers.rs:517-568`.
  Appends `User(ToolResult)` with body
  `{"status":"cancelled_by_context_limit"}` (`SYNTHETIC_TOOL_CANCEL_BODY`,
  `:497`) for every orphan. Never removes/reorders existing messages.
- `execute_compaction` guard — `interactive_helpers.rs:600-618`.

Existing test coverage (all PASS as-is; none cover the pop-regression shape):

- `rust/cli/tests/compaction_tool_call_preservation_test.rs`
  (CMPCT-029 scenarios, incl. "execute_compaction refuses to run when orphan
  tool_calls remain" at `:344`).
- Feature file: `spec/features/preserve-mid-tool-call-state-when-promptcancelled-fires.feature`
  (scenario "execute_compaction refuses to run when orphan tool_calls remain
  and reports the offending call_ids" at `:71`).
- `spec/attachments/CMPCT-029/plan.md` documents the original reconcile design.

The gap: no test exercises "PromptCancelled fires with a `User(ToolResult)` at
the tail of `session.messages`" — which is precisely the shape that
`pop_user_prompt=true` destroys.

## Proposed fix (for the implementing agent)

Three-part fix, one card:

1. **Restrict the pop to plain prompts.**
   In `begin_compaction_recovery` (`recovery_compaction.rs:279-292`), only pop
   when the trailing `Message::User` contains **no** `UserContent::ToolResult`
   items (i.e. it is a text prompt, possibly with images/reminders). A
   trailing `User(ToolResult)` is mid-turn conversation state and must stay.

2. **Move the guarantee to the choke point.**
   The reconcile/drain/inject cleanup must run on EVERY entry path, not just
   Path C. The minimal-risk shape: re-run
   `inject_synthetic_tool_results_for_orphans` as the final step of
   `begin_compaction_recovery` (after the pop), so Paths B/C/D/032/overflow
   all get the guarantee; Path C's explicit reconcile stays where it is
   (it needs the `PromptCancelled` payload that only the error chain carries).
   The guard in `execute_compaction` then remains a true last-line assertion.
   For the manual `/compact` paths (`handle_impl.rs:408`, NAPI twin), a
   pre-flight sanitize (or the same inject call) before `execute_compaction`
   closes the loop; at minimum those paths should emit a distinct, actionable
   message instead of the generic one.

3. **Regression test (ACDD: feature file + test first).**
   - Feature scenario: "Given session.messages ends with User(ToolResult) for
     call X and the matching Assistant(ToolCall) for call X earlier, when
     begin_compaction_recovery runs with pop_user_prompt=true, then the
     User(ToolResult) is preserved and validate_no_orphan_tool_calls passes."
   - Unit test in `recovery_compaction` (or a new `tests/*.rs` using
     `test-helpers`): construct the message list, call
     `begin_compaction_recovery`, assert tail is still the ToolResult and
     `validate_no_orphan_tool_calls(&session.messages) == Ok(())`.
   - Second scenario: trailing User **text** prompt is still popped (no
     behavior change for the original case).

## Out of scope (noted for follow-up cards)

- PROV-040 truncated-tool-call path `tool_calls_buffer` drain (see above).
- Interrupt-path turn-state inconsistency (dropped buffered calls).
- The misleading "will retry on next turn" suffix in
  `recovery_compaction.rs:478` — the turn does NOT retry; consider rewording
  to "compaction deferred — orphan tool_call(s) preserved; will re-sanitize on
  next compaction attempt" once the fix lands.

## Files referenced

- `rust/cli/src/interactive/stream_loop.rs` (Path C: 1709-1807; Path B: 1832-1876; overflow: 1894-1931; CMPCT-032: 1465-1491; macro: 640-756)
- `rust/cli/src/interactive/recovery_compaction.rs` (begin_compaction_recovery: 266-356; emit/error: 477-495)
- `rust/cli/src/interactive_helpers.rs` (guard: 600-618; helpers: 303-568)
- `rust/cli/src/interactive/gemini_continuation.rs` (Path D: 403-431; primary-loop arm: stream_loop.rs:1235-1267)
- `rust/cli/src/interactive/stream_handlers.rs` (handle_tool_call: 106-154; handle_tool_result: 157-242)
- `rust/agent-loop/src/background_output.rs` (CompactionFailed render: 559-568)
- `rust/sessions/src/handle_impl.rs` (manual /compact: 408-497)
- `rust/napi/src/session_bindings.rs` (NAPI /compact: ~3177)
- `rust/cli/tests/compaction_tool_call_preservation_test.rs` (existing CMPCT-029 tests)
- `spec/features/preserve-mid-tool-call-state-when-promptcancelled-fires.feature`
- `spec/attachments/CMPCT-029/plan.md` (original design)
