# RPC-074 — Fix plan

## Approach

Three-layer change, in dependency order:

1. **Layer 1 — Backend**: stop emitting the `UserNotification("history cleared")` chunk
2. **Layer 2 — TUI dispatch**: stop pushing the synthetic notice line into scrollback
3. **Layer 3 — Tests + features**: rewrite the assertions that codify the divergence

## Layer 1 — Backend (`codelet/core/src/session_manager_handle.rs`)

### Current (line ~1509)

```rust
// Broadcast a UserNotification chunk so RPC-037 cross-transport-parity
// observers see the clear event on chunks_rx.
let _ = self.chunks_tx.send(StreamChunk::UserNotification {
    session_id: id.clone(),
    message: "history cleared".to_string(),
});
```

### Target

Remove the entire `chunks_tx.send(UserNotification ...)` block. The
existing `SessionStateChange { state: Cleared }` broadcast (which lives
nearby in the same function — verify line number during implementation)
is the only chunk that should fire.

### Verification

- `grep -RIn '"history cleared"' codelet/` → zero matches
- `cargo test -p codelet-core` → still green
- New unit test: subscribe to `chunks_rx`, call `clear_history`, assert
  exactly **one** chunk arrives and it is `SessionStateChange { state: Cleared }`
  (not `UserNotification`)

## Layer 2 — TUI dispatch (`codelet/fspec-tui/src/app/dispatch_rpc046.rs`)

### Current

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

### Target

```rust
pub(crate) fn handle_slash_clear(&mut self) {
    // TS parity (AgentView.tsx:1554-1564): clear the input + scrollback
    // locally, then call backend.clear_history. NO notice is pushed —
    // the backend's SessionStateChange { state: Cleared } chunk drives
    // any further UI updates. Errors go to tracing, not scrollback.
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
    let handle = tokio::spawn(async move {
        if let Err(e) = backend.clear_history(session_id.clone()).await {
            tracing::error!(?session_id, error = %e, "/clear: backend.clear_history failed");
        }
    });
    self.pending_tasks.push(handle);
}
```

**Note**: `handle_emit_session_notice` itself can stay — it's still
used by other slash commands. Only the `/clear` caller stops emitting
the notice.

### Verification

- `grep -RIn '"\[notice\] /clear\|"\[error\] /clear failed' codelet/` → zero matches
- `cargo build --release -p codelet-fspec` → clean
- `cargo clippy -p codelet-fspec-tui` → clean

## Layer 3a — Tests (`codelet/fspec-tui/tests/slash_clear_rpc046.rs`)

The current 6 RPC-046 scenarios assert variations of:

```rust
assert!(
    s1.scrollback().lines().any(|l| l == "[notice] /clear: history cleared"),
    "expected `[notice] /clear: history cleared` line in s-1 scrollback, got {text:?}",
);
```

These all become:

```rust
// TS parity: scrollback after /clear must be empty; no synthetic notice.
let lines: Vec<_> = s1.scrollback().lines().collect();
assert!(lines.is_empty(), "expected empty scrollback after /clear, got {lines:?}");
```

Plus a new chunk-arrival assertion using `MockBackend`'s chunk recorder:

```rust
// The backend emits exactly one SessionStateChange { state: Cleared }
// chunk for s-1 — no UserNotification chunks.
let chunks = mock_backend.chunks_sent_to(s1_id);
assert_eq!(chunks.len(), 1);
assert!(matches!(chunks[0], StreamChunk::SessionStateChange {
    ref state, ..
} if *state == SessionState::Cleared));
```

## Layer 3b — Tests (`codelet/fspec-tui/tests/rpc037_cross_transport_parity.rs`)

Lines 151-205 subscribe to:

```rust
StreamChunk::UserNotification { session_id, message }
    if got == *sid && message.contains("history cleared") => ...
```

Replace with:

```rust
StreamChunk::SessionStateChange { session_id, state: SessionState::Cleared }
    if got == *sid => ...
```

## Layer 3c — Feature files

`spec/features/slash-command-clear.feature`: remove every step that
references the literal `[notice] /clear: history cleared` line. The
remaining scenarios still cover:

- The synchronous local scrollback reset
- `backend.clear_history` being called with the correct session_id
- The no-op behaviour when no current session
- Multi-session focus-isolation (s-1's clear doesn't touch s-2)

Add new step assertions for the `SessionStateChange { state: Cleared }`
chunk arrival (chunk-driven contract).

`spec/features/rpc037-cross-transport-parity.feature`: replace the
`UserNotification("history cleared")` step with a
`SessionStateChange { state: Cleared }` step.

## Verification checklist

- [ ] `grep -RIn '"history cleared"' codelet/ src/` → 0 matches
- [ ] `grep -RIn '\[notice\] /clear' codelet/ src/` → 0 matches
- [ ] `cargo test -p codelet-core --test session_manager_handle_*` → passes
- [ ] `cargo test -p codelet-fspec-tui --test slash_clear_rpc046` → passes
- [ ] `cargo test -p codelet-fspec-tui --test rpc037_cross_transport_parity` → passes
- [ ] `cargo build --release -p codelet-fspec` → clean
- [ ] Manual: build release binary, open Work Agent, type `/clear`,
      confirm scrollback empties with NO additional notice line
- [ ] `fspec validate` on the updated feature files → passes
- [ ] `fspec show-coverage` for the updated features → 100%

## Source-shape regression

Add a test in `codelet/sessions/tests/skeleton_invariants.rs` (or a
dedicated `rpc074_no_history_cleared_string.rs`):

```rust
#[test]
fn scenario_no_synthetic_history_cleared_string_in_dispatch_rpc046() {
    let body = read("codelet/fspec-tui/src/app/dispatch_rpc046.rs");
    assert!(
        !body.contains("history cleared"),
        "dispatch_rpc046.rs must not contain a synthetic `history cleared` \
         scrollback notice — see RPC-074 (TS parity)"
    );
    assert!(
        !body.contains("[notice] /clear"),
        "dispatch_rpc046.rs must not push `[notice] /clear` lines — TS parity"
    );
}

#[test]
fn scenario_no_user_notification_history_cleared_in_session_manager_handle() {
    let body = read("codelet/core/src/session_manager_handle.rs");
    assert!(
        !body.contains("\"history cleared\""),
        "session_manager_handle.rs must not broadcast `history cleared` \
         UserNotification chunks — TS parity (RPC-074)"
    );
}
```

This prevents the divergence from creeping back via any future card.

## Risk / out-of-scope

- This card does NOT touch `handle_emit_session_notice` itself — it
  stays for other slash commands (`/compact`, etc.).
- This card does NOT change the `StreamChunk::SessionStateChange`
  signal contract — only the redundant `UserNotification` is removed.
- Tests for `/compact` notices are unaffected (separate handler, separate chunk).
