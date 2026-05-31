# RPC-053 — AST Research: Existing Patterns to Mirror

Research conducted via AST analysis of the codelet/fspec-tui crate to anchor
the implementation in established conventions before writing tests.

## Existing dialog Component impls (template for PauseDialog + HitlDialog)

```
codelet/fspec-tui/src/components/help_dialog.rs:54           impl Component for HelpDialog
codelet/fspec-tui/src/components/hello.rs:42                 impl Component for HelloComponent
codelet/fspec-tui/src/components/model_selector_dialog.rs:159 impl Component for ModelSelectorDialog
codelet/fspec-tui/src/components/thinking_level_dialog.rs:101 impl Component for ThinkingLevelDialog
codelet/fspec-tui/src/components/disconnect_dialog.rs:76      impl Component for DisconnectDialog
```

### Canonical dialog skeleton (lifted from `model_selector_dialog.rs`)

1. Module doc comment naming the feature file + RPC card.
2. Stable id constant: `pub const PAUSE_DIALOG_ID: &str = "pause-dialog";`
3. Struct with: `id: String`, bound `session_id: SessionId`, optional
   `action_tx: Option<UnboundedSender<Action>>` and
   `pending_action: Option<Action>` for sync unit-test path.
4. `new(...)`, `with_action_tx(tx)` builder.
5. `impl Component for $TYPE`:
   - `priority() -> Priority::Critical` (per attachment + RPC-027 doc 09).
   - `id() -> &str` returning the stable id.
   - `handle_event(&mut self, event: &Event) -> EventResult` with Esc/Enter/arrow keys.
   - `render(&mut self, area, buf)` delegating to
     `dialog_theme::render_dialog` with an `FspecDialog` builder.
6. Esc returns `Consumed(Some(Box::new(|c| { let _ = c.remove(&id); })))`.
7. Enter on selection emits action via `action_tx` (Some) AND stashes into
   `pending_action` (None path for sync tests).
8. Render uses `Accent::Yellow` for PauseDialog (matches ThinkingLevelDialog)
   and `Accent::Cyan` for HitlDialog (matches ModelSelectorDialog).

### Mouse handling pattern

`ModelSelectorDialog` and `ThinkingLevelDialog` route
`MouseEventKind::ScrollUp` / `ScrollDown` into the same `move_up` /
`move_down` helpers as arrow keys. PauseDialog needs the same (mouse-wheel
nav across the 2/3 buttons or left/right paging); HitlDialog can route
wheel into option-row selection.

## Existing App dispatch handlers (template for dispatch_rpc053.rs)

```
codelet/fspec-tui/src/app/dispatch_rpc022.rs : handle_open_model_dialog, handle_open_thinking_dialog, …
codelet/fspec-tui/src/app/dispatch_rpc045.rs : handle_stream_chunk_state_updates, handle_session_status_changed, spawn_fspec_command_runner
codelet/fspec-tui/src/app/dispatch_rpc046.rs : handle_emit_session_notice
codelet/fspec-tui/src/app/dispatch_rpc050.rs : handle_attach_work_unit_to_session, handle_work_unit_attached, handle_work_unit_detached, handle_slash_detach
codelet/fspec-tui/src/app/dispatch_rpc052.rs : handle_pending_input_changed, handle_seed_pending_input, spawn_clear_pending_input, spawn_hydrate_pending_input
```

### Spawn-task-then-route-action idiom (lifted from `dispatch_rpc050.rs`)

```rust
pub(crate) fn handle_X(&mut self, ...) {
    if tokio::runtime::Handle::try_current().is_err() {
        return;   // synchronous unit-test fallback
    }
    let backend = self.backend.clone();
    let action_tx = self.action_tx.clone();
    let handle = tokio::spawn(async move {
        match backend.method(...).await {
            Ok(v) => { let _ = action_tx.send(Action::OnOk(v)); }
            Err(e) => { /* tracing::debug! and/or EmitSessionNotice */ }
        }
    });
    self.pending_tasks.push(handle);
}
```

### Idempotent compositor push (lifted from `dispatch_rpc022::handle_open_model_dialog`)

```rust
if !self.compositor.contains(MODEL_SELECTOR_DIALOG_ID) {
    self.compositor.push(Box::new(ModelSelectorDialog::new(...)));
}
```

## Chunk-driven trigger wiring (entry point in dispatch_rpc045)

`handle_stream_chunk_state_updates` already branches on
`StreamChunk::SessionStateChange { state }` and calls
`agent_view_store.set_session_status(...)`. RPC-053 extends this arm:

- on `SessionState::Paused` → dispatch a new `Action::PauseChunkReceived(session_id)`
  which routes through `dispatch_rpc053::handle_pause_chunk` (the parallel
  get_pause_state + get_hitl_request fetcher).
- on `SessionState::Running | SessionState::Idle` → dispatch `Action::PauseCleared(session_id)`
  which routes through `handle_pause_cleared` to pop any mounted dialog.

`SessionState::Compacting | Interrupted | Cleared` are out of scope for
pause-clear (they are not direct resume signals — `Interrupted` may even
co-exist with a pause and is handled by the agent loop).

## Compositor API surface used by RPC-053

```
pub fn push(&mut self, component: Box<dyn Component>)
pub fn remove(&mut self, id: &str) -> Option<Box<dyn Component>>
pub fn contains(&self, id: &str) -> bool
pub fn topmost_id(&self) -> Option<String>
```

All four are already public — no compositor extension needed.

## MockBackend extension surface (mirrored from RPC-052)

`tests/common/mod.rs::MockBackend` already exposes the `get_pause_state`,
`get_hitl_request`, `pause_resume`, `pause_confirm`, `pause_triple`,
`send_hitl_response` methods via the `FspecBackend` trait DEFAULTS (they
all currently return `Ok(None)` or `Ok(())`). RPC-053 must override every
one of those on `MockBackend` with:

- per-call counters (`AtomicUsize`),
- per-call captures (`Mutex<Option<(SessionId, ...)>>`),
- scripted return slots,
- scripted error slots.

The RPC-052 pending-input section (lines 864-918 + 1281-1322 in
`tests/common/mod.rs`) is the cleanest template — it exposes the
`script_*`, `set_*_error`, `*_calls`, `last_*` quartet for two methods,
which RPC-053 needs to replicate for six methods.

## Existing test conventions

- `#[tokio::test(flavor = "current_thread")]` for App-level tests (used
  by `slash_resume_rpc049.rs`, `slash_detach_rpc050.rs`).
- `tokio::task::yield_now().await` and short `tokio::time::sleep` after
  spawning to let `pending_tasks` drain before assertions.
- `tests/common/mod.rs` `test_app(backend)` fixture creates
  `App + Terminal<TestBackend>` with the supplied backend.
- Source-shape tests pin file layout (e.g.
  `tests/source_shape_rpc050.rs`); RPC-053 adds `source_shape_rpc053.rs`.

## Key files to modify

```
codelet/fspec-tui/src/components/mod.rs            -- new Action variants + pub mod declarations
codelet/fspec-tui/src/components/pause_dialog.rs   -- NEW
codelet/fspec-tui/src/components/hitl_dialog.rs    -- NEW
codelet/fspec-tui/src/app/dispatch.rs              -- route Action::PauseChunkReceived / PauseCleared / Open* / *Confirmed / *Triple / *Resumed / HitlSubmitted
codelet/fspec-tui/src/app/dispatch_rpc045.rs       -- branch SessionStateChange{Paused|Running|Idle} into the new actions
codelet/fspec-tui/src/app/dispatch_rpc053.rs       -- NEW
codelet/fspec-tui/src/app/mod.rs                   -- pub mod dispatch_rpc053
codelet/fspec-tui/tests/common/mod.rs              -- MockBackend overrides + scripting helpers
codelet/fspec-tui/tests/pause_hitl_rpc053.rs       -- NEW integration test suite
codelet/fspec-tui/tests/source_shape_rpc053.rs     -- NEW source-shape regression
```

## Key trait surface (already present in `FspecBackend` after RPC-037)

```rust
async fn get_pause_state(&self, _session_id: SessionId) -> Result<Option<PauseState>>;
async fn get_hitl_request(&self, _session_id: SessionId) -> Result<Option<HitlRequest>>;
async fn pause_resume(&self, _session_id: SessionId) -> Result<()>;
async fn pause_confirm(&self, _session_id: SessionId, _accept: bool) -> Result<()>;
async fn pause_triple(&self, _session_id: SessionId, _choice: ApprovalChoice) -> Result<()>;
async fn send_hitl_response(&self, _session_id: SessionId, _response: HitlResponse) -> Result<()>;
```

All have default impls returning `Ok(())` / `Ok(None)` so RPC-053's new
MockBackend overrides become the in-test source of truth.

## HITL trigger path (cross-checked against `codelet/napi/src/agent_loop.rs`)

The HITL handler at `codelet/napi/src/agent_loop.rs:781-797` performs:

1. `session_for_hitl.set_hitl_request(Some(request))`
2. `session_for_hitl.set_status(SessionStatus::Paused)`  ← emits chunk
3. `session_for_hitl.wait_for_hitl_response()`
4. `session_for_hitl.set_hitl_request(None)`
5. `session_for_hitl.set_status(SessionStatus::Running)` ← emits chunk

Therefore both pause-confirm and HITL flows arrive at the AgentView via
the SAME `SessionStateChange { state: Paused }` chunk. The dispatcher
disambiguates by polling both `get_pause_state` and `get_hitl_request`
after seeing Paused — HITL wins on tie because the HITL handler is the
only path that populates `hitl_request`.

The resume signal (`SessionStateChange { state: Running }`) is emitted by
BOTH paths after the user submits a response, which is why the
`handle_pause_cleared` arm pops any mounted pause/hitl dialog on Running
or Idle.
