# RPC-159 AST Research — clear_test_result wiring for Up/Down nav

## Goal
Mirror TS behavior: `setTestResult(null)` only fires inside the guarded `if` blocks for Up/Down arrow navigation (i.e. only when navigation actually moves the focus).

## Findings

### Existing test_result methods (codelet/fspec-tui/src/views/provider_settings/test_result.rs)
- `pub fn set_test_result(&mut self, provider_id: impl Into<String>, status: ProviderTestStatus)`
- `pub fn clear_test_result(&mut self)` — already exists; sets `self.test_result = None`. Pure: never touches selected_index, scroll_offset, mode, filter, filter_mode, expanded, nav_items, status.

### Current Up/Down handler (list.rs L65-72)
```rust
KeyCode::Up => {
    view.move_clamped(-1);
    ProviderSettingsEvent::Consumed
}
KeyCode::Down => {
    view.move_clamped(1);
    ProviderSettingsEvent::Consumed
}
```

### move_clamped (mod.rs L244)
- Clamps `(current + delta)` into `[0, total-1]`.
- Returns `()`. At boundary or with `total == 0`, `selected_index` is unchanged.
- We can detect "did movement happen" by snapshotting `view.selected_index` before vs after the call.

### Filter mode routing
- `handle_list_key` is short-circuited at the top by a `view.filter_mode` check that dispatches to `handle_filter_key`. So arrow handling while filter_mode==true never reaches our Up/Down arms. Test must construct view with filter_mode=true and assert test_result is untouched.

### Tab handling
- Already wired (L65 — `KeyCode::Tab => ProviderSettingsEvent::SwitchToModels`). No mutation of test_result; preserved by default.

### Enter handling
- Provider rows toggle expansion; ApiKey routes to Detail::EditApiKey. Neither path touches test_result. Preserved by default.

### Slash ('/') handling
- Activates filter_mode. Does not touch test_result.

## Implementation strategy
In `list.rs` Up/Down arms, snapshot `view.selected_index` before `move_clamped`; if it differs after, call `view.clear_test_result()`. This matches TS exactly: clear iff movement happened.

## Test file
`codelet/fspec-tui/tests/provider_settings_clear_test_result_on_nav_rpc159.rs` — one test per scenario; uses `view_with_n_providers()` helper pattern from rpc158 tests.
