# BUG-228 — CONT-009 parity test still assumes a duplicated NAPI loop

## Summary and confidence
- Audit baseline: HEAD `7e60de8e`, 2026-10-21; source-only audit.
- **Source-confirmed static mismatch; high confidence:** a CONT-009 source-shape test demands a local dispatch block in NAPI's agent-loop file, but that file now re-exports the shared engine.
- This predicts failure of the stale source-shape assertion; the test was **not executed**, so no observed test result or runtime arming failure is claimed.
- No builds, live requests, fspec mutations, or card/coverage changes were made for this investigation.

## Source chain and history
1. `rust/sessions/tests/cont009_completion_contract_sync.rs:330` defines the expected shared-helper name, `:331` a local lock-reacquisition anchor, and `:332` `BackgroundOutput::with_provider` as the dispatch-output anchor.
2. `rust/sessions/tests/cont009_completion_contract_sync.rs:380` starts `both_twins_call_shared_sync_helper_at_dispatch_site`; `:384` reads NAPI source and `:385` the shared agent-loop source.
3. `rust/sessions/tests/cont009_completion_contract_sync.rs:393` requires the output-construction anchor in **each** file; `:396` requires a preceding lock, and `:403` searches that window for the sync helper. For NAPI, the first local-dispatch requirement is already incompatible with its new source shape.
4. `rust/napi/src/agent_loop.rs:1` describes compatibility exports; `:6` re-exports `agent_loop`, provider dispatch support, BackgroundOutput, and BackgroundProgressEmitter from `codelet_agent_loop`. There is no local dispatch block to match.
5. `rust/agent-loop/src/agent_loop.rs:569` contains the actual lock reacquisition; `:576` calls `session.sync_completion_contract_for_user_turn` before output construction.
6. `rust/sessions/src/background_session.rs:1525` still owns the shared helper; `:1529` syncs continue state, `:1532` resets per-turn counters, and `:1558` arms the registry and syncs the goal.
7. Local history identifies commit `af50d419` (2026-10-21), `fix(compaction): resume unfinished tasks through the shared agent loop`, as the consolidation context. NAPI's re-export is intentional architecture, not evidence that the shared helper disappeared.

## Impact and boundaries
- The stale test can report a regression solely because duplication was removed. It no longer verifies the actual NAPI-to-shared-engine route.
- Removing the failing assertion without replacement would weaken protection against a frontend bypassing the shared dispatch/arming lifecycle.
- Behavioral helper coverage remains valuable and should be preserved. The presence of a shared call establishes source wiring, not full runtime correctness under every entry point.
- This is test/architecture drift, not proof that continue or goal arming is broken in production at this baseline.

## Reproduction outline — proposed, not executed
1. Inspect the comment-stripped NAPI source: it contains the shared-engine export but not `BackgroundOutput::with_provider`.
2. Trace the existing test's first `src.find(OUTPUT_ANCHOR)` for that file. Its own failure branch would report a missing local output construction.
3. In an authorized testing task, run only this source-shape test in the `cont009_completion_contract_sync` sessions integration target; do not describe that result as observed until actually run.
4. After updating the contract, verify a deliberately removed shared helper call and a deliberately replaced NAPI delegation are each detected, while the intended re-export passes.

## Existing specifications/tests and missing regressions
- `spec/features/completion-contract-dispatch-arming.feature:9` describes a shared helper but `:22` still describes two local loop sites; the twin-specific scenario at `:80` needs architecture-aware reconciliation.
- `rust/sessions/tests/cont009_completion_contract_sync.rs:85` behaviorally tests continue sync/registry arming; `:135`, `:182`, `:215`, `:240`, and `:294` exercise other helper invocations for goal/disarm/reset/clear/unchanged-goal cases.
- `rust/sessions/tests/cont009_completion_contract_sync.rs:411` rejects inline duplicated registry calls, and `:422` checks the shared helper home. Preserve those invariants in architecture-appropriate form.
- `spec/features/cont009-completion-contract-sync-tests-fail-because-fresh-session-returns-empty-session-id.feature:25` and the fixture setup at `rust/sessions/tests/cont009_completion_contract_sync.rs:43` concern BUG-155's cache/default-model setup; that is a different failure mode.
- Missing: an explicit NAPI re-export/delegation invariant plus a shared-engine dispatch-order assertion, and behavioral coverage proving frontend entry paths use the shared completion-contract lifecycle without duplicated logic.

## Recommended direction
- Revise the acceptance criterion from textual twin parity to one shared dispatch implementation plus frontend delegation.
- Assert the real shared dispatch window contains the helper before output/agent construction; separately verify NAPI exports/delegates to that engine and does not introduce a divergent local loop.
- Preserve the shared helper's real-session behavioral tests, goal-generation safeguards, per-turn reset checks, and registry arming/disarming assertions. Do not restore duplicate production logic merely to satisfy obsolete source scanning.
- Update the test's historical comments and specification architecture notes in the eventual implementation task, not in this documentation-only audit.

## Uncertainties and related cards
- The stale assertion was not run; compile prerequisites, independent test failures, and full frontend behavioral coverage are outside this audit's evidence.
- CONT-009 owns dispatch arming, BUG-155 owns earlier fixture setup, and CONT-008 owns goal-generation protection. Commit `af50d419` is relevant consolidation history, not a measured regression result.
- BUG-223 concerns a genuine later completion-settlement bypass; BUG-221/BUG-222 concern exhaustion/retry behavior. CONT-010 owns lifecycle semantics. This test drift should not be used to attribute any of those incident hypotheses.
