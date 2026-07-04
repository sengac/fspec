# Epic Review: PROV-125 — Cloud providers show empty model lists (slug ↔ models.dev-id key mismatch)

**Date:** 2026-07-04
**Reviewer:** Claude Code (fspec review skill) + subordinate ACDD reviewer agent
**Work Units Reviewed:** 1 (standalone bug, no children)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 1 (fixed)
- 🟢 Observations: 4

## Work Unit Results

### PROV-125: Cloud providers show empty model lists — WARN → PASS (after fix)

The reported defect is correctly fixed:
- `canonical_to_models_dev` (cloud_models.rs) maps `together→togetherai`,
  `moonshot→moonshotai`, `gemini→google` — all three verified as real keys in
  the live models.dev catalog.
- The miss-swallowing branch now distinguishes known-absent (silent empty) from
  a diagnosable slug/key divergence (`tracing::warn!` then empty).
- Wired end-to-end: `handle_impl.rs list_providers` calls `cloud_model_entries`,
  which populates the user-facing model selector.
- All 12 tests pass (5 new PROV-125 + 7 existing rpc073); build + clippy clean.

#### 🟡 Warnings (found → fixed)
1. **Factually incorrect known-absent entry `github-copilot`.** It was listed in
   `KNOWN_ABSENT_FROM_MODELS_DEV` with a doc comment claiming it is not on
   models.dev. In fact models.dev publishes `github-copilot` (25 tool-call
   models) and the canonical slug equals that key, so it resolves normally when
   credentialed. Keeping it in the set would silently swallow a genuine future
   miss instead of warning. The same error had propagated into rule [3] and an
   architecture note.
   → **✅ Fixed:** Removed `github-copilot` from the set
   (`&["codex", "galadriel"]`), corrected the doc comment, corrected example-map
   rule [3] and architecture note [3], and corrected `investigation.md §5.2`.

#### 🟢 Observations
1. No remaining slug/key divergences among the other canonical providers.
   Cross-checked every `CANONICAL_PROVIDERS` slug against the live models.dev
   keys: `openai, anthropic, cohere, mistral, xai, huggingface, openrouter,
   groq, deepseek, azure, zai` all hit verbatim. Only `gemini, together,
   moonshot` diverge (all mapped); `codex, galadriel` are genuinely absent
   (correctly handled). No further empty-row risk.
2. The `tracing::warn!` emission itself is not asserted (no tracing-capture
   harness in this crate); the observable contract asserted is the empty return.
   Accepted as out-of-scope.
3. `cloud_models.rs` is 161 lines (well under 300); imports clean, no dead code.
4. All 5 scenarios' `@step` comments match the Gherkin step text verbatim.

## Fix Results
### PROV-125
- 🟡 github-copilot mis-classified as known-absent → ✅ Fixed (code + spec artifacts + investigation.md)

## Final Verification
- New + existing tests pass: ✅ (prov125_slug_key_mapping 5/5, rpc073_cloud_model_catalog 7/7)
- Build succeeds: ✅ (`cargo build -p codelet-sessions`)
- Clippy clean: ✅
- Coverage complete: ✅ (5/5 scenarios, test + impl linked; `audit-coverage` 10/10 valid)
- Feature file valid: ✅
- Work unit status: ✅ done
