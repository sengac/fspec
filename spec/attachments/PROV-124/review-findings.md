# PROV-124 — ACDD Review Findings

**Reviewer:** independent review worker (fspec review-skill)
**Status:** WARN — 0 critical, 2 warnings, 4 observations
**Core fix verdict:** CORRECT. All four navigation methods (move_up/move_down/
page_up/page_down) set has_selection=true then clamp-move on the same press;
adjust_scroll runs on every path; has_selection still gates Enter only
(dispatch.rs Enter path unchanged, still no-ops on a model row when
has_selection=false); RPC-341 seed-on-current-model and PROV-101 Enter-no-op
intact; build passes; 5/5 first_press_nav tests pass; coverage 5/5.

## 🟡 Warnings (fixed)

1. **Misplaced/duplicated `@step` in scenario 4 test**
   (`tests_first_press_nav.rs:165`). The Gherkin scenario "Enter before any
   navigation is a no-op on a model row" has exactly ONE `And no selection is
   active` step — a Then-side step (feature line 62). The test emitted
   `// @step And no selection is active` twice: once at line 165 as a Given-side
   precondition and again at line 184 as the correct Then-side assertion. The
   line-165 occurrence reused a Then-side step's text in the wrong Gherkin
   position. FIX: relabel the precondition at 165 as a plain precondition
   comment (not an `@step`); keep the single Then-side `@step` at 184.

2. **PROV-101 reconciliation not documented on the work unit.** The
   reconciliation of `model-selector-no-auto-select.feature` (no contradictory
   scenario survives; no stale test encodes the old buggy behaviour) was
   verified but not recorded. FIX: add an assumption note to PROV-124.

## 🟢 Observations (no action)

1. Scenario 4's constructed state is semi-artificial but a defensible guard for
   the dispatch.rs Enter-on-selectable-row-with-has_selection=false branch.
2. Mouse-wheel path (handle_mouse → move_up/move_down) benefits automatically;
   wheel now moves on the first tick. No regression.
3. DRY is good — all four methods share the clamp helpers; no duplicated logic;
   no unwrap/expect/todo added to navigation.rs.
4. TS-parity claims verified accurate against move_up_clamped/move_down_clamped.
