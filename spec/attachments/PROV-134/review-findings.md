# Epic Review: PROV-134/135/136 — Provider settings profile fixes

**Date:** 2026-07-04
**Reviewer:** Claude Code (fspec review skill) via 3 parallel worker agents
**Work Units Reviewed:** 3

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 5 (PROV-134: 1, PROV-135: 1, PROV-136: 3)
- 🟢 Observations: several (all benign/documented)

All three build clean, clippy clean, tests green, coverage 100%. No critical issues. Warnings are Gherkin-quality and test-honesty concerns.

## Work Unit Results

### PROV-134 — PASS
🟡 1. Gherkin ordering: each scenario opens Given/When/Then then re-issues a cursor `Given` AFTER a `Then`. The cursor precondition logically belongs before `When I press`. Tests set the cursor before pressing (correct); only the feature reads misleadingly. FIX: reorder so `Given list state` → `Given cursor on header` → `When press` → `Then expanded` → `Then cursor unmoved`, and re-sync @step comments.

### PROV-135 — PASS
🟡 1. Gherkin ordering: scenarios "A field with a typed value…" and "Placeholder hints are never persisted…" have a `Given` after a `Then`. FIX: group all Givens before When/Then; re-sync @step comments.

### PROV-136 — WARN
🟡 1. Gherkin ordering in "Up arrow re-enters the name field in edit mode": Given→When→Then→**Given**→When→Then, AND the second Given ("Given the cursor is focused on the Base URL field") is factually wrong — at that point the cursor is on the NAME field (Up just moved it there). FIX: reword/split so preconditions are accurate and precede actions.
🟡 2. Test `edit_mode_save_emits_original_name_as_old_profile_name` (the crux rename-emit test) maps to NO scenario and borrows persistence-feature @step text that doesn't match what it asserts. FIX: add a scenario "Saving a renamed profile emits the original name" to provider-settings-profile-rename.feature, re-sync the test's @step comments, and link coverage.
🟡 3. The AlreadyExists collision-reject path is only tested at the persistence layer; no dispatch-level test proves the `✗` status surfaces. FIX (optional/lower priority): add a dispatch-level collision test. Decision: covered adequately at persistence layer; add a dispatch guard test as a 🟡 improvement.

## Fix Plan (all worker-driven, ACDD; move units back to specifying/testing as needed)
1. PROV-134: reorder feature steps + @step comments (feature-file + test edit, re-validate, re-link).
2. PROV-135: same.
3. PROV-136: fix feature step ordering/accuracy; add the missing scenario + re-sync/re-link the third test; add a dispatch-level collision test.
