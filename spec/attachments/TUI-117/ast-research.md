# TUI-117 AST research (2026-11-03)

## `fspec astgrep --pattern 'self.stdin_box.$$$' --lang rust --path rust/fspec-tui/src`

Every use of the single view-level answer box (TUI-116):

- `views/agent/exec_stdin_keys.rs:37,40,59,61,63,74,101` — key routing (focus, unfocus, Enter clears `input`, paste).
- `views/agent/stdin_answer_box.rs:109,142,144,167,174` — height, paint, `stdin_box_focused`, `focused_input`.
- `views/agent/input_area.rs:75,154,163` — layout height, paint gate, and the render-time `unfocus()` when the
  focused session has no request (this is what drops focus today, for whichever session is painted).

All of them go through `StdinAnswerBox { input, focused_session }`, so making the box per session means replacing
that struct with a per-session map and resolving the session from `last_exec_stdin` (already cached at render).

## `fspec astgrep --pattern 'Action::ExecStdinDismissed { $$$ }' --lang rust --path rust/fspec-tui/src`

- `app/dispatch_exec_stdin.rs:120` — write success clears the slot.
- `app/dispatch_exec_stdin.rs:170` — control success clears the slot.
- `views/agent/exec_stdin_keys.rs:44` — Ctrl+X dismiss.

The Ctrl+X site is the only user dismiss, so it gets its own `Action::ExecStdinMuted` (mute that exec run); the
two success sites and the backend clear (`StreamChunk::ExecStdinRequestCleared` in
`app/dispatch_stream_chunks.rs:187`, probe `None` in `try_dispatch_exec_stdin`) share the close path that moves
unsent box text into history.

## Backend clear on new output

`rust/tools/src/unified_exec/exec_stdin.rs` `spawn_exec_stdin_detector`: once fired, child exit, store removal and
`quiet < EXEC_STDIN_QUIET_THRESHOLD_SECS` (output resumed) each `emit(None)`, which the sessions layer turns into
`StreamChunk::ExecStdinRequestCleared`. No backend change is needed.
