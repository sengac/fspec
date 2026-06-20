# RPC-343 — Mid-session model re-resolution: TS-parity review findings

**Date:** 2026-06-20
**Reviewer:** Parallel ACDD review worker (impl-vs-TS comparison)
**Status:** WARN — core fix correct & tested (4/4), but real parity gaps remain.

## 🔴 Critical
None. No `unwrap()/expect()/todo!()/panic!()` in prod path. `set_model` wired end-to-end:
view → `Action::ModelSelected` → `handle_model_selected` → `backend.set_session_model` →
`rpc/src/lib.rs:1055` → `handle.set_model` → `handle_impl.rs:1008`.

## 🟡 Warnings (Must Fix)
1. **Facade override NOT re-resolved on mid-session switch — direct TS parity gap.**
   TS resolves `facade` on every selection (`useModelSelectorState.ts:255,269` → `lookupFacadeOverride`)
   and passes it to the Rust session (`modelSelectionService.ts:110,131`).
   Rust shared resolver hard-codes `None`: `model_resolution.rs:71`
   `pm.set_model_direct(registry_provider, model_part, None, None, None)` — 5th arg `facade_override`.
   `select_model` (cloud path, `manager.rs:437`) never touches `facade_override`.
   Result: after switching to/from a custom/profile model, inner manager's `facade_override`
   (`manager.rs:1152`) is stale.
2. **Reasoning not re-derived and not observable.** `SessionModel` (`handle_impl.rs:195-203`)
   exposes only provider_id/model_id/context_window/max_output_tokens/compaction_threshold —
   no reasoning field. Spec "reasoning re-resolved" claim is neither implemented nor verifiable.
   → Either implement or drop "reasoning" from scope wording.
3. **Undocumented busy-session guard divergence.** `handle_impl.rs:1044-1048` declines switch via
   `try_lock()` → `Err("Session is busy; cannot switch model right now")`. TS has NO such guard
   (`modelSelectionService.ts:93` issues NAPI call unconditionally). Not in any rule/example/note;
   no scenario covers it. → Document as intentional improvement + add a scenario.
4. **Feature doc-string contradicts its own scenarios.** Doc-string + example-map context
   (`mid-session-model-reresolution.feature:10,30-31`) describe the OLD opus→haiku approach
   (claims opus out=32000), but scenarios/tests use cross-family opus→gemini and Rule [9] +
   baseline test (`rpc343_*.rs:67-71`) assert opus `max_output_tokens == 8192`. → Reconcile doc-string.

## Fix plan
- Code: thread facade re-resolution through `apply_model_selection` so profile/custom selections
  re-derive `facade_override` (parity with TS `lookupFacadeOverride`).
- Spec: reconcile doc-string to the gemini cross-family examples; either implement reasoning surfacing
  or remove it from scope; add a scenario + note for the busy-session guard.
