# RPC-052 AST research — existing patterns to reuse

Date: 2026-05-23

## Targets

For RPC-052 (pending-input draft persistence) we need to:
1. Add new `Action` variants to a fixed-size enum without breaking `match` arms.
2. Emit a per-keystroke action from `AgentView::handle_event`.
3. Spawn a debounced tokio task that aborts the previous one on every keystroke.
4. Spawn a hydration task on session activation and fold the result via the action bus.
5. Spawn a clear-draft task after `Action::InputSubmitted`.
6. Extend `MockBackend` with `set_pending_input` / `get_pending_input` scripting.

Below is the AST + grep evidence the implementation slots into existing precedent without inventing new infrastructure.

## 1. Action enum extension precedent

`grep "RPC-051" codelet/fspec-tui/src/components/mod.rs` → `AgentEscPressed` variant
added inline at the tail of the enum. Same approach used by RPC-046 / RPC-049 /
RPC-050. Each was a single `#[derive(Debug, Clone)]` enum variant with a doc
comment block describing the action's contract.

For RPC-052 we will follow the exact same pattern:

```rust
/// RPC-052: emitted by AgentView's MultiLineInput when the buffer text
/// changes after a keystroke. ...
PendingInputChanged(String),

/// RPC-052: emitted by the spawned `backend.get_pending_input` task ...
SeedPendingInput {
    session_id: codelet_rpc_types::SessionId,
    text: String,
},
```

## 2. AgentView::handle_event emission point

`codelet/fspec-tui/src/views/agent/dispatch.rs` shows the established pattern:

```rust
let outcome = self.input.handle_event(event);
self.sync_popups();
match outcome {
    InputEventOutcome::Submitted(value) => { self.emit(Action::InputSubmitted(value)); ... }
    InputEventOutcome::Continued => EventResult::consumed(),
    InputEventOutcome::Ignored => EventResult::ignored(),
}
```

For RPC-052 we will snapshot the buffer value BEFORE `self.input.handle_event(event)`
and compare AFTER to suppress cursor-only `Continued` outcomes:

```rust
let before = self.input.value();
let outcome = self.input.handle_event(event);
self.sync_popups();
match outcome {
    InputEventOutcome::Submitted(value) => { ... }
    InputEventOutcome::Continued => {
        let after = self.input.value();
        if after != before {
            self.emit(Action::PendingInputChanged(after));
        }
        EventResult::consumed()
    }
    InputEventOutcome::Ignored => EventResult::ignored(),
}
```

## 3. Debounced spawn-and-abort precedent

No existing dispatcher has a debounced abort-and-respawn. Closest precedents:

- `pending_input_save_handle: Option<JoinHandle<()>>` on `App` would mirror the
  shape of existing fields like `subscriber_tasks: Vec<JoinHandle<()>>` and
  `pending_tasks: Vec<JoinHandle<()>>` (state.rs:50, 55).
- Spawn pattern (sleep then RPC) is identical to the post-Submit persistence
  task (`dispatch_rpc025.rs::handle_input_submitted_persistence`, line 173):
  ```rust
  let backend = self.backend.clone();
  let handle = tokio::spawn(async move {
      tokio::time::sleep(Duration::from_millis(300)).await;
      let _ = backend.set_pending_input(session, Some(text)).await;
  });
  ```
- Aborting an `Option<JoinHandle<()>>`: standard tokio API
  (`if let Some(h) = self.pending_input_save_handle.take() { h.abort(); }`).

## 4. Hydration on session activation precedent

`codelet/fspec-tui/src/app/dispatch_rpc018.rs::handle_session_chrome_refresh`
(lines 31, 40) spawns `backend.get_model_info` + `backend.get_thinking_level`
on `Action::SessionCreated` and routes the result through
`Action::ModelInfoLoaded` / `Action::ThinkingLevelLoaded`. We mirror that:

```rust
fn spawn_hydrate_pending_input(&mut self, session_id: SessionId) {
    if tokio::runtime::Handle::try_current().is_err() { return; }
    let backend = self.backend.clone();
    let action_tx = self.action_tx.clone();
    let id_for_task = session_id.clone();
    let handle = tokio::spawn(async move {
        match backend.get_pending_input(id_for_task.clone()).await {
            Ok(Some(text)) => { let _ = action_tx.send(Action::SeedPendingInput { session_id: id_for_task, text }); }
            _ => {}
        }
    });
    self.pending_tasks.push(handle);
}
```

## 5. Folding SeedPendingInput via dispatch

`dispatch_rpc050.rs::handle_work_unit_attached` shows the per-session fold
pattern: read `current_session()` from `AgentViewStore`, mutate
`SessionContext` via `session_context_mut_for(&session_id)`, and only seed
the live MultiLineInput when the supplied id matches the focused session.

```rust
fn handle_seed_pending_input(&mut self, session_id: SessionId, text: String) {
    if self.agent_view_store.current_session() == Some(&session_id) {
        self.navigator.agent.input.set_value(&text);
    }
    if let Some(ctx) = self.agent_view_store.session_context_mut_for(&session_id) {
        ctx.input_draft = text;
    }
}
```

## 6. MockBackend scripting precedent

`tests/common/mod.rs` lines 1166–1196 show the pattern for
`set_work_unit_context` (counter + last-arg slot + error-scripting slot).
RPC-052 mirrors:

```rust
// fields
pending_input_get_calls: AtomicUsize,
pending_input_set_calls: AtomicUsize,
last_set_pending_input: Mutex<Option<(SessionId, Option<String>)>>,
scripted_pending_input: Mutex<HashMap<SessionId, Option<String>>>,
get_pending_input_error: Mutex<Option<String>>,
set_pending_input_error: Mutex<Option<String>>,

// trait impl methods (override the FspecBackend defaults)
async fn get_pending_input(&self, session_id: SessionId) -> Result<Option<String>> { ... }
async fn set_pending_input(&self, session_id: SessionId, text: Option<String>) -> Result<()> { ... }
```

## 7. dispatch.rs 300-LoC ceiling

`grep -c "^" app/dispatch.rs` returns ~296. We CANNOT add new arms inline —
the new PendingInputChanged + SeedPendingInput arms will route to a fall-back
`_ => self.try_dispatch_rpc022(&action)` extension OR be added to a new arm
that delegates to `dispatch_rpc052::handle_pending_input_changed` /
`handle_seed_pending_input`. The pattern is identical to RPC-051's helper
file `dispatch_rpc051.rs`.

We will add the two arms via `try_dispatch_rpc022` (already the catch-all
that routes overflow arms to RPC-052 helpers) OR via a small extension to
dispatch.rs that stays under 300 LoC. Final decision in implementing phase.

## Conclusion

Every required piece of the RPC-052 wiring has a 1:1 precedent in the current
codebase. No new infrastructure is needed; the slice is purely an additional
debounced spawn pattern + action-bus fold layered on top of the existing
single-task `App::dispatch` pattern.
