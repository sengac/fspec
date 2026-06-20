# RPC-341 — Open on current model: TS-parity review findings

**Date:** 2026-06-20
**Reviewer:** Parallel ACDD review worker (impl-vs-TS comparison)
**Status:** PASS (warnings — spec drift). Cursor seeding faithfully restores TS behavior.

## 🔴 Critical
None. `set_current_model` (`mod.rs:101-103`) → `set_providers` seeds via `rows::index_of_model`
(`rows.rs:145-152`); dispatch order guarantees set_current_model before set_providers
(`dispatch_model_selector.rs:29` then `:43`). Matches TS model-id-only match (`ModelSelectorScreen.tsx:98-109`).

## 🟡 Warnings (Should Fix — spec reconciliation)
1. **Arch notes + feature doc-string are stale.** Arch note [2] / doc-string (`...feature:8`) say
   "keep expand-all behavior here ... mod.rs:474 asserts is_expanded('openai')", but shipped `set_providers`
   (`mod.rs:112-122`) collapses all then expands ONLY the current section (RPC-342 absorbed here). The end
   behavior is correct & more TS-faithful, but the notes contradict the code. **Fix:** reconcile arch note [2]
   and the feature doc-string.
2. **Duplicate-model-id across providers** matched by id only (TS parity) — supervisor's literal "correct provider
   AND model" not satisfied; untested. **Fix:** add a Red card / follow-up if provider data can collide on model id.
3. **Stale line ref** `mod.rs:474` in doc-string no longer holds. **Fix:** update reference.
