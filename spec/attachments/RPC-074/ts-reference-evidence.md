# RPC-074 — TS reference behaviour evidence

## Evidence 1 — TS `handleClearCommand` (the source of truth)

File: `src/tui/components/AgentView.tsx`
Lines: 1554-1564

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

## Evidence 2 — TS `/clear` dispatch sites

File: `src/tui/components/AgentView.tsx`

**Site A** (line 1621-1624):

```ts
if (userMessage === '/clear') {
  handleClearCommand();
  return;
}
```

**Site B** (line 2730-2733):

```ts
if (userMessage === '/clear') {
  handleClearCommand();
  return;
}
```

Both dispatch sites call `handleClearCommand` and return. **No
conversation entry is added** before, during, or after the call.

## Evidence 3 — TS chunk subscriber for `SessionStateChange`

File: `src/tui/components/AgentView.tsx`
Lines: 939, 987 (`// TUI-066: Handle SessionStateChange with Cleared state`)

The reactive piece — what makes the UI clear after the backend has
actually cleared — is the `SessionStateChange { state: Cleared }`
chunk subscriber. The TS contract is:

```
user types /clear
  → handleClearCommand fires
    → setInputValue('') (input clears immediately)
    → sessionClearHistory(currentSessionId) (async call to Rust)
  → Rust clear_history finishes
    → broadcasts StreamChunk::SessionStateChange { state: Cleared }
  → TS subscriber receives chunk
    → setConversation([]) (scrollback clears as side effect)
```

**Crucially**: there is no `setConversation(prev => [...prev, { type: 'status', content: 'history cleared' }])`
anywhere in this path. The user never sees a "history cleared" line.

## Evidence 4 — Absence of the literal string in TS

```bash
$ rg -i 'history cleared|message history cleared|messages cleared' src/
# (zero matches)
```

The string `"history cleared"` does not appear anywhere in
`src/tui/components/AgentView.tsx`, in any handler, in any subscriber,
or in any persisted-conversation utility.

## Evidence 5 — Rust divergence

File: `codelet/fspec-tui/src/app/dispatch_rpc046.rs`
Line: 59

```rust
Ok(()) => "[notice] /clear: history cleared".to_string(),
```

File: `codelet/core/src/session_manager_handle.rs`
Line: 1509

```rust
"history cleared".to_string(),
```

Both strings are pure Rust-side invention. Neither has a counterpart in
the TS reference.

## Evidence 6 — User-facing impact (screenshot)

`spec/attachments/RPC-073/screenshot-clear-panic.png` (pre-RPC-073 fix)
captures the runtime panic. After RPC-073 the panic is gone but the
synthetic `[notice] /clear: history cleared` line is still inserted
into scrollback — visible in any post-RPC-073 build of
`./codelet/target/release/fspec` after typing `/clear`.

## Evidence 7 — User directive (verbatim)

> `"message history cleared" with /clear IS NOT THE FUNCTIONALITY OF
> TYPESCRIPT IMPLEMENTATION! YOU MUST COPY THE EXACT WAY IT WORKS - NOT
> MAKE SHIT UP!!!`
> — 2026-05-27 12:03

The user observed the synthetic notice line in a Rust binary built from
the codelet-integration branch and explicitly rejected it as
non-conforming with the TS reference.

## Conclusion

The Rust `/clear` slash-command path must be reduced to exactly what
the TS implementation does:

1. Clear the input field
2. Call `backend.clear_history(session_id)`
3. Let the `SessionStateChange { state: Cleared }` chunk drive any
   downstream UI updates
4. On error, log via `tracing::error`, **do not** push to scrollback

Anything else (synthetic notice lines, `UserNotification` chunks
specifically for `/clear`, error strings pushed to conversation) is
divergence and must be removed.
