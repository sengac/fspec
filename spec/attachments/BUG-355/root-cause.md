# BUG-355 root cause

`fspec update-work-unit-status` enforces 1 feature file = 1 test file. It reads the test files named in the feature's coverage mappings. A test file split with `include!` (the root plus `tests/<name>/*.rs`, the inline_pause_prompt_rpc406 layout) maps one feature's scenarios to several files, so the gate refuses every card tagged on that feature:
- live-checkpoint-counts-in-the-board-header.feature → bug181_live_checkpoint_counts.rs + bug181_live_checkpoint_counts/watcher_updates.rs + polling_and_surface.rs (pre-existing).
- REFAC-039 (3544e5fc): stream-per-item-progress…counter-row.feature → tui109_checkpoints_progress/tui.rs + git_rpc.rs.
- REFAC-040 (c5f5bc10): session-events-tui-cleared-consumption.feature → rpc437 root + rpc437_session_cleared_tui_consumption/repeat_and_bootstrap.rs.
- REFAC-041 (f83b1f22): session-events-tui-consumption.feature → rpc434 root + rpc434_session_events_tui_consumption/removal_and_bootstrap.rs.

REFAC-039/040/041 should have used the REFAC-036/037 pattern: helpers in `tests/<name>_support/mod.rs`, tests in the one root file. Each feature's mapped tests must live in ONE file under 300 lines. Options per feature:
1. Merge the tests back into the root, move the helpers to a support module, and condense the bodies (one-line `/// Scenario:` banners, shared builders). This is feasible for rpc434 (about 290 lines) and for rpc437 (needs about 40 lines condensed).
2. Split the feature file so each part's scenarios map to one included file. This is likely needed for tui109 (about 470 test lines) and bug181.
Blocks BUG-351, which is tagged on these features and cannot leave testing until each feature has one test file.

## Fix (2026-11-02)
- Red: `fspec update-work-unit-status BUG-351 implementing` refused with "Multiple test files detected for feature file" (live-checkpoint-counts-in-the-board-header → bug181 root + polling_and_surface.rs). The new split features were also refused with "No test files linked" until they were linked.
- rpc434 and rpc437: the fragments were folded back into the root files (204 and 227 lines). Shared `sid`/`app_with_mock`/`booted_app`/`tabs_for`/`wait_for_action` moved to tests/session_events_support/, and the rpc437 scrollback helpers moved to tests/rpc437_support/. Those helpers now `expect` the SessionContext; before, `unwrap_or(0)`/`unwrap_or(true)`/`unwrap_or_default` let a missing context pass silently.
- tui109: the feature was split. The TUI fold scenarios stay in the original feature (tests/tui109_checkpoints_progress.rs, helpers in tests/tui109_support/). The capped and non-streaming CLI scenarios moved to checkpoint-enumeration-streams-per-item-progress-frames.feature (tests/tui109_checkpoint_enumeration_stream.rs).
- bug181: the feature was split. The App/transport scenarios stay (tests/bug181_live_checkpoint_counts.rs, bus helpers in tests/bug181_support/). The watcher and source-shape scenarios moved to live-checkpoint-counts-watcher-push.feature (tests/bug181_checkpoint_watcher_push.rs).
- The repo fixtures that tui109 and bug181 had duplicated (`setup_test_repo`, `make_checkpoint`, `service_for`) now live once, in tests/common/checkpoint_repo.rs.
- Green: all 6 targets pass (bug181 6+5, tui109 5+6, rpc434 5, rpc437 5), and narrow clippy with `-D warnings` is clean. Each feature's coverage names exactly one test file.
