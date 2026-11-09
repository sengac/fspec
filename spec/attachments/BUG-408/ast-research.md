# BUG-408 AST research

- `fspec astgrep --lang rust --pattern 'tokio::runtime::Handle::try_current()' --path rust/fspec-tui/src/app` lists
  the guards in dispatch_pause_hitl.rs (handle_pause_chunk) and dispatch_exec_stdin.rs (probe_exec_stdin_for,
  handle_exec_stdin_submit, handle_exec_stdin_control — the last already logs at error).
- `App::handle_exec_stdin_write_failed` (dispatch_exec_stdin_recovery.rs:44) is the restore path to reuse.
