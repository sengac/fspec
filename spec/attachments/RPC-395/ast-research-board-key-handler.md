# AST Research — RPC-395 Board '.' key starts new agent

## Objective
Locate the exact code the Rust TUI board uses to (a) start a new agent and
(b) render the header hint text, so a new `.` key handler and header string
change can be implemented ACDD-style.

## Findings

### 1. New-agent trigger — `Action::OpenAgentView`
AST search (`self.emit(Action::OpenAgentView($X))`) in
`codelet/fspec-tui/src/views/board.rs`:

```
board.rs:116:13: self.emit(Action::OpenAgentView(target))
```

Full Shift+Right handler (board.rs:113-118):
```rust
// Shift+Right → open AgentView (with or without an attached session).
if key.code == KeyCode::Right && key.modifiers.contains(KeyModifiers::SHIFT) {
    let target = self.selected_session(store);
    self.emit(Action::OpenAgentView(target));
    return EventResult::consumed();
}
```

`selected_session` (board.rs:212-215) returns `Option<SessionId>` for the
currently selected work unit, or `None` when nothing is selected.

### 2. Key dispatch `match` block
The modifier-free single-char hints (`h/j/k/l`, `[`, `]`, `f/F`, `c/C`,
`d/D`, `a/A`) live in the `match key.code { ... }` block at board.rs:129-207.
A new `KeyCode::Char('.')` arm belongs here (before the `_ => {}` catch-all)
OR as an early `if` mirroring the Shift+Right handler. There is currently
**no** `KeyCode::Char('/')` or `KeyCode::Char('.')` arm.

### 3. Header hint string
`codelet/fspec-tui/src/views/board/keybinding_shortcuts.rs`:
- Line 32 (rendered string): `"C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ / New Agent"`
- Line 8 (doc-comment mirror) and lines 10-11 (doc note) reference the `/` key.

### 4. Tests / snapshots asserting the old string
- `tests/view_board_unit_rpc015.rs:128-132` asserts buffer contains `"/ New Agent"`.
- Snapshot `.snap` files containing `/ New Agent`:
  - `tests/snapshots/app_with_mock_backend__help_dialog_dismissed.snap`
  - `tests/snapshots/app_with_mock_backend_repl__repl_bootstrap_rpc012.snap`
  - `tests/snapshots/app_with_mock_backend__help_dialog_visible.snap`

## Implementation plan
1. Add `KeyCode::Char('.')` handler in board.rs emitting
   `Action::OpenAgentView(self.selected_session(store))`, modifier-free.
2. Change keybinding_shortcuts.rs line 32 to `. New Agent` and sync doc comments.
3. Update the RPC-015 test assertion + the three `.snap` files.
4. Add new RPC-395 tests for the `.` key behavior (selected + unselected) and
   the header string.
