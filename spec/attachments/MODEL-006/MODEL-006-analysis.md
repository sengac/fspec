# MODEL-006: Selecting a model in `/model` with no active session does nothing

## Summary

When a user opens the `/model` view while **no session is active**
(`session_id = None`) and selects a model, the TUI calls
`backend.set_default_model(...)` (which succeeds) but **never re-triggers
`create_session`**. The default model is recorded in memory, the
ModelSelector view closes back to the Agent view, and then nothing else
happens. From the user's perspective, "selecting a model does nothing" — the
agent still has no usable session.

This is the **primary, user-visible** bug. (The companion bug PROV-119 covers
the fact that the default model is also not persisted across restarts.)

## Evidence (from `~/.fspec/logs/fspec-combined.log.2026-06-24`)

Two separate process runs in the log show the identical dead-end sequence. The
relevant lines from the first run:

```
26: ERROR create_session declined: no default model set (PROV-101: no anthropic fallback)
...
54: INFO [MODEL-SELECT] Enter -> EMIT Action::ModelSelected session_id=None provider_key=anthropic model_id=claude-opus-4-8
58: INFO [MODEL-SELECT] handle_model_selected: session_id is None -> setting DEFAULT model (PROV-118)
59: INFO [MODEL-SELECT] navigator apply_action: closing ModelSelector view -> Agent
67: INFO [MODEL-SELECT] backend.set_default_model OK model=anthropic/claude-opus-4-8
```

Line 67 (`set_default_model OK`) is the **last meaningful event** in the run.
No `create_session` RPC follows it. The session that was declined at line 26 is
never retried.

## Root Cause

File: `codelet/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs`
Function: `handle_model_selected` (lines ~53–105)

```rust
let Some(session_id) = session_id else {
    // session_id == None branch
    ...
    let handle = tokio::spawn(async move {
        match backend.set_default_model(model_string.clone()).await {
            Ok(()) => tracing::info!(... "set_default_model OK"),
            Err(e) => tracing::error!(... "set_default_model FAILED"),
        }
    });
    self.pending_tasks.push(handle);
    return;   // <-- DEAD END: nothing creates a session afterward
};
```

The doc comment above the function even *claims* the deadlock is broken
"...so the **next** `create_session` succeeds" — but there is **no next
`create_session`**. Nothing schedules one.

### Why the navigator does not save it

File: `codelet/fspec-tui/src/views/navigator.rs` (lines ~122–130)

```rust
Action::CloseModelSelectorView | Action::ModelSelected(..)
    if self.active_view == ViewMode::ModelSelector =>
{
    self.active_view = ViewMode::Agent;   // only flips the view; emits nothing
}
```

Closing the selector flips `active_view` back to `Agent` but emits neither
`EnterWorkUnit` nor `OpenAgentView(Some(_))`.

### Why the existing retry never fires

The only "create a session if none exists" retry lives in the
`Action::EnterWorkUnit` arm of `codelet/fspec-tui/src/app/dispatch.rs`
(lines ~78–109, the `current_session().is_none()` block). Because closing the
model selector emits neither `EnterWorkUnit` nor `OpenAgentView(Some(_))`, that
retry path is never reached.

**Net effect:** the PROV-101 deadlock guard (which correctly refuses a silent
anthropic fallback — see `codelet/sessions/src/handle_impl.rs:89` and
`codelet/sessions/src/session_manager.rs:220`) is only *half*-fixed. Picking a
model sets the default but the UI never re-attempts the session creation the
default was meant to unblock.

## Expected Behaviour

After a user selects a model in `/model` while no session is active:

1. The default model is set (already works).
2. Once the default is committed, the TUI **re-attempts session creation**.
3. On success an `Action::SessionCreated` is dispatched and the Agent view
   becomes usable.
4. On a (now-unexpected) decline, the existing `SessionCreationDeclined`
   error-dialog path is shown — never a silent no-op.

## Suggested Implementation Direction

In the `session_id == None` branch of `handle_model_selected`, after
`set_default_model` returns `Ok(())` **inside the spawned task** (so it runs
*after* the default is committed), dispatch a follow-up that funnels through the
existing session-creation routing rather than `return`-ing.

Preferred: send an action that already routes through
`post_create_session_action` / `route_bootstrap_create_session`
(`codelet/fspec-tui/src/app/session_creation.rs`) — e.g. trigger the same lazy
create path used by `Action::EnterWorkUnit`, or emit
`Action::CreateSessionSubmitted { isolated: false }`.

Key constraints:
- The re-create must happen **only after** `set_default_model` succeeds
  (ordering matters; do it on the task's `Ok` branch).
- An empty `SessionId` must still map to `Action::SessionCreationDeclined`
  (never seed an empty active session — PROV-101 FIX 1 invariant).
- Preserve the `tokio::runtime::Handle::try_current().is_err()` guard already
  present (no-runtime → skip).

## Acceptance Criteria (for Example Mapping)

- **Rule:** Selecting a model with no active session must result in a usable
  session being created (or an explicit decline dialog), never a silent no-op.
- **Example:** No session active → open `/model` → pick
  `anthropic/claude-opus-4-8` → default model set → `create_session` retried →
  `Action::SessionCreated` dispatched → Agent view usable.
- **Example:** `set_default_model` succeeds but the retried `create_session`
  still returns an empty id → `Action::SessionCreationDeclined` → error dialog
  shown.
- **Rule:** The re-creation only fires after `set_default_model` resolves `Ok`.

## Key Files

| File | Role |
|------|------|
| `codelet/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs` | `handle_model_selected` — the dead-end branch to fix |
| `codelet/fspec-tui/src/app/session_creation.rs` | `post_create_session_action` / `route_bootstrap_create_session` routing helpers |
| `codelet/fspec-tui/src/app/dispatch.rs` | `EnterWorkUnit` arm with the existing `current_session().is_none()` retry |
| `codelet/fspec-tui/src/views/navigator.rs` | view-flip on `ModelSelected` |
| `codelet/fspec-tui/src/app/dispatch_create_session_dialog.rs` | `handle_session_creation_declined` error dialog |

## Out of Scope

- Persisting the default model across process restarts — tracked separately in
  **PROV-119**.
