# REFAC-039 AST research

## fspec astgrep --pattern 'subscriber_task_count' --lang rust --path rust/fspec-tui/tests/tui109_checkpoints_progress
rust/fspec-tui/tests/tui109_checkpoints_progress/tui.rs:116:13:subscriber_task_count


## fspec astgrep --pattern 'wait_for_action' --lang rust --path rust/fspec-tui/tests/tui109_checkpoints_progress
rust/fspec-tui/tests/tui109_checkpoints_progress/tui.rs:21:18:wait_for_action
rust/fspec-tui/tests/tui109_checkpoints_progress/tui.rs:129:18:wait_for_action
rust/fspec-tui/tests/tui109_checkpoints_progress/tui.rs:143:18:wait_for_action
rust/fspec-tui/tests/tui109_checkpoints_progress/tui.rs:157:18:wait_for_action


The helpers (fresh_app, sid, tabs_for, wait_for_action, ...) stay in the root and are shared with the included files through `include!` (one module). Nothing outside this test binary uses them.
