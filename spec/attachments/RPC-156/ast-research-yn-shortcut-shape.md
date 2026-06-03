# RPC-156 AST Research — n/N cancel shortcut binding in confirm_dialog.rs

## Goal

Verify the n/N cancel-shortcut binding is present in
`codelet/fspec-tui/src/views/agent/confirm_dialog.rs` so the regression-shape
tests can pin the exact source-string assertions.

## Source location

`codelet/fspec-tui/src/views/agent/confirm_dialog.rs` — `ConfirmDialog::handle_key`
method, lines 139–168.

```rust
pub fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> ConfirmDialogOutcome {
    if mods.contains(KeyModifiers::CONTROL) || mods.contains(KeyModifiers::ALT) {
        return ConfirmDialogOutcome::Ignored;
    }
    match code {
        KeyCode::Esc => ConfirmDialogOutcome::Cancel,
        KeyCode::Left => { self.focus_prev(); ConfirmDialogOutcome::Continued }
        KeyCode::Right => { self.focus_next(); ConfirmDialogOutcome::Continued }
        KeyCode::Tab => { self.focus_next(); ConfirmDialogOutcome::Continued }
        KeyCode::Enter => self.outcome_for_index(self.focused),
        // RPC-164: TS parity — 'y'/'Y' confirms (Primary), 'n'/'N' cancels.
        // Mirrors src/tui/inputHandlers/deleteConfirmModeHandler.ts.
        // Focus state is intentionally NOT consulted: the shortcut is a
        // direct outcome dispatch, regardless of which button is focused.
        KeyCode::Char('y') | KeyCode::Char('Y') => self.outcome_for_index(0),
        KeyCode::Char('n') | KeyCode::Char('N') => {
            self.outcome_for_index(self.cancel_index())
        }
        _ => ConfirmDialogOutcome::Ignored,
    }
}
```

## Substrings pinned by RPC-156 regression-shape tests

| # | Substring | Rule |
|---|-----------|------|
| 1 | `KeyCode::Char('n') \| KeyCode::Char('N')` | Rule [0] — n/N cancel-shortcut keybind present |
| 2 | `KeyCode::Char('y') \| KeyCode::Char('Y')` | Rule [1] — y/Y primary-shortcut keybind present (sibling parity) |
| 3 | `outcome_for_index(self.cancel_index())` | Rule [2] — n/N is wired to cancel-index path |
| 4 | `mods.contains(KeyModifiers::CONTROL)` | Rule [3] — Ctrl modifier guard remains |
| 5 | `mods.contains(KeyModifiers::ALT)` | Rule [3] — Alt modifier guard remains |
| 6 | `ConfirmDialogOutcome::Ignored` | Rule [3] — modifier guard returns Ignored |

## Relation to existing coverage

- **RPC-164** (done, 13 scenarios): full integration coverage in
  `codelet/fspec-tui/tests/confirm_dialog_yn_shortcut_rpc164.rs` exercising
  real `ConfirmDialog::handle_key` calls + ProviderSettingsView delete-credentials
  dispatch path. Slow due to full ratatui compile cost.
- **RPC-156** (this card): fast structural source-string scanning to prevent
  silent regression of the keybind without paying the ratatui compile cost.

Pattern mirrors RPC-077 (handle_impl redundant-clone shape) and RPC-149
(provider settings list mode keybinds shape).

## AST queries used

```
ast-grep --lang rust --pattern 'KeyCode::Char($_) | KeyCode::Char($_)' \
    codelet/fspec-tui/src/views/agent/confirm_dialog.rs
```

Confirmed presence of the y/Y and n/N arms at lines 162 and 163.

## Test file plan

`codelet/fspec-tui/tests/rpc156_delete_confirm_yn_shortcut_shape.rs` — sub-ms
execution, no key event simulation, just source-string scanning via
`include_str!()` of confirm_dialog.rs.
