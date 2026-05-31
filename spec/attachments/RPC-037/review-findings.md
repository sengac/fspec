# Epic Review: RPC-037 — Widen SessionManagerHandle + FspecService + both backends + stub with cross-transport parity tests

**Date:** 2026-05-20
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (single story, no children)

## Summary

- 🔴 Critical: 0
- 🟡 Warnings: 4 issues (test scenarios under-asserted relative to Gherkin)
- 🟢 Observations: 3

---

## Work Unit Result

### RPC-037 — WARN (PASS after fixes)

**Type:** story  
**Status:** done (will be moved back through implementing → validating → done)  
**Estimate:** 13 points  
**Parent:** RPC-030  
**Depends on:** RPC-036

#### Files Reviewed

- `spec/features/widen-sessionmanagerhandle-fspecservice-both-backends-stub-with-cross-transport-parity-tests.feature` (204 lines, 20 scenarios)
- `codelet/core/src/session_manager_handle.rs` (1046 lines — trait + StubSessionManagerHandle)
- `codelet/rpc/src/lib.rs` (1292 lines — tarpc trait + FspecServiceImpl)
- `codelet/rpc-server/src/server.rs` (350 lines — fanout wiring)
- `codelet/rpc-server/src/envelope.rs` (Envelope::StatusUpdate variant)
- `codelet/fspec-tui/src/transport/mod.rs` (472 lines — FspecBackend trait)
- `codelet/fspec-tui/src/transport/embedded.rs` (523 lines — EmbeddedFspecBackend)
- `codelet/fspec-tui/src/transport/websocket.rs` (970 lines — WebSocketFspecBackend)
- `codelet/fspec-tui/tests/rpc037_cross_transport_parity.rs` (1115 lines — 25 tests)

---

## 🔴 Critical Issues (Must Fix)

None. The card's stated surface (~30 widened methods, both backend impls, stub overrides, source-shape verifiers, push-driven status fanout, Envelope::StatusUpdate, set_thinking_level_default closure) is fully present. Build is green; clippy is green; all 25 tests pass.

---

## 🟡 Warnings (Should Fix)

### W1. Test scenarios under-assert chunk emissions that the stub demonstrably produces

Rule [5] mandates that the stub override `clear_history`, `compact_session`, `pause_*`, and `toggle_debug` to emit chunks. The stub correctly does so (session_manager_handle.rs lines 782–791, 793–806, 906–918, 920–931). Four scenarios in the feature file explicitly include `And ... arrives on chunks_rx` Then-steps:

1. **Scenario "clear_history emits a UserNotification chunk and returns Ok"** (feature line 68–72)
   - Specifies: `And within 1 second a StreamChunk::UserNotification chunk for that session is observed on chunks_rx whose message contains "history cleared"`
   - Test `clear_history_returns_ok_on_both_transports` does not subscribe to chunks_rx, does not assert the chunk arrives, and contains a misleading comment claiming "emission is wired by a later card" — emission IS wired in the stub already.

2. **Scenario "compact_session returns the canned CompactionResult and emits CompactionComplete"** (feature line 74–78)
   - Specifies: `And within 1 second a StreamChunk::CompactionComplete arrives on chunks_rx for that session carrying the same CompactionResult`
   - Test `compact_session_canned_result_matches_across_transports` does not subscribe to chunks_rx and does not assert the chunk arrives; the in-code comment again misleadingly claims emission is for a later card.

3. **Scenario "pause_confirm / pause_triple / pause_resume update pause state and emit SessionStateChange"** (feature line 132–143)
   - Specifies: `And a StreamChunk::SessionStateChange { state: SessionState::Running } arrives on chunks_rx for sid within 1 second`
   - Test `pause_state_round_trips_across_transports` does not subscribe to chunks_rx and does not assert SessionStateChange chunks arrive after pause_confirm / pause_triple / pause_resume.

4. **Scenario "debug capture toggle is wired through both transports"** (feature line 121–130)
   - Specifies: `Then the call returns Ok(<some path string>) and a StreamChunk::DebugStateChange chunk is observed on chunks_rx for that session`
   - Test `debug_toggle_round_trips_across_transports` does not subscribe to chunks_rx and does not assert the DebugStateChange chunk arrives; the in-code comment claims emission is for a later card.

**Fix:** Subscribe to `chunks_rx()` BEFORE the action in each test, then loop-with-timeout to assert the expected chunk arrives within ~1–2 seconds. Remove the misleading "emission is wired by a later card" comments. This brings the tests into line with the Gherkin and tightens the cross-transport parity guarantee.

---

## 🟢 Observations (Nice to Have)

### O1. Test file location deviates from rule [6] but is defensible

Rule [6] specifies cross-transport parity tests live in `codelet/rpc-embedded/tests/` and `codelet/rpc-server/tests/`. The actual file is at `codelet/fspec-tui/tests/rpc037_cross_transport_parity.rs`. This is the only crate that can import BOTH `EmbeddedFspecBackend` AND `WebSocketFspecBackend`, since neither rpc-embedded nor rpc-server depends on fspec-tui. The deviation is structurally necessary. Not fixing.

### O2. File sizes exceed the 300-line CLAUDE.md guideline

Several core files are well over 300 lines (session_manager_handle.rs 1046, rpc/src/lib.rs 1292, websocket.rs 970, embedded.rs 523, test 1115). These are contract-surface files where splitting would hurt locality. Pre-existing condition; not in scope for this card to refactor.

### O3. Source-shape verifier tests rely on substring matching

`every_new_method_appears_on_*` tests use `body.contains("fn {method}")` — brittle but matches the established pattern from earlier RPC cards. Documented tradeoff.

---

## Coverage Verification

- **Feature file:** `spec/features/widen-sessionmanagerhandle-fspecservice-both-backends-stub-with-cross-transport-parity-tests.feature` — OK (20 scenarios, all @WORK-UNIT-ID tagged, architecture doc string present, no prefill placeholders)
- **Test file:** `codelet/fspec-tui/tests/rpc037_cross_transport_parity.rs` — ISSUE: four scenarios under-asserted (see W1)
- **Impl files:** session_manager_handle.rs / rpc/src/lib.rs / server.rs / transport/mod.rs / embedded.rs / websocket.rs — OK
- **Scenario coverage:** 20/20 linked, build & clippy green, 25/25 tests passing

---

## Fix Results

### RPC-037 — All 🟡 warnings resolved

- **🟡 W1.1 clear_history**: Added `chunks_rx()` subscription before the call on both transports. Added helper `observe_user_notification_history_cleared` that loops 16× with 1s budget. Asserts both transports see a `StreamChunk::UserNotification` whose message contains "history cleared". Removed misleading "emission wired by a later card" comment.
- **🟡 W1.2 compact_session**: Added `chunks_rx()` subscription on both transports before the call. Added helper `observe_compaction_complete` that returns the inner `CompactionResult`. Asserts both transports observe the chunk and that the chunk's `CompactionResult` matches the return value. Removed misleading comment.
- **🟡 W1.3 pause_state**: Added `chunks_rx()` subscriptions on both transports. Added helper `observe_session_state_change_running`. Asserts both transports observe `SessionStateChange { state: SessionState::Running }` after EACH of `pause_confirm`, `pause_triple`, and `pause_resume` (three independent assertion sites).
- **🟡 W1.4 debug_toggle**: Added `chunks_rx()` subscriptions. Added helper `observe_debug_state_change`. Asserts both transports observe `DebugStateChange` chunk after `toggle_debug`. Removed misleading comment.

### Coverage Re-Linking

All 20 scenarios had their test-line ranges updated via `link-coverage` after the test file grew. Implementation line ranges were also corrected to point at the matching stub overrides / tarpc impls.

## Final Verification

- All 25 tests in `codelet/fspec-tui/tests/rpc037_cross_transport_parity.rs` pass (exit 0): ✅
- `cargo build -p codelet-core -p codelet-rpc -p codelet-rpc-types -p codelet-rpc-embedded -p codelet-rpc-server -p codelet-fspec-tui`: ✅ green
- `cargo clippy -p codelet-core -- -D warnings` (rule [7] requirement): ✅ green
- `cargo clippy --tests -p codelet-fspec-tui -- -D warnings`: ✅ green
- `fspec validate` (Gherkin syntax across all 949 features): ✅ all valid
- Coverage: 20/20 scenarios linked with corrected line ranges: ✅
- All embedded transport tests in `codelet/rpc-embedded/tests/` pass: ✅ (24 tests)
- All cross-transport parity tests in `codelet/rpc-server/tests/` pass (10/11; 1 PRE-EXISTING failure in `regression_invariants_rpc011::earlier_rpc_005_010_test_suites_still_pass` due to 12 `#[ignore]` markers in `codelet/fspec/tests/cargo_shape.rs` — this failure is UNRELATED to RPC-037; the failing test is from RPC-011 and the file with too many `#[ignore]` markers was not touched by this card)

### Summary Table

```
┌─────────────┬──────────────────────────────────────────────────────────────────────────┬─────────┬────────────┐
│ Work Unit   │ Title                                                                    │ Status  │ Issues     │
├─────────────┼──────────────────────────────────────────────────────────────────────────┼─────────┼────────────┤
│ RPC-037     │ Widen SessionManagerHandle + FspecService + both backends + stub + tests │ ✅ PASS │ 4 fixed    │
└─────────────┴──────────────────────────────────────────────────────────────────────────┴─────────┴────────────┘
```
