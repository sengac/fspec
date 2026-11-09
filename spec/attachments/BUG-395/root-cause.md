# BUG-395 root cause

Filed while working BUG-364 (exec-stdin probe failures).

`rust/fspec-tui/src/app/dispatch_pause_hitl.rs` fire-and-forget writers:

- `handle_pause_confirmed` (`backend.pause_confirm`), `handle_pause_triple` (`backend.pause_triple`),
  `handle_pause_resumed` (`backend.pause_resume`) and `handle_hitl_submitted` (`backend.send_hitl_response`) each
  clear the per-session slot first, then log a failed write with `tracing::debug!(.., "... failed (silently
  dropped)")`. The agent loop stays paused waiting for an answer that never arrived, and the prompt is already gone,
  so the user sees a stuck session with nothing at normal log levels.
- Every one of them, plus `probe_exec_stdin_for` and `handle_exec_stdin_submit` in `dispatch_exec_stdin.rs`, starts
  with `if tokio::runtime::Handle::try_current().is_err() { return; }`. For `handle_exec_stdin_submit` the key handler
  has already moved the text out of the answer box, so the text is lost silently. (`handle_exec_stdin_control`
  already logs this case at error level.)

Fix direction: report each failed write at error level with the session id and restore the prompt (or paint an error
row) so the user can answer again; replace the silent no-runtime returns with the same reporting.
