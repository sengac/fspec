# RPC-024 — AST research: multi-session integration points in the Rust TUI

Performed via the AstGrep tool (rust language) against
`codelet/fspec-tui/src/**/*.rs` while RPC-024 was in `specifying`.

Goal: enumerate every call site that touches the single-slot
`current_session: Option<SessionId>` field on `AgentViewStore` (or the
single `scrollback: ScrollbackList` field on `AgentView`) so the
implementation slice knows exactly which files must be touched when the
field is replaced by a `Vec<SessionContext>` + `current_session_index`.

## 1. `AgentViewStore::set_current_session` — sole producer

```rust
pattern: pub fn $NAME($$$ARGS) { $$$BODY }
```

Match: `store/agent_view.rs:90` —
```rust
pub fn set_current_session(&mut self, session: Option<SessionId>) {
    self.current_session = session;
}
```

This setter goes away in RPC-024. The replacement is implicit:
`Action::SessionCreated` appends a new `SessionContext` and that becomes
the focus by definition.

## 2. `AgentViewStore::current_session()` — read-only callers

```rust
pattern: self.agent_view_store.$METHOD($$$ARGS)
```

Callers that read `current_session()` (after the refactor these all
continue to compile because the accessor signature is preserved — it
just derives the answer from `open_sessions.get(current_session_index)`):

| Site | Purpose |
|---|---|
| `app/dispatch.rs:27` | `Action::Interrupt` — clone the session id to pass into `backend.interrupt(...)` |
| `app/dispatch.rs:93` | `Action::EnterWorkUnit` — gate lazy `create_session` on absence |
| `app/dispatch_rpc020.rs:81` | `handle_input_submitted` — clone the session id to forward to `backend.send_input(...)` |
| `app/state.rs:233` | Test-only helper that reads the current session |
| `views/agent.rs:181` | `render_with_store` — fetch the SessionId for the title strip + per-session chrome (model/thinking/tokens) |

Two important call sites in `views/agent.rs:182-189` chain through the
session id:

```rust
let sid = store.current_session();
let model = sid.and_then(|s| store.model_info_for(s));
let thinking = sid.and_then(|s| store.thinking_level_for(s).copied()) ...
let tokens = sid.and_then(|s| store.token_state_for(s).copied()) ...
```

These keep working unchanged because `current_session()` keeps its
signature and the per-session HashMaps from RPC-018 (`model_info_by_session`,
`thinking_level_by_session`, `token_state_by_session`) are not touched
by RPC-024.

## 3. `agent_view_store.apply_chunk_to_token_state(id, chunk)` — per-session fold

```rust
self.agent_view_store.apply_chunk_to_token_state(id, chunk)
```

Match: `app/dispatch.rs:42` (inside the `Action::ChunkReceived(id, chunk)`
arm). This call ALREADY uses the chunk's `id` (not `current_session`),
so RPC-018's per-session token fold is correctly per-session. RPC-024
must do the SAME for scrollback — route `ctx.record_chunk(chunk)` into
the SessionContext whose `id == id` rather than into the current one.

## 4. `AgentView.scrollback` — every call site

```rust
pattern: self.scrollback.$METHOD($$$ARGS)
```

The seven call sites on `AgentView.scrollback`:

| Site | Method | After RPC-024 |
|---|---|---|
| `views/agent.rs:98` | `.chunk_count()` | route through `store.current_session_context()` |
| `views/agent.rs:116` | `.push(...)` (from `push_line`) | route through `store.current_session_context_mut()` |
| `views/agent.rs:124` | `.reset()` (from `reset_scrollback`) | route through `store.current_session_context_mut()` |
| `views/agent.rs:154` | `.push(...)` (from `record_chunk`) | indirect — `record_chunk` now takes `&mut SessionContext` or looks up by `id` |
| `views/agent.rs:199` | `.render_count_visited(...)` | route through `store.current_session_context()` |
| `views/agent/dispatch.rs:144` | `.scroll_up(...)` | route through `store.current_session_context_mut()` |
| `views/agent/dispatch.rs:148` | `.scroll_down(...)` | route through `store.current_session_context_mut()` |

## 5. `self.navigator.agent.$METHOD` — App-layer consumers

```rust
pattern: self.navigator.agent.$METHOD($$$ARGS)
```

| Site | Method |
|---|---|
| `app/dispatch.rs:38` | `record_chunk(chunk)` — `Action::ChunkReceived` arm; must change to thread the chunk's `id` |
| `app/events.rs:156` | `cursor_position()` |
| `app/dispatch_rpc020.rs:33` | `reset_scrollback()` |
| `app/dispatch_rpc020.rs:40` | (input reset on /clear via `agent.input.reset()`) |
| `app/dispatch_rpc020.rs:68` | `set_file_search_results(matches)` |
| `app/dispatch_rpc020.rs:78` | (slash command dispatch entry) |
| `app/dispatch_rpc020.rs:85` | `push_line(...)` — `[notice] ...` scrollback line |

`reset_scrollback` and `push_line` both need to route through the
current SessionContext after RPC-024. `set_file_search_results` and
`cursor_position` stay AgentView-owned (they're popup / input cursor
state, not scrollback).

## 6. `AgentViewStore::set_session_index` — RPC-018 setter being removed

```rust
pub fn set_session_index(&mut self, current: usize, total: usize) {
    self.session_index = (current, total);
}
```

Match: `store/agent_view.rs:150`. The RPC-024 store derives the (current,
total) pair from `(current_session_index + 1, open_sessions.len())`, so
this setter is removed and any caller (currently none in the codebase
based on the AST search) goes away with it.

## 7. Action enum `Action::SessionCreated` consumer

`Action::SessionCreated(session) => { ... }` lives in `app/dispatch.rs`
(the existing arm at the case beginning around line 128). The pattern
`Action::SessionCreated($BINDING) => { $$$BODY }` matched nothing under
strict AST equality (rustc-parsed match arms render slightly
differently in the AST), but a substring grep at `app/dispatch.rs:128`
shows it. The RPC-024 work is to:

1. Replace the existing `self.agent_view_store.set_current_session(Some(...))`
   call with `self.agent_view_store.append_session(SessionContext::new(session.clone()))`.
2. Keep the rest of the arm (RPC-018 spawned-task fetches for model
   info / thinking level) untouched.

## 8. Action enum `Action::SessionPrev / SessionNext` — currently swallowed

Confirmed via `components/mod.rs` — both variants exist (added in RPC-019,
documented in their doc comment: "RPC-021 will route through App::dispatch
to cycle to the previous session in the AgentViewStore's session list").
No App::dispatch arm matches them today; the catch-all `_ => {}` swallows
them. RPC-024 adds the two new arms.

## Summary of touch list for the implementation slice

Files modified by RPC-024:

| File | Change |
|---|---|
| `codelet/fspec-tui/src/store/agent_view.rs` | Replace single-slot session field with `Vec<SessionContext>` + `current_session_index`; remove `set_current_session` + `set_session_index`; add `append_session` + `cycle_session` + `current_session_context[_mut]` + `set_input_draft` |
| `codelet/fspec-tui/src/store/agent_view/session_context.rs` | NEW — owns `SessionContext` struct |
| `codelet/fspec-tui/src/views/agent.rs` | Remove `scrollback` + `next_seq` fields; rewrite the 7 `.scrollback` call sites to route through `store.current_session_context[_mut]()`; tolerate empty `open_sessions` (no-op render) |
| `codelet/fspec-tui/src/views/agent/dispatch.rs` | Rewrite the 2 PageUp/PageDown sites |
| `codelet/fspec-tui/src/app/dispatch.rs` | `Action::SessionCreated` calls `append_session`; new arms for `Action::SessionPrev` / `Action::SessionNext`; `Action::ChunkReceived` routes scrollback into the matching SessionContext |
| `codelet/fspec-tui/src/app/dispatch_rpc020.rs` | `handle_slash_command(Clear)` and `push_line(...)` route through the current SessionContext |
| `codelet/fspec-tui/src/app/events.rs` | `cursor_position()` stays AgentView-owned — no change |

NO changes required:
- `codelet/rpc/`, `codelet/rpc-types/`, `codelet/rpc-embedded/`, `codelet/rpc-server/` — RPC-024 is client-side only.
- `codelet/napi/`, `src/tui/**/*.tsx` — TS preservation invariant.
- `codelet/fspec-tui/src/transport/` — no new backend methods.

## Confidence the lift is safe

The pattern `self.agent_view_store.set_current_session($EXPR)` matched
NO call sites — the setter is private, called only from the same
`Action::SessionCreated` arm in `app/dispatch.rs`. Removing it from the
public surface is a non-breaking change to the rest of the crate.
