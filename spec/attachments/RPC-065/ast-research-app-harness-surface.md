# AST research — App surface available to `AppTestHarness` (RPC-065)

This document captures the AST-level interrogation of the `App` struct,
its `pub` accessors, and the dispatch entry points the harness will
drive. The goal is to confirm that every observable transition the
parity matrix asserts is reachable from `pub` methods on `App` (no need
to add new test seams).

## 1. `App` struct fields and constructor (codelet/fspec-tui/src/app/state.rs)

```rust
pub struct App {
    pub(crate) compositor: Compositor,
    pub(crate) action_tx: UnboundedSender<Action>,
    pub(crate) action_rx: UnboundedReceiver<Action>,
    pub(crate) backend: Arc<dyn FspecBackend>,
    pub(crate) theme: Arc<Theme>,
    pub(crate) navigator: Navigator,
    pub(crate) board_store: BoardStore,
    pub(crate) agent_view_store: AgentViewStore,
    pub(crate) should_quit: bool,
    pub(crate) should_render: bool,
    pub(crate) subscriber_tasks: Vec<JoinHandle<()>>,
    pub(crate) pending_tasks: Vec<JoinHandle<()>>,
    pub(crate) active_session_tx: watch::Sender<Option<SessionId>>,
    pub(crate) active_session_rx: watch::Receiver<Option<SessionId>>,
    pub(crate) pending_input_save_handle: Option<JoinHandle<()>>,
    pub(crate) search_history_debounce_handle: Option<tokio::task::AbortHandle>,
}

impl App {
    pub fn new(backend: Arc<dyn FspecBackend>) -> Self { … }
    pub fn with_action_bus(backend, action_tx, action_rx) -> Self { … }
}
```

## 2. Public observables (state.rs lines 119-260)

| Method                                | Returns / Effect                                  | Used by parity test |
|---------------------------------------|---------------------------------------------------|---------------------|
| `compositor() -> &Compositor`         | Iterate / `.contains(id)`                         | dialog-push asserts |
| `compositor_mut() -> &mut Compositor` | Seed a HelpDialog for the Esc cascade case        | Esc cascade test    |
| `backend() -> &Arc<dyn FspecBackend>` | Not used directly; tests hold `Arc<MockBackend>`  | — |
| `should_quit() -> bool`               | `/quit` assertion                                 | `/quit`             |
| `try_recv_action() -> Option<Action>` | Drain pending actions                             | `drain_pending`     |
| `next_pending_task() -> Option<JH>`   | Pop a spawned task to await                       | `drain_pending`     |
| `board_store()/board_store_mut()`     | Board state — not in scope                        | — |
| `agent_view_store()`                  | `current_session()`, `current_session_context()`  | every test          |
| `agent_view_store_mut()`              | Mutate per-session ctx (seed_chunks, role, etc.)  | seed helpers        |
| `navigator() -> &Navigator`           | `.active_view`, `.agent.search_view`, `.agent.resume_view`, `.agent.input`, `.provider_settings`, `.blocklist` | view-mode asserts |
| `navigator_mut()`                     | Mutate input value, push synthetic events         | submit_input helper |
| `active_view() -> ViewMode`           | One-shot active-view check                        | provider/blocklist  |
| `current_session() -> Option<SessionId>` | Legacy convenience                              | `/clear`-style       |
| `action_tx_clone() -> Sender<Action>` | Push synthetic actions onto the bus               | dispatch_slash       |

**Conclusion:** every parity assertion lives behind these pub methods.
No production-code changes required.

## 3. Dispatch entry points

```rust
// codelet/fspec-tui/src/app/dispatch.rs::App::dispatch
pub fn dispatch(&mut self, action: Action) { … }

// codelet/fspec-tui/src/app/dispatch.rs::App::handle_event
pub fn handle_event(&mut self, event: &Event) -> EventResult { … }
```

Both are public so the harness can:
- Drive a slash command via `app.dispatch(Action::SlashCommandSelected(action))`.
- Drive a keyboard shortcut via `app.handle_event(&Event::Key(KeyEvent::new(...)))`.

## 4. Slash command routing (dispatch_rpc020.rs::handle_slash_command)

| Variant         | Effect                                                                |
|-----------------|-----------------------------------------------------------------------|
| Help            | `compositor.push(HelpDialog::new())` synchronously                    |
| Clear           | `handle_slash_clear()` (sync wipe + spawned `backend.clear_history`)  |
| Quit            | `self.should_quit = true` synchronously                               |
| Resume          | `handle_open_resume_view()` synchronously (sets `agent.resume_view`)  |
| Search          | `handle_open_search_view()` synchronously (sets `agent.search_view`)  |
| Model           | `handle_open_model_dialog()` → compositor push + spawn list_providers |
| Thinking        | `handle_open_thinking_dialog()` → compositor push synchronously        |
| Role            | `handle_open_role_dialog()` → compositor push synchronously            |
| Compact         | spawn `backend.compact_session`                                       |
| Detach          | `handle_slash_detach()` → spawn `backend.set_work_unit_context(None)` |
| Provider / Providers | `action_tx.send(OpenProviderSettingsView)` (async via bus)        |
| Debug           | `handle_slash_debug()` → spawn `backend.toggle_debug`                 |
| Blocklist       | `action_tx.send(OpenBlocklistView)`                                   |
| MergeWorktree   | `handle_slash_merge_worktree()` → spawn `backend.inspect_session_changes` |
| Schedule        | `handle_slash_schedule_help()` synchronously pushes a notice line     |
| Loop            | `handle_slash_loop_help()` synchronously pushes a notice line         |
| Isolation       | `action_tx.send(OpenCreateSessionDialog{ preselect: Isolated })`      |

**Implication for the harness:** sync paths (Help, Quit, Resume, Search,
Thinking, Role, Schedule, Loop) are observable immediately after
`app.dispatch(...)`. Async paths (Clear, Compact, Detach, Debug, Provider,
Blocklist, MergeWorktree, Isolation, Model) need `drain_pending()` first.

## 5. Slash input parser (slash_parser.rs::parse_slash_command)

Used by `handle_input_submitted` to intercept `/thinking <level>`,
`/role <text>`, `/schedule <sub>`, `/loop <sub>` BEFORE forwarding to
`backend.send_input`. The harness `submit_input("/thinking high")`
helper drives `Action::InputSubmitted("/thinking high")` and the
existing parser then dispatches the right backend call.

## 6. Keyboard routing (views/agent/dispatch.rs)

| Key            | Action emitted                          | Test path             |
|----------------|-----------------------------------------|-----------------------|
| Esc            | `Action::AgentEscPressed`               | Esc cascade smoke     |
| Ctrl+C         | `Action::Interrupt`                     | Ctrl+C interrupt      |
| PageUp         | `Action::ScrollbackPageUp`              | PageUp smoke          |
| PageDown / End | `Action::ScrollbackPageDown`            | PageDown / End smoke  |
| Shift+Arrows   | `cycle_session` / history actions       | Shift arrow tests     |
| Tab            | **no handler** — falls through to input | placeholder #[ignore] |
| (typed text)   | `Action::PendingInputChanged(after)`    | (not in scope)        |
| Enter on input | `Action::InputSubmitted(value)`         | plain-text submit     |
| Ctrl+R         | (routed in App::handle_event for chord) | search-view smoke     |

## 7. MockBackend counter surface (tests/common/mod.rs)

Every matrix assertion has a corresponding counter:

- `clear_history_calls()` / `last_clear_history_session()`
- `compact_session_calls()` / `last_compact_session()`
- `toggle_debug_calls()` / `last_toggle_debug()`
- `set_thinking_level_calls()` / `last_set_thinking_level()`
- `set_session_role_calls()` / `last_set_session_role()`
- `set_work_unit_context_calls()` / `last_set_work_unit_context()`
- `inspect_session_changes_calls()`
- `schedule_list_calls()` / `loop_list_calls()`
- `send_input_calls()` / `last_send_input()`
- `interrupt_calls()` / `last_interrupt()`
- `search_history_calls()` (history search)
- `script_history(&SessionId, Vec<String>)` (Shift+↑ seed)

**Implication for the harness:** no MockBackend extensions needed for
RPC-065 — every matrix observable is already exported.

## 8. Compositor `contains` API

`compositor.contains(id: &str) -> bool` exists and is `pub`. Dialog ids:

- HelpDialog → `"help-dialog"`
- ModelSelectorDialog → `MODEL_SELECTOR_DIALOG_ID`
- ThinkingLevelDialog → `THINKING_LEVEL_DIALOG_ID`
- RoleDialog → `ROLE_DIALOG_ID`
- CreateSessionDialog → `CREATE_SESSION_DIALOG_ID`
- MergeConfirmDialog → `MERGE_CONFIRM_DIALOG_ID`

All are re-exported from `codelet_fspec_tui::*` (verified in
`src/lib.rs` lines 47-55).

## 9. Findings summary

1. ✅ Every parity-matrix observable is reachable from `pub` App
   methods. No production-code changes needed.
2. ✅ MockBackend already exposes every counter the matrix needs.
3. ⚠️ Tab → turn-selection has no Rust handler today. The test will be
   `#[ignore = "..."]` per the answered question.
4. ⚠️ /merge-worktree's observable is `inspect_session_changes_calls()`,
   NOT compositor-push (the dialog push is gated on a non-zero summary).
5. ⚠️ /isolation routes through `action_tx → navigator.apply_action`,
   so the harness must drain pending actions before asserting
   `compositor.contains(CREATE_SESSION_DIALOG_ID)`.
6. ⚠️ /provider and /providers both push via `action_tx`, so the
   harness must drain pending actions before asserting
   `navigator.active_view == ViewMode::ProviderSettings`.

Conclusion: the AppTestHarness can be built ENTIRELY against the
existing public App surface, and a single new test file
`tests/behaviour_parity_rpc065.rs` plus a single new harness module
`tests/common/harness.rs` cover the whole matrix.
