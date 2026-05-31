# RPC-073 — Fix Plan

> Per-bug fix specification with file-level changes and test surfaces.

---

## Approach

Three independent bugs, three independent fixes. Each fix lands with its own
regression test. We do **not** ship a fix without a failing test first
(ACDD discipline).

Recommended order (lowest risk first):

1. **Bug 3** (list_providers wiring) — pure additive change, no behaviour
   regression possible. Two lines of code, one test.
2. **Bug 1** (/clear panic + sibling audit) — defensive wrap, no behaviour
   change for callers that already passed. New regression test catches future
   omissions.
3. **Bug 2** (dispatch-order inversion) — riskier because it changes the
   priority of every keystroke. Needs the most test coverage.

---

## Phase 1: Bug 3 — Wire list_providers to ProviderManager

### Goal

The model selector dialog shows providers from
`codelet_providers::custom::management::list_providers_info`, matching what the
NAPI `list_providers` binding already returns.

### Files

| File | Change |
|------|--------|
| `codelet/sessions/Cargo.toml` | Confirm `codelet-providers` is in `[dependencies]` (likely already there) |
| `codelet/sessions/src/handle_impl.rs:709-715` | Replace `Vec::new()` body with a call into `list_providers_info()` |
| `codelet/fspec-tui/tests/list_providers_rpc073.rs` | New integration test |

### Code

```rust
fn list_providers(&self) -> Vec<codelet_rpc_types::ProviderInfo> {
    match codelet_providers::custom::management::list_providers_info() {
        Ok(providers) => providers,
        Err(e) => {
            tracing::error!(error = %e, "list_providers_info failed in handle_impl");
            Vec::new()
        }
    }
}
```

If the return type from `list_providers_info()` is a different concrete
`ProviderInfo` than `codelet_rpc_types::ProviderInfo`, add a one-shot `.map(Into::into).collect()`
adapter (verify field-by-field shape first — RPC-054 may have aligned them).

### Test

Integration test in `codelet/fspec-tui/tests/list_providers_rpc073.rs`:

```rust
//! Feature: spec/features/rpc073-list-providers-wiring.feature

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_providers_returns_at_least_one_when_credentials_present() {
    // @step Given a SessionManager with ProviderManager-detected credentials
    // @step And ANTHROPIC_API_KEY is set (or test credentials shimmed)
    // ...
    // @step When the client calls list_providers via embedded tarpc
    // ...
    // @step Then the returned Vec contains at least one ProviderInfo
    // @step And the anthropic ProviderInfo carries a non-empty models list
}
```

Plus a source-shape scenario asserting the body of `fn list_providers` in
`handle_impl.rs` contains the string `list_providers_info`.

---

## Phase 2: Bug 1 — Wrap blocking_lock calls in block_in_place

### Goal

Every sync `SessionManagerHandle` trait override on `SessionManager` that
internally reaches a `blocking_lock()` on `BackgroundSession::inner` MUST be
wrapped in `tokio::task::block_in_place(...)` so it never panics when called
from a multi-thread tokio worker.

### Audit step

Run a one-time grep:

```bash
rg -n "blocking_lock\(\)" codelet/sessions/src/background_session.rs
```

For each match, locate its caller chain back into `handle_impl.rs` and wrap.

### Confirmed wrap sites in handle_impl.rs

- `clear_history` (line 229)
- `compact_session` (line 240) — calls `session.get_tokens()`; check
- `set_model`, `set_role`, `set_thinking_level`, `destroy_session`,
  `restore_session_messages`, `restore_session_token_state`,
  `set_work_unit_context` — audit each

### Pattern

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

### Tests

1. **Reproduce + assert no-panic**: `codelet/fspec/tests/rpc073_clear_history_no_panic.rs`
   modelled on `rpc070_create_session_no_panic.rs`. Spin up an embedded tarpc
   transport on a multi-thread runtime, create a session, call `clear_history`,
   assert it returns Ok(()) without panic.

2. **Source-shape regression**: Extend `rpc070_create_session_no_panic.rs` (or
   add a sibling) to scan `handle_impl.rs` for the lock-pattern. For every
   sync trait override that contains the substring `.blocking_lock()` reached
   from a session call, assert the method body contains
   `tokio::task::block_in_place`.

3. **E2E**: Extend `e2e/rpc-072-work-agent-roundtrip.test.ts` (or add a
   sibling `e2e/rpc-073-slash-clear-no-panic.test.ts`) — launch the fspec
   binary, open a Work Agent, send `hello`, send `/clear`, assert the process
   is still alive and the scrollback is cleared.

---

## Phase 3: Bug 2 — Invert App dispatch priority

### Goal

App-level fallback shortcuts (`?`, `q`, Ctrl+D) only fire when neither the
Compositor nor the Navigator consume the keystroke. The AgentView's input
field becomes type-able with `?` and `q`.

### Files

| File | Change |
|------|--------|
| `codelet/fspec-tui/src/app/events.rs:34-71` | Reorder `handle_event` body |
| Existing keyboard tests | Audit + update — most should still pass, but any that depended on `?` opening HelpDialog from any view need to reaffirm they push from BoardView only |
| `codelet/fspec-tui/tests/agent_input_typeable_chars_rpc073.rs` | New regression test |

### Code change in `events.rs:handle_event`

Replace the current body with:

```rust
pub fn handle_event(&mut self, event: &Event) -> EventResult {
    let topmost_is_critical = matches!(
        self.compositor.topmost_priority(),
        Some(Priority::Critical)
    );
    let topmost_is_disconnect =
        self.compositor.topmost_id().as_deref() == Some(DISCONNECT_DIALOG_ID);

    if topmost_is_disconnect {
        return self.handle_disconnect_dialog_event(event);
    }

    // 1. Compositor (popups, modals, dialogs) gets first crack.
    let composer_result = self.compositor.handle_event(event);
    if let EventResult::Consumed(Some(callback)) = composer_result {
        callback(&mut self.compositor);
        self.should_render = true;
        return EventResult::consumed();
    }
    if composer_result.is_consumed() {
        self.should_render = true;
        return composer_result;
    }

    // 2. Navigator (BoardView or AgentView with its input) gets next crack.
    let nav_result = self.navigator.handle_event(event, &self.board_store);
    if nav_result.is_consumed() {
        self.should_render = true;
        return nav_result;
    }

    // 3. App-level fallback shortcuts only if nobody consumed the event.
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

### Required navigator change

`navigator.handle_event` must return `Ignored` (not `Consumed`) when it has
NOTHING to do with the keystroke — otherwise `?` will be silently swallowed
by an idle navigator instead of falling through to the app-shortcut handler.

Audit `views/navigator.rs` + `views/board.rs` + `views/agent.rs` to confirm
they each return `Ignored` for unmatched keys.

### Tests

New integration test
`codelet/fspec-tui/tests/agent_input_typeable_chars_rpc073.rs`:

```rust
//! Feature: spec/features/rpc073-agent-input-typeable-chars.feature

#[test]
fn question_mark_in_agent_input_is_appended_to_buffer() {
    let mut app = App::new_for_test_in_agent_view();
    let _ = app.handle_event(&synth_key(KeyCode::Char('?')));
    assert_eq!(app.agent_view_input_text(), "?");
    assert!(!app.help_dialog_is_open(), "HelpDialog must not open from AgentView input");
}

#[test]
fn q_in_agent_input_does_not_quit_app() {
    let mut app = App::new_for_test_in_agent_view();
    let _ = app.handle_event(&synth_key(KeyCode::Char('q')));
    assert_eq!(app.agent_view_input_text(), "q");
    assert!(!app.should_quit, "`q` must not quit when typed into AgentView input");
}

#[test]
fn question_mark_from_board_view_still_opens_help_dialog() {
    let mut app = App::new_for_test_in_board_view();
    let _ = app.handle_event(&synth_key(KeyCode::Char('?')));
    assert!(app.help_dialog_is_open(), "HelpDialog must open from BoardView");
}

#[test]
fn q_from_board_view_still_quits() {
    let mut app = App::new_for_test_in_board_view();
    let _ = app.handle_event(&synth_key(KeyCode::Char('q')));
    assert!(app.should_quit);
}
```

---

## Feature files (Gherkin scaffolding)

Two or three feature files (one per bug), each with `@RPC-073` work-unit tag.
Use `fspec generate-scenarios` after the Example Mapping cards are in place,
or write manually using `fspec add-scenario` + `fspec add-step`.

Suggested capability-named feature files:

- `spec/features/rpc073-slash-clear-no-panic.feature`
- `spec/features/rpc073-agent-input-typeable-chars.feature`
- `spec/features/rpc073-list-providers-wiring.feature`

---

## Verification checklist

After landing all three phases:

- [ ] `cargo test -p codelet-sessions -p codelet-fspec -p codelet-fspec-tui`
      passes 100% with the new rpc073 tests.
- [ ] `cargo test --workspace --no-fail-fast` shows no NEW failures
      (pre-existing untracked-file failures from the codelet-integration
      branch remain unchanged).
- [ ] E2E: `npx tui-test e2e/rpc-073-slash-clear-no-panic.test.ts` passes.
- [ ] Manual: build release binary, open Work Agent, type
      `hello? is this card done?`, see the `?` characters in the buffer,
      submit, see a real response chunk arrive.
- [ ] Manual: open model dialog, see at least anthropic in the list.
- [ ] Manual: send `/clear`, scrollback clears, no panic.
