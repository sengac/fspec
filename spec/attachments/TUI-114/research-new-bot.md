# Research: New Bot (TUI-114)

## Request

"I want a 'new bot' feature, similar to the 'new agent' feature … it is a bit like how
Claude Code works — where it's a bot that is an orchestrator for other agents." A **bot** is an
orchestrator assistant: one session that delegates work to sub-agents it spawns and
coordinates their results, rather than doing the work itself.

## How "New Agent" works today

- **Entry points:** they all resolve to `MenuAction::NewAgent` → `Action::OpenCreateSessionDialog { preselect: None }`
  (`rust/fspec-tui/src/components/menu_bar/items.rs:87`).
  - **Board menu bar Zone C:** a single right-aligned `[ New Agent ]` button (`BOARD_ZONE_C`, `rust/fspec-tui/src/views/board/menu_snapshot.rs:30`).
  - **Agent bar Zone C:** `[ New Agent ]` (`rust/fspec-tui/src/views/agent/menu_render.rs:52`, `dispatch_menu_zone_c.rs:59`).
  - **Kanban menu row:** key `.` (`KANBAN` entries in `items.rs:150`); the same registry rows feed the `u` menu-bar help dialog
    (`menu_bar_help.rs`) and the mux top bar.
- **Dialog:** `CreateSessionDialog` (`rust/fspec-tui/src/components/create_session_dialog.rs`, 272 lines) offers Yes /
  Yes - Isolated / Cancel and emits `CreateSessionSubmitted { isolated }` or `CreateSessionCancelled`.
- **Dispatch:** `dispatch_create_session_dialog.rs` calls `backend.create_session(None)` or
  `backend.create_isolated_session(None)` and folds the reply into `Action::SessionCreated`. The board flow
  (`board-new-agent-gesture-prompts-create-session-dialog.feature`) then flips to the Agent view on the new
  session, without binding it to a work unit.
- **Both creation calls already take `role: Option<String>`.** `FspecBackend::create_session(role)`
  (`transport/backend/core.rs:47`), `create_isolated_session(role)` (`interaction.rs:89`), and
  `SessionManagerHandle::create_session(role)` / `create_session_checked(role)` (codelet-core) all accept it.
  `create_session_in_project` applies a non-empty role via `BackgroundSession::set_role`
  (`rust/sessions/src/handle_creation.rs:29`).

## What a "role" already is

- `BackgroundSession::{set_role, get_role, clear_role}` (`rust/sessions/src/background_session/messaging.rs`)
  stores a **system-prompt overlay**. Every provider path prepends it as `role_preamble` to the request
  (`agent-loop/src/dispatch.rs:66`, `agent_loop/provider_openai.rs:20`, `provider_custom.rs:17`).
- `SessionInfo.role: Option<String>` (`rust/rpc-types/src/sessions.rs:136`) carries it to clients, so a client can
  tell how a session was created.
- AgentManager's `set_role` action also sets this overlay (`agent-loop/src/agent_manager_handler.rs:419`).

## Orchestration building blocks that already exist

- **AgentManager tool** (`rust/tools/src/agent_manager/mod.rs`) has these actions: `spawn` (subordinate session with an optional
  role, inheriting the spawner's model), `list`, `get_status`, `close` (spawner only), `message`, `set_role` and
  `await_idle` (block until sessions go idle).
- **Supervisor/subordinate links:** `RPC-061` exposes these, and the `agentview-spawned-subordinate-session-registration`
  and `session-role-and-authority-model` features cover them. Spawned agents show up as the spawner's subordinates.

These are the same primitives Claude Code's orchestrating main agent uses: spawn sub-agents, message them, wait,
collect results. A bot is therefore **a session whose role overlay instructs it to orchestrate with AgentManager.**
It needs no new engine machinery.

## Design

1. **One source for the orchestrator role.** A `codelet_sessions::bot` module defines the orchestrator role
   overlay text and `is_bot_role(&str)`. The overlay starts with a stable marker line, so bots stay recognisable
   from `SessionInfo.role` after restore. The overlay tells the session to:
   - break the request into work for sub-agents;
   - spawn them with AgentManager (one per card or task);
   - message them, and wait with `await_idle`;
   - check their results, and report back;
   - avoid doing implementation work itself.
2. **Entry points that mirror New Agent:**
   - `MenuAction::NewBot` → `Action::OpenCreateSessionDialog { preselect: None, bot: true }`;
   - a board Zone C `[ New Bot ]` button beside `[ New Agent ]`;
   - an agent bar Zone C `[ New Bot ]`;
   - a Kanban registry row on key `b`, which is unused on the board today. The board uses `. / D A F C M P ?` plus
     the view keys `a c d f h j k l m p s u w`.
   The `u` help dialog and the mux top bar pick up the new row through the shared registry.
3. **Dialog:** the same `CreateSessionDialog` with a bot flag. It shows the title "New Bot" and offers the same
   Yes / Yes - Isolated / Cancel options. `CreateSessionSubmitted { isolated, bot }` → `create_session(Some(role))`
   or `create_isolated_session(Some(role))`, where `role` is the orchestrator overlay.
4. **Visibility:** the agent bar chip of a session whose `SessionInfo.role` is a bot role carries a bot marker,
   so bots and agents can be told apart.
5. **No engine changes.** The role path, AgentManager and subordinate links are reused as they are.

## Files to touch (estimate)

- `rust/sessions/src/bot.rs` (new, small) and `lib.rs`.
- fspec-tui:
  - `components/menu_bar/items.rs`: `NewBot` action and the Kanban row;
  - `views/board/menu_snapshot.rs`: `BOARD_ZONE_C`;
  - `views/agent/menu_render.rs`: the agent bar Zone C;
  - `app/dispatch_menu_zone_c.rs`;
  - `components/action_session_controls.rs`: the `bot` flag on the two dialog actions;
  - `components/create_session_dialog.rs`: the title;
  - `app/dispatch_create_session_dialog.rs`: pass the role;
  - chip rendering in `views/board/menu_snapshot.rs` and the agent bar.
- Several of these files are near 300 lines, so splits may be needed as the work lands.

## Open points (defaulted, and changeable in review)

- **Key `b`** for New Bot. It is free on the board today.
- **The bot marker** on chips is the prefix `bot:` before the session name, kept ASCII so it works in every terminal.
- **Bots start with the default model,** as New Agent does. They are not bound to a work unit at creation, and the
  user gives the bot its goal in the first prompt.
