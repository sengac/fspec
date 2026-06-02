# AST Research — RPC-160 Tab Switch-to-Models Keybind

## Goal

Identify the precise editing surface for adding a new
`ProviderSettingsEvent::SwitchToModels` variant and binding `KeyCode::Tab`
in List mode to emit it.

## TS Reference (Canonical Behaviour)

`src/tui/inputHandlers/listModeHandler.ts` lines 56-60:

```typescript
// Tab: switch to model selector
if (key.tab) {
  onSwitchToModels();
  return;
}
```

`onSwitchToModels` is a sibling callback to `onClose`, never dispatches a
Redux action, just signals the parent React tree to mount the models view
instead of the provider-settings panel.

## Rust Editing Sites

### 1. `codelet/fspec-tui/src/views/provider_settings/mod.rs` (line 56)

Current enum:

```rust
pub enum ProviderSettingsEvent {
    Consumed,
    Ignored,
    Emit(Action),
    Close,
}
```

Add new variant `SwitchToModels` — a pure UI navigation event with no
payload, analogous to `Close`.

### 2. `codelet/fspec-tui/src/views/provider_settings/list.rs` (line 37 `handle_list_key` match)

Filter-mode check is at the top (lines 34-36); arrow / Enter / `/` / `d`
/ Esc arms are after. Insert `KeyCode::Tab => ProviderSettingsEvent::SwitchToModels`
anywhere between Esc and the catch-all `_ => Consumed` at line 133.

`handle_filter_key` (line 137) catch-all already returns `Consumed`, so
Tab in filter sub-mode is automatically a no-op without any new code.

### 3. `codelet/fspec-tui/src/views/navigator.rs` (line 104 match)

Exhaustive match on `ProviderSettingsEvent`. Adding the variant forces a
new arm. Insert `ProviderSettingsEvent::SwitchToModels => EventResult::consumed()`
for now — actual model-settings view doesn't exist yet, so emit no
Action. Follow-up card will replace this with the real transition.

## Existing Test Sites That May Need Updates

`codelet/fspec-tui/tests/provider_settings_view_rpc054.rs` —
no Tab-related test currently exists in List mode (it would have been a
deviation pre-RPC-160). Any pre-existing test that drives `handle_key`
with `KeyCode::Tab` would need its expected return value reviewed.

```
$ rg "KeyCode::Tab" codelet/fspec-tui/
codelet/fspec-tui/src/views/agent/confirm_dialog.rs  (focus cycling — unrelated)
```

No conflicting List-mode Tab test exists.

## Detail-Mode Tab

`detail.rs::handle_summary_key`, `handle_edit_key`, `handle_oauth_notice_key`
each have a catch-all `_ => Consumed` arm. Tab falls into them and is
silently consumed — matches Rule 4.

## ConfirmDialog Tab Routing

`mod.rs::handle_key` (line 177) routes keys to `delete_confirm` first if
the dialog is open (lines 187-209). The dialog has its own Tab focus
cycling. Therefore Tab while dialog is open will never reach
`handle_list_key` — matches Rule 4 / Example 6.

## Filter-Mode Routing

`handle_list_key` line 34: `if view.filter_mode { return handle_filter_key(...) }`
intercepts ALL keys when filter_mode is true. Tab falls into
`handle_filter_key`'s catch-all `_ => Consumed` (line 172). Matches
Rule 5.

## Estimate Justification

- 1 enum variant addition (1 line)
- 1 list.rs match arm (1 line)
- 1 navigator.rs match arm (1 line)
- ~8 integration tests (~250 LoC test file)
- 0 file rename / restructure

Net implementation: ~3 LoC of source + ~250 LoC of tests.
Story points: 5 (covers spec + tests + verification of three call
sites + matrix of edge cases).
