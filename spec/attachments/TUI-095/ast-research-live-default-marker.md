# TUI-095 — AST Research: live `(default)` marker D-key handler

Tool: AstGrep (language: rust)

## Query 1 — locate the dialog event handler that owns the D-key branch
Pattern: `fn handle_event(&mut self, event: &Event) -> EventResult { $$$BODY }`

Result:
```
codelet/fspec-tui/src/components/thinking_level_dialog.rs:122  fn handle_event(&mut self, event: &Event) -> EventResult { ... }
```

The `KeyCode::Char('d') | KeyCode::Char('D')` arm (lines 153-158) currently:
```rust
KeyCode::Char('d') | KeyCode::Char('D') => {
    let level = self.selected_level();
    let action = Action::SetThinkingLevelDefault(self.session_id.clone(), level);
    self.emit_action(action);
    return EventResult::consumed();
}
```
It emits the persistence action but never mutates `self.default_index`. This is the
single point to change: set `self.default_index = Some(self.selected_index)` before the
return.

## Query 2 — confirm render reads default_index (the field drawn as the marker)
Pattern: `fn render(&mut self, area: Rect, buf: &mut Buffer) { $$$BODY }`

Result:
```
codelet/fspec-tui/src/components/thinking_level_dialog.rs:182  fn render(&mut self, area: Rect, buf: &mut Buffer) { ... }
```

`render()` builds rows via `label_description_default_row(label, desc, i == self.selected_index, Some(i) == self.default_index)`. Because the marker is derived from `self.default_index`, mutating that field in the D-key arm makes the next frame reflect the new default — no reopen, no parent involvement.

## Conclusion
- Field already exists: `default_index: Option<usize>` (line 41).
- `selected_index` is the row chosen by the user; `LEVELS[selected_index].0 == selected_level()`.
- Minimal, surgical fix: one assignment in the D-key arm. No changes to dispatch,
  `dialog_theme_rows.rs`, or the persistence layer.
- New integration test `tests/tui095_live_default_marker.rs` drives `handle_event` with a
  `KeyCode::Char('d')` event and asserts the rendered buffer + emitted action.
