# BUG-225 validation report

## Result and boundaries

- Continue state hydrates on create and attach through a session-keyed action.
- Unknown relative commands wait for the backend; failed reads never submit a
  guessed off/10 state. Failed absolute commands restore unknown when neither
  backend reconciliation nor a prior known value is available.
- Reads and writes retain submission order. Revision guards protect newer
  commands and live updates from delayed hydration or failure rollbacks.
- Notices and deferred commands remain bound to their originating session.
- No session/backend seeding or active-turn synchronization behavior changed.
  Creation-only profile seeding and next-dispatch synchronization tests pass.
- No staging or commits. Card remains validating because broad gates include
  unrelated failures described below; the parent owns final acceptance.

## Owned files

Production:

- `rust/fspec-tui/src/app/dispatch_continue_sync.rs` (new)
- `rust/fspec-tui/src/app/dispatch_slash_continue.rs`
- `rust/fspec-tui/src/app/dispatch_session_chrome.rs` (continue hydration hook only)
- `rust/fspec-tui/src/app/dispatch_capability.rs` (continue action routing only)
- `rust/fspec-tui/src/app/mod.rs` (continue module declaration only)
- `rust/fspec-tui/src/app/state/state_types.rs` (continue sync field/init only)
- `rust/fspec-tui/src/components/mod.rs` (ContinueStateLoaded variant only)
- `rust/fspec-tui/src/store/agent_view.rs` (continue revision map only)
- `rust/fspec-tui/src/store/agent_view/chrome_state.rs`

Tests and specification:

- `rust/fspec-tui/tests/bug225_continue_cache.rs`
- `rust/fspec-tui/tests/bug225/absolute_failure.rs`
- `rust/fspec-tui/tests/common/continue_backend.rs`
- `spec/features/continue-cache-reconciliation.feature`
- `spec/features/continue-cache-reconciliation.feature.coverage`
- BUG-225's workflow/examples in `spec/work-units.json`, via locked fspec commands.

The shared app/module/action files also contain BUG-226's concurrent model-sync
changes. Those hunks are not part of BUG-225 ownership.

## ACDD evidence

- Feature first, exact 1:1 scenario/test mappings with mandatory step comments.
- Initial behavioral red: six failures, two guard cases already passed:
  `/tmp/bug225-red.log`.
- Pending-write refresh red: hydration ran too early:
  `/tmp/bug225-pending-write-red.log`.
- Unknown rollback red: expected unknown but observed `Some((false, 10))`:
  `/tmp/bug225-absolute-failure-red-broad.log`.
- Final own target: **10/10 passing**, `/tmp/bug225-green.log`.
- Scenario coverage: **100%, 10/10**, `/tmp/bug225-coverage.log`.
- Coverage audit: all 20 linked files found; all mappings valid:
  `/tmp/bug225-coverage-audit.log`.

## Passing gates

All Cargo invocations use `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, explicit packages/test targets, and `-j 2`.

- `cargo check -p codelet-fspec-tui --test bug225_continue_cache -j 2`:
  `/tmp/bug225-check.log`.
- `cargo clippy -p codelet-fspec-tui --test bug225_continue_cache -j 2`:
  succeeds, with unrelated warnings listed below; `/tmp/bug225-clippy.log`.
- Five TUI targets, **43 passing**: own target (10),
  `cont002_continue_command_test` (12), `app_bootstrap_rpc018` (7),
  `agent_chrome_parity_rpc018` (9), `source_shape_rpc024` (5):
  `/tmp/bug225-final-green.log`.
- `cont007_live_continue_status_test`: **5 behavioral tests passing**, one
  unrelated source-shape test filtered after the full target exposed it:
  `/tmp/bug225-cont007-behavior.log`.
- `cont008_goal_back_sync_test`: **8 behavioral tests passing**, one unrelated
  source-shape test filtered after the full target exposed it:
  `/tmp/bug225-cont008-behavior.log`.
- `codelet-sessions` targets `cont009_completion_contract_sync` (6) and
  `prov142_session_seed` (3): **9 passing**:
  `/tmp/bug225-lifecycle-compatibility.log`.
- Scoped rustfmt check on all owned Rust files passes:
  `/tmp/bug225-fmt-scoped.log`. All newly created files are under 300 lines.
- `git diff --check`, feature validation, tag validation, and coverage audit pass.

## Unrelated broad-gate findings

- Full CONT-007 target fails
  `emission_sites_twin_mappings_and_chat_print_removal_are_pinned_in_the_sources`
  at line 523: "napi twin must match the ContinueState event".
  Evidence: `/tmp/bug225-regressions.log`.
- Full CONT-008 target fails
  `wire_chunk_and_twin_serializers_carry_the_goal_cleared_flag_and_rejection_count`
  at line 806: NAPI shared write-back source-shape assertion.
  Evidence: `/tmp/bug225-absolute-failure-red-broad.log`.
- Strict `-D warnings` clippy is blocked by unrelated warnings in
  `dispatch_model_sync.rs:73` (`nonminimal_bool`, concurrent BUG-226) and
  `store/agent_view/token_state.rs:100` (`collapsible_match`). Own code has no
  lint suppressions and ordinary scoped clippy succeeds.
- Package-wide `cargo fmt -p codelet-fspec-tui -- --check` reports pre-existing
  formatting in `dispatch_session_lifecycle.rs`,
  `model_selector_refresh_forces_catalog_refetch_bug207.rs`, and
  `rpc437_session_cleared_tui_consumption.rs`; `/tmp/bug225-fmt.log`.
  Those files were not reformatted by this card.
