# CMPCT-050: Code Evidence (verbatim excerpts, 2026-09-21)

All line numbers from `git` working tree at time of diagnosis.

## E1 — The user-visible error (emission site)

`rust/cli/src/interactive/recovery_compaction.rs:450-496`

```rust
    match execute_compaction(session, compaction_in_progress, Some(prompt)).await {
        Ok(()) => {
            ...
            output.emit_compaction_continuing();
            session.token_tracker.reset_after_compaction();
            Ok(())
        }
        Err(e) => {
            output.emit_compaction_failed(&format!("{e} - will retry on next turn"));
            ...
            Err(anyhow::anyhow!("Compaction failed: {e}"))
        }
    }
```

Sub-agent rendering: `rust/agent-loop/src/background_output.rs:559-568`

```rust
            StreamEvent::CompactionFailed { reason } => {
                self.session.set_status(SessionStatus::Idle);
                ...
                StreamChunk::user_notification(
                    format!("Compaction failed: {reason}"),
                    NotificationSeverity::Warning,
                )
            }
```

## E2 — The guard that refuses (CMPCT-029)

`rust/cli/src/interactive_helpers.rs:600-618`

```rust
    // CMPCT-029: defensive guard — refuse to run when session.messages still
    // contains orphan tool_calls. Recovery paths (stream_loop.rs Path C)
    // must have already reconciled with rig's PromptCancelled chat_history
    // and injected synthetic cancelled tool_results for any remaining
    // dangling calls. If we reach here with orphans still present, the
    // compaction input would corrupt the next API request's tool-pair
    // invariant — fail loudly instead of proceeding.
    if let Err(orphans) = validate_no_orphan_tool_calls(&session.messages) {
        warn!(
            orphan_count = orphans.len(),
            orphan_call_ids = ?orphans,
            "[execute_compaction] Orphan tool_calls detected — refusing to compact"
        );
        return Err(anyhow::anyhow!(
            "execute_compaction refuses to proceed: {} orphan tool_call(s) without matching tool_result: [{}]",
            orphans.len(),
            orphans.join(", ")
        ));
    }
```

## E3 — Path C: cleanup runs FIRST (stream_loop.rs:1711-1762)

```rust
                    if matches!(branch, CompactionBranch::Recover { .. }) {
                        // CMPCT-029: reconcile in-flight tool state BEFORE
                        // begin_compaction_recovery runs. The recovery order is:
                        //   1. If PromptCancelled carries a rig chat_history,
                        //      merge any tool_call/tool_result pairs rig has
                        //      but fspec's session.messages doesn't yet.
                        //   2. Drain fspec's tool_calls_buffer into
                        //      session.messages (Path 486 recovery — the hook
                        //      cancelled BEFORE the tool executed, so there
                        //      is no result to merge; the call is still in
                        //      the buffer from handle_tool_call).
                        //   3. Close any remaining orphan tool_calls with a
                        //      synthetic "cancelled_by_context_limit" result.
                        //
                        // After this, execute_compaction's defensive orphan
                        // guard will pass cleanly — regardless of which cancel
                        // site rig fired at.
                        if let Some(rig_chat_history) = extract_prompt_cancelled(&e) {
                            ...
                            reconcile_session_messages(&mut session.messages, rig_chat_history);
                        } else {
                            ...
                        }

                        if !tool_calls_buffer.is_empty() {
                            ...
                            add_assistant_tool_calls_message(
                                &mut session.messages,
                                tool_calls_buffer.clone(),
                            )?;
                            tool_calls_buffer.clear();
                        }

                        let injected =
                            inject_synthetic_tool_results_for_orphans(&mut session.messages);
                        if injected > 0 {
                            warn!(...);
                        }
```

NOTE the comment at the bottom: "After this, execute_compaction's defensive
orphan guard will pass cleanly" — this invariant is what E4 breaks.

## E4 — THE BUG: pop of ANY trailing User message (recovery_compaction.rs:274-292)

```rust
    // Step 1: pop last user message when it is at the tail of session.messages
    // but has not been consumed by the API yet (Paths B and C). Path D passes
    // false because the continuation prompt is mid-flight. This MUST happen
    // before flushing partial text, otherwise the appended Assistant ends up
    // at the tail and the pop becomes a no-op.
    if pop_user_prompt {
        if let Some(last) = session.messages.last() {
            if matches!(last, rig::message::Message::User { .. }) {
                session.messages.pop();
                debug!(
                    "[begin_compaction_recovery] Popped trailing User message (pop_user_prompt=true)"
                );
            } else {
                debug!(
                    "[begin_compaction_recovery] pop_user_prompt=true but tail is not a User message; leaving messages unchanged"
                );
            }
        }
    }
```

`matches!(last, Message::User { .. })` matches a `User(ToolResult)` exactly the
same as a `User(text prompt)`.

Then at `stream_loop.rs:1792` the same arm calls:

```rust
                        let policy = super::recovery_compaction::begin_compaction_recovery(
                            session,
                            &token_state,
                            &streaming_display,
                            &mut assistant_text,
                            output,
                            true,
                        )?;
```

→ tail `User(ToolResult)` popped → call orphaned → E2 guard fires.

## E5 — How the tail becomes a ToolResult (stream_handlers.rs:157-177)

```rust
/// Handle tool result - adds to messages, emits via output
pub(super) fn handle_tool_result<O: StreamOutput>(
    tool_result: &rig::message::ToolResult,
    messages: &mut Vec<rig::message::Message>,
    tool_calls_buffer: &mut Vec<rig::message::AssistantContent>,
    last_tool_name: &Option<String>,
    output: &O,
) -> Result<()> {
    ...
    // CRITICAL: Add buffered tool calls as assistant message (CLI-008) - shared
    if !tool_calls_buffer.is_empty() {
        add_assistant_tool_calls_message(messages, tool_calls_buffer.clone())?;
        tool_calls_buffer.clear();
    }

    // CRITICAL: Add tool result to message history (CLI-008) - shared
    let tool_result_clone = tool_result.clone();
    messages.push(Message::User {
        content: OneOrMany::one(UserContent::ToolResult(tool_result_clone)),
    });
```

Every tool result leaves `session.messages` ending in `User(ToolResult)`. If
the CompactionHook cancels on the following usage update (Path C), that result
is the tail when E4 runs.

## E6 — Other entry paths with NO orphan cleanup

Path B ("prompt too long"), `stream_loop.rs:1832-1876`:

```rust
                    if is_prompt_too_long && has_compactable_turns {
                        ...
                        let policy = super::recovery_compaction::begin_compaction_recovery(
                            session, &token_state, &streaming_display,
                            &mut assistant_text, output, true,
                        )?;
                        ...
                        in_loop_compaction_restart!(policy);
                        continue;
                    }
```

Overflow (CMPCT-044), `stream_loop.rs:1896-1931`: same shape,
`in_loop_compaction_restart!(policy, compaction_retry_count > 0)`.

CMPCT-032 clean-exit, `stream_loop.rs:1465-1491`: same shape,
`pop_user_prompt=false` (but a persisted orphan from a PRIOR turn still trips
the guard on this path too).

Path D (Gemini continuation), `gemini_continuation.rs:403-431`:
`begin_compaction_recovery(..., false)` then
`return Ok(GeminiContinuationResult::CompactionNeeded(policy))` →
`stream_loop.rs:1235-1267` → `in_loop_compaction_restart!(policy)`.

Manual `/compact`, `rust/sessions/src/handle_impl.rs:448-455`:

```rust
                if let Err(e) =
                    execute_compaction(&mut inner, session.compaction_in_progress.clone(), None)
                        .await
                {
                    session.set_compaction_progress(None);
                    session.set_status(SessionStatus::Idle);
                    return Err(format!("Compaction failed: {e}"));
                }
```

NAPI twin: `rust/napi/src/session_bindings.rs:3177`
(`return Err(Error::from_reason(format!("Compaction failed: {e}")));`).

## E7 — Correlation key the guard uses (interactive_helpers.rs:303-308)

```rust
pub fn tool_call_correlation_key(id: &str, call_id: Option<&str>) -> String {
    match call_id {
        Some(cid) => cid.to_string(),
        None => id.to_string(),
    }
}
```

The reported orphan id `call_66016913094b485a8b5b189d` is a provider tool-call
id of exactly this shape (OpenAI-style `call_` prefix), consistent with an
OpenAI-compatible provider sub-agent whose tool result was popped by E4.

## E8 — Existing tests (pass as-is; none cover the E4+E5 shape)

- `rust/cli/tests/compaction_tool_call_preservation_test.rs`
  - `:168` "precondition: exactly one orphan tool_call must be present"
  - `:344` "Scenario: execute_compaction refuses to run when orphan tool_calls remain"
- `spec/features/preserve-mid-tool-call-state-when-promptcancelled-fires.feature`
  - `:71` "Scenario: execute_compaction refuses to run when orphan tool_calls remain and reports the offending call_ids"
- `spec/attachments/CMPCT-029/plan.md:129` — original design intent:
  "The `reconcile_session_messages` helper detects orphan tool_calls in
  `session.messages` (calls without results), looks them up in
  `rig_chat_history`, and pulls in the matching tool_result."

None of these construct a `session.messages` whose tail is
`User(ToolResult)` at the moment `begin_compaction_recovery(pop_user_prompt=true)`
runs — the exact regression shape of this bug.
