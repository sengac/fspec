# BUG-381 root cause

## Symptom
Dispatching `Action::EnterWorkUnit(id)` for a card that has no open session and is not in any board column does
nothing: no dialog, no view change, no log line. Seen while fixing BUG-380, where the storage-diagnostics test
dispatched `EnterWorkUnit("BUG-271")` on an empty board and the screen stayed on the plain board.

## Cause
`App::handle_enter_work_unit` (`rust/fspec-tui/src/app/dispatch_work_unit_entry.rs`) since 626eee1ec:

```rust
if let Some(index) = associated { ... }
else if let Some(context) = self.lookup_work_unit_context(id) {
    self.open_creation_dialog(None, Some(context));
}
// no else: unknown id is dropped
```

`lookup_work_unit_context` (`dispatch_work_unit_binding.rs:109`) returns `None` for an id missing from the board
columns. Emitters: `components/board_card/events.rs:35` (board card dialog), which can outlive a board refresh that
removed the card or moved it to another workspace.

## Fix direction
Log the miss with `tracing::warn!` and surface it to the user (for example `SessionOperationFailed("Work unit <id>
is no longer on the board")` or a board notice), with a test that enters an id missing from the board and asserts
the visible message.
