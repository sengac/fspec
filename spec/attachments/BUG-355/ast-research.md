# BUG-355 AST research

## coverage testFiles per affected feature
live-checkpoint-counts-in-the-board-header ['rust/fspec-tui/tests/bug181_live_checkpoint_counts/watcher_updates.rs', 'rust/fspec-tui/tests/bug181_live_checkpoint_counts.rs', 'rust/fspec-tui/tests/bug181_live_checkpoint_counts/polling_and_surface.rs']
stream-per-item-progress-idx-total-for-checkpoint-enumeration-into-the-shared-loadingdialog-counter-row ['rust/fspec-tui/tests/tui109_checkpoints_progress/git_rpc.rs', 'rust/fspec-tui/tests/tui109_checkpoints_progress/tui.rs']
session-events-tui-consumption ['rust/fspec-tui/tests/rpc434_session_events_tui_consumption.rs', 'rust/fspec-tui/tests/rpc434_session_events_tui_consumption/removal_and_bootstrap.rs']
session-events-tui-cleared-consumption ['rust/fspec-tui/tests/rpc437_session_cleared_tui_consumption.rs', 'rust/fspec-tui/tests/rpc437_session_cleared_tui_consumption/repeat_and_bootstrap.rs']

## After the fix: one test file per feature
- live-checkpoint-counts-in-the-board-header → rust/fspec-tui/tests/bug181_live_checkpoint_counts.rs
- live-checkpoint-counts-watcher-push (new) → rust/fspec-tui/tests/bug181_checkpoint_watcher_push.rs
- stream-per-item-progress-…-counter-row → rust/fspec-tui/tests/tui109_checkpoints_progress.rs
- checkpoint-enumeration-streams-per-item-progress-frames (new) → rust/fspec-tui/tests/tui109_checkpoint_enumeration_stream.rs
- session-events-tui-consumption → rust/fspec-tui/tests/rpc434_session_events_tui_consumption.rs
- session-events-tui-cleared-consumption → rust/fspec-tui/tests/rpc437_session_cleared_tui_consumption.rs
