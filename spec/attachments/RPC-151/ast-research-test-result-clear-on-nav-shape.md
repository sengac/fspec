# RPC-151 AST Research — testResult clears on ↑/↓ arrow navigation shape

## Goal

Verify the test_result-clear-on-arrow-nav behaviour is wired into
`handle_list_key` in `codelet/fspec-tui/src/views/provider_settings/list.rs`
so the regression-shape tests can pin the exact source-string assertions.

## Source location

`codelet/fspec-tui/src/views/provider_settings/list.rs` —
`pub(super) fn handle_list_key(...) -> ProviderSettingsEvent { ... }`,
lines 30–159.

### KeyCode::Up arm (lines 65–77)

```rust
KeyCode::Up => {
    // RPC-159: mirror TS contract — clear inline test_result
    // ONLY when navigation actually moves the focus. At index 0
    // (boundary), move_clamped is a no-op and test_result is
    // preserved, matching `if (key.upArrow && selectedIndex > 0)`
    // in src/tui/inputHandlers/listModeHandler.ts.
    let before = view.selected_index;
    view.move_clamped(-1);
    if view.selected_index != before {
        view.clear_test_result();
    }
    ProviderSettingsEvent::Consumed
}
```

### KeyCode::Down arm (lines 78–87)

```rust
KeyCode::Down => {
    // RPC-159: same contract as Up arm — clear test_result only
    // on actual movement, preserving it at the last visible row.
    let before = view.selected_index;
    view.move_clamped(1);
    if view.selected_index != before {
        view.clear_test_result();
    }
    ProviderSettingsEvent::Consumed
}
```

### Non-arrow arms (Esc / `/` / Tab / Enter / d/D / fallthrough)

None of these call `view.clear_test_result()` — only the Up and Down arms
perform the clear. Verified via grep:

```
grep -n "clear_test_result" codelet/fspec-tui/src/views/provider_settings/list.rs
# → 74:                view.clear_test_result();
# → 84:                view.clear_test_result();
```

Exactly two call sites, both gated by `view.selected_index != before`.

## Substrings pinned by RPC-151 regression-shape tests

| # | Substring                                       | Rule           |
|---|-------------------------------------------------|----------------|
| 1 | Exactly 2 occurrences of `clear_test_result(`   | Rule [0]+[1]   |
| 2 | KeyCode::Up arm: `let before = view.selected_index;` + `view.move_clamped(-1);` + `if view.selected_index != before {` + `view.clear_test_result();` | Rule [0] |
| 3 | KeyCode::Down arm: `let before = view.selected_index;` + `view.move_clamped(1);` + `if view.selected_index != before {` + `view.clear_test_result();` | Rule [1] |
| 4 | KeyCode::Enter arm body does NOT contain `clear_test_result(` | Rule [2] |
| 5 | KeyCode::Tab arm body does NOT contain `clear_test_result(` | Rule [2] |
| 6 | KeyCode::Esc arm body does NOT contain `clear_test_result(` | Rule [2] |

## Relation to existing coverage

- **RPC-159** (done, ~14 scenarios): full integration coverage in
  `codelet/fspec-tui/tests/provider_settings_clear_test_result_on_nav_rpc159.rs`
  exercising real `handle_list_key` calls with key event simulation.
- **RPC-151** (this card): fast structural source-string scanning to prevent
  silent regression without paying the ratatui compile cost.

Pattern mirrors RPC-077 (handle_impl redundant-clone shape), RPC-149
(provider settings list mode keybinds shape), RPC-156 (delete-confirm
n/N shortcut shape).

## Test file plan

`codelet/fspec-tui/tests/rpc151_test_result_clear_on_nav_shape.rs` —
sub-ms execution; uses brace-balanced extraction of `handle_list_key`
body + per-arm slicing to scope assertions to the correct match arm.
