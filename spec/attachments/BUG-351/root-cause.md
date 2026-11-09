# BUG-351 root cause

**Symptom.** Five fspec-tui tests fail at HEAD (seen in TUI-119's regression run) with `left: 11`:
- bug181_live_checkpoint_counts `app_bootstrap_subscriber_folds_pushed_checkpoint_counts_into_the_board_store` (right: 9)
- tui109_checkpoints_progress `tui_bootstrap_spawns_checkpoints_progress_subscriber_forwarding_to_the_bus` (right: 9)
- spawned_subordinate_session_registration_parity_rpc385 `the_subscriber_recovers_from_a_lagged_broadcast_receiver` (right: 9)
- rpc434_session_events_tui_consumption `bootstrap_spawns_the_session_removed_subscriber_with_degenerate_defaults` (right: 10)
- rpc437_session_cleared_tui_consumption `bootstrap_spawns_the_session_cleared_subscriber_with_degenerate_defaults` (right: 10)

**Cause.** `App::bootstrap` spawns 11 subscriber tasks: bootstrap.rs (work_units, chunks, logs, status_changes, session_created, checkpoints_progress), bootstrap_git_state.rs, bootstrap_focus.rs, bootstrap_session_removed.rs, bootstrap_session_cleared.rs, and dispatch_workspaces.rs (RPC-445, e178ad5f, the workspace work-units stream). Each card that added a subscriber bumped only the count literals it knew about. RPC-445 bumped app_bootstrap_rpc009 and reconnect_resubscribe_rpc415 to 11. The five above were already behind (9 or 10) and were left there. The total is copied as a literal in seven test files, and in two scenario steps ("exactly 9", "nine subscriber tasks").

**Production is correct.** All 11 tasks are intended.

**Fix.**
- One `pub const BOOTSTRAP_SUBSCRIBER_TASKS: usize = 11;` in tests/common/mod.rs, documenting the 11 tasks. All seven `subscriber_task_count()` assertions compare against it.
- The two scenario steps name the full bootstrap subscriber set (`BOOTSTRAP_SUBSCRIBER_TASKS`) instead of a number.
- The next card that adds a subscriber changes one constant.

## Status and handover (2026-11-02)

- Done in the working tree and committed as WIP:
  - `BOOTSTRAP_SUBSCRIBER_TASKS = 11` in tests/common/mod.rs.
  - All seven count assertions use it (bug181, tui109/tui.rs, rpc385 lagged test, rpc434/removal_and_bootstrap.rs, rpc437/repeat_and_bootstrap.rs, app_bootstrap_rpc009, reconnect_resubscribe_rpc415).
  - The two scenario steps are reworded (live-checkpoint-counts, session-events-tui-cleared).
  - The seven features are tagged `@BUG-351` and set to `@wip`.
  - Coverage test ranges are re-pointed by @step match.
- Green (ci-test run): app_bootstrap_rpc009 9/9, bug181 11/11, rpc434 5/5, rpc437 5/5, tui109 11/11. The rpc385 count assertion passes. rpc385's two remaining failures are BUG-352 (SessionObserved).
- Still failing in the touched targets, for other causes: rpc385 subordinate and lagged tests (BUG-352) and reconnect_resubscribe_rpc415 `each_broadcast_stream_delivers_a_post_reconnect_event_to_the_app` (BUG-353, pre-existing).
- BLOCKED from leaving testing: `update-work-unit-status BUG-351 implementing` refuses with "Multiple test files detected for feature file" (BUG-355). Next step: fix BUG-355, then move BUG-351 through implementing/validating/done. The validating gate may also ask for impl mappings on all scenarios of the seven features. Then set the seven features back to `@done`.

## Closed (2026-11-02)
- The blockers are gone:
  - BUG-355 (120952de) gave each split feature one test file.
  - BUG-352 (0baf5b0e) made the rpc385 lagged test wait for SessionObserved.
  - BUG-353 (29e4d279) fixed the rpc415 baseline race.
  - BUG-356 (fd0f66b7) gave the RPC-009 feature one test file.
- Green, all seven count tests:
  - BUG-355 batch: bug181 6/6, tui109 5/5, rpc434 5/5, rpc437 5/5, rpc415 4/4.
  - BUG-352 run: rpc385 5/5.
  - Run 4: rpc009 9/9, rpc415 4/4.
- The seven features are back to @done.
