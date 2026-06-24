# PROV-107 — Refactor oversized model_selector / provider_settings megafiles

## Status
Follow-up task from the `provider-settings-parity` epic (recorded at completion of
PROV-101..104). **Card creation only — not yet implemented.** Pre-existing debt,
NOT introduced by the epic.

## Problem (verified by `wc -l`, 2026-06-21)

The project standard is **300 LoC per file** (CLAUDE.md "Keep files under 300
lines"). Current offenders in the touched area:

| File | LoC | Over budget |
|------|-----|-------------|
| `codelet/fspec-tui/src/views/model_selector/mod.rs` | **2132** | +1832 (7x) |
| `codelet/fspec-tui/src/views/model_selector/rows.rs` | **819** | +519 (2.7x) |
| `codelet/fspec-tui/src/views/model_selector/form.rs` | 313 | +13 |
| `codelet/fspec-tui/src/views/provider_settings/mod.rs` | 296 | at edge |
| `codelet/fspec-tui/src/views/model_selector/scroll_tests.rs` | 294 | at edge |

`mod.rs` (2132) and `rows.rs` (819) are the priority targets.

## Why it matters
- Repeated cargo-fmt churn on over-budget files has caused incidental breakage
  across this epic (the RPC-094 `components/mod.rs` LoC budget test; the
  self-inflicted `cargo fmt` reflow that broke `session_manager_shape.rs`).
- Both PROV-105 and PROV-106 will add code to this area; refactoring first (or
  concurrently) reduces the risk of pushing more files over budget.

## Scope
1. **`model_selector/mod.rs` (2132 → < 300 each)**: extract cohesive submodules —
   candidate seams: state/struct definitions, key/event dispatch, rendering,
   filtering, selection/scroll logic. Split inline `#[cfg(test)]` modules via
   `#[path]` siblings (established pattern from RPC-344/RPC-094 fmt detours).
2. **`rows.rs` (819 → < 300 each)**: separate row-building from row-rendering;
   keep the PROV-104 scrollbar-column / full-window-slice logic intact.
3. **`form.rs` (313)** and **`provider_settings/mod.rs` (296)**: trim/extract to
   comfortably under budget.

## Hard requirements (behavior-preserving refactor)
- **ZERO behavior change.** Pure structural moves + re-exports.
- All `codelet-fspec-tui` tests green BEFORE and AFTER (esp. PROV-104
  `scroll_tests` 6/6, model-selector scroll/open-on-current, CRUD form tests).
- clippy `-D warnings` clean; cargo fmt clean (run fmt, then confirm no file
  popped back over budget — split further if so).
- No public-API change visible to `app/dispatch*` callers (keep re-exports).

## Out of scope
- OAuth flows (→ PROV-105), profile CRUD (→ PROV-106).

## Notes
- This is a `task` (operational/refactor) — no feature file required, but tests
  MUST stay green as the safety net.
- **NO git** (user directive). Work directly in the working tree.

## Estimation note
Mechanical but large surface; ~5–8 points. The 2132-line `mod.rs` dominates.
