# RPC-346 — Backend custom-model persistence: TS-parity review findings

**Date:** 2026-06-20
**Reviewer:** Parallel ACDD review worker (impl-vs-TS comparison)
**Status:** WARN — write path field-by-field faithful (7/7), two divergences + stale coverage.

## 🔴 Critical
None. camelCase keys, skip-None, add-appends, edit-replace-in-place (order preserved),
delete-drops-`customModels`-when-empty, openai-only guard, missing-profile no-op, unrelated-keys
preserved via whole-file `serde_json::Value` round-trip with `preserve_order`. All match TS.

## 🟡 Warnings (Must Fix)
1. **Numeric narrowing — `value`, `contextWindow`, `maxOutputTokens` are `u32` in Rust vs `number` (f64) in TS.**
   `provider-config.ts:64,103,105` use JS `number`. Rust narrows to `u32` (`profile_sections.rs:104,106,123`).
   READ-side risk: `load_local_server_profiles_from` (`:152-178`) deserializes via
   `serde_json::from_value::<LocalServerProfile>`; a TS-written config with a fractional/negative/`>u32::MAX`
   value fails deserialization and the WHOLE profile is silently dropped (`if let Ok(...)` at `:169` swallows error).
   TS would load it. Backward-compat gap; only integer percentage 80 is tested.
   **Fix:** widen to `u64` (or lenient deserialize) for these fields to match TS `number` width.
2. **JSON output formatting differs.** TS `writeConfig` = `JSON.stringify(config,null,2)` with NO trailing newline
   (`config.ts:119`). Rust appends trailing `'\n'` (`profile_sections.rs:326`). Byte-different files for same config.
   (Harmless for round-trip; note for byte-exactness expectations.)
3. **Stale `.coverage` line ranges.** `custom-model-persistence.feature.coverage` maps tests to `392-414` etc.,
   but actual tests live at `:585-729` (off by ~190 lines, pointing at unrelated RPC-338 code).
   **Fix:** re-link coverage with real ranges. (Impl ranges `244-283`/`285-313`/`315-331` are correct.)

## 🟢 Observations
- `facade` is unconstrained `String` (`:102`) vs TS union; no write-path validation (UI validates upstream).
