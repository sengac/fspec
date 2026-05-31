# RPC-063 — AST research: existing wiring points for the RoleDialog

This document captures the AST search results performed during Example Mapping for RPC-063. All call sites listed below were verified live before scenario generation.

## 1. Slash parser — current `/role` parsing path

```
codelet/fspec-tui/src/app/slash_parser.rs:70:1:pub fn parse_slash_command(text: &str) -> SlashCommandParse
```

Current relevant arms (lines 97–106):

```rust
if trimmed == "/role" {
    return SlashCommandParse::ClearRole;          // ← changes to OpenRoleDialog
}
if let Some(rest) = trimmed.strip_prefix("/role ") {
    let arg = rest.trim();
    if arg.is_empty() || arg.eq_ignore_ascii_case("clear") {
        return SlashCommandParse::ClearRole;      // ← empty arg now becomes OpenRoleDialog,
                                                  //   "clear" stays ClearRole
    }
    return SlashCommandParse::SetRole(arg.to_string());
}
```

`SlashCommandParse` enum lives at lines 22–53 of the same file. New variant `OpenRoleDialog` lands beside `OpenModelDialog` / `OpenThinkingDialog`.

## 2. Palette `/role` arm — must call `handle_open_role_dialog`

```
codelet/fspec-tui/src/app/dispatch_rpc020.rs:62  SlashCommandAction::Role => { … handle_set_session_role(sid, None) … }
```

The branch currently clears the role on bare-palette pick. RPC-063 replaces it with `handle_open_role_dialog(sid)`.

## 3. Existing role accessors (no changes required — RPC-063 reuses them)

```
codelet/fspec-tui/src/store/agent_view/role_state.rs:23:5:pub fn role_for(&self, session: &SessionId) -> Option<&str>
codelet/fspec-tui/src/store/agent_view/role_state.rs:30:5:pub fn set_role(&mut self, session: SessionId, role: Option<String>)
```

The dialog seeds its draft from `AgentViewStore::role_for(current_session)`. No additional backend round-trip.

## 4. Existing backend wiring (no changes required — RPC-063 reuses them)

```
codelet/fspec-tui/src/transport/mod.rs:214:5:async fn get_session_role(&self, session_id: SessionId) -> Result<Option<String>>
codelet/fspec-tui/src/transport/mod.rs:218:5:async fn set_session_role(&self, session_id: SessionId, role: Option<String>) -> Result<()>
```

`handle_set_session_role` (codelet/fspec-tui/src/app/dispatch_rpc022.rs:162) already does the AgentViewStore mutation + spawn `backend.set_session_role`. RPC-063 wires the dialog's Enter/Ctrl+D outcomes through this existing helper via `Action::SetSessionRole`.

## 5. Dialog component prior-art (the template we follow)

```
codelet/fspec-tui/src/components/thinking_level_dialog.rs:23:1:pub const THINKING_LEVEL_DIALOG_ID: &str = "thinking-level-dialog";
codelet/fspec-tui/src/components/thinking_level_dialog.rs:101:1:impl Component for ThinkingLevelDialog
```

`RoleDialog` follows the same pattern:

- `pub const ROLE_DIALOG_ID: &str = "role-dialog";`
- `impl Component for RoleDialog` with `priority()` returning `Priority::Foreground`
- `take_pending_action()` test accessor
- Calls `render_dialog(rect, buf, &dialog)` from `components::dialog_theme`
- `Accent::Cyan` (matching the cyan accent used for the role banner)

## 6. Existing tests that need updating

```
codelet/fspec-tui/tests/slash_command_wiring_rpc022.rs:63                  assert_eq!(parse_slash_command("/role"), SlashCommandParse::ClearRole);
codelet/fspec-tui/tests/slash_command_wiring_rpc022.rs:275                 fn submitting_bare_slash_role_is_treated_as_a_clear() { … }
codelet/fspec-tui/tests/slash_command_wiring_rpc022.rs:378                 fn slash_popup_selection_of_role_is_treated_as_a_clear_with_no_notice() { … }
codelet/fspec-tui/src/app/slash_parser.rs:135                              assert_eq!(parse_slash_command("/role"), SlashCommandParse::ClearRole);
```

Per architecture note [I] these tests change to assert "opens the dialog" instead of "clears the role". The `/role clear` paths remain.

## 7. App::dispatch routing — current 299 LoC ceiling

```
codelet/fspec-tui/src/app/dispatch.rs       299 lines
codelet/fspec-tui/src/app/dispatch_rpc020.rs 299 lines
```

Both files are at the 300-LoC ceiling already. RPC-063 lands the new `handle_open_role_dialog` helper in a new sibling file `codelet/fspec-tui/src/app/dispatch_rpc063.rs` mirroring the `dispatch_rpc061.rs` precedent.

## 8. Component implementations of `priority() -> Priority::Foreground`

```
codelet/fspec-tui/src/components/model_selector_dialog.rs:159  impl Component for ModelSelectorDialog
codelet/fspec-tui/src/components/thinking_level_dialog.rs:101  impl Component for ThinkingLevelDialog
codelet/fspec-tui/src/components/create_session_dialog.rs:153  impl Component for CreateSessionDialog
```

All three return `Priority::Foreground` — same priority `RoleDialog` will use.

## 9. Existing constants the RoleDialog footer reuses

```
codelet/fspec-tui/src/components/dialog_theme.rs:68:1:pub const FOOTER_SEPARATOR: &str = " │ ";
```

Footer string: `format!("Enter Save{sep}Ctrl+D Clear{sep}Esc Cancel", sep = FOOTER_SEPARATOR)`.

## 10. Open work in `Action::OpenRoleDialog` necessity check

There is no `Action::OpenRoleDialog` variant required. Both call sites (palette + submit-line) invoke `self.handle_open_role_dialog()` synchronously inside `App::dispatch`, mirroring the way `OpenModelDialog`/`OpenThinkingDialog` route through their helpers without going onto the action bus.
