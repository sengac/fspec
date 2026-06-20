# RPC-344 — Custom-model CRUD UI: TS-parity review findings

**Date:** 2026-06-20
**Reviewer:** Parallel ACDD review worker (impl-vs-TS comparison)
**Status:** WARN — functionally faithful (13/13 covered, end-to-end wired), two real divergences.

## 🔴 Critical
None.

## 🟡 Warnings (Must Fix)
1. **Double footer in form/confirm overlays (parity + UX bug).** Rust paints TWO footers when an
   overlay is open: the form hint (last body line) AND the scaffold's stale browse footer pinned at
   the bottom advertising `r Refresh | / Filter | Enter Select | ←→ Expand/Collapse` — keys that do
   NOT work while the form is open.
   - Scaffold hard-codes `rows::FOOTER` for all modes: `model_selector/mod.rs:609`; browse text `rows.rs:30-31`.
   - Form/confirm footers appended in body: `form_render.rs:117-121` (FORM_FOOTER), `:161` (CONFIRM_FOOTER).
   - TS shows a SINGLE footer that fully replaces browse: `ModelSelectorScreen.tsx:228-249` early-returns
     only the form/confirm view; browse footer (`ModelSelectorView`) renders only in the Browse fall-through.
   - **Fix:** pass `FORM_FOOTER`/`CONFIRM_FOOTER` (or empty) to the scaffold in overlay mode and drop the
     duplicate body footer.
2. **Edit prefill silently materializes `displayName = id` (data-write divergence).** Opening Edit on a
   custom model with no stored displayName and saving writes `displayName = "<id>"` — a write TS never makes.
   - Rust prefills display name from `row.label` = wire `ModelEntry.display_name` (`mod.rs:324-326` → `rows.rs:95`).
   - Backend hardcodes `display_name = id` for custom entries (`profile_sections.rs:~356`); `ModelEntry.display_name`
     is non-optional (`rpc-types/src/lib.rs:343`). `build_definition` only nulls when empty (`form.rs:263-273`).
   - TS prefills from the STORED `customDef` (`ModelSelectorScreen.tsx:166,169`) and omits displayName when blank
     (`useCustomModelFormState.ts:111-113`).
   - **Fix:** derive wire `display_name` fallback only at render time (mirror TS `custom.displayName || custom.id`)
     or carry the stored optional through to prefill. Existing `edit_saves_in_place_under_same_id` only covers the
     clear-display-name case → add a test for the "never had displayName" round-trip.

## 🟢 Observations
- Number-field edit UX: Rust stores raw String, parses at build (final value identical; arguably better UX).
- Rule 2 "a only on profile headers" — neither TS nor Rust enforces header-only; both fire on any profile-section row.
