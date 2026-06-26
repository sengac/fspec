# TUI-093 — AST research (restore-parity touch points)

AST/structural analysis of the functions and types the TS-parity fix must
modify. All evidence gathered via AstGrep + Grep on the live tree.

## Application / dispatch choke points (codelet/fspec-tui)

- `pub(crate) fn refresh_session_chrome(&mut self, session: SessionId)`
  — `src/app/dispatch_session_chrome.rs:24`.
  Central chrome-refresh; already spawns `get_model_info` + `get_thinking_level`
  + `get_session_role`. Called from BOTH:
    * `handle_session_created` (`src/app/dispatch_create_session_dialog.rs:137`) — SessionCreated path
    * resume path (`src/app/dispatch_resume_search_views.rs:105`)
  → single insertion point that covers new + resumed + activated sessions.

- `pub(crate) fn handle_set_thinking_level_default(&mut self, …)`
  — `src/app/dispatch_model_thinking_dialogs.rs:196`.
  Currently spawns only `backend.set_thinking_level_default(...)`; NO repaint.
  Sibling `handle_thinking_level_selected` (`:169-187`) is the repaint template:
  spawns `set_thinking_level` then `get_thinking_level` → `Action::ThinkingLevelLoaded`.

- `Action::ThinkingLevelLoaded(session_id, level)` dispatch arm
  — `src/app/dispatch.rs:193-196` → `agent_view_store.set_thinking_level(...)`.
  Badge data source = `AgentViewStore`.

## State + store

- `pub struct App { … }` — `src/app/state.rs:33`. Constructor
  `with_action_bus` (`:91-117`). Need a new field
  `applied_default_thinking: HashSet<SessionId>` (Rust analogue of TS
  `appliedToSessionRef`) initialised empty.
- `active_session_rx_snapshot(&self) -> Option<SessionId>` — `:178` — read active
  session at bootstrap time.
- `AgentViewStore`: `thinking_level_by_session: HashMap<SessionId, ThinkingLevel>`
  (`src/store/agent_view.rs:57`); `thinking_level_for` /
  `set_thinking_level` (`src/store/agent_view/chrome_state.rs:43-49`).

## Bootstrap (model parallel)

- `initialize_startup_model` — `src/app/bootstrap.rs:67-85`, invoked at
  `bootstrap.rs:53`. Add `initialize_default_thinking_level` alongside,
  applying the loaded default to the active-session snapshot via the shared
  guarded helper.

## Backend / sessions (already present; storage unchanged)

- `load_default_thinking_level()` / `load_default_thinking_level_with_dirs`
  — `codelet/sessions/src/default_thinking_level_persistence.rs:120 / :95`.
  Collapses absent-key → `Off`, LOSING the TS `null` vs `Off(0)` distinction.
  Parity needs an `Option`-returning variant (`None` = no/invalid key) so the
  guarded apply does NOT clobber when no default was ever set.
- `set_thinking_level_default` handle impl — `codelet/sessions/src/handle_impl.rs:844-866`
  persists ALWAYS + applies in-memory base level when session exists.
- `get_thinking_level` — `handle_impl.rs:919-929` reads base level.
- Construction-time restore sites — `session_manager.rs:574-580` (create_session),
  `:854-864` (create_isolated_session).

## Conclusion

Three surgical edits + one new Option-returning load fn:
1. `handle_set_thinking_level_default`: add get_thinking_level → ThinkingLevelLoaded repaint.
2. `refresh_session_chrome`: guarded first-activation apply of the persisted default.
3. `bootstrap.rs`: `initialize_default_thinking_level` for the active snapshot.
4. `default_thinking_level_persistence.rs`: `load_default_thinking_level_opt[_with_dirs]`.
Plus `App.applied_default_thinking: HashSet<SessionId>` guard field.
