# Epic Review: RPC-058 — Lift scheduler engine into codelet-core::scheduler; /schedule subcommand handler

**Date:** 2026-05-24
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (RPC-058 — no children)
**Feature File Slices Reviewed:** 4

## Summary
- 🔴 Critical: 0 issues
- 🟡 Warnings: 6 issues across 4 feature files (build/tests all PASS)
- 🟢 Observations: numerous (mostly informational)

All 40 scenarios across 4 feature files pass their tests. The work unit's
acceptance criteria are fully met. Findings below are quality/coverage
hygiene fixes only.

---

## Work Unit Results

### Slice 1: rpc058-scheduler-engine-lift.feature — PASS
- 7/7 scenarios covered, tests green, build green.
- 🟡 Coverage line ranges for the "no `session_bindings` references"
  scenario map the entire 352-line `engine.rs` (technically correct for
  a whole-file negative scan; inflates `totalLinesCovered`).
- 🟡 Same broad-range pattern for the "scheduler engine modules live
  under …" scenario (lines 1‒123 of `mod.rs`).
- 🟢 `SchedulerHooks` adds a 5th `find_session_by_schedule_name` method
  with a default impl — forward-compatible extension.

### Slice 2: rpc058-schedule-source-shape.feature — PASS
- 8/8 scenarios covered, tests green, build green.
- 🟡 Coverage for the "Both transports implement the five new methods"
  scenario listed only `embedded.rs`; the scenario explicitly checks
  BOTH `embedded.rs` AND `websocket.rs`. The `websocket.rs` impl lines
  (1011‒1055) were not linked.

### Slice 3: rpc058-schedule-cross-transport-parity.feature — PASS
- 5/5 scenarios covered, tests green.
- 🟡 Coverage `implMappings` for each parity scenario listed only ONE
  transport file per scenario (e.g. only `embedded.rs` for schedule_add)
  even though the scenario itself proves BOTH transports land on the
  same stub.
- 🟡 Coverage also omitted `session_manager_handle.rs` (the stub impls +
  per-call counters) and `rpc/src/lib.rs` (FspecServiceImpl routing) —
  both essential to the round-trip behaviour under test.

### Slice 4: rpc058-schedule-dispatch.feature — PASS
- 20/20 scenarios covered, tests green.
- 🟡 `codelet/fspec-tui/src/app/dispatch.rs` was **exactly 300 LoC**.
  Rule [10] of RPC-058 says dispatch.rs "stays under the 300-LoC
  ceiling" — equality is at the edge; the next addition would push it
  over.
- 🟡 In `schedule_dispatch_rpc058.rs::schedule_popup_pick_with_no_session_is_silent_noop`
  (lines 115‒117) the `@step And no scrollback notice is emitted`
  was followed only by a parenthetical comment, not a positive
  assertion.

---

## Fix Results

### Slice 1 / Slice 2: rpc058-scheduler-engine-lift + rpc058-schedule-source-shape
- 🟡 Broad coverage line-range warnings (whole-file negative scans)
  → ⚪ Left as-is. They are technically correct for the source-shape
    test design (whole-file negative-presence scans). Trimming would
    weaken the test pin without changing behaviour.

### Slice 2: rpc058-schedule-source-shape
- 🟡 "Both transports" missing `websocket.rs` impl mapping
  → ✅ Fixed. Linked `codelet/fspec-tui/src/transport/websocket.rs:1011-1055`
    to the scenario via `fspec link-coverage`.

### Slice 3: rpc058-schedule-cross-transport-parity
- 🟡 Each parity scenario listed only one transport
  → ✅ Fixed. For every scenario, added the second transport's impl
    lines via `fspec link-coverage`:
    * schedule_add → +websocket.rs:1011-1019
    * schedule_list → +embedded.rs:638-640
    * schedule_pause → +websocket.rs:1027-1035
    * schedule_resume → +embedded.rs:649-654
    * schedule_remove → +websocket.rs:1047-1055
- 🟡 Coverage omitted stub + FspecServiceImpl impls
  → ✅ Fixed. For every scenario, added:
    * `codelet/core/src/session_manager_handle.rs:1790-1859`
      (StubSessionManagerHandle::schedule_* per-call counter impls)
    * `codelet/rpc/src/lib.rs:1511-1556`
      (FspecServiceImpl::schedule_* routing through session_manager())

### Slice 4: rpc058-schedule-dispatch
- 🟡 dispatch.rs at 300 LoC ceiling
  → ✅ Fixed. Collapsed the catch-all `if !A && !B && !C && !D && !E
    { let _ = F; }` block into an `||` short-circuit chain. Semantics
    preserved (F still only runs when A–E all return false). File is
    now **298 lines** (under the documented ceiling).
- 🟡 "no scrollback notice" assertion was comment-only
  → ✅ Fixed. Replaced parenthetical comment with a positive assertion
    `assert!(app.agent_view_store().open_sessions().is_empty(), …)`
    that pins the invariant: any leaked notice would have spawned a
    SessionContext via `Action::SessionCreated`, and the assertion
    fails if `open_sessions` is non-empty.

## Final Verification
- `cargo build -p codelet-core -p codelet-sessions -p codelet-napi -p codelet-fspec-tui -p codelet-rpc -p codelet-rpc-types`: ✅ PASS (exit 0)
- `cargo test -p codelet-fspec-tui` (full suite): ✅ PASS — 112 test
  result blocks all green, 0 failures
- `cargo test -p codelet-fspec-tui --test schedule_dispatch_rpc058 --test source_shape_rpc058 --test scheduler_engine_lift_rpc058 --test rpc058_cross_transport_parity`: ✅ PASS — 40/40 green:
  * `schedule_dispatch_rpc058`: 20 passed
  * `source_shape_rpc058`: 8 passed
  * `scheduler_engine_lift_rpc058`: 7 passed
  * `rpc058_cross_transport_parity`: 5 passed
- Feature files valid: ✅ all 4 pass `fspec validate`
- Coverage: ✅ 4/4 features at 100% scenario coverage; impl mappings
  expanded for parity + source-shape slices
- No regressions in pre-existing tests after the dispatch.rs refactor
