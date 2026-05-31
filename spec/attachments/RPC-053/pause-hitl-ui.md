# RPC-053 — Pause / HITL UI (`ConfirmDialog` + `HitlDialog` end-to-end)

**Parent:** RPC-030 · **Phase:** 6.8 · **Estimate:** 8 pts · **Depends on:** RPC-052

## Goal

Wire the pause / human-in-the-loop (HITL) flow end-to-end on the Rust AgentView. Two interaction shapes:

1. **Pause (tool approval)** — `ConfirmDialog` (2-choice) or `TripleConfirmDialog` (3-choice).
2. **HITL (request user input)** — `HitlDialog` with structured options + free text.

## Backend state (already lifted in RPC-039/040)

`BackgroundSession` fields (lines 539-555 in original `session_manager.rs`):

```rust
pause_state: RwLock<Option<PauseState>>,
pause_response_tx/rx: std::sync::mpsc Sender/Receiver<PauseResponse>,
hitl_response_tx/rx: mpsc Sender/Receiver<codelet_tools::request_user_input::HitlResponse>,
hitl_request: RwLock<Option<HitlRequest>>,
```

`BackgroundSession` methods:
- `get_pause_state() -> Option<PauseState>` (line 1011)
- `set_pause_state(state)` (1019), `clear_pause_state()` (1030)
- `wait_for_pause_response() -> PauseResponse` (1038), `send_pause_response(response)` (1052)
- `get_hitl_request() -> Option<HitlRequest>` (1137), `set_hitl_request(request)` (1128)
- `wait_for_hitl_response() -> HitlResponse` (1106), `send_hitl_response(response)` (1118)

## Trait wiring (already in RPC-037)

- `FspecBackend::get_pause_state(SessionId) -> Result<Option<PauseState>>`
- `FspecBackend::pause_resume(SessionId) -> Result<()>`
- `FspecBackend::pause_confirm(SessionId, accept: bool) -> Result<()>`
- `FspecBackend::pause_triple(SessionId, choice: ApprovalChoice) -> Result<()>`
- `FspecBackend::get_hitl_request(SessionId) -> Result<Option<HitlRequest>>`
- `FspecBackend::send_hitl_response(SessionId, HitlResponse) -> Result<()>`

## Triggering — chunk variant

When the agent loop hits a pause point, `BackgroundSession::set_pause_state(...)` is called AND a chunk is broadcast. There are two possible designs:

**Option A — Use `SessionStateChange { state: Paused }`** (already exists). UI polls `get_pause_state` after seeing the state change. **Recommended.**

**Option B — Add a `PauseRequested { state: PauseState }` chunk variant.** More direct but requires another `StreamChunk` variant (RPC-036 didn't add one — pause state currently flows via `SessionStateChange` + the implicit `SupervisorPendingInjection`).

Stick with Option A.

## Work

### Step 1 — Dispatcher reacts to `SessionStateChange { Paused }`

In RPC-045's chunk dispatcher, when receiving `SessionStateChange { state: SessionState::Paused }`:

```rust
StreamChunk::SessionStateChange { state: SessionState::Paused } => {
    // existing: update store status
    self.agent_view_store.set_session_status(session_id.clone(), SessionStatus::Paused);
    // new: fetch pause state and open dialog
    let backend = self.backend.clone();
    let sender = self.dispatch_sender.clone();
    let id = session_id.clone();
    tokio::spawn(async move {
        if let Ok(Some(state)) = backend.get_pause_state(id.clone()).await {
            let _ = sender.send(Action::OpenPauseDialog { session_id: id, state });
        }
    });
}
```

### Step 2 — Pause dialog component

Create `codelet/fspec-tui/src/components/dialogs/pause_dialog.rs` matching the existing dialog theming (RPC-027). Renders different layouts for `PauseKind::Confirm` (2 buttons: Accept / Deny) and `PauseKind::Triple` (3 buttons: Approve / Approve Session / Deny).

Theme: reuse `ConfirmDialog` infrastructure from RPC-027.

### Step 3 — Pause dialog actions

```rust
Action::OpenPauseDialog { session_id, state } => {
    let dialog = PauseDialog::new(session_id, state);
    self.compositor.push(Box::new(dialog));
}

Action::PauseConfirmed { session_id, accept } => {
    self.compositor.pop_topmost();
    let backend = self.backend.clone();
    tokio::spawn(async move {
        let _ = backend.pause_confirm(session_id, accept).await;
    });
}

Action::PauseTriple { session_id, choice } => {
    self.compositor.pop_topmost();
    let backend = self.backend.clone();
    tokio::spawn(async move {
        let _ = backend.pause_triple(session_id, choice).await;
    });
}

Action::PauseResume { session_id } => {
    // Triggered by Esc or explicit "resume" — different from interrupt.
    self.compositor.pop_topmost();
    let backend = self.backend.clone();
    tokio::spawn(async move {
        let _ = backend.pause_resume(session_id).await;
    });
}
```

### Step 4 — HITL dialog component

Create `codelet/fspec-tui/src/components/dialogs/hitl_dialog.rs`. Renders the `HitlRequest`:

```
┌── HITL: question text ─────────────────────┐
│ Description / header text                  │
│                                            │
│ ▸ [a] Option label one                     │
│   [b] Option label two                     │
│   [c] Free-text…                           │
└────────────────────────────────────────────┘
```

Key handling: hotkey per option, free-text input on `c`, `Enter` to submit.

### Step 5 — HITL dialog trigger

The agent loop emits a chunk when calling `request_user_input`. Identify the chunk — likely `SupervisorPendingInjection` is reused, OR introduce a new mechanism. Confirm by reading `codelet_tools::request_user_input` and how it signals the UI today.

Reasonable approach: agent calls `set_hitl_request(req)` AND `chunks_tx.send(SupervisorPendingInjection { ... })` (or a dedicated `HitlRequested` chunk variant — add to RPC-036 if missing). UI sees the chunk, calls `get_hitl_request`, opens the dialog.

For this card, document the trigger but defer "make TS emit a HitlRequested chunk variant" to a follow-up if it isn't already wired.

### Step 6 — HITL submit

```rust
Action::HitlSubmitted { session_id, response } => {
    self.compositor.pop_topmost();
    let backend = self.backend.clone();
    tokio::spawn(async move {
        let _ = backend.send_hitl_response(session_id, response).await;
    });
}
```

## Acceptance criteria

1. PauseDialog opens when session pauses (2-choice and 3-choice variants).
2. Accept / Deny / Approve Session buttons send the right `pause_*` call.
3. Esc on PauseDialog calls `pause_resume` (treating it as "user dismissed without choosing").
4. HitlDialog opens when `get_hitl_request` returns Some.
5. HitlDialog supports option-hotkey selection + free-text input.
6. Submit sends `HitlResponse { id, value }`.
7. Integration test in `codelet/fspec-tui/tests/pause_hitl.rs` drives a stub backend through both dialog types.

## Risks

- The agent loop is currently `mpsc::Receiver<PauseResponse>`-driven. The RPC boundary makes this async — `pause_confirm` must complete the backend's pause-response oneshot. Verify via `BackgroundSession::send_pause_response` (line 1052).
- `HitlRequest` may include serialised `serde_json::Value` shapes (image data, etc.). Confirm `HitlRequest` struct in RPC-036 has a clean wire shape.
- Tool-call ID round-trip: PauseDialog needs `tool_call_id` to associate the response with the right call. Already in `PauseState.tool_call_id`.

## Out of scope

- Skipping pause for trusted tool patterns (a TS feature; not yet ported).
