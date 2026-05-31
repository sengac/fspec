# Epic Review: RPC-051 — Keyboard shortcut parity (Shift+up/down history, Ctrl+R search, Esc interrupt cascade)

**Date:** 2026-05-23
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (RPC-051 — leaf card, no children)

## Summary
- 🔴 Critical: 1 issue (clippy redundant clone in RPC-051's own file)
- 🟡 Warnings: 1 issue (clippy doc-lazy-continuation on RPC-051's new Action variant)
- 🟢 Observations: 2 (minor coverage line-range slack, both non-blocking)

## Work Unit Results

### RPC-051: Keyboard shortcut parity — FAIL (before fixes) / PASS (after fixes)

#### A. Feature File Compliance
- `spec/features/keyboard-shortcut-cascade-parity.feature` — ✅ OK
  - All 12 scenarios use correct Given/When/Then ordering.
  - No placeholders (`[role]`, `[action]`, `[benefit]`).
  - Architecture doc string present (lines 13–45) and matches implementation.
  - `@RPC-051`, `@rust`, `@tui`, `@agent-view`, `@keyboard-navigation`, `@navigation`,
    `@interrupt`, `@multi-session`, `@rpc`, `@done` tags present.
  - `fspec validate` passes; `fspec validate-tags` reports NO violations on this feature.

#### B. Example Map Alignment
- 8 rules → all reflected in scenarios (1:1 mapping for cascade levels 1–5 plus
  Ctrl+R focus, Shift+↑/↓ snapshot-and-walk, source-shape constraint).
- 9 examples → all map to scenarios (scenarios 1–6 cover cascade levels 1–5,
  scenarios 10–12 cover Ctrl+R and Shift+↑/↓).
- No unanswered questions (0 red cards).
- Architecture notes match the actual implementation:
  - `Action::AgentEscPressed` added at `components/mod.rs:444` ✓
  - `views/agent/dispatch.rs::handle_event` Esc arm emits `Action::AgentEscPressed`
    at line 237 ✓
  - `app/dispatch_rpc051.rs::handle_agent_esc_pressed` routed via
    `try_dispatch_rpc022` at `app/dispatch_rpc022.rs:241` ✓

#### C. Test Coverage Compliance
- Test file: `codelet/fspec-tui/tests/keyboard_cascade_rpc051.rs` (529 LoC)
- All 12 scenarios have a corresponding `#[tokio::test]` function.
- Every `@step` comment matches the feature file step text exactly (spot-checked
  every scenario).
- Tests exercise `App::handle_event` end-to-end (real compositor → AgentView
  routing), with `MockBackend::interrupt_calls()` / `last_interrupt()` for
  level-4 assertions and `script_history` for Shift+↑/↓ recall.
- `fspec show-coverage keyboard-shortcut-cascade-parity` reports 100% (12/12).
- All 12 tests pass: `cargo test --package codelet-fspec-tui --test
  keyboard_cascade_rpc051` → `test result: ok. 12 passed; 0 failed`.

#### D. Implementation Quality
- `app/dispatch_rpc051.rs` (65 LoC) — single responsibility (routing one
  `Action::AgentEscPressed` variant), depends only on `App` state + the
  `FspecBackend` trait. ✓
- `views/agent/dispatch.rs` (285 LoC, under the 300-LoC ceiling from rule [7]). ✓
- `codelet-fspec-tui` does NOT depend on `codelet-napi` (verified via
  `cargo tree -p codelet-fspec-tui`). Satisfies rule [7] second clause. ✓
- No `todo!()` / `unimplemented!()` / `unwrap()` in RPC-051 production code.
- Async `backend.interrupt(session)` properly awaited via `tokio::spawn` +
  `JoinHandle` pushed onto `pending_tasks` (consistent with RPC-045 pattern).
- 🔴 **CRITICAL (now FIXED):** `app/dispatch_rpc051.rs:54` had a redundant
  `session.clone()` because `session` was not used after the `if is_active`
  branch — flagged by `clippy::redundant_clone` (denied via `-D` in CI).
- 🟡 **WARNING (now FIXED):** `components/mod.rs:442-443` triggered
  `clippy::doc_lazy_continuation` because the trailing paragraph of the
  `AgentEscPressed` doc comment was un-indented after a list.

#### E. Build & Test Verification
- `cargo build -p codelet-fspec-tui` — ✅ compiles cleanly.
- `cargo test --package codelet-fspec-tui --test keyboard_cascade_rpc051` — ✅
  12/12 tests pass.
- `cargo clippy -p codelet-fspec-tui --tests` for files OWNED by RPC-051:
  - `dispatch_rpc051.rs` — ✅ clean after fix.
  - `components/mod.rs` (AgentEscPressed) — ✅ clean after fix.
  - (Out-of-scope: pre-existing clippy errors in `dispatch_rpc050.rs` are NOT
    RPC-051's responsibility and are left for the RPC-050 owner.)

#### F. Cross-Cutting Concerns
- The interrupt-on-Esc path correctly mirrors the existing `Action::Interrupt`
  arm in `app/dispatch.rs:21` (Ctrl+C path) — both spawn `backend.interrupt(id)`
  on the current session. No DRY violation: the two arms diverge in side-
  effects (Esc does NOT call `BackToBoard`; Ctrl+C also does not navigate).
- No security or performance concerns (single fire-and-forget spawn per Esc).

## Coverage Verification
- Feature file: `spec/features/keyboard-shortcut-cascade-parity.feature` — OK
- Test file: `codelet/fspec-tui/tests/keyboard_cascade_rpc051.rs` — OK
- Impl files:
  - `codelet/fspec-tui/src/app/dispatch_rpc051.rs` — OK
  - `codelet/fspec-tui/src/views/agent/dispatch.rs` — OK
  - `codelet/fspec-tui/src/app/events.rs` — OK (level-2 compositor path)
  - `codelet/fspec-tui/src/app/dispatch_rpc025.rs` — OK (Shift+↑/↓ regression
    coverage from RPC-025 reused here)
- Scenario coverage: 12/12 (100%)

## Files Reviewed
- spec/features/keyboard-shortcut-cascade-parity.feature
- spec/features/keyboard-shortcut-cascade-parity.feature.coverage
- spec/attachments/RPC-051/keyboard-parity.md
- spec/attachments/RPC-051/ast-research-keyboard-cascade.md (referenced; not re-read)
- codelet/fspec-tui/tests/keyboard_cascade_rpc051.rs
- codelet/fspec-tui/src/app/dispatch_rpc051.rs
- codelet/fspec-tui/src/app/dispatch_rpc022.rs (routing arm only)
- codelet/fspec-tui/src/app/dispatch.rs (catch-all routing only)
- codelet/fspec-tui/src/app/events.rs
- codelet/fspec-tui/src/components/mod.rs (AgentEscPressed variant)
- codelet/fspec-tui/src/views/agent/dispatch.rs

## Fix Results

### RPC-051: Keyboard shortcut parity
- 🔴 Issue 1: `app/dispatch_rpc051.rs:54` redundant `session.clone()` →
  ✅ Fixed: removed intermediate `session_for_task` binding and moved `session`
  directly into the `tokio::spawn` closure.
- 🟡 Issue 2: `components/mod.rs:442-443` doc-lazy-continuation on the new
  `AgentEscPressed` paragraph → ✅ Fixed: inserted a blank `///` line before the
  "When no current session is open..." paragraph so the rustdoc list ends
  cleanly.

## Final Verification
- All tests pass: ✅ (12/12 in `keyboard_cascade_rpc051`)
- Build succeeds: ✅ (`cargo build -p codelet-fspec-tui`)
- Coverage complete: ✅ (12/12 scenarios linked)
- Feature file valid: ✅ (`fspec validate`)
- Tags valid for this feature: ✅ (no violations on
  `keyboard-shortcut-cascade-parity.feature`)
- Clippy clean for RPC-051's own files: ✅
- `views/agent/dispatch.rs` LoC: 285 / 300 (rule [7] respected): ✅
- `codelet-fspec-tui` does NOT depend on `codelet-napi` (rule [7]): ✅

## Out of Scope (noted, NOT fixed per "no scope creep")
- `app/dispatch_rpc050.rs` has 3 pre-existing `clippy::redundant_clone` errors
  on lines 47, 48, and 117 — these belong to RPC-050 and should be picked up
  by a separate review/fix on that card.
- Coverage line ranges are off by 1–4 lines (mostly include the
  `#[tokio::test]` attribute or trailing blank lines beyond the closing brace).
  Minor imprecision but coverage points at the correct test bodies.
