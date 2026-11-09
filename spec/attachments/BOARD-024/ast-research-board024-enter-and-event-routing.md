# BOARD-024 AST research: Enter and modal event routing

Read-only AstGrep calls on 2026-10-28, attributed to BOARD-024.

## Board emission

Language: rust; path: rust/fspec-tui/src/views/board.rs

Pattern: `self.emit(Action::EnterWorkUnit($ID))`

Raw result:

```text
/home/rquast/projects/fspec/rust/fspec-tui/src/views/board.rs:193:17:self.emit(Action::EnterWorkUnit(unit.id.clone()))
```

## Mux interception

Language: rust; path: rust/fspec-tui/src/views/navigator_mux_events.rs

Pattern: `$TX.send(Action::MuxEnterWorkUnit($ID))`

Raw result:

```text
/home/rquast/projects/fspec/rust/fspec-tui/src/views/navigator_mux_events.rs:153:29:tx.send(Action::MuxEnterWorkUnit(unit.id.clone()))
```

## Actual App key path

Language: rust; path: rust/fspec-tui/src/app/events.rs

Pattern: `pub fn handle_event(&mut self, event: &Event) -> EventResult { $$$BODY }`

Raw result:

```text
/home/rquast/projects/fspec/rust/fspec-tui/src/app/events.rs:50:5:pub fn handle_event(&mut self, event: &Event) -> EventResult {
```

## Consequence for causal RED

Integration tests must send actual Event::Key Enter through App::handle_event,
drain the existing action bus into App::dispatch, and inspect a rendered Buffer
and unchanged stores. Existing EnterWorkUnit/MuxEnterWorkUnit actions bypass the
requested chooser and must not be substituted for key events in the RED tests.
The current behavior directly enters agent policy, so absence of both chooser
labels after the first Enter is a causal behavior failure, not an API/compiler
failure. No production files were edited by this research.
