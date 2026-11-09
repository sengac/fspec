# BUG-351 AST research

## fspec astgrep --pattern 'self.subscriber_tasks.push($X)' --lang rust --path rust/fspec-tui/src/app
rust/fspec-tui/src/app/bootstrap_session_removed.rs:46:9:self.subscriber_tasks.push(session_removed_task)
rust/fspec-tui/src/app/bootstrap_git_state.rs:49:9:self.subscriber_tasks.push(git_state_task)
rust/fspec-tui/src/app/bootstrap.rs:161:9:self.subscriber_tasks.push(work_units_task)
rust/fspec-tui/src/app/bootstrap.rs:184:9:self.subscriber_tasks.push(chunks_task)
rust/fspec-tui/src/app/bootstrap.rs:201:9:self.subscriber_tasks.push(logs_task)
rust/fspec-tui/src/app/bootstrap.rs:227:9:self.subscriber_tasks.push(status_task)
rust/fspec-tui/src/app/bootstrap.rs:251:9:self.subscriber_tasks.push(session_created_task)
rust/fspec-tui/src/app/bootstrap.rs:277:9:self.subscriber_tasks.push(checkpoints_progress_task)
rust/fspec-tui/src/app/bootstrap_focus.rs:24:9:self.subscriber_tasks.push(tokio::spawn(async move {
rust/fspec-tui/src/app/dispatch_workspaces.rs:134:9:self.subscriber_tasks.push(tokio::spawn(async move {
rust/fspec-tui/src/app/bootstrap_session_cleared.rs:49:9:self.subscriber_tasks.push(session_cleared_task)

## grep -rn 'subscriber_task_count()' rust/fspec-tui/tests (literal expectations)
rust/fspec-tui/tests/bug181_live_checkpoint_counts.rs-154-        9,
rust/fspec-tui/tests/spawned_subordinate_session_registration_parity_rpc385.rs-209-        9,
rust/fspec-tui/tests/reconnect_resubscribe_rpc415.rs-50-        SUBSCRIBER_STREAM_COUNT,
rust/fspec-tui/tests/reconnect_resubscribe_rpc415.rs-175-        SUBSCRIBER_STREAM_COUNT,
rust/fspec-tui/tests/reconnect_resubscribe_rpc415.rs-208-        SUBSCRIBER_STREAM_COUNT,
rust/fspec-tui/tests/rpc434_session_events_tui_consumption/removal_and_bootstrap.rs-148-        10,
rust/fspec-tui/tests/tui109_checkpoints_progress/tui.rs-117-        9,
rust/fspec-tui/tests/rpc437_session_cleared_tui_consumption/repeat_and_bootstrap.rs-128-        10,
rust/fspec-tui/tests/app_bootstrap_rpc009.rs:109:    assert_eq!(app.subscriber_task_count(), 11);
rust/fspec-tui/tests/app_bootstrap_rpc009.rs:272:    assert_eq!(app.subscriber_task_count(), 11);
