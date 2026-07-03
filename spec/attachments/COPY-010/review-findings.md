# Review Findings — COPY-010: Text selection anchors at pressed column

**Date:** 2026-07-03 · **Reviewer:** ACDD review worker (review-skill) · **Status: PASS**

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 2
- 🟢 Observations: 3

Bug fix is correct, complete, tested, and traceable end-to-end. 6/6 scenarios covered with verbatim `// @step` comments; tests assert exact OSC 52 bytes; full suite green (0 failed); no new clippy warnings.

## 🟡 Warnings
1. **DRY — whole-line/precise anchor logic duplicated across 4 surfaces.** The `BeginLine` arm (`anchor={row,0}`, `cursor={row,width}`) and the precise `Begin` arm (`anchor=cursor=cell`) are each repeated in four near-identical sites:
   - scrollback_copy.rs:88-95 (`selection_begin_line`) / :78-82 (`selection_begin`)
   - multiline_input_select.rs:120-144
   - turn_modal_select.rs:101-121
   - details_select.rs:91-115
   Recommend shared constructors on `Selection` in src/mouse/selection.rs: `Selection::collapsed(cell)` and `Selection::whole_line(row, width)`, collapsing all 8 sites to one-liners. The dossier itself flagged this duplication (§2). → **FIX (this card).**
2. **components/mod.rs is 1203 lines** — over the 300-line standard. COPY-010 only added the `SelectionBeginLine` variant to an already-oversized, pre-existing file; the source_shape ceiling only enforces `views/agent/`. → **OUT OF SCOPE** for this bug; recommend a separate refactor work unit. Not introduced here.

## 🟢 Observations
1. `long_press → drag` recognizer semantics (tick emits BeginLine; subsequent Drag emits only Extend, anchor stays whole-line) is intentional and pinned by `long_press_then_drag_begins_extends_and_commits`. No action.
2. **Stale doc comments**: turn_modal_select.rs:93-96 and details_select.rs:78-81 still say "Begin anchors the line start… whole line" — that is now `BeginLine`; `Begin` anchors precisely. Code correct, doc text stale. → **FIX (this card).**
3. Zero-width drag correctness path is elegant (collapsed anchor → empty spans → commit early-returns). No action.

## Coverage Verification
100% (6/6). All test+impl links verified pointing at real code. All 20 `// @step` comments match the feature step text exactly.

## Fix plan (this card)
- Add `Selection::collapsed(cell)` + `Selection::whole_line(row, width)` to selection.rs; refactor the 8 anchor sites to use them.
- Correct the stale doc comments in turn_modal_select.rs and details_select.rs.
- components/mod.rs size: documented as out-of-scope / follow-up.

---

## Fix Results (applied)
- 🟡 DRY duplication → ✅ Fixed: added `Selection::collapsed(cell)` (selection.rs:43-47) and `Selection::whole_line(row,width)` (selection.rs:52-56) with unit tests; all 8 anchor sites across the 4 surfaces now use them. Removed newly-unused `Cell` imports.
- 🟢 Stale doc comments → ✅ Fixed: turn_modal_select.rs and details_select.rs apply-gesture docs now describe precise `Begin` vs whole-line `BeginLine`.
- 🟡 components/mod.rs 1203 lines → ⏸️ Out of scope (pre-existing, unrelated to this bug; recommend a dedicated refactor work unit).

## Final Verification
- Full suite: ✅ 0 failed (230 `test result: ok` blocks; COPY-010 6/6 pass)
- Clippy: ✅ no new warnings (only pre-existing repro_env_pause example)
- Coverage: ✅ 100% (6/6), impl line ranges re-tightened after refactor, audit valid
- All touched src/ files <300 lines (max scrollback_copy.rs 296)
