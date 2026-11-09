# BUG-394 AST research

- `fspec astgrep --pattern 'let _ = action_tx.send($A);' --lang rust --path rust/fspec-tui/src/app/dispatch_pause_hitl.rs`
  → three sends inside the Paused probe task.
- `grep -n "silently dropped" rust/fspec-tui/src/app/dispatch_pause_hitl.rs` → the pause, HITL and exec-stdin reads.
- `rust/fspec-tui/src/app/dispatch_exec_stdin_recovery.rs::send_to_app` is the existing logging send helper.
