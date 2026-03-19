# Epic Review: SCHED-013 — Per-Entry Spawned Task for Sub-Minute Loop Intervals

**Date:** 2026-03-19
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1

## Summary
- 🔴 Critical: 2 issues across 1 work unit
- 🟡 Warnings: 4 issues across 1 work unit
- 🟢 Observations: 5

## Work Unit Results

### SCHED-013: Per-Entry Spawned Task for Sub-Minute Loop Intervals — WARN

---

# Review: SCHED-013 — Per-Entry Spawned Task for Sub-Minute Loop Intervals

## Status: WARN

## 🔴 Critical Issues (Must Fix)

1. **Missing interval validation in NAPI endpoint — tight-loop DoS possible**
   - **File:** `codelet/napi/src/session_manager.rs:5901-5957`
   - The `loop_register()` NAPI function accepts `interval_seconds: u32` and passes it directly to `register_with_task_and_idle_check()` with **no validation**. If `interval_seconds = 0`, `tokio::time::sleep(Duration::from_secs(0))` creates a near-tight spin loop that floods the session with `send_input` calls and burns a CPU core.
   - The `try_register_with_task()` method exists in `loop_store.rs:238-251` to enforce the `>= 1s` minimum, but `loop_register()` bypasses it by calling `register_with_task_and_idle_check()` directly.
   - **Fix:** Either add `if interval_seconds < 1 { return Err(...) }` in `loop_register()`, or create a `try_register_with_task_and_idle_check()` variant that combines validation + idle check.
   - **Spec rule violated:** Rule[7] — "Minimum interval is 1 second (enforced at registration) — prevents accidental tight loops". The enforcement exists in a method that no production code calls.

2. **`last_run_at` never updated for task-based loops — observability gap**
   - **File:** `codelet/napi/src/scheduler/loop_store.rs:287-312` (spawned task inner loop)
   - The spawned task fires `on_fire(prompt.clone())` but never calls `mark_executed()` or updates `last_run_at`. The `loop_list()` NAPI endpoint returns `lastRunAt` from the entry, which will **always be `null`** for every active loop — even ones that have fired thousands of times.
   - **Impact:** Any consumer of `loop_list` (TypeScript TUI, external API) sees `lastRunAt: null` forever.

## 🟡 Warnings (Should Fix)

1. **Dead code: `mark_executed()`, `get_due()`, `is_due()` have zero production callers**
   - **File:** `codelet/napi/src/scheduler/loop_store.rs:34-42, 133-137, 140-144`
   - These three methods were part of the pre-SCHED-013 polling architecture. With the engine-polled path removed, they are only called by inline unit tests. Either deprecate/remove them or document they exist for the backward-compatible passive `register()` path.

2. **`loop_store.rs` is 465 lines — exceeds 300-line file limit**
   - **File:** `codelet/napi/src/scheduler/loop_store.rs` (465 lines)
   - The project standard mandates files under 300 lines. The inline `#[cfg(test)]` module (lines 332-465, 133 lines of tests) could be extracted to a separate test file, or the passive API methods could be moved to a separate module.

3. **Build could not be verified (disk space issue)**
   - `cargo build` and `cargo test` both fail with `No space left on device (os error 28)`. This is an infrastructure issue, not a code issue, but it means **tests could not be verified passing** during this review.
   - The compiler reached the linking stage successfully, so the code compiles; only the final archive write/link step fails due to disk space.

4. **Compiler warnings in session_manager.rs (not from this work unit)**
   - **File:** `codelet/napi/src/session_manager.rs:2085, 2100`
   - Two `unused_assignments` warnings for `current_role`. Unrelated to SCHED-013 but present in the same file.

## 🟢 Observations (Nice to Have)

1. **Investigation attachment is excellent** — `spec/attachments/SCHED-013/investigation-findings.md` provides thorough SOLID analysis, alternatives considered, and root cause analysis. Exemplary documentation.

2. **`loopCommandParser.ts` header was updated** — The old "30 second minimum resolution" note was correctly replaced with "Each loop entry spawns its own tokio task — no polling tick bottleneck. Minimum interval is 1 second (enforced at registration)."

3. **Architecture doc string in feature file matches implementation** — The three-line architecture doc string accurately describes `LoopStore` as active task manager, the `HashMap<String, (LoopEntry, JoinHandle<()>)>` pattern (implemented as two separate HashMaps but conceptually equivalent), and removal of `evaluate_and_fire_loops()`.

4. **Test timing approach is pragmatic** — Tests use 1-2 second intervals with 2.5s sleeps instead of literally 5 seconds, keeping the test suite fast. Documented inline.

5. **`try_register_with_task()` is only used in tests** — It exists as a validated variant but no production code uses it. Consider making it the default path or wiring it into `loop_register()`.

## Coverage Verification

- **Feature file:** `spec/features/per-entry-spawned-task-for-sub-minute-loop-intervals.feature` — ✅ OK
  - `@SCHED-013` tag present on line 1
  - 7 scenarios, all with correct Given/When/Then ordering
  - Architecture doc string present (lines 4-8)
  - No placeholder text
  - Background present with proper user story
- **Test file(s):** `codelet/napi/tests/loop_task_test.rs` — ✅ OK
  - File header references the feature file (line 1)
  - 7 tests mapping 1:1 to 7 scenarios
  - All @step comments match Gherkin step text exactly
  - Tests verify actual behavior (timing, fire counts, cancellation, idle-check)
- **Impl file(s):** `codelet/napi/src/scheduler/loop_store.rs`, `codelet/napi/src/scheduler/engine.rs` — ✅ OK (with warnings noted above)
  - loop_store.rs: Per-entry spawned task API implemented correctly
  - engine.rs: `evaluate_and_fire_loops()` removed from tick, loops fully self-managed
  - NAPI endpoint `loop_register()` wired up in session_manager.rs
- **Scenario coverage:** 7/7 scenarios covered (100%)

## Example Map Alignment

- ✅ All 8 rules mapped to scenarios
- ✅ All 6 examples mapped to scenarios
- ✅ No unanswered questions
- ✅ All 3 architecture notes match implementation

## Files Reviewed

- `spec/features/per-entry-spawned-task-for-sub-minute-loop-intervals.feature`
- `codelet/napi/tests/loop_task_test.rs`
- `codelet/napi/src/scheduler/loop_store.rs`
- `codelet/napi/src/scheduler/engine.rs`
- `codelet/napi/src/session_manager.rs` (lines 5895-5957)
- `src/tui/utils/loopCommandParser.ts`
- `src/tui/services/loop-service.ts`
- `spec/attachments/SCHED-013/investigation-findings.md`
