# REFAC-040 AST research

## fspec astgrep --pattern 'subscriber_task_count' --lang rust --path rust/fspec-tui/tests/rpc437_session_cleared_tui_consumption
rust/fspec-tui/tests/rpc437_session_cleared_tui_consumption/repeat_and_bootstrap.rs:127:13:subscriber_task_count


## fspec astgrep --pattern 'wait_for_action' --lang rust --path rust/fspec-tui/tests/rpc437_session_cleared_tui_consumption
rust/fspec-tui/tests/rpc437_session_cleared_tui_consumption/repeat_and_bootstrap.rs:134:17:wait_for_action


The helpers (fresh_app, sid, tabs_for, wait_for_action, ...) stay in the root and are shared with the included files through `include!` (one module). Nothing outside this test binary uses them.
