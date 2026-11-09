# TUI-116 AST research: where the exec-stdin overlay lives today

Queries run with `fspec astgrep --lang rust --path rust/fspec-tui/src`.

## `self.last_exec_stdin = $X`
```
views/agent/input_area.rs:160  self.last_exec_stdin = Some((session.clone(), request.exec_session_id.clone()))
views/agent/input_area.rs:169  self.last_exec_stdin = None
views/agent/input_area.rs:258  self.last_exec_stdin = None   (ghost row)
```
The focused-pane paint pass is the only writer of the key-routing cache. The notice keeps using it: `Some` means a
notice (or the box) is live for the focused session, so `exec_stdin_keys.rs` can tell Ctrl+O / Ctrl+X apart from chat
keys. The ghost row keeps clearing it, which is what keeps ghost panes free of the notice.

## `Action::ExecStdinSubmit { $$$ }`
```
views/agent/exec_stdin_keys.rs:60
```
Only the key handler builds the submit action, and today it reads and clears the SHARED chat input
(`self.input`). TUI-116 changes the source to the separate answer-box input, so the chat draft can never be sent to
the command.

## `async fn control_exec_stdin($$$) -> $R`
No match inside `fspec-tui/src` with an explicit return pattern (the trait methods are declared with
`-> Result<()>` through `async_trait`), so a text search located them instead:
`transport/backend/subscriptions.rs:175`, `transport/embedded/subscriptions.rs:172`,
`transport/websocket/scheduling_loops.rs:193`. TOOL-027 wired every transport; the TUI only needs a new Action and
a reducer that calls `backend.control_exec_stdin(session, exec_session, control)`.

## Related findings
- `views/agent/dispatch.rs:126` consults `handle_exec_stdin_prompt_key` after HITL keys and before pause keys; the
  ordering stays, but the handler now returns `None` for every chat key while the box is closed.
- `app/events.rs:207` quits on Ctrl+D only when the navigator did not consume the key, so the focused box consuming
  Ctrl+D keeps the TUI running.
- `views/agent/hitl_keys.rs:65` forces the cursor visible whenever `last_exec_stdin` is set; with the notice the chat
  composer keeps its own cursor rules and only the focused box forces it.
