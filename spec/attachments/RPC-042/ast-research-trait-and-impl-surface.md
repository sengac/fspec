# RPC-042 — AST Research: Trait surface and existing impl shape

Research performed during the SPECIFYING phase for RPC-042 to inform the
`impl codelet_core::SessionManagerHandle for codelet_sessions::SessionManager`
work. Three AST queries were executed against the working tree at
`/Users/rquast/projects/fspec/codelet`.

## Query 1 — Existing impls of SessionManagerHandle

```ast-grep
language: rust
pattern: impl codelet_core::SessionManagerHandle for $TYPE { $$$BODY }
path:    codelet/
```

ast-grep cannot currently match multi-line `impl … { … }` blocks, so the
shape was confirmed instead via complementary single-line patterns and
`Grep`. Result: **no `impl codelet_core::SessionManagerHandle for
SessionManager` block currently exists** anywhere in
`codelet/sessions/`, `codelet/napi/`, or `codelet/rpc/`. The only
existing impl is the inline `impl SessionManagerHandle for
StubSessionManagerHandle` in
`codelet/core/src/session_manager_handle.rs` (RPC-037 stub used by
cross-transport parity tests). This card adds the first real
production impl.

## Query 2 — Trait method declarations

```ast-grep
language: rust
pattern: fn $NAME(&self, $$$ARGS) -> $RET;
path:    codelet/core/src/session_manager_handle.rs
```

Result:

```
codelet/core/src/session_manager_handle.rs:71:5:fn create_session(&self, role: Option<String>) -> SessionId;
codelet/core/src/session_manager_handle.rs:83:5:fn get_session_status(&self, session_id: &SessionId) -> SessionStatus;
```

```ast-grep
language: rust
pattern: fn $NAME(&self, $$$ARGS);
path:    codelet/core/src/session_manager_handle.rs
```

Result:

```
codelet/core/src/session_manager_handle.rs:76:5:fn send_input(&self, session_id: &SessionId, text: String);
codelet/core/src/session_manager_handle.rs:80:5:fn interrupt(&self, session_id: &SessionId);
```

The remaining ~45 trait methods have explicit `{ … }` default bodies
inline on the trait (RPC-037), so they do not match the
"signature-only" patterns above. They were enumerated via `grep`:

* `list_sessions`, `chunks_rx`, `chunks_tx`, `logs_rx`, `logs_tx`,
  `status_changes_rx`, `status_changes_tx`
* `get_model_info`, `get_thinking_level`, `list_providers`, `set_model`,
  `set_thinking_level`, `set_thinking_level_default`, `get_role`,
  `set_role`
* `send_input_with_thinking`, `get_session_tokens`, `get_session_model`,
  `get_compaction_progress`, `get_buffered_output`
* `clear_history`, `compact_session`, `restore_session_messages`,
  `restore_session_token_state`
* `get_work_unit_context`, `set_work_unit_context`,
  `get_pending_input`, `set_pending_input`
* `set_active_session`, `clear_active_session`, `get_active_session`
* `get_effective_cwd`, `get_supervisors`
* `get_debug_enabled`, `set_debug_enabled`, `toggle_debug`
* `pause_resume`, `pause_confirm`, `pause_triple`, `send_hitl_response`
* `get_pause_state`, `get_hitl_request`, `send_fspec_result`
* `create_isolated_session`, `destroy_session`

**Total surface: ~49 methods** (4 declared `;`-only, ~45 declared with
default bodies). The RPC-042 impl block must override ALL of them.

## Query 3 — Target struct exists at expected path

```ast-grep
language: rust
pattern: pub struct SessionManager
path:    codelet/sessions/src/session_manager.rs
```

Result:

```
codelet/sessions/src/session_manager.rs:164:1:pub struct SessionManager {
```

Confirms the struct moved by RPC-040 is at the expected path. The
fields RPC-040 added (`chunks_tx`, `logs_tx`, `status_changes_tx`,
`hooks`) and the existing fields (`sessions: RwLock<IndexMap<Uuid,
Arc<BackgroundSession>>>`, `chain_of_command`, `active_session_id`,
`scheduler_handle`, `default_model`) are all visible from line 164
through line 182.

## Implication for RPC-042

The impl block lives in `codelet/sessions/src/session_manager.rs`
(co-located with the struct) OR in a sibling `handle_impl.rs`
re-exported from `lib.rs`. It owns ~49 method overrides. Each method:

1. For per-session methods — calls `self.get_session(&uuid_from(sid).to_string())`,
   pattern-matches on `Ok` vs `Err`, returns safe default on `Err`.
2. For broadcast accessors — delegates to `self.chunks_tx()`/`logs_tx()`/
   `status_changes_tx()` accessors that already exist on `SessionManager`
   (added by RPC-040).
3. For manager-scoped queries (`list_sessions`, active session
   getters/setters) — delegates to existing `SessionManager` methods
   that already operate on `Uuid`.
4. For sync→async bridge methods (`create_session`,
   `create_isolated_session`) — wraps the existing async
   `create_session(model, project)` / `create_isolated_session_with_id`
   methods via `tokio::runtime::Handle::current().block_on(...)`.

The helper `fn uuid_from(id: &SessionId) -> Uuid` parses
`Uuid::parse_str(id.as_str())` with `Uuid::nil()` fallback on parse
failure. Every per-session method uses it exactly once at the top of
its body.
