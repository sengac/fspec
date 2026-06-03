# RPC-152 AST Research — Tab → SwitchToModels event shape

## Goal

Verify the Tab → SwitchToModels event dispatch is wired into
`handle_list_key` in `codelet/fspec-tui/src/views/provider_settings/list.rs`
and that the matching enum variant exists in `mod.rs`, so the
regression-shape tests can pin the exact source-string assertions.

## Source locations

### `codelet/fspec-tui/src/views/provider_settings/mod.rs` (enum variant)

```rust
#[derive(Debug, Clone)]
pub enum ProviderSettingsEvent {
    Consumed,
    Ignored,
    Emit(Action),
    Close,
    /// RPC-160: list-mode Tab keybind emits this variant — distinct from
    /// `Close` and `Emit(Action)`. The Navigator translates it to the
    /// model-settings view transition (TS analog:
    /// `onSwitchToModels()` callback in
    /// src/tui/inputHandlers/listModeHandler.ts lines 56-60). Pure UI
    /// navigation event — no Action payload.
    SwitchToModels,
}
```

`SwitchToModels,` variant at line 69 (inside enum block 57–70).

### `codelet/fspec-tui/src/views/provider_settings/list.rs` (Tab arm)

```rust
pub(super) fn handle_list_key(
    view: &mut ProviderSettingsView,
    key: KeyEvent,
) -> ProviderSettingsEvent {
    if view.filter_mode {
        return handle_filter_key(view, key);   // ← guards Tab in filter mode
    }
    match key.code {
        KeyCode::Esc => { ... }
        KeyCode::Char('/') => { ... }
        // RPC-160: Tab in List mode emits the new SwitchToModels event.
        KeyCode::Tab => ProviderSettingsEvent::SwitchToModels,
        ...
    }
}
```

Tab arm at line 64. The filter-mode guard at line 34 ensures Tab while
the user is typing in the filter is routed to `handle_filter_key`, never
to `SwitchToModels`.

### `handle_filter_key` (negative assertion)

```
grep -n "SwitchToModels" codelet/fspec-tui/src/views/provider_settings/list.rs
# → 56:        // RPC-160: Tab in List mode emits the new SwitchToModels event.
# → 64:        KeyCode::Tab => ProviderSettingsEvent::SwitchToModels,
```

Two occurrences total — both inside `handle_list_key` (one comment, one
arm). Zero occurrences in `handle_filter_key`.

## Substrings pinned by RPC-152 regression-shape tests

| # | File   | Substring                                                    | Rule        |
|---|--------|--------------------------------------------------------------|-------------|
| 1 | mod.rs | `pub enum ProviderSettingsEvent {` … `SwitchToModels,` (inside enum block) | Rule [0] |
| 2 | list.rs handle_list_key body | `KeyCode::Tab => ProviderSettingsEvent::SwitchToModels` | Rule [1] |
| 3 | list.rs handle_list_key body | `if view.filter_mode {` appears BEFORE `KeyCode::Tab => ProviderSettingsEvent::SwitchToModels` | Rule [2] |
| 4 | list.rs handle_filter_key body | NO occurrence of `SwitchToModels` | Rule [3] |

## Relation to existing coverage

- **RPC-160** (done): full integration coverage in
  `codelet/fspec-tui/tests/provider_settings_tab_switch_to_models_rpc160.rs`
  exercising real `handle_key` calls with simulated Tab events across
  list mode, filter sub-mode, Detail sub-modes, and the delete-confirm dialog.
- **RPC-152** (this card): fast structural source-string scanning to prevent
  silent regression without paying the ratatui compile cost.

Pattern mirrors RPC-077 / RPC-149 / RPC-151 / RPC-156 fast regression-shape
coverage cards.

## Test file plan

`codelet/fspec-tui/tests/rpc152_tab_switch_to_models_shape.rs` — sub-ms
execution; uses brace-balanced extraction of the `pub enum
ProviderSettingsEvent { ... }` block, `handle_list_key` body, and
`handle_filter_key` body to scope assertions correctly.
