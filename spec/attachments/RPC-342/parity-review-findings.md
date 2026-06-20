# RPC-342 — Collapse-by-default: TS-parity review findings

**Date:** 2026-06-20
**Reviewer:** Parallel ACDD review worker (impl-vs-TS comparison)
**Status:** PASS (intentional TS divergences to document + a Gherkin nit).

## 🔴 Critical
None. `set_providers` (`mod.rs:111-141`) replaces expand-all with seed-empty + auto-expand-current,
porting TS default (`useModelSelectorState.ts:148-150` + `ModelSelectorScreen.tsx:93-119`).

## 🟡 Warnings (Should Fix — document as intentional, not "parity")
1. **Title model-count divergence.** TS title counts only models in EXPANDED sections
   (`ModelSelectorView.tsx:128` + `flat-model-list.ts:25`), so TS would show `(0 models)` on open. Rust shows
   grand total across all providers (`mod.rs:212-223`). Intentional (arch note [1], rule [3]). **Fix:** annotate
   the feature as an intentional deviation, not strict parity, so future reviewers don't "fix" it back.
2. **Filter force-expand divergence.** TS filtering does NOT reveal models in collapsed sections; Rust force-expands
   surviving providers during filter (`rows.rs:70`). Intentional better UX (rule [5]). **Fix:** annotate as deviation.
3. **Gherkin nit** — "Reloading providers re-applies the collapse default" precondition `And the model selector has
   loaded the providers with only "openai" expanded` reads as setup; ordering is technically fine. **Fix:** minor wording.
