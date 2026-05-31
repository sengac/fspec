# Review: RPC-054 — /provider ProviderSettingsView + provider-credentials RPC surface

**Date:** 2026-05-23
**Reviewer:** Claude Code (fspec review-skill, 4 parallel workers)
**Feature Files Reviewed:** 4

## Summary
- 🔴 Critical: 0 issues
- 🟡 Warnings: 3 in-scope (and 7 out-of-scope refactor concerns)
- 🟢 Observations: numerous positive findings

All 4 feature files pass `fspec show-coverage` at 100% (16/16 scenarios linked). Build and tests pass.

---

## In-Scope Fixes (Will Apply)

### 🟡 W1 — Three test-file header comments reference the wrong feature file

ACDD convention (TESTING.md) requires the header `Feature:` comment on each test file to point at the feature whose scenarios the test asserts. All three RPC-054 dispatch / parity / source-shape test files copy-paste `rpc054-provider-settings-view.feature` instead.

| File | Line | Current (Wrong) | Correct |
|---|---|---|---|
| `codelet/fspec-tui/tests/provider_settings_dispatch_rpc054.rs` | 3 | `spec/features/rpc054-provider-settings-view.feature` | `spec/features/rpc054-provider-settings-dispatch.feature` |
| `codelet/fspec-tui/tests/rpc054_cross_transport_parity.rs` | 3 | `spec/features/rpc054-provider-settings-view.feature` | `spec/features/rpc054-provider-settings-cross-transport-parity.feature` |
| `codelet/fspec-tui/tests/source_shape_rpc054.rs` | 4 | `spec/features/rpc054-provider-settings-view.feature` | `spec/features/rpc054-provider-settings-source-shape.feature` |

These are non-functional doc comments but the convention is mandatory.

---

## Out-of-Scope Warnings (Documented Only — No Fix in This Review)

The user explicitly requested no scope creep. The following are real concerns but require refactoring beyond what RPC-054 introduced:

### 🟡 OOS-1 — `provider_settings/mod.rs` is 520 lines (>300 LoC guideline)

Suggested split into `mod.rs` (state) + `render.rs` + `keys.rs`. Tests-in-module (lines 373-519) duplicate `tests/provider_settings_view_rpc054.rs` and could be removed. **Deferred.**

### 🟡 OOS-2 — `dispatch_rpc054.rs` at 276 LoC, six near-identical `spawn_*` helpers (DRY)

A `spawn_backend_call<T>(…)` helper could collapse the six tokio-spawn skeletons. **Deferred** — pattern matches RPC-049/050/053 helpers throughout the codebase.

### 🟡 OOS-3 — Extra test `full_credential_lifecycle_round_trips_across_transports`

`codelet/fspec-tui/tests/rpc054_cross_transport_parity.rs:140-213` exercises list/get/delete/refresh round-trips across transports but is not bound to any Gherkin scenario. It exercises rules [1]+[3]+[5] implicitly and is valuable defence-in-depth coverage. **Deferred** — would either need a 3rd scenario in the feature file or a refactor; both are scope creep.

### 🟡 OOS-4 — `#[cfg(test)] mod tests` block in `provider_settings/mod.rs:373-519` duplicates external test file

Same scenarios covered both inline and in `tests/provider_settings_view_rpc054.rs`. Inline draft uses `"sk-1"` while integration uses `"sk-test"`. **Deferred.**

### 🟢 Observations (Positive)

- Security: API key value never reaches a log sink from the view layer; rendered as `•`-masked dots; no `println!`/`tracing::*` of the key string.
- Slash command wiring is clean and traceable: `SlashCommandAction::Provider | Providers` (`dispatch_rpc020.rs:145`) → `Action::OpenProviderSettingsView` → `handle_open_provider_settings_view` (`dispatch_rpc054.rs:23`) → `spawn_list_provider_credentials` + Navigator `apply_action` arm (`navigator.rs:121-123`).
- `WebSocketFspecBackend` actually uses the tarpc client (verified — not a fallback to embedded).
- All `unwrap()`/`panic!`/`expect()` usage in production code: **zero**. All are gated by `#[cfg(test)]` or `#![allow(clippy::unwrap_used)]` in `tests/*.rs`.
- Stub counter naming consistent with RPC-049/050 conventions.
- Error handling consistent: every backend failure path → `tracing::warn!` + `Action::ProviderSettingsStatus("✗ …")`, never pushes to scrollback (matches rule [14]).
- All 16 Gherkin scenarios across 4 features have @step comments matching step text verbatim.
- 100% coverage per `fspec show-coverage` on all 4 feature files.

---

## Coverage Verification (All Feature Files)

| Feature | Scenarios | Coverage | Notes |
|---|---|---|---|
| rpc054-provider-settings-view | 4/4 | 100% | OK |
| rpc054-provider-settings-dispatch | 8/8 | 100% | OK |
| rpc054-provider-settings-cross-transport-parity | 2/2 | 100% | OK (+ 1 extra defence-in-depth test) |
| rpc054-provider-settings-source-shape | 2/2 | 100% | OK |

## Fix Plan

1. Update header comment line in each of the 3 test files (W1).
2. Re-run `cargo test --test provider_settings_dispatch_rpc054 --test rpc054_cross_transport_parity --test source_shape_rpc054`.
3. Re-run `cargo build` on codelet workspace.
4. Re-run `fspec validate` + `fspec validate-tags` + `fspec show-coverage rpc054-*`.

---

## Fix Results

### W1 — Test file header comments updated ✅

- `codelet/fspec-tui/tests/provider_settings_dispatch_rpc054.rs:3` → now reads `Feature: spec/features/rpc054-provider-settings-dispatch.feature`
- `codelet/fspec-tui/tests/rpc054_cross_transport_parity.rs:3` → now reads `Feature: spec/features/rpc054-provider-settings-cross-transport-parity.feature`
- `codelet/fspec-tui/tests/source_shape_rpc054.rs:4` → now reads `Feature: spec/features/rpc054-provider-settings-source-shape.feature`

### Final Verification

- `cd codelet && cargo build` → `Finished dev profile target(s) in 0.42s` ✅
- `cargo test --test provider_settings_dispatch_rpc054 --test rpc054_cross_transport_parity --test source_shape_rpc054 --test provider_settings_view_rpc054`:
  - provider_settings_dispatch_rpc054: 8 passed; 0 failed ✅
  - provider_settings_view_rpc054: 11 passed; 0 failed ✅
  - rpc054_cross_transport_parity: 3 passed; 0 failed ✅
  - source_shape_rpc054: 2 passed; 0 failed ✅
- `fspec validate` → All 980 feature files valid ✅
- All RPC-054 feature files retain the required component (`@rpc`/`@tui`/`@session-management`), feature-group (`@agent-view`/`@provider-settings`), and `@RPC-054` tags. (Project-wide tag violations on 312 unrelated files are pre-existing and out of scope.)
- `fspec show-coverage rpc054-*` → 100% (16/16 scenarios linked across 4 feature files) ✅

All in-scope critical & warning issues resolved. Out-of-scope refactor concerns documented above for follow-up.

