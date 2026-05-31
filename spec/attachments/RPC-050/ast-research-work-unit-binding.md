# RPC-050 — AST Research: work-unit context binding wiring

Captured during the SPECIFYING phase to confirm the trait surface and integration points before writing tests.

## 1. Trait + service surface (already in place from RPC-037)

### `codelet/core/src/session_manager_handle.rs`
```text
332:    fn set_work_unit_context(
333:        &self,
334:        _session_id: &SessionId,
335:        _ctx: Option<WorkUnitContext>,
336:    ) -> Result<(), String> {
337:        Ok(())
338:    }

874: fn get_work_unit_context(&self, session_id: &SessionId) -> Option<WorkUnitContext>
881: fn set_work_unit_context(
882:     &self,
883:     session_id: &SessionId,
884:     ctx: Option<WorkUnitContext>,
885: ) -> Result<(), String>
```

StubSessionManagerHandle (lines ~500–588) already stores `work_unit_ctx: Arc<Mutex<HashMap<SessionId, WorkUnitContext>>>`, so set/get round-trip is already faithful. It DOES NOT yet expose call counters — RPC-050 will add two `AtomicU64` counters mirroring the `resume_session_calls` pattern at lines 521 / 581 / 588.

### `codelet/sessions/src/handle_impl.rs`
```text
277:    fn set_work_unit_context(
```
Real `SessionManager` impl already delegates to `BackgroundSession`.

### `codelet/rpc/src/lib.rs`
```text
253: async fn get_work_unit_context(session_id: SessionId) -> Option<WorkUnitContext>;
256: async fn set_work_unit_context(...);
1063: async fn get_work_unit_context(...)  // FspecServiceImpl::get_work_unit_context
1074: async fn set_work_unit_context(...)  // FspecServiceImpl::set_work_unit_context
```

### `codelet/fspec-tui/src/transport/{embedded,websocket}.rs`
Both backends already declare:
- `async fn get_work_unit_context(&self, session_id: SessionId) -> Result<Option<WorkUnitContext>>`
- `async fn set_work_unit_context(&self, session_id: SessionId, context: Option<WorkUnitContext>) -> Result<()>`

No new transport-level work for RPC-050.

## 2. Read sites that must change

### `codelet/fspec-tui/src/views/agent.rs`
```text
248:            work_unit_id: store.current_work_unit_id(),
249:            work_unit_status: store.current_work_unit_status(),
```

These are the only two read sites in the AgentView render path. RPC-050 changes both to prefer `store.work_unit_context_for(sid)` and fall back to the legacy slots so RPC-029 chrome-parity tests keep passing.

## 3. Slash command registry

### `codelet/fspec-tui/src/views/agent/slash_commands.rs`
`SlashCommandAction::Detach` already declared (line 36) and registered with description "Detach session from work unit" (line 145). Currently lands in `handle_slash_command`'s `other => [notice] not yet implemented` catch-all.

## 4. Dispatch routing reference (mirror pattern)

### `codelet/fspec-tui/src/app/dispatch_rpc046.rs` (handle_emit_session_notice — 31 lines)
This is the prototype for the small helper that routes a `[notice]` / `[error]` line back to the *originating* session regardless of focus.

### `codelet/fspec-tui/src/app/dispatch_rpc026.rs::handle_session_resume_complete` (~115 lines)
This is the prototype for "spawn backend round-trip → dispatch follow-up action" pattern that RPC-050's attach + detach flows will mirror.

## 5. New shape (target)

### New file: `codelet/fspec-tui/src/app/dispatch_rpc050.rs`
Holds:
- `handle_attach_work_unit_to_session(work_unit_id: String)` — spawns backend.set_work_unit_context(s, Some(ctx)) → Ok→Action::WorkUnitAttached(s, ctx), Err→Action::EmitSessionNotice(s, "[error] /attach failed: …").  No-session branch emits Action::Custom("[notice] /attach: no active session — create one first").
- `handle_work_unit_attached(session, ctx)` — folds into store.work_unit_context_by_session via store.set_work_unit_context(session, ctx).
- `handle_work_unit_detached(session)` — clears the binding, resets scrollback via `navigator.agent.reset_scrollback`, and calls `store.reset_token_state(&session)`.
- `handle_slash_detach()` — the /detach entry point: (1) no-session → silent return, (2) no-binding → Action::EmitSessionNotice(s, "[notice] /detach: no work unit attached"), (3) bound → spawn backend.set_work_unit_context(s, None) → Ok→Action::WorkUnitDetached(s), Err→Action::EmitSessionNotice(s, "[error] /detach failed: {e}").

### New AgentViewStore fields + methods (in `store/agent_view.rs`)
- `work_unit_context_by_session: HashMap<SessionId, WorkUnitContext>` (private)
- `pub fn work_unit_context_for(&self, session: &SessionId) -> Option<&WorkUnitContext>`
- `pub fn set_work_unit_context(&mut self, session: SessionId, ctx: WorkUnitContext)`
- `pub fn clear_work_unit_context(&mut self, session: &SessionId)`
- `pub fn reset_token_state(&mut self, session: &SessionId)` — wipes the entry in `token_state_by_session`.

### New `components::Action` variants
- `AttachWorkUnitToSession(String)` — board → app
- `WorkUnitAttached(SessionId, WorkUnitContext)` — spawned task → app
- `WorkUnitDetached(SessionId)` — spawned task → app

### Stub counters
Add to `StubSessionManagerHandle` (mirror the RPC-049 `resume_session_calls` pattern):
- `set_work_unit_context_calls: AtomicU64`
- `get_work_unit_context_calls: AtomicU64`
- `pub fn set_work_unit_context_calls(&self) -> u64`
- `pub fn get_work_unit_context_calls(&self) -> u64`

## 6. Integration points (who calls)

- **BoardView**: existing Enter handler emits `Action::EnterWorkUnit`. The new `Action::AttachWorkUnitToSession` is an explicit alternative path. EnterWorkUnit dispatch arm gains a follow-up spawn that calls backend.set_work_unit_context once the session is known (immediately if current_session is Some, deferred to SessionCreated arm if lazily created).
- **App::dispatch**: routes the three new Action variants through the new helpers in `dispatch_rpc050.rs`.
- **AgentView render**: pulls per-session context for the SessionHeader chip.
- **dispatch_rpc020.rs::handle_slash_command**: the `SlashCommandAction::Detach` arm calls `self.handle_slash_detach()`.

## 7. 300-LoC budget snapshot (current)

```
codelet/fspec-tui/src/app/dispatch.rs        — 299 lines (after RPC-049)
codelet/fspec-tui/src/app/dispatch_rpc020.rs — 282 lines (after RPC-048)
codelet/fspec-tui/src/app/dispatch_rpc026.rs — 218 lines (after RPC-049)
codelet/fspec-tui/src/app/dispatch_rpc046.rs —  31 lines
```

dispatch.rs is at 299 — adding new Action variants must NOT push it over 300. Plan: the WorkUnitAttached / WorkUnitDetached / AttachWorkUnitToSession arms in dispatch.rs each route to a one-liner helper call (`self.handle_*()`) so the LoC delta is +4 lines max (one `Action::Variant => self.helper(...)` per variant, plus a follow-up line in EnterWorkUnit). If the file would go ≥300 we move the entire RPC-050 routing block into the existing `try_dispatch_rpc022` follow-up via the `_ => self.try_dispatch_rpc050(&action)` pattern.
