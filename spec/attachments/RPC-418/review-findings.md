# Epic Review: RPC-418 — Rust TUI `/compact` lands on a no-op stub

**Date:** 2026-07-08
**Reviewer:** Claude Code (fspec review skill) + ACDD reviewer subordinate
**Work Units Reviewed:** 1 (single bug card, no children)

## Summary
- 🔴 Critical: 0 issues
- 🟡 Warnings: 1 issue (RPC-418)
- 🟢 Observations: 5

## Work Unit Results

### RPC-418: Rust TUI /compact command lands on a no-op stub — PASS (with 1 warning)

The implementation is real, complete, and wired end-to-end. All four Gherkin
scenarios have passing tests, and the fix mirrors the NAPI reference on every
functionally-significant behavior:
- Real `execute_compaction` call (clears conversation, injects instruction,
  resets token tracker) — not a no-op.
- Empty-session guard returns `"Nothing to compact - no messages yet"` and
  leaves the session untouched (return happens before `set_status`).
- `"Continue"` sent to the agent loop after the lock is dropped.
- Real token counts + real `compression_ratio` (no hard-coded 1.0).
- Unknown session id returns `"Session not found: ..."`.

#### 🔴 Critical Issues
None.

#### 🟡 Warnings (Should Fix)
1. **Success status left as `Compacting` on the happy path.** `set_status(Compacting)`
   (handle_impl.rs:293) is only reverted to `Idle` on the failure paths
   (execute_compaction error, send_input error). On success the status is never
   explicitly reset — it relies on `send_input("Continue")` flipping status to
   `Running`. This is *parity-correct* with the NAPI reference (which also defers
   to `send_input`), but it is an implicit coupling. **Fix:** add an explicit
   comment documenting the status contract so a future change to `send_input`'s
   side effect doesn't silently strand the session in `Compacting`.

#### 🟢 Observations (Nice to Have)
1. `turns_summarized: 0` / `turns_kept: 0` are placeholders in BOTH the Rust impl
   and NAPI reference (in-view DAG defers turn counts to the agent). Consistent
   parity, not a defect. A one-line note aids future readers.
2. Debug-capture events intentionally omitted — confirmed out of scope. Nothing
   else functional is missing vs NAPI.
3. `block_in_place` correctness verified — lock dropped before `send_input`, no
   deadlock risk, multi-thread runtime honored (tests use flavor="multi_thread").
4. DRY — compaction logic not duplicated; both Rust handle and NAPI delegate to
   shared `codelet_cli::interactive_helpers::{execute_compaction, compression_ratio}`.
5. Coverage line ranges accurate (impl 261-333; tests 129-194, 201-239, 245-276,
   282-305).

## Coverage Verification
- Feature file: `spec/features/rust-tui-compact-real-compaction.feature` — OK
- Test file: `codelet/sessions/tests/rpc418_compact_session.rs` — OK (4 tests,
  verbatim @step comments, behavioral assertions, header references feature)
- Impl file: `codelet/sessions/src/handle_impl.rs` (compact_session, 261-333) — OK
- Scenario coverage: 4/4 (100%)

## Build & Test
- `cargo test -p codelet-sessions` — full crate suite green (all binaries pass,
  rpc418_compact_session 4/4)
- `cargo build -p codelet-sessions` — exit 0
- `cargo clippy -p codelet-sessions --tests` — zero warnings

## Files Reviewed
- spec/features/rust-tui-compact-real-compaction.feature
- codelet/sessions/tests/rpc418_compact_session.rs
- codelet/sessions/src/handle_impl.rs
- codelet/napi/src/session_bindings.rs (reference)
- codelet/cli/src/interactive_helpers.rs (execute_compaction, compression_ratio)
- codelet/cli/src/compaction_dag.rs (COMPACTION_INSTRUCTION_FRESH)
- codelet/sessions/src/background_session.rs (API surface)
- spec/attachments/RPC-418/root-cause-and-fix-plan.md

## Fix Results

### RPC-418
- 🟡 W1: success-path status left as Compacting (implicit coupling) → ✅ Fixed:
  added an explicit comment documenting the status contract (send_input flips
  Compacting→Running on the happy path; explicit Idle revert only on failure) so
  the coupling is intentional and discoverable. No behavior change; NAPI parity
  preserved.

## Final Verification
- All tests pass: ✅ (codelet-sessions full suite green after fix)
- Build succeeds: ✅
- Clippy clean: ✅
- Coverage complete: ✅ (4/4)
- Feature file valid: ✅
</content>
