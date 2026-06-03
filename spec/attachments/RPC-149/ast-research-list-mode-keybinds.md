# RPC-149: AST Research — List Mode Keybind Shape

## Goal
Verify that `codelet/fspec-tui/src/views/provider_settings/list.rs::handle_list_key` and `mod.rs::move_clamped` already match the TS contract surface (no `r/R`, no `PageUp/PageDown/Home/End`, no wrap-around), and identify the exact strings to assert in regression-shape tests.

## Findings

### list.rs (`handle_list_key`)
File: `codelet/fspec-tui/src/views/provider_settings/list.rs`

Top-level comment (line 8) already documents the resolution:
```
//!     nav (no wrap-around, no PgUp/PgDn/Home/End — RPC-157),
```

`handle_list_key` match arms at lines 37-158 enumerate exactly:
- `KeyCode::Esc` (line 38)
- `KeyCode::Char('/')` (line 52)
- `KeyCode::Tab` (line 64)
- `KeyCode::Up` (line 65)
- `KeyCode::Down` (line 78)
- `KeyCode::Enter` (line 88)
- `KeyCode::Char('d') | KeyCode::Char('D')` (line 139)
- `_` catch-all (line 157)

No `KeyCode::Char('r')`, `KeyCode::Char('R')`, `KeyCode::PageUp`, `KeyCode::PageDown`, `KeyCode::Home`, or `KeyCode::End` arms exist in this file.

(`detail.rs:68` still has `KeyCode::Char('r') | KeyCode::Char('R')` — but that's detail mode refresh-models, not list mode; it stays.)

### mod.rs (`move_clamped`)
File: `codelet/fspec-tui/src/views/provider_settings/mod.rs:244-254`

```rust
pub(crate) fn move_clamped(&mut self, delta: i32) {
    let total = self.visible_providers().len();
    if total == 0 {
        return;
    }
    let max_idx = (total - 1) as i32;
    let current = self.selected_index as i32;
    let new_idx = (current + delta).clamp(0, max_idx);
    self.selected_index = new_idx as usize;
    self.adjust_scroll();
}
```

Uses `.clamp(0, max_idx)` — no `% total`, no `% max`, no `if … wrap` arithmetic. Behavioural test in RPC-159 already covers the runtime; this card adds the structural pin.

## Conclusion
RPC-149 is already implicitly resolved by RPC-157. This card writes fast (sub-ms) source-string regression tests pinning:
1. Absence of `KeyCode::Char('r')` and `KeyCode::Char('R')` in `list.rs`
2. Absence of `KeyCode::PageUp`, `KeyCode::PageDown`, `KeyCode::Home`, `KeyCode::End` in `list.rs`
3. Presence of `.clamp(` and absence of `% total` / `% max` in `mod.rs::move_clamped`
4. Exact enumeration of allowed `KeyCode` arms in `handle_list_key`

Test file: `codelet/fspec-tui/tests/rpc149_list_mode_keybinds_shape.rs`.
