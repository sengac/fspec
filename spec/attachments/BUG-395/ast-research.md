# BUG-395 AST research

- `rust/fspec-tui/src/app/dispatch_pause_hitl.rs`: `App::handle_pause_confirmed`, `App::handle_pause_triple`,
  `App::handle_pause_resumed`, `App::handle_hitl_submitted` — each `tokio::spawn`s a backend write whose `Err` arm is
  `tracing::debug!` only; each begins with a silent `Handle::try_current().is_err()` return.
- `rust/fspec-tui/src/app/dispatch_exec_stdin.rs`: `App::handle_exec_stdin_submit` silent no-runtime return (the
  text was already taken from the box by the key handler); `App::handle_exec_stdin_control` is the reference pattern
  (`tracing::error!` "no tokio runtime").
- Tests: `tests/common/mock_scripting_4.rs` already has `set_pause_confirm_error`, `set_pause_triple_error`,
  `set_pause_resume_error`, `set_send_hitl_response_error` for scripting the failures.
