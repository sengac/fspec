# RPC-345 — Tab to Provider Settings: TS-parity review findings

**Date:** 2026-06-20
**Reviewer:** Parallel ACDD review worker (impl-vs-TS comparison)
**Status:** PASS (observations + minor coverage-line drift).

## 🔴 Critical / 🟡 Warnings
None. Bidirectional Tab toggle complete; filter-mode interception works; form/confirm overlays intercept
Tab before the navigation arm (`mod.rs:478-486` returns before `:510`); destination matches TS; 4/4 scenarios covered.

## 🟢 Observations / Minor Fix
1. Reciprocity-asymmetry vs TS: TS flips local state in `AgentView`; Rust routes
   `ModelSelectorEvent::SwitchToProviders` (`mod.rs:48,510`) → Navigator `Action::OpenProviderSettingsView`
   (`navigator_events.rs:80-85`) → `ViewMode::ProviderSettings`. Equivalent; structurally different. No defect.
2. TS Esc-while-filtering clears filter (out of scope for RPC-345; Rust handles in `handle_filter_key`).
3. **Coverage-line drift:** scenario-1 impl range `mod.rs:311-317` does NOT point at the literal `KeyCode::Tab`
   arm (`mod.rs:510`). **Fix:** re-link coverage to the real Tab-arm line range.
