# Epic Review: RPC-375 — Rust attachment-viewer feature parity with the TypeScript markdown viewer

**Date:** 2026-06-28
**Reviewer:** Claude Code (fspec review skill, 3 parallel review workers)
**Work Units Reviewed:** 3 children (RPC-376, RPC-377, RPC-378); parent RPC-375 is an umbrella with no feature file of its own.

## Summary
- 🔴 Critical: 0 across all work units
- 🟡 Warnings: 6 actionable (1 in RPC-376, 2 in RPC-377, 3 in RPC-378)
- 🟢 Observations: several (Rust port is strictly more defensive than the TS source in places)

All three work units passed (PASS). Tests pass (44/44), clippy clean, fmt clean, every file < 300 lines, `viewer_template` signature unchanged, axum architecture intact, no production `unwrap()/expect()`.

---

## Work Unit Results

### RPC-376: Heading anchor IDs + render-option parity — PASS
- **W1 (fix):** `smart_punctuation_is_not_applied` — the Gherkin step (feature line 81) asserts the output should contain the literal entity `&#39;`, but pulldown-cmark emits a raw straight apostrophe `'`. The test accepts either branch, so the `&#39;` branch is never exercised. The negative assert (no curly `\u{2019}`) is the real guarantee. **Fix:** tighten the scenario wording to "a straight (non-curly) apostrophe" and align the test @step text + assertion to the actual rendered output.
- W2/W3 (no action): stale explanatory comment referencing `ENABLE_SMART_PUNCTUATION` (option is correctly not set); broad `#![allow]` in the test crate (appropriate).
- Slug table parity verified exactly (summary/summary-1/summary-2, domain-to-tag-mapping-rules, whats-new). Coverage 8/8.

### RPC-377: Prism + copy/badge + theme toggle + font controls — PASS
- **W3 (fix, most important):** Rule [4] "mermaid theme follows dark/default" has **zero test coverage**. The implementation wires `theme: isDark ? 'dark' : 'default'` from `fspec-theme`, but no test asserts it. **Fix:** add a scenario + assertion that the emitted mermaid init derives its theme from `localStorage['fspec-theme']`.
- **W2 (fix):** the Prism language-alias map enumerates 8 mappings in the rule, but tests assert only 2 (`sh→bash`, `ts→typescript`); `shell`, `console`, `js`, `py`, `rb`, `yml`, and the `text→plaintext` special case are untested. **Fix:** extend the alias scenario/test to cover the remaining mappings incl. `text→plaintext`.
- W1/W4 (no action): alias-map wording ("text→plaintext" is a special-case branch, not a map entry) is behaviorally faithful to TS; omitted HTML comments are not asserted anywhere.
- Observations: Rust port adds null-guards the TS lacks and fixes a TS `prefers-color-scheme` paren typo — improvements, kept.

### RPC-378: Fullscreen mermaid modal + Panzoom — PASS
- **W1 (fix):** cursor-centered **wheel zoom** (TS `handleModalWheel`, viewer-scripts.ts:343–479) was not ported — only button zoom exists, no `wheel` listener. The design doc (RPC-378/design.md:24–27,45) explicitly calls for cursor-centered wheel zoom. **Fix:** port cursor-centered wheel zoom (clamped 0.5×–5×).
- **W2 (fix):** horizontal-scroll panning in zoom mode (TS 458–473) and the `showModeIndicator` fade timer (TS 525–539) were not ported. **Fix:** port both to complete the interaction layer.
- **W3 (fix):** dead CSS rule `.diagram-container.pan-mode { cursor: move; }` (modal_styles.rs:123) is never applied by any JS. **Fix:** wire the `pan-mode` class on Space-to-pan (preferred) or remove the rule.
- W4 (no action): `updateZoomLevel` guard divergence is benign/safer.
- Coverage 7/7; all CDN/clamp/Blob constants verified present.

---

## Fix Plan (sequential, ACDD, via worker)
1. RPC-376 → specifying: correct scenario #7 wording + test alignment → re-validate → done.
2. RPC-377 → specifying: add mermaid-theme-follows + full alias-map scenarios/tests → implementing (behavior already present; link coverage) → re-validate → done.
3. RPC-378 → specifying: add wheel-zoom + horizontal-pan + mode-indicator-timer + pan-mode-class scenarios → testing (failing) → implementing → re-validate → done.

---

## Fix Results (all applied via worker, full ACDD)

### RPC-376 — DONE
- 🟡 W1: smart-punctuation scenario reworded via `update-step` to assert a real straight
  apostrophe (`it's` present, `it&#39;s` absent, no curly `\u{2019}`); test `@step` text
  realigned and the "either branch" looseness removed. No production change.
- Coverage 100% (8/8). cargo test/clippy/fmt/validate green.

### RPC-377 — DONE
- 🟡 W3: added scenario "Mermaid theme follows the saved viewer theme" — asserts the emitted
  init reads `localStorage['fspec-theme']` and selects `theme: dark|default`.
- 🟡 W2: added scenario "Prism language aliases map shorthand languages to Prism grammars" —
  asserts shell/console→bash, js→javascript, py→python, rb→ruby, yml→yaml, text→plaintext.
- Behavior already existed; 2 new tests pass. Coverage 100% (9/9, was 7). Green.

### RPC-378 — DONE (real implementation, red→green)
- 🟡 W1: ported cursor-centered **wheel zoom** (clamped 0.5×–5×) — new `wheel` listener +
  `handleModalWheel` with locked zoom-point math.
- 🟡 W2: ported horizontal-scroll panning in zoom mode + `showModeIndicator` fade timer.
- 🟡 W3: wired the previously-dead `.diagram-container.pan-mode` CSS by toggling the
  `pan-mode` class on Space.
- New JS split into `template/mermaid_wheel.rs` (106 lines) to keep files <300. 4 new
  scenarios (failed red first, then green). Coverage 100% (11/11, was 7). Green.

## Final Verification
- attachment-viewer crate: **50 tests pass, 0 fail** (lib 4, markdown_and_path 11,
  markdown_heading_anchors 8, viewer_mermaid_fullscreen 11, viewer_prism_theme_fonts 9,
  viewer_server 7).
- `cargo build` (full workspace): ✅  · `cargo clippy --all-targets`: ✅ 0 warnings ·
  `cargo fmt --check`: ✅
- All viewer source files < 300 lines (max 232).
- `fspec validate` ✅ · feature coverage 100% across all three features.
- `viewer_template(title, content_html)` signature and fspec.pro axum architecture unchanged.
