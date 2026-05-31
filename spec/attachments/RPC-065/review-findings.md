# RPC-065 Review — Behaviour-parity test suite for every slash command + keyboard shortcut

**Date:** 2026-05-25
**Reviewer:** Claude Code (fspec review-skill)
**Work Unit:** RPC-065 (story, parent RPC-030)
**Card Scope (strict):** Single integration test file + reusable AppTestHarness driving the AgentView through MockBackend; observable store-state transitions only.

## Summary

- 🔴 Critical: **2 issues** (both fixed)
- 🟡 Warnings: **1 issue** (fixed)
- 🟢 Observations: **2** (no action — within card scope and intentional)

## Files Reviewed

- `spec/features/agent-view-behaviour-parity-matrix.feature` (265 lines)
- `codelet/fspec-tui/tests/behaviour_parity_rpc065.rs` (was 785, now 808)
- `codelet/fspec-tui/tests/common/harness.rs` (was 326, now 366)
- `spec/attachments/RPC-065/behaviour-parity-tests.md`
- `spec/attachments/RPC-065/ast-research-app-harness-surface.md`
- `codelet/fspec-tui/src/views/agent/scrollback.rs` (read-only — to verify observable API)
- `codelet/fspec-tui/src/app/dispatch.rs` (read-only — to verify dispatch path)
- `spec/tags.json` (updated — added `@ignore`)

## Findings

### 🔴 Critical-1: Tag violation — `@ignore` unregistered

**Where:** `spec/features/agent-view-behaviour-parity-matrix.feature` line 227.

The Tab placeholder scenario carries `@ignore` (per rule [9] which mirrors the Rust `#[ignore]` attribute). `fspec validate-tags spec/features/agent-view-behaviour-parity-matrix.feature` failed with:

```
✗ spec/features/agent-view-behaviour-parity-matrix.feature has tag violations:
  Unregistered tag: @ignore in spec/features/agent-view-behaviour-parity-matrix.feature
```

**Fix:** Registered `@ignore` in `spec/tags.json` under Testing Tags with the description: "Marks a scenario as ignored — paired with `#[ignore]` on the corresponding test. Used for placeholder behaviour-parity assertions whose test body compiles but is awaiting a future work unit to wire the production behaviour." Re-validation now passes.

### 🔴 Critical-2: Missing observable assertions in PageDown/End test

**Where:** `behaviour_parity_rpc065.rs` `key_pagedown_and_end_navigate_scrollback_viewport` (lines 759–785 before fix).

The pre-fix test had `// @step Then the scrollback viewport has advanced by one page` and `// @step Then the scrollback is at the bottom` with NO actual assertions — only commented-out claims that "the event was consumed". This violates work-unit rule [6]: "Parity tests only assert OBSERVABLE store-state transitions". The test trivially passed because no `assert!` could fail.

Additionally, two issues hid the gap:

1. The test relied on `KeyModifiers::SHIFT` as a `let _ = …;` dummy to keep an otherwise-unused import alive.
2. `press_key` emits `Action::ScrollbackPageUp/PageDown` via `action_tx`; without a subsequent `drain_pending()` the action never reaches `App::dispatch` and the scrollback state never updates — so even the smoke claim "the event was consumed and dispatched" was untrue at the time the next assertion would have run.

**Fix:**
1. Extended `AppTestHarness` with three accessor methods so the test can probe the scrollback's `ScrollState` without breaking the "no production-code change" rule [G]:
   - `scrollback_offset(&SessionId) -> usize`
   - `scrollback_stick_to_bottom(&SessionId) -> bool`
   - `set_scrollback_viewport_height(&mut self, &SessionId, u16)`
2. The test now (a) seeds a deterministic 10-row viewport; (b) presses PageUp twice with `drain_pending()` after each to step the offset to 0 and assert `!stick_to_bottom && offset == 0`; (c) presses PageDown and asserts `offset > offset_at_top`; (d) presses End and asserts `stick_to_bottom == true`.
3. Removed the dead `KeyModifiers` import and the `let _ = KeyModifiers::SHIFT;` workaround.

All four observable transitions named in the Gherkin scenario are now asserted. Test passes.

### 🟡 Warning-1: Unused import workaround (resolved as part of Critical-2)

The `crossterm::event::KeyModifiers` import was being kept alive solely by `let _ = KeyModifiers::SHIFT;` at the end of the PageDown test (commented "silence unused-import lint on win32 stubs"). The import is unnecessary because the harness now owns all `KeyModifiers::…` references. Removed.

### 🟢 Observation-1: File sizes exceed CLAUDE.md's 300-line guideline

- `behaviour_parity_rpc065.rs`: 808 lines after fix (was 785).
- `common/harness.rs`: 366 lines after fix (was 326).

Both exceed the 300-line refactoring threshold in CLAUDE.md. However:
- Rule [3] / Architecture [D] EXPLICITLY mandate "a single new integration test file" containing the full parity matrix. Splitting would be scope creep against the card.
- Test scaffolding files are commonly larger than production files; the CLAUDE.md guideline targets production code refactoring.

**No action.** The file size is an intentional consequence of the card's design choice.

### 🟢 Observation-2: Architecture note [B] says "private to tests/common/harness.rs" but the struct is `pub`

The architecture note describes the API surface as "private" while `pub struct AppTestHarness` exists. This is a phrasing nuance, not a defect — "private" in this context means "only accessible from `tests/common/`" (since it lives in a test-only module that no production code imports). The struct must be `pub` so `behaviour_parity_rpc065.rs` can construct it via `common::harness::AppTestHarness`.

**No action.**

## Verification

### Build
- `cargo build -p codelet-fspec-tui`: ✅ pass

### Tests
- `cargo test --test behaviour_parity_rpc065`: ✅ 29 passed, 1 ignored (Tab placeholder, per rule [9]), 0 failed, 0.06s wall-clock (well under the 30s budget in AC #4).
- Full crate `cargo test`: ✅ 125 test binaries, **zero failures** (no regressions introduced).

### Spec validation
- `fspec validate spec/features/agent-view-behaviour-parity-matrix.feature`: ✅ valid
- `fspec validate-tags spec/features/agent-view-behaviour-parity-matrix.feature`: ✅ pass (was failing before tag registration)
- `fspec show-coverage agent-view-behaviour-parity-matrix`: ✅ 100% (30/30 scenarios linked)

### Card scope discipline (no scope creep)
- No source code under `codelet/fspec-tui/src/**` changed (architecture note [G] preserved).
- The new harness helpers (`scrollback_offset`, `scrollback_stick_to_bottom`, `set_scrollback_viewport_height`) live in `tests/common/harness.rs` and use ONLY already-public APIs (`scroll_state()`, `set_viewport_height()`).
- The `@ignore` tag registration is the only file outside `tests/` / `spec/features/` touched; it's a tooling registration, not a behaviour change.
- The Tab `#[ignore]` placeholder is preserved exactly as rule [9] mandates.

## Fix Results

| # | Severity | Issue | Status |
|---|----------|-------|--------|
| 1 | 🔴 | `@ignore` tag unregistered → `validate-tags` fails | ✅ Fixed (tag registered in spec/tags.json) |
| 2 | 🔴 | PageDown/End test has no observable assertions | ✅ Fixed (three harness accessors + four real assertions) |
| 3 | 🟡 | Unused `KeyModifiers` import workaround | ✅ Fixed (import + workaround removed) |
| 4 | 🟢 | Two test files >300 lines | No action (intentional per rule [3]) |
| 5 | 🟢 | Architecture note says "private" but struct is `pub` | No action (phrasing nuance) |

## Final Status: ✅ PASS

All critical and warning issues are resolved. RPC-065 ready to return to `done`.
