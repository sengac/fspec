# RPC-074 — `/clear` emits TS-divergent scrollback notice

## Summary

The Rust `fspec` binary's `/clear` slash command pushes a synthetic
`[notice] /clear: history cleared` (Ok path) or `[error] /clear failed: <e>`
(Err path) line into the focused session's scrollback. The reference
TypeScript implementation does no such thing — it only blanks the input and
calls `sessionClearHistory(currentSessionId)`. The scrollback reset is a
side effect of the Rust `clear_history()` call broadcasting a
`StreamChunk::SessionStateChange { state: Cleared }` chunk (TUI-066
contract), NOT of a synchronous string push from the dispatcher.

This divergence was codified in **RPC-046** (2026-05-22) — the very first
`/clear` wiring — and has been propagated by every cross-transport-parity
and slash-clear test written since.

## Reference: TS behaviour (the contract we must match)

`src/tui/components/AgentView.tsx:1554-1564`:

```ts
// TUI-066: Shared handler for /clear command - clears session history
const handleClearCommand = useCallback(() => {
  setInputValue('');
  if (currentSessionId) {
    try {
      sessionClearHistory(currentSessionId);
    } catch (err) {
      logger.error('[AgentView] Failed to clear session history:', err);
    }
  }
}, [currentSessionId]);
```

**Observations:**

- No success notice pushed into conversation/scrollback
- No error notice pushed into conversation/scrollback — errors go to
  the logger only
- `setConversation` is NOT called from this handler — the scrollback
  reset comes from the `SessionStateChange { state: Cleared }` chunk
  flowing back through the chunks subscriber

`grep -RIn '"history cleared"\|"messages cleared"\|"message history cleared"' src/`
returns **zero matches**. The string does not exist anywhere in the TS
source.

## Reference: Rust divergence (the bug)

`codelet/fspec-tui/src/app/dispatch_rpc046.rs:35-65`:

```rust
pub(crate) fn handle_slash_clear(&mut self) {
    self.navigator
        .agent
        .reset_scrollback(&mut self.agent_view_store);
    let Some(session_id) = self.agent_view_store.current_session().cloned() else {
        return;
    };
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    let backend = self.backend.clone();
    let action_tx = self.action_tx.clone();
    let handle = tokio::spawn(async move {
        let text = match backend.clear_history(session_id.clone()).await {
            Ok(()) => "[notice] /clear: history cleared".to_string(),
            Err(e) => format!("[error] /clear failed: {e}"),
        };
        let _ = action_tx.send(Action::EmitSessionNotice(session_id, text));
    });
    self.pending_tasks.push(handle);
}
```

**Two problems vs. the TS contract:**

1. The Ok arm pushes `[notice] /clear: history cleared` via
   `Action::EmitSessionNotice` → `handle_emit_session_notice` →
   `ctx.push_line(text)`. The TS version does not produce this line.
2. The Err arm pushes `[error] /clear failed: <e>` into scrollback.
   The TS version sends errors to the logger (`logger.error`), never
   to the user-visible conversation.

Additionally, the Rust `handle_slash_clear` resets the scrollback
**synchronously** in the dispatcher (line 47-48) rather than waiting
for the `StreamChunk::SessionStateChange { state: Cleared }` broadcast.
The TS contract is "the chunk drives the UI state change" — see
`AgentView.tsx:987` ("Handle SessionStateChange with Cleared state").

## Reference: tests that codify the wrong contract

`codelet/fspec-tui/tests/slash_clear_rpc046.rs`:

- Lines 172-177: asserts `[notice] /clear: history cleared` line exists
  in s-1 scrollback
- Lines 287-292: same assertion in multi-session focus-isolation test
- Lines 296-301: asserts s-2's scrollback does NOT contain the same
  line (i.e. "the notice only goes to the originating session")

`codelet/fspec-tui/tests/rpc037_cross_transport_parity.rs`:

- Lines 151-205: subscribes to `StreamChunk::UserNotification` chunks
  for `(SessionId, "history cleared")` and asserts both embedded and
  websocket transports observe the chunk

These tests must be retired/rewritten as part of RPC-074 because they
encode behaviour that does not match the TypeScript reference.

## Reference: backend-side `UserNotification` chunk

`codelet/core/src/session_manager_handle.rs:1509`:

```rust
"history cleared".to_string(),
```

This is inside `clear_history()` — the backend broadcasts a
`StreamChunk::UserNotification { session_id, message: "history cleared" }`
when `/clear` succeeds. The TS reference does NOT emit a
`UserNotification` chunk for `/clear` — `sessionClearHistory` only
emits `SessionStateChange { state: Cleared }` and the React component
re-renders on that. The `UserNotification("history cleared")` chunk is
itself a TS-divergence that landed in RPC-037's cross-transport-parity
proof — see `rpc037_cross_transport_parity.rs:151-205`.

## Scope

This card fixes both layers:

1. **TUI dispatch layer** (`codelet/fspec-tui/src/app/dispatch_rpc046.rs`):
   remove the `Action::EmitSessionNotice` push entirely. The scrollback
   reset already happens via `navigator.agent.reset_scrollback`, which
   matches the TS `setConversation([])` path. Errors must be logged via
   `tracing::error`, not pushed to scrollback.

2. **Backend chunk-emission layer**
   (`codelet/core/src/session_manager_handle.rs:1509`): remove the
   `StreamChunk::UserNotification("history cleared", ...)` broadcast.
   The `SessionStateChange { state: Cleared }` chunk is sufficient
   (and is what TS actually uses).

3. **Test-suite layer**: rewrite the three impacted tests in
   `slash_clear_rpc046.rs` and `rpc037_cross_transport_parity.rs` so
   they assert the TS-equivalent contract instead — the
   `SessionStateChange { state: Cleared }` chunk arrives, scrollback
   resets, no notice line appears.

## Affected files (estimated)

| Path | Change |
|---|---|
| `codelet/fspec-tui/src/app/dispatch_rpc046.rs` | Remove `tokio::spawn` + `Action::EmitSessionNotice` push; await `clear_history` inline (or fire-and-forget) without producing scrollback text |
| `codelet/core/src/session_manager_handle.rs` | Remove the `UserNotification("history cleared", ...)` broadcast inside `clear_history()` |
| `codelet/fspec-tui/tests/slash_clear_rpc046.rs` | Rewrite the 6 RPC-046 scenarios to assert chunk-driven contract |
| `codelet/fspec-tui/tests/rpc037_cross_transport_parity.rs` | Replace `UserNotification("history cleared")` subscription with `SessionStateChange { state: Cleared }` subscription |
| `spec/features/slash-command-clear.feature` | Update steps that mention `[notice] /clear: history cleared` |
| `spec/features/rpc037-cross-transport-parity.feature` | Same |

## Why this matters

> "**MUST COPY THE EXACT WAY IT WORKS — NOT MAKE SHIT UP!!!**"
> — user, 2026-05-27

The whole point of the Rust port is byte-for-byte behavioural parity
with the TS reference. A user typing `/clear` in the TS TUI gets:

- input cleared
- conversation list emptied
- (no message added)

A user typing `/clear` in the Rust TUI currently gets:

- input cleared
- conversation list emptied
- **an extra `[notice] /clear: history cleared` line that does not
  exist in the TS version**

This is invented behaviour. RPC-074 removes it.
