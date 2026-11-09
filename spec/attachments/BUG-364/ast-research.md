# BUG-364 AST research

## fspec astgrep --pattern 'let _ = action_tx.send($A);' --lang rust --path rust/fspec-tui/src/app/dispatch_exec_stdin.rs
rust/fspec-tui/src/app/dispatch_exec_stdin.rs:51:21:let _ = action_tx.send(Action::ExecStdinProbeFetched {
rust/fspec-tui/src/app/dispatch_exec_stdin.rs:120:21:let _ = action_tx.send(Action::ExecStdinDismissed { agent_session: id });

