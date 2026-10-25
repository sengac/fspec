# BUG-265 runtime RED and controller handoff checkpoint

This supersedes the current-phase/status/resource statements in
`preparation-checkpoint.md`, whose preparation evidence remains historical.

## Authoritative state

- BUG-265 is natively **implementing** after actual test compilation and causal
  RED, not Done. Its feature remains `@wip`. No production move or accounting
  repair has been made. The approved full split is still the next operation.
- The controller cleared the build hold after the human's exact approved
  `cargo clean --offline` in the project Rust directory. This session did not
  repeat that clean. The separate old Documents/Codex cache was not deleted.
- Free space was 27 GiB before rebuilding and 14 GiB at this checkpoint, with
  the strict 6 GiB reserve unchanged. No further cleanup is authorized.
- Read-only self-status confirmed session
  `64d78cce-6820-4adc-b7c5-dca958c23b6d`, model `gpt-6.1-sol`, no subordinates.
  No session/model mutation or manual compaction was performed.
- Controller handoff is requested before context loss and before starting the
  large mechanical extraction. This is not batch completion or a Done claim.

## Actual commands and outcomes

All commands ran from `rust/`, offline, package/target scoped, profile `ci-test`,
jobs 2. Command output is retained in the named logs under this attachment dir.

### Missing CONT-008 baseline: four passes

```sh
cargo test --offline --profile ci-test -p codelet-fspec-tui --test cont008_goal_back_sync_test -j 2 goal_satisfied_snapshot_writes_the_chrome_goal_state_back -- --exact
```

The clean rebuild finished in 7m32s; the selected test passed. Three subsequent
invocations of the same command changed only the exact test filter, each passing
with exit 0:

- `a_goal_replaced_mid_turn_survives_the_goal_satisfied_write_back`
- `a_satisfied_goal_is_not_resurrected_on_the_next_dispatched_user_message`
- `the_dispatch_sync_never_reapplies_a_chrome_goal_the_engine_already_consumed`

Evidence: `baseline-cont008.log`. These complete the missing pre-move goal and
generation controls. The earlier 19 runtime control passes remain separately
preserved in `baseline-runtime-controls.log`; they were not re-run in this phase.

### Real BUG-265 RED: five failures and two passing controls

```sh
cargo test --offline --profile ci-test -p codelet-agent-loop --test bug265_recovery_completion_accounting -j 2
```

Compiled successfully in 3m36s. Test execution finished in 18.98s, exit **101**.
Evidence: `causal-red.log`.

- Automatic compaction nudge preservation failed: **four** actual inference
  requests rather than the required three, exposing a renewed nudge allowance.
- Loop-abort nudge preservation failed for the same **4 versus 3** reason.
- Genuine next-task budget reset passed.
- Goal rejections across automatic compaction failed: escalation incorrectly
  reported two consecutive zero-activity nudges instead of four rejected native
  completion claims for the unchanged goal.
- Goal rejections across loop-abort recovery failed for that same causal reason.
- Genuine user dispatch with the unchanged goal failed: registry count **1**
  rather than **2** after the next rejected native Done.
- Replacing and clearing a goal passed, including nonzero inner/registry
  rejection preconditions and disarming after clear.

These are causal runtime assertions, not fixture/type/admission failures. Real
native Done rejection and actual loop/compaction dispatch were reached. The
tests produced three ordinary dead-code warnings from reused shared fixture
members/helpers; they were not silenced. No production code was written first.

The only test-source refinement during resumption adds `PauseRelease` to the
new support file, ensuring a failing fixture cannot leave a synchronous pause
receiver parked during runtime shutdown. Normal synthetic pause resolution
still uses the request-ID-checked response API. This refinement was included
in the actual compiled RED target above.

### Safe source-shape baseline: nine passes, five explicit exclusions

```sh
cargo test --offline --profile ci-test -p codelet-sessions --test background_session_shape -j 2 -- --skip scenario_codelet_sessions_builds_standalone_with_background_session_at_its_new_home --skip scenario_codelet_napi_still_builds_against_the_re_exported_background_session --skip scenario_codelet_sessions_has_no_transitive_dependency_on_codelet_napi --skip scenario_pre_existing_in_file_unit_tests_in_codelet_napi_still_pass_via_the_re_exports --skip scenario_napi_typescript_surface_is_byte_stable_across_the_move
```

Compiled successfully in 2m54s; **9 passed / 0 failed / 5 filtered out**, exit 0.
Evidence: `baseline-safe-shape.log`. The five subprocess scenarios, including
the metadata subprocess, were deliberately excluded; they are not passed.
Do not run the broad target during continuation.

## Integrity and remaining work

Both production files still have empty scoped `git diff --numstat` output:

| Path | SHA-256 before and after runtime RED |
| --- | --- |
| `rust/sessions/src/background_session.rs` | `97c2376986b07e1d9a164bd5cf2760f505e5c969b5e75576ba13a8a17dab048a` |
| `rust/agent-loop/src/agent_loop.rs` | `b1db92a3f686aceea842d3f5b37af60d96017fedb2fea5a0d1ba74dd21b46942` |

Final lightweight checks passed: rustfmt check on the new test entry point and
child modules, and `git diff --check`. New Rust file lengths are 176, 210 and 76.
The earlier Gherkin/tag checks and seven native coverage links are retained.
There is still no implementation coverage, green, Clippy or final quality review.

Next: execute the already approved mechanical split in `module-plan.md`, split
oversized source-shape test modules while preserving every condition and reading
actual moved implementations, then replay the runtime controls and the same
causal RED before any accounting repair. Private helpers that cross extracted
child modules may gain only the ledger's `pub(super)` visibility. Do not change
fields, defaults, channel/drop behavior, atomic ordering, admission or request-ID
gates. Existing `interruption`, `compaction_handoff`, `agent_message` stay unchanged.

The behavior repair then separates genuine input resets from synthetic recovery
sync and avoids resetting registry rejections for an unchanged goal, retaining
CONT-008 generations and CONT-009 arming before output/provider construction.
All seven regressions must pass on final source along with affected controls,
quality and native gates before tagging/transitioning Done.

Canonical status read at handoff: BUG-259–264 remain done; BUG-265 implementing;
BUG-266/267/268/269, PROV-162 and CONT-011 backlog; BUG-254 specifying. No remaining
card is complete. Continue the same authorized batch, coordinator last, with no
commits/publication/restarts/new sessions/credentials changes/extra cleanup.
