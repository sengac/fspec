# REFAC-041 AST research

## fspec astgrep --pattern 'subscriber_task_count' --lang rust --path rust/fspec-tui/tests/rpc434_session_events_tui_consumption
rust/fspec-tui/tests/rpc434_session_events_tui_consumption/removal_and_bootstrap.rs:147:13:subscriber_task_count


## fspec astgrep --pattern 'wait_for_action' --lang rust --path rust/fspec-tui/tests/rpc434_session_events_tui_consumption
rust/fspec-tui/tests/rpc434_session_events_tui_consumption/removal_and_bootstrap.rs:50:18:wait_for_action
rust/fspec-tui/tests/rpc434_session_events_tui_consumption/removal_and_bootstrap.rs:154:17:wait_for_action


The helpers (fresh_app, sid, tabs_for, wait_for_action, ...) stay in the root and are shared with the included files through `include!` (one module). Nothing outside this test binary uses them.
