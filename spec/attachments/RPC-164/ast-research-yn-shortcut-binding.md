# RPC-164 — AST Research: y/Y & n/N shortcut binding for ConfirmDialog

## Target site

**File:** `codelet/fspec-tui/src/views/agent/confirm_dialog.rs`
**Function:** `ConfirmDialog::handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> ConfirmDialogOutcome`
**Lines:** 139–160

### Current shape (verified via AstGrep, rust language)

```
pattern: 'pub fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> ConfirmDialogOutcome { $$$BODY }'
match:   codelet/fspec-tui/src/views/agent/confirm_dialog.rs:139:5
```

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
        _ => ConfirmDialogOutcome::Ignored,
    }
}
```

## Caller sites for ConfirmDialog

**AstGrep pattern:** `ConfirmDialog::new($$$ARGS)` — returns two production callers:

1. `codelet/fspec-tui/src/views/provider_settings/list.rs:87` —
   `view.delete_confirm = Some(ConfirmDialog::new(...))` — delete provider credentials flow.
2. `codelet/fspec-tui/src/views/agent/resume_session_view.rs:240` —
   `self.delete_confirm = Some(ConfirmDialog::new(...))` — delete session flow.

**Pattern:** `if let Some(dialog) = self.delete_confirm.as_mut() { $$$BODY }`
Matched at `codelet/fspec-tui/src/views/provider_settings/mod.rs:153:9` (the outcome
dispatcher already maps Primary→ConfirmDeleteProviderCredentials, Cancel/Secondary→silent dismiss).

`MergeConfirmDialog` (codelet/fspec-tui/src/views/agent/merge_confirm_dialog.rs) is a
SEPARATE component that does NOT share this enum. Out of scope.

## TS reference (parity source)

**File:** `src/tui/inputHandlers/deleteConfirmModeHandler.ts`
**Function:** `handleConfirmation(input, key, onConfirm, onCancel)` (lines 14–29)

```typescript
if (input === 'y' || input === 'Y') {
  void onConfirm().then(onCancel);
  return true;
}
if (key.escape || input === 'n' || input === 'N') {
  onCancel();
  return true;
}
return true; // Consume all input in confirmation mode
```

Visible UI hint (ProviderSettingsPanel.tsx lines 198, 225, 251, 254):

> "Delete profile {name}? (y/n)"
> "Press 'y' to confirm, 'n' or Esc to cancel"

## Planned change

Insert two new `KeyCode::Char(...)` arms BEFORE the catch-all `_ => Ignored`:

```rust
KeyCode::Char('y') | KeyCode::Char('Y') => self.outcome_for_index(0),
KeyCode::Char('n') | KeyCode::Char('N') => self.outcome_for_index(self.cancel_index()),
```

The existing top-level modifier guard already returns `Ignored` for Ctrl/Alt + any
character, so no per-arm guard is needed.

## Caller-side impact

Zero. Both existing callers (`provider_settings/mod.rs:155-178` and
`resume_session_view.rs:240` consumer site) already destructure all 5 outcomes — they
will route Primary/Cancel from y/n exactly the same way they currently route Enter/Esc.

## Test surface

| Scenario | Layer | Assertion |
|----------|-------|-----------|
| 'y' on 2-button dialog → Primary | `ConfirmDialog::handle_key` direct | Outcome::Primary; focused unchanged |
| 'Y' on 2-button dialog → Primary | direct | case-insensitive |
| 'n' on 2-button dialog → Cancel | direct | Outcome::Cancel; focused unchanged |
| 'N' on 2-button dialog → Cancel | direct | case-insensitive |
| 'n' on 3-button dialog → Cancel | direct | NOT Secondary |
| 'y' on 3-button dialog → Primary | direct | |
| 'y' with focused=1 (Cancel) → Primary | direct | focused unchanged |
| Ctrl+'n' → Ignored | direct | modifier guard |
| Alt+'y' → Ignored | direct | modifier guard |
| 'q' → Ignored | direct | catch-all preserved |
| 'y' in ProviderSettingsView delete dialog → emits Action::ConfirmDeleteProviderCredentials | integration | dialog cleared |
| 'n' in ProviderSettingsView delete dialog → silent dismiss | integration | no Action; dialog cleared |
| Pre-existing keybinds (Esc/Tab/Left/Right/Enter) unchanged | regression | snapshot-style |

## File constraints

- `confirm_dialog.rs` currently 246 lines (with existing tests). Adding ~2 lines to
  handle_key + ~10 new unit-test blocks → safely under 300 LoC ceiling.
- Integration tests will be a NEW file
  `codelet/fspec-tui/tests/confirm_dialog_yn_shortcut_rpc164.rs`.
