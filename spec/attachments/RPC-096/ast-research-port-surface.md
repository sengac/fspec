# AST research — RPC-096 port surface

## Existing public symbols to modify

### `codelet/fspec-tui/src/store/agent_view.rs`

| Symbol                                | Location | Action |
|---------------------------------------|----------|--------|
| `pub fn cycle_session(&mut self, delta: isize)` | line 167 | DELETE — no callers after this story |
| `pub fn request_create_session_dialog(&mut self)` | line 241 | KEEP — used by Action::OpenAgentView(None); sets BOTH flags |
| (new) `pub enum NavTarget`            | n/a      | ADD — `Session(usize) \| CreateDialog \| Board` |
| (new) `pub fn navigate_next(&self) -> NavTarget` | n/a | ADD |
| (new) `pub fn navigate_prev(&self) -> NavTarget` | n/a | ADD |
| (new) `pub fn request_create_session_dialog_no_auto(&mut self)` | n/a | ADD — sets only `show_create_session_dialog = true` (not `should_auto_create_session`) |

### `codelet/fspec-tui/src/app/dispatch_rpc024.rs`

| Symbol                                | Location | Action |
|---------------------------------------|----------|--------|
| `pub(crate) fn handle_session_cycle(&mut self, delta: isize)` | line 25 | REPLACE — match on `navigate_next`/`navigate_prev` return |

### `codelet/fspec-tui/src/app/dispatch.rs`

| Symbol                                | Location | Action |
|---------------------------------------|----------|--------|
| `Action::BackToBoard => { self.navigator.active_view = ViewMode::Board; }` | line 111 | REUSE for NavTarget::Board path |
| `Action::SessionPrev => self.handle_session_cycle(-1)` | line 225 | REUSE — handler internally chooses target |
| `Action::SessionNext => self.handle_session_cycle(1)` | line 228 | REUSE |

## Existing infra already in place

- `codelet/fspec-tui/src/components/create_session_dialog.rs` — full
  `CreateSessionDialog` Component (line 153) with `CREATE_SESSION_DIALOG_ID`
  (line 23). The compositor already renders it when
  `agent_view_store.show_create_session_dialog() == true`.
- `Action::BackToBoard` already plumbed at `app/dispatch.rs:111-113`.
- `show_create_session_dialog` boolean flag already plumbed on the store
  (`agent_view.rs:83`, getter at 233).
- `should_auto_create_session` boolean flag exists (`agent_view.rs:237`) —
  distinguishes "user explicitly requested dialog" from "off-end navigation"
  so our new method must NOT set it.

## Existing test inventory

- `codelet/fspec-tui/tests/rpc024_multi_session_store.rs` — store-level
  cycle_session tests. Some scenarios pin wrap-around behaviour and must be
  REMOVED or REPLACED with end-of-list semantics tests after this story.
- `codelet/fspec-tui/tests/rpc024_app_dispatch.rs` — App::dispatch tests for
  SessionPrev/SessionNext wrap-around. Same fate.

## RPC-024 source-shape invariants to preserve

- `agent_view.rs` < 300 LoC. Currently larger; navigate_next/navigate_prev
  add ~25 LoC. May need extraction.
- `pub fn session_index` MUST stay in `agent_view.rs` (pinned by
  `rpc024-source-shape.feature` scenario "AgentViewStore exposes the
  multi-session surface").

## Migration plan

1. Add NavTarget enum + navigate_next + navigate_prev + new no-auto setter.
2. Replace handle_session_cycle body with target match.
3. Delete cycle_session (and the RPC-024 wrap-around scenarios that pinned
   it; replace with end-of-list scenarios in this story).
4. Adjust any TS source-shape regression tests that grep for
   `pub fn cycle_session`.
