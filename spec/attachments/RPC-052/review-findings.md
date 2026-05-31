# Review: RPC-052 — Pending-input draft persistence on session switch

**Date:** 2026-05-23
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (single story, no children)

## Summary

- 🔴 Critical: 0
- 🟡 Warnings: 1 (fixed)
- 🟢 Observations: 0

## Status: ✅ PASS (after 1 minor fix)

---

## Work Unit Result

### RPC-052: Pending-input draft persistence on session switch — ✅ PASS

#### 🔴 Critical Issues
None.

#### 🟡 Warnings
1. **Clippy `int_plus_one` warning** in `codelet/fspec-tui/tests/pending_input_durability_rpc052.rs:212` —
   `mock.set_pending_input_calls() >= calls_before + 1` should be
   `mock.set_pending_input_calls() > calls_before`.
   → ✅ **Fixed** in this review pass.

#### 🟢 Observations
None.

#### Coverage Verification
- Feature file: `spec/features/pending-input-draft-persistence.feature` — OK
  - 16 scenarios, valid Gherkin (`fspec validate` passes)
  - Architecture doc-string present and accurate (file list + wire shape + dependencies on RPC-037/RPC-051)
  - `@RPC-052` tag present + component (`@rpc`, `@tui`) + feature-group (`@persistence`, `@session-management`, `@multi-session`, `@agent-view`)
  - Given/When/Then ordering verified scenario-by-scenario; no preconditions hiding after Then
- Test file: `codelet/fspec-tui/tests/pending_input_durability_rpc052.rs` — OK
  - 16 `#[tokio::test]` / `#[test]` functions — one per scenario
  - Every Gherkin step has a matching `@step` comment in the test body
  - Tests exercise real behaviour (assert backend call counts, last-write-wins coalesce, hydration race window, error tolerance)
  - Header comment references the feature file
- Impl files — all under the 300-LoC ceiling:
  - `src/app/dispatch_rpc052.rs` (173 lines) — hosts `handle_pending_input_changed`, `handle_seed_pending_input`, `spawn_clear_pending_input`, `spawn_hydrate_pending_input`
  - `src/app/dispatch.rs` (299 raw lines, well under 300 logical) — routes `SessionCreated` → `spawn_hydrate_pending_input`
  - `src/app/dispatch_rpc022.rs` (254 lines) — routes `Action::PendingInputChanged` / `Action::SeedPendingInput` arms via the catch-all `try_dispatch_rpc022` helper
  - `src/app/dispatch_rpc020.rs` (292 lines) — `handle_input_submitted` calls `spawn_clear_pending_input` after send
  - `src/app/dispatch_rpc026.rs` (222 lines) — `handle_attach_to_session` calls `spawn_hydrate_pending_input`
  - `src/views/agent/dispatch.rs` (295 lines) — emits `Action::PendingInputChanged` only when buffer text actually changed
  - `src/app/state.rs` (249 lines) — adds `pending_input_save_handle: Option<JoinHandle<()>>` field with the documented last-write-wins debounce semantics
  - `src/components/mod.rs` — `Action::PendingInputChanged(String)` and `Action::SeedPendingInput { session_id, text }` variants documented
  - `tests/common/mod.rs` — `MockBackend` extended with scripted `pending_input_store`, error scripting, counters, last-write slot
- Scenario coverage: 16/16 (100%) per `fspec show-coverage`.

#### ACDD Compliance
- Example map: 8 rules, 9 examples, 0 unanswered questions, 8 architecture notes — all reflected in scenarios
- Architecture notes accurately describe the implementation (file paths match, helper names match, MockBackend scripting surface matches)
- Coverage links point at real test/impl line ranges

#### Build & Test Verification
- `cargo build -p codelet-fspec-tui` — ✅ succeeds
- `cargo test --test pending_input_durability_rpc052` — ✅ 16/16 pass (0.41s)
- `cargo clippy --test pending_input_durability_rpc052` — ✅ clean (after fix)
- `fspec validate spec/features/pending-input-draft-persistence.feature` — ✅ valid

#### Cross-Cutting Concerns
- Last-write-wins race handled by `JoinHandle::abort` on each new `PendingInputChanged`
- Hydration race handled by re-checking `current_session()` when `SeedPendingInput` arrives (Scenario 10)
- Synchronous unit-test path preserved via `tokio::runtime::Handle::try_current().is_err()` guards on every spawn
- Errors from `set_pending_input` / `get_pending_input` are silently logged via `tracing::debug!` — verified by Scenarios 13 & 14
- RPC-024 Shift+Left/Right fast path preserved: `SessionContext.input_draft` mirror updated synchronously inside `handle_pending_input_changed`, no per-cycle backend round-trip (Scenario 15)
- No security or performance concerns introduced
- No floating promises, no `unwrap()` in production code, no `todo!()` / `unimplemented!()`
- All async paths in impl wrapped in `if let Err(...)` with `tracing::debug!` — no panics propagate

#### Files Reviewed
- `spec/features/pending-input-draft-persistence.feature`
- `spec/features/pending-input-draft-persistence.feature.coverage`
- `spec/attachments/RPC-052/pending-input-durability.md`
- `spec/attachments/RPC-052/ast-research-existing-patterns.md`
- `codelet/fspec-tui/src/app/mod.rs`
- `codelet/fspec-tui/src/app/state.rs`
- `codelet/fspec-tui/src/app/dispatch.rs`
- `codelet/fspec-tui/src/app/dispatch_rpc020.rs`
- `codelet/fspec-tui/src/app/dispatch_rpc022.rs`
- `codelet/fspec-tui/src/app/dispatch_rpc024.rs`
- `codelet/fspec-tui/src/app/dispatch_rpc026.rs`
- `codelet/fspec-tui/src/app/dispatch_rpc052.rs`
- `codelet/fspec-tui/src/components/mod.rs`
- `codelet/fspec-tui/src/views/agent/dispatch.rs`
- `codelet/fspec-tui/tests/common/mod.rs`
- `codelet/fspec-tui/tests/pending_input_durability_rpc052.rs`

---

## Fix Results

### RPC-052: Pending-input draft persistence on session switch
- 🟡 Clippy `int_plus_one` warning in `tests/pending_input_durability_rpc052.rs:212` →
  ✅ Fixed: replaced `>= calls_before + 1` with `> calls_before`.

## Final Verification
- All tests pass: ✅ (16/16)
- Build succeeds: ✅
- Clippy clean (on RPC-052 files): ✅
- Coverage complete: ✅ (16/16 scenarios)
- Feature file valid: ✅
- Tags valid (for this feature): ✅

| Work Unit | Title                                            | Status  | Issues  |
|-----------|--------------------------------------------------|---------|---------|
| RPC-052   | Pending-input draft persistence on session switch | ✅ PASS | 1 fixed |
