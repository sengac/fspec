# Bug 1 Research — `/clear` Panic in Rust fspec Binary (TS vs Rust Parity)

**Work unit:** RPC-073
**Panic site:** `codelet/sessions/src/background_session.rs:1156:36`
**Panic message:** `thread 'tokio-rt-worker' panicked at … Cannot block the current thread from within a runtime.`
**Trigger:** User types `/clear` in a Work Agent session over tarpc (embedded or WebSocket transport).

---

## 1. Executive Summary

The Rust `/clear` slash command panics because `BackgroundSession::clear_history()` calls
`tokio::sync::Mutex::blocking_lock()` while the caller is already running on a Tokio runtime
worker thread (the tarpc service worker). The original TypeScript Ink path called the same
function (via the NAPI binding `sessionClearHistory`) but did so synchronously from a Node
libuv worker thread — **not** a Tokio worker — so `blocking_lock()` was legal.

When the codebase moved from the NAPI-bridge architecture to a pure-Rust tarpc transport
(RPC-037/RPC-046/RPC-070), the synchronous `BackgroundSession::clear_history` is now invoked
from inside an `async fn` running on the Tokio runtime, violating the documented contract at
`codelet/sessions/src/background_session.rs:1147-1148`:

> *Note: Caller is responsible for wrapping in `tokio::task::block_in_place()` if calling
> from an async context (like the bridge control handler).*

The RPC-070 regression test (`codelet/fspec/tests/rpc070_create_session_no_panic.rs`) only
checks `Handle::current().block_on(` patterns — it has zero needles for `.blocking_lock()`,
`.blocking_read()`, `.blocking_write()`, etc., so this latent panic slipped through.

The fix is to wrap the call in `tokio::task::block_in_place` at the handle layer
(`codelet/sessions/src/handle_impl.rs:229-238`), matching the pattern already used for
`create_session`, `create_isolated_session`, `test_provider_connection`, and the three
`loop_*` methods.

---

## 2. TypeScript Implementation Walk-Through

### 2.1 Slash command registry

`src/tui/utils/slashCommands.ts:39`
```ts
{ name: 'clear', description: 'Clear conversation history' },
```

### 2.2 Palette hook & keyboard routing

`src/tui/components/AgentView.tsx:1082-1093`
```ts
const slashCommand = useSlashCommandInput({
  inputValue,
  onInputChange: setInputValue,
  onExecuteCommand: cmd => executeSlashCommandRef.current?.(cmd),
  disabled:
    isResumeMode ||
    isBlocklistMode ||
    showModelSelector ||
    showSettingsTab ||
    showThinkingLevelDialog,
});
```

`src/tui/components/AgentView.tsx:4613-4617`
```ts
// TUI-050: Slash command palette keyboard handling
if (slashCommand.handleInput(input, key)) {
  return true;
}
```

### 2.3 Palette parses `/`-prefixed input

`src/tui/hooks/useSlashCommandInput.ts:197-208`
```ts
const filterText = newValue.slice(1);
// Show palette when "/" is typed at position 0, hide when space typed (arguments)
if (newValue.startsWith('/') && !filterText.includes(' ')) {
  if (!isVisible) {
    show();
  }
  updateFilter(filterText);
} else if (isVisible) {
  hide();
}
```

`src/tui/hooks/useSlashCommandInput.ts:246-258`
```ts
if (key.return) {
  const selected = getSelectedCommand();
  if (selected) {
    const commandText = `/${selected.name}`;
    // Hide FIRST, then clear input, then execute
    hide();
    onInputChange('');
    onExecuteCommand(commandText); // → executeSlashCommandRef.current?.(cmd)
  }
  return true;
}
```

### 2.4 Shared `/clear` handler

`src/tui/components/AgentView.tsx:1554-1564`
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

Dispatch site A — `handleSubmit` (`AgentView.tsx:1621-1624`):
```ts
if (userMessage === '/clear') {
  handleClearCommand();
  return;
}
```

Dispatch site B — `handleSubmitWithCommand` (`AgentView.tsx:2730-2733`):
```ts
if (userMessage === '/clear') {
  handleClearCommand();
  return;
}
```

### 2.5 NAPI binding declaration

`codelet/napi/index.d.ts:1875`
```ts
export declare function sessionClearHistory(sessionId: string): void
```

**Key point:** the binding returns `void`, not `Promise<void>` — there is no `await`, no
Promise wrapping. The call is invoked on a Node libuv worker thread.

### 2.6 Rust NAPI implementation (the TS-side entrypoint, in Rust)

`codelet/napi/src/session_bindings.rs:1450-1463`
```rust
/// TUI-065: Clear session history and reinject context reminders
///
/// DRY: This is the single source of truth for clear functionality.
/// Both TUI /clear command and Telegram bridge /clear should use this.
#[napi]
pub fn session_clear_history(session_id: String) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    session.clear_history();
    Ok(())
}
```

### 2.7 Why TS path does NOT panic

`#[napi] pub fn session_clear_history` is a **synchronous** N-API export. NAPI dispatches
synchronous bindings on a Node libuv worker thread, which is **not a Tokio runtime worker**.
`tokio::sync::Mutex::blocking_lock()` only panics when called from within a Tokio runtime
context — it is perfectly legal on a non-Tokio thread (libuv).

### 2.8 TS-path call chain (no panic)
```
User types "/clear" + Enter
  └─ useSlashCommandInput.ts:246  (palette Enter)
      └─ onExecuteCommand → executeSlashCommandRef
          └─ AgentView.tsx:2730 handleSubmitWithCommand
              └─ AgentView.tsx:1554 handleClearCommand
                  └─ sessionClearHistory(currentSessionId)   [sync, no await]
                      └─ NAPI on libuv worker thread (NOT tokio)
                          └─ session_bindings.rs:1459 session_clear_history
                              └─ SessionManager::get_session() [std::sync::RwLock]
                              └─ BackgroundSession::clear_history()
                                  └─ self.inner.blocking_lock()  ✅ SAFE
                                     (libuv thread is not a tokio worker)
```

---

## 3. Rust Implementation Walk-Through

### 3.1 Slash command registry (TUI palette)

`codelet/fspec-tui/src/views/agent/slash_commands.rs:87-95`
```rust
pub const SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        action: SlashCommandAction::Help,
        description: "Show help dialog",
    },
    SlashCommand {
        action: SlashCommandAction::Clear,
        description: "Clear conversation history",
    },
    ...
```

### 3.2 Action dispatcher

`codelet/fspec-tui/src/app/dispatch_rpc020.rs:36-39`
```rust
SlashCommandAction::Clear => {
    // RPC-046: handler body lives in dispatch_rpc046.rs.
    self.handle_slash_clear();
}
```

### 3.3 TUI handler — `handle_slash_clear`

`codelet/fspec-tui/src/app/dispatch_rpc046.rs:45-65`
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
The `tokio::spawn` puts the rest of the work onto a Tokio runtime worker — every layer below
runs **inside an async context**.

### 3.4 tarpc backend trait

`codelet/fspec-tui/src/transport/mod.rs:292-295`
```rust
/// RPC-037: clear session history.
async fn clear_history(&self, _session_id: SessionId) -> Result<()> {
    Ok(())
}
```

### 3.5 Embedded tarpc client

`codelet/fspec-tui/src/transport/embedded.rs:304-309`
```rust
async fn clear_history(&self, session_id: SessionId) -> Result<()> {
    self.client
        .clear_history(context::current(), session_id)
        .await?
        .map_err(|e| anyhow::anyhow!("{e}"))
}
```

### 3.6 WebSocket tarpc client

`codelet/fspec-tui/src/transport/websocket.rs:578-586`
```rust
async fn clear_history(&self, session_id: SessionId) -> Result<()> {
    let guard = self.client.read().await;
    let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
    client
        .client()
        .clear_history(context::current(), session_id)
        .await?
        .map_err(|e| anyhow::anyhow!("{e}"))
}
```

### 3.7 tarpc server impl

Trait method — `codelet/rpc/src/lib.rs:230-231`:
```rust
/// RPC-037: clear session history.
async fn clear_history(session_id: SessionId) -> Result<(), String>;
```

Service impl — `codelet/rpc/src/lib.rs:1129-1138`:
```rust
async fn clear_history(
    self,
    _ctx: Context,
    session_id: SessionId,
) -> Result<(), String> {
    match self.inner.session_manager() {
        Some(handle) => handle.clear_history(&session_id),
        None => Ok(()),
    }
}
```
This is the **first** point that hops from `async` into the synchronous `SessionManagerHandle`
trait — but it does NOT use `tokio::task::block_in_place`.

### 3.8 `SessionManagerHandle::clear_history`

`codelet/sessions/src/handle_impl.rs:229-238`
```rust
fn clear_history(&self, session_id: &SessionId) -> Result<(), String> {
    let uuid = uuid_from(session_id);
    match self.get_session(&uuid.to_string()) {
        Ok(session) => {
            session.clear_history();   // ← unguarded sync call into blocking_lock
            Ok(())
        }
        Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
    }
}
```

### 3.9 The panic site — `BackgroundSession::clear_history`

`codelet/sessions/src/background_session.rs:1140-1173`
```rust
/// This method clears the session's messages, turns, and token tracker,
/// then reinjects the context reminders (CLAUDE.md, environment info) so
/// the AI retains project context after clearing.
///
/// DRY: This is the single source of truth for clear functionality.
/// Both TUI /clear command (via NAPI) and Telegram bridge /clear use this.
///
/// Note: Caller is responsible for wrapping in `tokio::task::block_in_place()`
/// if calling from an async context (like the bridge control handler).
pub fn clear_history(&self) {
    // Clear the output buffer (conversation history display)
    if let Ok(mut buffer) = self.output_buffer.write() {
        buffer.clear();
    }

    // Clear actual session state (messages, turns, tokens)
    let mut inner = self.inner.blocking_lock();        // ← LINE 1156, PANIC
    inner.messages.clear();
    inner.turns.clear();
    inner.token_tracker = codelet_core::compaction::TokenTracker::default();

    // CRITICAL: Reinject context reminders so AI retains project context
    let isolation = self.build_isolation_context();
    inner.inject_context_reminders_with_isolation(isolation.as_ref());
    drop(inner);

    // Reset the interrupt flag
    self.reset_interrupt();

    // TUI-066: Emit chunk so React updates state as side effect
    self.handle_output(StreamChunk::session_state_change(SessionState::Cleared));
}
```

`self.inner` is declared at `background_session.rs:295` as
`Arc<Mutex<codelet_cli::session::Session>>`, where `Mutex` is imported from
`tokio::sync::{Mutex, Notify, broadcast, mpsc}` at `background_session.rs:55`. This is the
**tokio** async-mutex, not `std::sync::Mutex`. Calling `.blocking_lock()` on it from within
a Tokio runtime is the exact pattern that yields `Cannot block the current thread from within a runtime`.

### 3.10 Rust-path call chain (panics)

```
User types "/clear" + Enter
  └─ palette → dispatch_rpc020.rs:36 → handle_slash_clear
      └─ dispatch_rpc046.rs:57  tokio::spawn(async move { ... })  ← enters Tokio runtime
          └─ backend.clear_history(session_id).await
              └─ embedded.rs:304 OR websocket.rs:578 — tarpc client call (async)
                  └─ rpc/src/lib.rs:1129 async fn clear_history (tarpc server, on tokio worker)
                      └─ handle.clear_history(&session_id)   [SYNC, still on tokio worker]
                          └─ handle_impl.rs:233 session.clear_history()
                              └─ background_session.rs:1156 self.inner.blocking_lock()
                                   💥 PANIC: Cannot block the current thread from within a runtime
```

---

## 4. Side-by-Side Differences Table

| Aspect | TS (NAPI) | Rust (tarpc) | Required change |
|---|---|---|---|
| Frontend palette parser | `useSlashCommandInput.ts:197-258` | `slash_commands.rs:87-95` + `dispatch_rpc020.rs:36` | None |
| Frontend dispatch | `handleClearCommand` `AgentView.tsx:1554-1564` (sync) | `handle_slash_clear` `dispatch_rpc046.rs:45-65` (spawns async task) | None |
| Wire format | NAPI sync function returning `void` | tarpc `async fn clear_history` → `Result<(),String>` | None |
| Thread executing handle method | Node libuv worker (NOT tokio) | Tokio runtime worker (tarpc server) | This is the root cause |
| `BackgroundSession::clear_history` invocation context | Sync, on libuv thread | Sync, but **inside async fn on tokio worker** | Must wrap in `tokio::task::block_in_place(...)` |
| `tokio::sync::Mutex::blocking_lock()` safe? | ✅ Yes (libuv ≠ tokio worker) | ❌ No — panics | Wrap caller or replace mutex semantics |
| Caller honors `background_session.rs:1147-1148` contract? | N/A — libuv path inherently fine | ❌ NO — `handle_impl.rs:229-238` doesn't wrap | Add `block_in_place` |

---

## 5. Exact Patch Sites (Before / After)

### 5.1 Primary fix — `handle_impl.rs::clear_history`

**File:** `codelet/sessions/src/handle_impl.rs`
**Lines:** 229-238

**Before:**
```rust
fn clear_history(&self, session_id: &SessionId) -> Result<(), String> {
    let uuid = uuid_from(session_id);
    match self.get_session(&uuid.to_string()) {
        Ok(session) => {
            session.clear_history();
            Ok(())
        }
        Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
    }
}
```

**After:**
```rust
fn clear_history(&self, session_id: &SessionId) -> Result<(), String> {
    let uuid = uuid_from(session_id);
    match self.get_session(&uuid.to_string()) {
        Ok(session) => {
            // RPC-073: BackgroundSession::clear_history uses `tokio::sync::Mutex::blocking_lock`
            // which panics on a Tokio runtime worker. The tarpc service calls this method from
            // within an `async fn`, so we MUST wrap in block_in_place. See
            // background_session.rs:1147-1148 for the documented caller contract.
            tokio::task::block_in_place(|| session.clear_history());
            Ok(())
        }
        Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
    }
}
```

**Why this matches TS behavior:** TS path runs `session.clear_history()` synchronously from
a non-Tokio thread (libuv). `block_in_place` tells Tokio "this current worker is going to
block, redirect new tasks to other workers" — semantically equivalent to running on a
non-Tokio thread for the duration of the synchronous closure. The behavior observed by the
caller (await returns `Ok(())` after history is cleared and `SessionStateChange::Cleared` is
emitted) is identical.

**Caveat:** `block_in_place` requires the multi-thread Tokio runtime flavor. The pattern is
already established by `create_session` (handle_impl.rs:89), `create_isolated_session` (:631),
`test_provider_connection` (:896), and the three `loop_*` methods via the `loop_block_on`
helper at `handle_impl.rs:1320` — all of which document the precondition at lines 11-27 of
the same file.

### 5.2 Alternative (more robust) fix — push the wrapper into `BackgroundSession`

If we want to honor the "All methods are synchronous and non-blocking" contract documented
on the trait (`codelet/core/src/session_manager_handle.rs:66-68`) without per-call-site
discipline, push the wrap one layer down:

**File:** `codelet/sessions/src/background_session.rs`
**Lines:** 1149-1173

**Before (excerpt):**
```rust
pub fn clear_history(&self) {
    if let Ok(mut buffer) = self.output_buffer.write() {
        buffer.clear();
    }
    let mut inner = self.inner.blocking_lock();
    ...
}
```

**After (excerpt):**
```rust
pub fn clear_history(&self) {
    if let Ok(mut buffer) = self.output_buffer.write() {
        buffer.clear();
    }
    // RPC-073: blocking_lock panics inside a Tokio runtime worker. Detect the runtime
    // and wrap accordingly so callers do not have to know our internal mutex flavor.
    if tokio::runtime::Handle::try_current().is_ok() {
        tokio::task::block_in_place(|| self.clear_history_inner());
    } else {
        self.clear_history_inner();
    }
}

fn clear_history_inner(&self) {
    let mut inner = self.inner.blocking_lock();
    inner.messages.clear();
    inner.turns.clear();
    inner.token_tracker = codelet_core::compaction::TokenTracker::default();
    let isolation = self.build_isolation_context();
    inner.inject_context_reminders_with_isolation(isolation.as_ref());
    drop(inner);
    self.reset_interrupt();
    self.handle_output(StreamChunk::session_state_change(SessionState::Cleared));
}
```

**Recommendation:** Apply **5.1** (the minimal targeted patch at the handle layer) for
RPC-073 since it matches the existing pattern set by `create_session` et al. and keeps the
diff small. Track 5.2 as a follow-up tech-debt item to eliminate the documented caller
contract entirely.

---

## 6. Sibling Audit — All Sync Trait Methods in `SessionManagerHandle`

Source: `codelet/sessions/src/handle_impl.rs`. Trait defined at
`codelet/core/src/session_manager_handle.rs:69`. Methods reach `blocking_lock` /
`block_on` either directly or transitively.

| # | Method | handle_impl.rs:line | Wraps in `block_in_place`? | Reaches `blocking_lock`/`block_on`? | Panic risk on tokio worker |
|---|---|---|---|---|---|
| 1 | `list_sessions` | 71 | — | no | ✅ safe |
| 2 | `create_session` | 82 | ✅ yes (:89) | uses `Handle::current().block_on` | safe |
| 3 | `send_input` | 104 | — | no (mpsc) | ✅ safe |
| 4 | `send_input_with_thinking` | 111 | — | no | ✅ safe |
| 5 | `interrupt` | 124 | — | no | ✅ safe |
| 6 | `get_session_status` | 131 | — | no | ✅ safe |
| 7 | `chunks_rx` / `logs_rx` / `chunks_tx` / `logs_tx` | 138/142/146/150 | — | no | ✅ safe |
| 8 | `status_changes_rx` / `status_changes_tx` | 154/158 | — | no | ✅ safe |
| 9 | `get_session_tokens` | 162 | — | no (atomics) | ✅ safe |
| 10 | `get_session_model` | 178 | — | no | ✅ safe |
| 11 | `get_compaction_progress` | 214 | — | no | ✅ safe |
| 12 | `get_buffered_output` | 222 | — | no | ✅ safe |
| **13** | **`clear_history`** | **229** | **❌ NO** | **YES → background_session.rs:1156 `blocking_lock`** | **🔥 PANICS** |
| 14 | `compact_session` | 240 | — | no (placeholder) | ✅ safe |
| 15 | `restore_session_messages` | 262 | — | no (placeholder) | ✅ safe |
| 16 | `restore_session_token_state` | 274 | — | no (placeholder) | ✅ safe |
| 17 | `get_work_unit_context` | 286 | — | no | ✅ safe |
| 18 | `set_work_unit_context` | 294 | — | no | ✅ safe |
| 19 | `get_pending_input` | 316 | — | no | ✅ safe |
| 20 | `set_pending_input` | 323 | — | no | ✅ safe |
| 21 | `set_active_session` / `clear_active_session` / `get_active_session` | 330/335/339 | — | no | ✅ safe |
| 22 | `get_effective_cwd` | 343 | — | no | ✅ safe |
| 23 | `get_supervisors` / `add_supervisor` / `remove_supervisor` / `get_subordinate` / `get_subordinates` | 351/362/374/381/388 | — | no | ✅ safe |
| 24 | `receive_incoming_message` | 400 | — | no (mpsc send) | ✅ safe |
| 25 | `get_debug_enabled` / `set_debug_enabled` | 424/431 | — | no | ✅ safe |
| 26 | `toggle_debug` | 438 | — | no (`std::sync::Mutex`, not tokio) | ✅ safe |
| 27 | `set_debug_directory` | 483 | — | no (std::sync::Mutex) | ✅ safe |
| 28 | `pause_resume` / `pause_confirm` / `pause_triple` | 502/515/530 | — | no | ✅ safe |
| 29 | `send_hitl_response` | 545 | — | no | ✅ safe |
| 30 | `get_pause_state` / `get_hitl_request` | 565/573 | — | no | ✅ safe |
| 31 | `send_fspec_result` | 598 | — | no | ✅ safe |
| 32 | `create_isolated_session` | 618 | ✅ yes (:631) | uses `block_on` | safe |
| 33 | `set_thinking_level_default` / `set_thinking_level` / `get_thinking_level` | 647/733/697 | — | no (atomics) | ✅ safe |
| 34 | `destroy_session` | 662 | — | no | ✅ safe |
| 35 | `get_model_info` | 673 | — | no | ✅ safe |
| 36 | `set_model` | 717 | — | no | ✅ safe |
| 37 | `get_role` / `set_role` | 748/755 | — | no | ✅ safe |
| 38 | `list_providers` | 709 | — | no | ✅ safe |
| 39 | `list_provider_credentials` / `get_provider_credential` / `set_provider_credentials` / `delete_provider_credentials` | 797/825/831/868 | — | no | ✅ safe |
| 40 | `test_provider_connection` | 877 | ✅ yes (:896) | uses `block_on` | safe |
| 41 | `refresh_models_cache` | 934 | — | no | ✅ safe |
| 42 | `blocklist_list` | 982 | — | no | ✅ safe |
| 43 | `merge_session_worktree` / `discard_session_worktree` / `prune_orphaned_worktrees` / `list_session_worktrees` / `inspect_session_changes` | 1039/1075/1086/1100/1137 | — | no | ✅ safe |
| 44 | `schedule_add` / `schedule_list` / `schedule_pause` / `schedule_resume` / `schedule_remove` | 1180/1189/1197/1206/1215 | — | no | ✅ safe |
| 45 | `loop_add` | 1231 | ✅ yes (via `loop_block_on` :1272/1320) | uses `block_on` | safe |
| 46 | `loop_cancel` | 1281 | ✅ yes (via `loop_block_on` :1283) | uses `block_on` | safe |
| 47 | `loop_list` | 1291 | ✅ yes (via `loop_block_on` :1296) | uses `block_on` | safe |

**Verdict:** Exactly **one** method (`clear_history` at `handle_impl.rs:229`) has the latent
panic on the tarpc/tokio path today. All other methods that need to bridge sync↔async
already wrap in `block_in_place` via the established pattern.

**Note on `background_session.rs::blocking_lock` occurrences:** the file contains **only one**
`.blocking_lock()` call (line 1156). All other `self.inner` accesses use `try_lock()`
(lines 684, 690), which return `Err` instead of panicking and are therefore safe in any
context.

---

## 7. Why RPC-070 Enforcement Missed This

`codelet/fspec/tests/rpc070_create_session_no_panic.rs` (437 lines) contains 5 scenarios:

| # | Scenario | Kind |
|---|---|---|
| 1 | `scenario_create_session_does_not_panic_on_multi_thread_worker` | Behavioral — only exercises `create_session` |
| 2 | `scenario_create_session_over_embedded_tarpc_does_not_panic` | Behavioral — only exercises `create_session` |
| 3 | `scenario_every_block_on_is_wrapped_in_block_in_place` | Source-shape grep over `handle_impl.rs` |
| 4 | `scenario_test_provider_connection_uses_block_in_place` | Source-shape grep over one method |
| 5 | `scenario_pre_existing_handle_impl_tests_still_pass` | Sanity |

### What the source-shape scan looks for

Scenario #3's needle is the literal string `"tokio::runtime::Handle::current().block_on("`.
For each match it slices the preceding 200 chars and asserts the window contains
`"tokio::task::block_in_place("`. There are **zero** needles for:

- `.blocking_lock(`
- `.blocking_read(`
- `.blocking_write(`
- `.blocking_recv(`
- `.blocking_send(`

A grep across the entire file confirms there is **no mention** of `blocking_lock` anywhere.

### Why `clear_history` slipped through

1. **Different mechanism, same class:** `clear_history` does not call
   `Handle::current().block_on(...)` itself. It calls `BackgroundSession::clear_history()`,
   which uses `tokio::sync::Mutex::blocking_lock()` — a different API that produces the
   identical "Cannot block the current thread from within a runtime" panic.
2. **Different file:** the offending call lives in `codelet/sessions/src/background_session.rs:1156`,
   not in `codelet/sessions/src/handle_impl.rs`. The shape scan only reads `handle_impl.rs`.
3. **Transitive call invisible to shape scan:** `handle_impl.rs::clear_history` looks
   pristine — it only contains `session.clear_history()` with no blocking idiom in scope.
4. **Behavioral tests too narrow:** scenarios #1 and #2 only call `create_session`. They
   never invoke `clear_history`, so the panic never fires under test.

### What the test needs

Three additions, mirroring the existing structure:

**(a) Behavioral coverage:**
```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_clear_history_does_not_panic_on_multi_thread_worker() {
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;
    let sid = handle.create_session(None);
    let h = Arc::clone(&handle);
    let join = tokio::spawn(async move { h.clear_history(&sid) });
    let result = tokio::time::timeout(Duration::from_secs(5), join)
        .await
        .expect("clear_history did not return within 5s")
        .expect("clear_history task panicked — blocking_lock regression");
    let _ = result;
}
```
Plus an embedded-tarpc variant.

**(b) Broaden the shape scan to cover all `blocking_*` APIs:**
```rust
for needle in [
    "tokio::runtime::Handle::current().block_on(",
    ".blocking_lock(",
    ".blocking_read(",
    ".blocking_write(",
    ".blocking_recv(",
    ".blocking_send(",
] { /* assert each is preceded by tokio::task::block_in_place( */ }
```
Scope this scan to **both** `handle_impl.rs` and `background_session.rs` (or, more pragmatically,
restrict the `blocking_*` needles to `handle_impl.rs` only, since that's the documented
contract layer — making `background_session.rs` an implementation-detail file the handle
layer must protect).

**(c) Per-method body assertion (mirrors scenario #4):**
```rust
#[test]
fn scenario_clear_history_wraps_inner_call_in_block_in_place() {
    let src = strip_comments(&read_handle_impl_src());
    let body = extract_method_body(&src, "clear_history");
    assert!(
        body.contains("tokio::task::block_in_place"),
        "handle_impl::clear_history must wrap session.clear_history() in block_in_place; body was:\n{body}",
    );
}
```

A more robust alternative would use `AstGrep` (`pattern: '$EXPR.blocking_lock()'`,
`language: rust`) to avoid false positives from string literals and doc comments.

### Root philosophical issue

The RPC-070 test universe was defined by a single substring (`Handle::current().block_on(`)
rather than by the broader class of "tokio APIs that panic when called from a runtime
worker." `tokio::sync::Mutex::blocking_lock`, `blocking_read`, `blocking_write`,
`blocking_recv`, `blocking_send`, and `Handle::current().block_on` all belong to that
class — but only the last was in scope. RPC-073 should generalize the enforcement.

---

## 8. Recommended Action Plan

1. **Apply patch 5.1** at `codelet/sessions/src/handle_impl.rs:229-238` — wraps
   `session.clear_history()` in `tokio::task::block_in_place`. Minimal, matches existing
   pattern, fixes the panic.
2. **Add the three test additions in §7** to `codelet/fspec/tests/rpc070_create_session_no_panic.rs`
   (or a new `rpc073_clear_history_no_panic.rs`) to prevent regression and catch any
   future `blocking_*` panic sites at compile-time-of-test.
3. **Backlog (optional 5.2):** push the `block_in_place` wrap into
   `BackgroundSession::clear_history` itself and drop the documented caller contract at
   lines 1147-1148 — eliminates per-call-site discipline.
4. **Backlog:** audit other potential `blocking_lock` sites elsewhere in `codelet/sessions/`
   and `codelet/core/` using the broadened shape scan, and verify each handle-layer caller
   either wraps in `block_in_place` or runs on a non-Tokio thread.

---

## 9. Source Cross-Reference Index

| File | Lines | Purpose |
|---|---|---|
| `src/tui/utils/slashCommands.ts` | 39 | TS palette `/clear` entry |
| `src/tui/hooks/useSlashCommandInput.ts` | 197-258 | TS palette key handling |
| `src/tui/components/AgentView.tsx` | 1554-1564, 1621-1624, 2730-2733, 4613-4617 | TS dispatch + handler |
| `codelet/napi/index.d.ts` | 1875 | NAPI binding declaration |
| `codelet/napi/src/session_bindings.rs` | 1450-1463 | NAPI binding implementation (sync, libuv-safe) |
| `codelet/fspec-tui/src/views/agent/slash_commands.rs` | 87-95 | Rust palette `/clear` entry |
| `codelet/fspec-tui/src/app/dispatch_rpc020.rs` | 36-39 | Action routing |
| `codelet/fspec-tui/src/app/dispatch_rpc046.rs` | 45-65 | `handle_slash_clear` — spawns tokio task |
| `codelet/fspec-tui/src/transport/mod.rs` | 292-295 | Backend trait |
| `codelet/fspec-tui/src/transport/embedded.rs` | 304-309 | Embedded tarpc client |
| `codelet/fspec-tui/src/transport/websocket.rs` | 578-586 | WebSocket tarpc client |
| `codelet/rpc/src/lib.rs` | 230-231, 1129-1138 | tarpc trait + service impl |
| `codelet/sessions/src/handle_impl.rs` | **229-238** | **Primary patch site** |
| `codelet/sessions/src/background_session.rs` | 1140-1173, **1156** | Panic site |
| `codelet/sessions/src/background_session.rs` | 295, 55 | `tokio::sync::Mutex` declaration & import |
| `codelet/core/src/session_manager_handle.rs` | 66-69 | Trait synchronous-and-non-blocking contract |
| `codelet/fspec/tests/rpc070_create_session_no_panic.rs` | entire | RPC-070 regression test (needs broadening) |
