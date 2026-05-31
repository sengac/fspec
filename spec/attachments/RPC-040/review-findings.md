# Review: RPC-040 — Move SessionManager from codelet-napi into codelet-sessions

**Date:** 2026-05-21
**Reviewer:** Claude Code (fspec review skill)
**Work Unit Status:** done
**Reviewed Against:** spec/skills/review-skill.md

## Status: ✅ PASS (after fix)

All 12 scenarios in `spec/features/move-sessionmanager-from-codelet-napi-into-codelet-sessions.feature`
are implemented and verified by the integration test suite at
`codelet/sessions/tests/session_manager_shape.rs`. Build and clippy both
green for `codelet-sessions` and `codelet-napi`. All 12 shape tests pass.

---

## 🔴 Critical Issues (Must Fix)

None.

## 🟡 Warnings (Should Fix)

1. **(FIXED)** `codelet/sessions/tests/session_manager_shape.rs:239`
   — the `@step` comment for the regex grep was missing the literal
   backslash escape `\[`. Feature step is
   ``napi::|use napi|#\[napi`` (note backslash); the test comment read
   `#[napi`. Per the ACDD reviewer rules, `@step text must EXACTLY match
   the Gherkin step text`.
   - **Fix:** added the missing `\` so the @step comment now reads
     ``napi::|use napi|#\[napi``. The substring scan in the test is
     unaffected (the assertion still uses three direct `contains`
     checks on the raw needles).
   - **Verification:** `diff` of all 84 Gherkin steps against all 84
     `@step` comments returned no differences after the fix.

## 🟢 Observations (Nice to Have)

1. `codelet/sessions/src/session_manager.rs` is 986 lines, exceeding the
   project's 300-line guideline in `CLAUDE.md`. This is the
   explicit out-of-scope concern for this card — the file is a verbatim
   lift of `codelet/napi/src/session_manager.rs:2135-3013` (878 lines)
   plus the new `SessionManagerHooks` trait, `NoopSessionManagerHooks`,
   the four broadcast fields, and their initialization. Splitting the
   module is correctly deferred and is **not part of RPC-040's
   contract**. No action needed.

2. The work unit `description` still cites the old line span
   `3141-4025`. Architecture note `[2]` already documents the drift to
   `2135-3013` and instructs that "all implementation work uses the
   actual current line numbers", so the discrepancy is acknowledged in
   the work unit itself. No action needed.

3. The `pub use codelet_sessions::session_manager::SessionManager`
   re-export in `codelet/napi/src/session_manager.rs:123-124` preserves
   the napi-side import path. The matching `chain_of_command::ChainOfCommand`
   re-export at line 124 makes the pre-existing napi unit tests
   continue to compile against the moved symbol. Both verified by
   `cargo build -p codelet-napi`.

---

## Coverage Verification

- Feature file: `spec/features/move-sessionmanager-from-codelet-napi-into-codelet-sessions.feature`
  — **OK** (validates clean via `fspec validate`)
- Test file: `codelet/sessions/tests/session_manager_shape.rs` — **OK**
  (12 `#[test]` fns, one per scenario, all `@step` comments verbatim
  match feature steps after the warning fix)
- Implementation files:
  - `codelet/sessions/src/session_manager.rs` — **OK** (moved file)
  - `codelet/sessions/src/chain_of_command.rs` — **OK** (lifted module)
  - `codelet/sessions/src/navigation.rs` — **OK** (lifted module)
  - `codelet/sessions/src/credentials/` — **OK** (lifted directory)
  - `codelet/sessions/src/lib.rs` — **OK** (re-exports)
  - `codelet/napi/src/session_manager.rs` — **OK** (re-exports +
    `NapiSessionManagerHooks` impl + `From<IsolatedSessionInfo>` +
    `install_napi_session_manager_hooks()`)
- Scenario coverage: **12/12 scenarios covered (100%)**

---

## Rule-by-Rule Verification

| Rule | Verification | Status |
|------|--------------|--------|
| [0] SessionManager in codelet-sessions | `pub struct SessionManager` exists in `codelet/sessions/src/session_manager.rs:180` | ✅ |
| [1] ChainOfCommand lifted | `codelet/sessions/src/chain_of_command.rs` exists, re-exported via `lib.rs:24` | ✅ |
| [2] No `napi::` references in moved files | grep returns only docstring matches in `session_manager.rs:4-5` (allowed by reviewer convention — comments) | ✅ |
| [3] No `crate::navigation::` to NAPI nav | navigation module lifted to `codelet/sessions/src/navigation.rs`; `crate::navigation::*` paths now resolve to codelet-sessions-internal module | ✅ |
| [4] No `crate::credentials::` to NAPI creds | credentials lifted to `codelet/sessions/src/credentials/`; `crate::credentials::resolve_and_set_env_var` paths now resolve to codelet-sessions-internal module | ✅ |
| [5] Hooks indirection in place | `SessionManagerHooks` trait at `session_manager.rs:87-128`; `NoopSessionManagerHooks` impl at 135-167; all 7 required methods present; call sites use `self.hooks().*` | ✅ |
| [6] Four new broadcast/hooks fields | `chunks_tx`, `logs_tx`, `status_changes_tx`, `hooks` exist at `session_manager.rs:191-197`; initialized in `new()` at 208-224 | ✅ |
| [7] SessionManager::instance() in codelet-sessions | `instance()` at `session_manager.rs:298-302`; napi re-exports it; `install_napi_session_manager_hooks()` at `napi/src/session_manager.rs:6701-6703` | ✅ |
| [8] Build invariants | `cargo build -p codelet-sessions` ✅, `cargo build -p codelet-napi` ✅, `cargo clippy -p codelet-sessions --all-targets -- -D warnings` ✅ | ✅ |
| [9] Pre-existing ChainOfCommand tests pass | Verified by scenario `Pre-existing ChainOfCommand unit tests in codelet-napi still pass via the re-export` (test passes) | ✅ |
| [10] No behavioural changes | `IsolatedSessionResult` stays in NAPI with `From<IsolatedSessionInfo>` impl at `napi/src/session_manager.rs:4250-4258`; `session_manager_create_isolated` maps Ok via `.map(IsolatedSessionResult::from)` and Err via `.map_err(napi::Error::from_reason)` at lines 4242-4248 | ✅ |

---

## Build & Test Verification

```
$ cargo build -p codelet-sessions
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 32.41s
✅ OK

$ cargo clippy -p codelet-sessions --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 26.69s
✅ OK (no warnings)

$ cargo build -p codelet-napi
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 13s
✅ OK

$ cargo test -p codelet-sessions --test session_manager_shape -- --test-threads=1
running 12 tests
test scenario_codelet_napi_still_builds_against_the_re_exported_session_manager_and_chain_of_command ... ok
test scenario_codelet_sessions_builds_standalone_with_session_manager_chain_of_command_navigation_credentials_and_the_hooks_trait_at_their_new_home ... ok
test scenario_codelet_sessions_has_no_transitive_dependency_on_codelet_napi ... ok
test scenario_codelet_sessions_tests_assert_session_manager_is_reachable_from_its_new_home_and_the_hooks_design_is_sound ... ok
test scenario_create_isolated_session_with_id_returns_the_wire_type_isolated_session_info_and_the_napi_wrapper_converts_to_isolated_session_result ... ok
test scenario_create_session_with_id_is_rewritten_to_a_non_napi_result_type_and_routes_side_effects_through_the_hooks ... ok
test scenario_napi_typescript_surface_is_byte_stable_across_the_move ... ok
test scenario_pre_existing_chain_of_command_unit_tests_in_codelet_napi_still_pass_via_the_re_export ... ok
test scenario_session_manager_gains_four_new_fields_beyond_the_original_five ... ok
test scenario_the_moved_session_manager_rs_and_chain_of_command_rs_have_no_napi_references ... ok
test scenario_the_moved_session_manager_rs_has_no_crate_references_to_napi_private_modules_or_free_functions ... ok
test scenario_the_napi_side_installs_its_hooks_at_startup_so_existing_ts_behaviour_is_preserved ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
✅ ALL 12 SCENARIOS PASS
```

---

## Files Reviewed

- `spec/features/move-sessionmanager-from-codelet-napi-into-codelet-sessions.feature`
- `spec/attachments/RPC-040/move-session-manager.md`
- `spec/attachments/RPC-040/ast-research-session-manager-move.md`
- `codelet/sessions/src/lib.rs`
- `codelet/sessions/src/session_manager.rs`
- `codelet/sessions/src/chain_of_command.rs`
- `codelet/sessions/src/navigation.rs`
- `codelet/sessions/src/credentials/mod.rs`
- `codelet/sessions/src/credentials/resolver.rs`
- `codelet/sessions/src/credentials/store.rs`
- `codelet/sessions/src/credentials/types.rs`
- `codelet/sessions/tests/session_manager_shape.rs`
- `codelet/napi/src/session_manager.rs` (sections: lines 114-129 re-exports, 4213-4258 IsolatedSessionResult + impl From, 4290-4314 init path, 6620-6703 NapiSessionManagerHooks)

---

## Fix Results

### RPC-040: Move SessionManager from codelet-napi into codelet-sessions
- 🟡 Warning 1: @step comment missing backslash escape for `#\[napi`
  regex → ✅ **Fixed**: edited
  `codelet/sessions/tests/session_manager_shape.rs:239` to add the
  literal `\` so the comment text now exactly matches the Gherkin step.
  All 84 `@step` comments now diff-clean against the 84 Gherkin steps.
  Test `scenario_the_moved_session_manager_rs_and_chain_of_command_rs_have_no_napi_references`
  still passes after the edit.

## Final Verification

- All RPC-040 shape tests pass: ✅ (12/12)
- `cargo build -p codelet-sessions`: ✅
- `cargo build -p codelet-napi`: ✅
- `cargo clippy -p codelet-sessions --all-targets -- -D warnings`: ✅
- Feature file valid: ✅ (`fspec validate`)
- Coverage complete: ✅ (12/12 scenarios linked)
- All `@step` comments match feature file verbatim: ✅
