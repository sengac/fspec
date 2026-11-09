# BUG-356 root cause

**Symptom.** fspec-tui-app-bootstrap-rpc009.feature mapped two test files: app_bootstrap_rpc009.rs (300 lines) and the include!d app_bootstrap_rpc009/action_shape.rs, which breaks VAL-005 (1 feature = 1 test file). Its coverage was also stale:
- "App bootstrap calls backend.create_session(None)…" claimed 62-77, inside the list_work_units test.
- "Action::FocusNext mutates RootView's focused_pane field" claimed 215-231, inside the Interrupt test.
- Other ranges were shifted, e.g. "work_units broadcast…" at 111-138 and InputSubmitted at 174-193.

The step comments for create_session and FocusNext existed nowhere.

**Cause.** RPC-012 superseded both contracts: session creation is lazy, and RootView/FocusedPane were removed. The tests were deleted, but the scenarios and their coverage stayed. Later edits (BUG-299, RPC-445, BUG-351) re-pointed ranges by @step match, which cannot fix a scenario that has no steps in the file. Nothing caught this because the step-docstring gate only checks features whose FIRST work-unit tag is the moving card (step_docstrings.rs `matching_features`). For this feature that is RPC-009, which is done.

**Red evidence.** At 120952de, `stats.testFiles` for the feature listed two files, and `grep '@step Given a MockBackend with create_session scripted'` on the test file found nothing.

**Fix.**
- Delete the two superseded scenarios.
- Fold action_shape.rs into the root, with wu/live_transcript_for/wait_for_action in tests/app_bootstrap_rpc009_support/ (280 lines).
- Re-link all 9 scenarios to the test fn that carries their @step comments, keeping the impl mappings.

**Lagged-log step.** The Lagged scenario's step "And the task logs at debug level with a \"Lagged\" message" had never been tested. The test now installs a thread-local tracing capture (LogCapture in the support module; tracing-subscriber added as a dev-dependency) and asserts a DEBUG line "work_units subscriber lagged by". Red: with the bootstrap.rs debug! temporarily commented out, the test failed at :242 (run 3). Restored with git checkout.

**Green (2026-11-02).** `cargo test --profile ci-test -p codelet-fspec-tui --test app_bootstrap_rpc009` passed 9/9 in run 4, and narrow clippy with `-D warnings` is clean. Coverage now lists exactly one test file, with all 9 scenarios linked to their own test fns.
