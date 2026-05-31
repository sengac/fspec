# AST Research — RPC-060 Dialog Patterns

## 1. Existing `Component` impls in `codelet/fspec-tui/src/components/`

AstGrep search: `impl Component for $NAME { $$$BODY }`

| File | Line | Component |
|---|---|---|
| `help_dialog.rs` | 54 | `HelpDialog` (Critical, no Action, Esc only) |
| `pause_dialog.rs` | 201 | `PauseDialog` (Critical, emits PauseConfirmed/PauseTriple/PauseResumed) |
| `hitl_dialog.rs` | 244 | `HitlDialog` (Critical, emits HitlSubmitted) |
| `hello.rs` | 42 | `HelloComponent` (Background) |
| `model_selector_dialog.rs` | 159 | `ModelSelectorDialog` (Foreground, emits ModelSelected) |
| `thinking_level_dialog.rs` | 101 | `ThinkingLevelDialog` (Foreground, emits ThinkingLevelSelected / SetThinkingLevelDefault) |
| `disconnect_dialog.rs` | 76 | `DisconnectDialog` (Critical) |

**Best template for CreateSessionDialog**: `thinking_level_dialog.rs` — a Priority::Foreground dialog with row-based selection, Enter-to-confirm, Esc-to-cancel. Pattern is:

- `pub const <NAME>_DIALOG_ID: &str = "<name>-dialog";` constant id.
- `struct Dialog { id, ..state.., action_tx: Option<UnboundedSender<Action>>, pending_action: Option<Action> }`.
- `fn new(...)` plus `with_action_tx(mut self, tx)` builder.
- `take_pending_action()` test accessor.
- `Component::handle_event` matches on `KeyCode::{Esc, Left, Right, Enter}`.
- Uses `super::dialog_theme::{render_dialog, Accent, FspecDialog}` + `super::dialog_theme_rows::label_description_row`.

## 2. `try_dispatch_rpc0XX` extension points in `App::dispatch`

Existing helpers (all `pub(crate) fn try_dispatch_rpcNNN(&mut self, action: &Action) -> bool`):

```
src/app/dispatch_rpc053.rs:256  try_dispatch_rpc053
src/app/dispatch_rpc054.rs:240  try_dispatch_rpc054
src/app/dispatch_rpc055.rs:65   try_dispatch_rpc055
src/app/dispatch_rpc056.rs:87   try_dispatch_rpc056
src/app/dispatch_rpc057.rs:174  try_dispatch_rpc057
src/app/dispatch_rpc058.rs:194  try_dispatch_rpc058
src/app/dispatch_rpc059.rs:125  try_dispatch_rpc059
```

`app/dispatch.rs:284-292` fallback chain ends with `|| self.try_dispatch_rpc059(&action);` — RPC-060 will append `|| self.try_dispatch_rpc060(&action)`.

## 3. `create_isolated_session` already wired through the stack

ripgrep `create_isolated_session`:

```
codelet/core/src/session_manager_handle.rs           # trait method (RPC-037)
codelet/sessions/src/handle_impl.rs                  # SessionManager impl (RPC-042)
codelet/sessions/src/session_manager.rs              # create_isolated_session_with_id
codelet/rpc-types/src/lib.rs                         # IsolatedSessionInfo type (RPC-036)
codelet/rpc/src/lib.rs                               # FspecService::create_isolated_session
codelet/fspec-tui/src/transport/mod.rs:462           # FspecBackend trait method (default impl)
codelet/fspec-tui/src/transport/embedded.rs:508      # EmbeddedFspecBackend forwarder
codelet/fspec-tui/src/transport/websocket.rs:848     # WebSocketFspecBackend forwarder
codelet/git/tests/isolated_session_tests.rs          # codelet-git create_worktree test
codelet/fspec-tui/tests/rpc037_cross_transport_parity.rs  # cross-transport parity test
```

**Conclusion**: zero new RPC/trait plumbing needed for RPC-060. We only need UI + dispatch wiring.

## 4. `SlashCommandAction::Isolation` currently has no handler

`grep "SlashCommandAction::Isolation" codelet/fspec-tui/src`:

- Defined in `views/agent/slash_commands.rs:30` and `name() => "isolation"` at line 56.
- Listed in the `SLASH_COMMANDS` registry at line 138 with description "Toggle worktree isolation".
- **Not matched** in `dispatch_rpc020.rs::handle_slash_command` — falls through to the catch-all `[notice] /isolation not yet implemented in Rust TUI` arm at line 139-145.

RPC-060 replaces this catch-all fallthrough with a real handler that dispatches `Action::OpenCreateSessionDialog { preselect: Some(Isolated) }`.

## 5. TS reference — `src/components/CreateSessionDialog.tsx`

- TUI-090: three flat options Yes / Yes - Isolated / Cancel.
- TUI-067: context-aware title — `Work on ${workUnit.id}?` vs `Start New Agent?`.
- Cyan accent (`borderColor="cyan"`).
- Left/Right arrows cycle with wrap-around. Enter confirms. Esc cancels.
- Confirm callback `onConfirm(isolated: boolean)`; cancel `onCancel()`.

Rust port mirrors this exactly. The `workUnit` lookup is replaced with `AgentViewStore::work_unit_context_for(current_session)` at the App::dispatch dialog-open path so the dialog itself stays UI-only.

## 6. `MockBackend` scripting pattern for new RPC methods

Existing pattern from `tests/common/mod.rs` for `loop_add` (lines 540-547, 718-723, 1667-1684, 2582-2611):

1. Field on struct: `loop_add_result: Mutex<Result<RegisteredLoop, anyhow::Error>>` + `loop_add_calls: AtomicUsize`.
2. `Default::default()` seeds: `Mutex::new(Ok(RegisteredLoop::default()))`, `AtomicUsize::new(0)`.
3. Test seeders: `pub fn seed_loop_add_result(&self, r: Result<RegisteredLoop, anyhow::Error>)` + `pub fn loop_add_calls(&self) -> usize`.
4. `impl FspecBackend`: increments counter then forwards mutex-stored result.

RPC-060 mirrors this exactly for `create_isolated_session`.
