# RPC-073 — Root Cause Analysis

> Three post-RPC-072 regressions discovered when the user ran the freshly-built
> fspec Rust binary on the `codelet-integration` branch (2026-05-27).

---

## Bug 1: `/clear` panics the binary

### Symptom

```
[error] /clearpfailed: the request exceeded its deadline

thread 'tokio-rt-worker' (8588248) panicked at sessions/src/background_session.rs:1156:36:
Cannot block the current thread from within a runtime. This happens because a function
attempted to block the current thread while the thread is being used to drive
asynchronous tasks.
```

See `screenshot-clear-panic.png`.

### Root cause

The chain:

1. TUI sends `/clear` → `App::dispatch_rpc046` calls
   `backend.clear_history(session_id).await`.
2. `EmbeddedFspecBackend::clear_history` (codelet/fspec-tui/src/transport/embedded.rs:304)
   calls `client.clear_history(context::current(), session_id).await` over tarpc.
3. Server side `FspecServiceImpl::clear_history` (codelet/rpc/src/lib.rs:1129) calls
   the SYNCHRONOUS trait method `handle.clear_history(&session_id)`.
4. `SessionManager::clear_history` (codelet/sessions/src/handle_impl.rs:229-238)
   calls `session.clear_history()` directly.
5. `BackgroundSession::clear_history` (codelet/sessions/src/background_session.rs:1149)
   reaches **line 1156**:

   ```rust
   let mut inner = self.inner.blocking_lock();
   ```

The doctring at line 1147-1148 explicitly warns:

> *Caller is responsible for wrapping in `tokio::task::block_in_place()` if calling
> from an async context (like the bridge control handler).*

The bridge control handler honours this contract. The new tarpc dispatch path does
NOT — `handle_impl.rs::clear_history` is a plain sync function with no
`block_in_place` wrapper, called from inside a tokio worker that is actively driving
the `FspecServiceImpl::clear_history` future.

### Why RPC-070's enforcement missed this

RPC-070 (test file: `codelet/fspec/tests/rpc070_create_session_no_panic.rs`)
enforces wrappers around the panic-prone idiom
`tokio::runtime::Handle::current().block_on(`. It does NOT scan for
`.blocking_lock()` calls on tokio mutexes — which is a different but equivalent
panic source (`tokio::sync::Mutex::blocking_lock` panics if called from an async
context just like `block_on`).

`background_session.rs::clear_history` is sync, so its `blocking_lock()` was fine
when called from a NAPI binding (no tokio runtime context). The regression
appeared only after RPC-072 made the tarpc path drive `clear_history` from inside
a multi-thread runtime worker.

### Likely siblings

Any sync trait method in `handle_impl.rs` that ultimately reaches a
`blocking_lock()` on `self.inner` has the same latent bug. Audit candidates:

- `clear_history` (confirmed broken)
- `compact_session` (lines 240-263) — calls `session.get_tokens()`; needs source check
- `set_role`, `set_model`, `set_thinking_level`, `restore_session_messages`,
  `restore_session_token_state`, `destroy_session` — all likely touch
  `self.inner` via `BackgroundSession` methods

### Fix sketch

Wrap each affected trait override in `tokio::task::block_in_place`:

```rust
fn clear_history(&self, session_id: &SessionId) -> Result<(), String> {
    let uuid = uuid_from(session_id);
    match self.get_session(&uuid.to_string()) {
        Ok(session) => {
            tokio::task::block_in_place(|| session.clear_history());
            Ok(())
        }
        Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
    }
}
```

And extend the RPC-070 source-shape regression test to also scan for naked
`blocking_lock(` calls reached from handle_impl entry points.

---

## Bug 2: `?` and `q` are trapped globally even while typing in AgentView input

### Symptom

User typed `is this card done?` in the Work Agent input. As soon as `?` was
pressed, the **Help Dialog popup** appeared and the `?` never made it into the
input buffer. Same for `q` (which quits the app entirely while composing).

### Root cause

`App::handle_event` (codelet/fspec-tui/src/app/events.rs:34-71):

```rust
pub fn handle_event(&mut self, event: &Event) -> EventResult {
    // ... DisconnectDialog topmost check ...

    if !topmost_is_critical {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Release {
                if let Some(result) = self.handle_app_shortcut(key) {  // ← TRAPS `?` / `q`
                    return result;
                }
            }
        }
    }

    let result = self.compositor.handle_event(event);   // ← NEVER REACHED FOR `?`
    // ... navigator dispatch ...
}
```

`handle_app_shortcut` (events.rs:96-113):

```rust
fn handle_app_shortcut(&mut self, key: &KeyEvent) -> Option<EventResult> {
    if key.code == KeyCode::Char('?') && key.modifiers == KeyModifiers::NONE {
        self.compositor.push(Box::new(HelpDialog::new()));
        self.should_render = true;
        return Some(EventResult::consumed());
    }
    if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE {
        self.should_quit = true;
        return Some(EventResult::consumed());
    }
    // ... Ctrl+D ...
}
```

App shortcuts are checked BEFORE the compositor and navigator. The AgentView's
input field (`MultiLineInput`) never sees `?` or `q` keystrokes.

### Why this passed all existing tests

The RPC-009/RPC-013/RPC-018 keyboard tests verify the BoardView keyboard contract,
where `?` and `q` ARE the correct global shortcuts. No test covers the case where
the user has an input field focused — RPC-019 (multiline input) added the input,
RPC-020 added popups, but neither asserted that `?` is type-able into the buffer.

### Fix sketch

Invert the dispatch priority — app-level shortcuts should fire LAST, only after
the compositor and navigator decline to consume the event:

```rust
pub fn handle_event(&mut self, event: &Event) -> EventResult {
    // DisconnectDialog still trumps everything (RPC-011 CR-1)
    if topmost_is_disconnect { return self.handle_disconnect_dialog_event(event); }

    // 1. Compositor first (popups, modals, dialogs)
    let result = self.compositor.handle_event(event);
    if result.is_consumed() { /* ... apply callback ... */ return result; }

    // 2. Navigator (current view: Board or Agent)
    let nav_result = self.navigator.handle_event(event, &self.board_store);
    if nav_result.is_consumed() {
        self.should_render = true;
        return nav_result;
    }

    // 3. App-level fallback shortcuts only if nobody else cared
    if !topmost_is_critical {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Release {
                if let Some(result) = self.handle_app_shortcut(key) {
                    return result;
                }
            }
        }
    }

    nav_result
}
```

Alternative (more surgical): keep current order but in `handle_app_shortcut`,
suppress when `self.navigator.is_agent_view_input_active()` returns true.

The cleanest answer is dispatch-order inversion. Add a regression test:

> Scenario: pressing `?` while AgentView input is focused inserts `?` into the buffer
> and does NOT open the HelpDialog.

> Scenario: pressing `q` while AgentView input is focused inserts `q` into the buffer
> and does NOT quit the app.

---

## Bug 3: Model selector dialog is empty / doesn't read fspec-config.json

### Symptom

User opened the model selector (Ctrl+? / `/model`). The list was empty — no
providers shown, no models to select. The actual default model
(`anthropic/claude-opus-4-7` per `~/.fspec/fspec-config.json:tui.lastUsedModel`)
is the one the agent loop USES (proven by the 429 response in
`screenshot-agentview-429.png`), but the selector dialog can't show alternatives.

### Root cause

`SessionManager::list_providers` (codelet/sessions/src/handle_impl.rs:709-715):

```rust
fn list_providers(&self) -> Vec<codelet_rpc_types::ProviderInfo> {
    // RPC-042 scope: providers registry wiring lives in RPC-054
    // (`/provider` ProviderSettingsView + credentials surface).
    // The trait default returns `Vec::new()`; keep that semantics
    // by returning an empty Vec here too.
    Vec::new()
}
```

The comment defers wiring to RPC-054. But **RPC-054 wired
`list_provider_credentials`** (the right hand side of the `/provider` settings
view), NOT `list_providers` (the model selector dialog from RPC-022). They are
different RPC methods. The implementer of RPC-054 didn't notice that this
stub-return was still in place.

### Where the real list comes from

`codelet/providers/src/custom/management.rs:95`:

```rust
pub fn list_providers_info() -> Result<Vec<ProviderInfo>, ProviderError> {
    // ... enumerate builtin providers + custom configs from ~/.fspec/providers/
    //     + check credentials from ProviderCredentials::detect()
    //     + return ProviderInfo with model lists
}
```

This is what the NAPI binding already uses (codelet/napi/src/session_bindings.rs:3469).
The Rust trait override should call the same helper.

### Fix sketch

```rust
fn list_providers(&self) -> Vec<codelet_rpc_types::ProviderInfo> {
    codelet_providers::custom::management::list_providers_info()
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "list_providers_info failed");
            Vec::new()
        })
}
```

Add a regression test: open the model dialog, assert at least the anthropic
provider row appears when `ANTHROPIC_API_KEY` env var or
`~/.fspec/credentials/anthropic.json` is present.

---

## Bug 4 (observed but in-scope of fix above)

The "/clearpfailed: the request exceeded its deadline" line that precedes the
panic backtrace is a *symptom* of Bug 1 — the tarpc client's `/clear` RPC times
out because the server worker panicked while servicing it. Fixing Bug 1 will
remove this error too. No separate fix needed.

---

## Files to touch (proposed)

| Bug | File | Change |
|-----|------|--------|
| 1 | `codelet/sessions/src/handle_impl.rs` | Wrap `clear_history`, `compact_session`, and any other sync trait method that reaches `blocking_lock` in `tokio::task::block_in_place(...)` |
| 1 | `codelet/fspec/tests/rpc070_create_session_no_panic.rs` (or new RPC-073 test) | Extend source-shape scan to flag naked `blocking_lock(` calls in handle_impl |
| 1 | new test file | E2E: open session, send `/clear`, assert no panic |
| 2 | `codelet/fspec-tui/src/app/events.rs` | Invert dispatch order — Compositor → Navigator → app-shortcut |
| 2 | new test file | `?` and `q` in AgentView input do not trigger app-shortcut |
| 3 | `codelet/sessions/src/handle_impl.rs` | Wire `list_providers` to `codelet_providers::custom::management::list_providers_info` |
| 3 | `codelet/providers/Cargo.toml` (if not already a dep of codelet-sessions) | Add codelet-providers as a dep |
| 3 | new test file | Model dialog list is non-empty when at least one provider has credentials |

---

## RPC-072 wiring sanity check

The user's screenshots also confirm RPC-072's happy path works:

- Input "is this card done" was sent.
- The agent loop drove the claude provider (model `claude-opus-4-5`).
- The provider returned HTTP 429 (rate-limited) — a real provider response.
- The error was surfaced as `[error] provider error: [claude] API error: Rig
  completion failed: HttpError: Invalid status code 429 Too Many Requests`.
- Followed by `[done]` chunk so the session returns to Idle.

So RPC-072 is genuinely complete and the agent loop is wired end-to-end. These
three bugs are pre-existing latent bugs that only became visible once user input
started reaching the binary.
