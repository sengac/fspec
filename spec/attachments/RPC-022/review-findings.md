# Epic Review: RPC-022 — Modal dialogs: ModelSelector + ThinkingLevel + RoleBanner

**Date:** 2026-05-19
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (RPC-022 has no children)

## Summary
- 🔴 Critical: 1 issue (Rule [5] violation in handle_slash_command)
- 🟡 Warnings: 1 issue (stale work-unit architecture note)
- 🟢 Observations: many minor (file at 299 LoC, scenario outlines collapsed to single tests, substring-shape gaps in source-shape feature) — all non-blocking

---

## 🔴 Critical Issues

### C1 — Rule [5] violation: `/role` popup picker falls through to `[notice]`

**File:** `codelet/fspec-tui/src/app/dispatch_rpc020.rs:60-66`

Rule [5] of RPC-022 states:

> "The notice fallback handler in handle_slash_command stops surfacing [notice] for these four routes."

The four routes are `/model`, `/thinking`, `/role <text>`, `/role clear`.

`handle_slash_command` currently handles `SlashCommandAction::Model` and `SlashCommandAction::Thinking` explicitly, but `SlashCommandAction::Role` is NOT handled — it falls into the `other =>` arm and produces:

```
[notice] /role not yet implemented in Rust TUI
```

This contradicts rule [5]. When a user opens the slash popup, navigates to `/role`, and presses Enter (before typing a space), they get the legacy notice — exactly the behaviour rule [5] requires removed.

**Fix:** Add `SlashCommandAction::Role` arm to `handle_slash_command` that clears the role (equivalent semantics to the bare `/role` submit path documented in the feature file).

**Test gap:** `rpc022-slash-command-wiring.feature` has a popup-integration scenario for `/model` but none for `/role`. Add a parallel scenario for the popup `/role` path.

---

## 🟡 Warnings

### W1 — Stale architecture note in work unit

**Source:** `fspec show-work-unit RPC-022` architectureNotes[4]

The note references "dispatch_rpc020.rs / dispatch_rpc024.rs / **dispatch_rpc025.rs** / dispatch_rpc026.rs split pattern". The actual feature file doc strings only mention 020, 024, 026 — `dispatch_rpc025.rs` does not exist. Minor inconsistency; harmless but trace-undermining.

---

## 🟢 Observations (Not Fixed — Style/Edge)

1. `dispatch_rpc022.rs` = 299 LoC (1 below the 300 ceiling). Any addition tips it over; future cards must split.
2. Multiple Scenario Outlines are collapsed into single `#[test]` functions with serial asserts. Row-2 failures mask row-3+ failures. Acceptable today; consider `rstest` later.
3. `rpc022-source-shape.feature` uses substring matchers that are weaker than the Gherkin literal text (rustfmt-wrapped vs single-line). Functionally correct.
4. "Existing TS modal dialog files are untouched" scenario only checks `.exists()`. Git log shows no edits since 2026-05-13 (the rule [13] invariant holds in practice).
5. Several feature files place `"""` architecture doc strings between `Feature:` and `Background:` — an fspec convention; `fspec validate` accepts it.

---

## Work Unit Results

### RPC-022: Modal dialogs — FAIL (one critical, one warning, rest observational)

#### Per-feature pass/fail (worker-reported)

| Feature                                       | Status        |
|-----------------------------------------------|---------------|
| rpc022-app-dispatch.feature                   | PASS (warns)  |
| rpc022-cross-transport-parity.feature         | PASS          |
| rpc022-model-selector-dialog.feature          | PASS (warns)  |
| rpc022-role-banner.feature                    | PASS          |
| rpc022-slash-command-wiring.feature           | **FAIL**      |
| rpc022-source-shape.feature                   | PASS (warns)  |
| rpc022-thinking-level-dialog.feature          | PASS          |

#### Build & Test (full crate)
- `cargo test -p codelet-fspec-tui --tests` → all suites PASS (89 + many integration tests).
- File sizes for all new RPC-022 files are under 300 LoC.

---

## Fix Results (post-review)

### RPC-022 — applied fixes
- 🔴 **C1: `/role` popup picker fell through to `[notice]`** → ✅ **Fixed.**
  - Added `SlashCommandAction::Role` arm to `handle_slash_command` in `codelet/fspec-tui/src/app/dispatch_rpc020.rs` (clears the session role via `handle_set_session_role(sid, None)`, matching the bare `/role` submit-line semantics).
  - Added Gherkin scenario "Slash popup selection of /role is treated as a clear and does not surface a [notice]" to `spec/features/rpc022-slash-command-wiring.feature`.
  - Added test `slash_popup_selection_of_role_is_treated_as_a_clear_with_no_notice` to `codelet/fspec-tui/tests/slash_command_wiring_rpc022.rs` (verifies role cleared, backend.set_session_role called with `None`, and zero `[notice] /role` scrollback chunks appended).
  - Coverage linked via `fspec link-coverage`.
- 🟡 **W1: Stale architecture note in work unit ("dispatch_rpc025.rs")** → ❌ **Not a violation.**
  - `dispatch_rpc025.rs` does exist on disk; the worker was mistaken. No fix required.

### RPC-022 — additional issues found during fix verification
- 🟡 **Pre-existing `clippy::redundant_clone = "deny"` violations in new code** → ✅ **Fixed.**
  - `codelet/fspec-tui/src/app/dispatch_rpc022.rs:84` — removed redundant `session_id.clone()` in `ModelSelectorDialog::new(...)`.
  - `codelet/fspec-tui/src/app/dispatch_rpc022.rs:226` — replaced `let sid = session_id.clone()` with `let sid = session_id` (last use, move suffices).
  - Workspace clippy lints include `redundant_clone = "deny"`; the original RPC-022 implementation tripped this when `cargo clippy` was run. Workers only ran `cargo test` and missed it.

---

## Final Verification (post-fix)

- `cargo test -p codelet-fspec-tui --tests` → **560 passed; 0 failed** across 71 test binaries.
- `cargo clippy -p codelet-fspec-tui --tests` → **clean (no warnings or errors)**.
- `cargo build -p codelet-fspec-tui` → **clean**.
- `fspec validate spec/features/rpc022-slash-command-wiring.feature` → valid.
- `fspec show-coverage rpc022-slash-command-wiring` → **100% (10/10 scenarios fully covered)**.
- File sizes for new RPC-022 files (all under 300 LoC):
  - dispatch_rpc022.rs = 299 LoC (at edge)
  - dispatch_rpc020.rs = 172 LoC
  - model_selector_dialog.rs = 272 LoC
  - thinking_level_dialog.rs = 204 LoC
  - role_banner.rs = 140 LoC
