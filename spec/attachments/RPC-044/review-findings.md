# Review: RPC-044 — Wire codelet-sessions::SessionManager into codelet-fspec build_service

**Date:** 2026-05-22
**Reviewer:** Claude Code (fspec review-skill.md)
**Work Units Reviewed:** 1 (RPC-044 standalone — no children)

## Status: WARN (passes builds and tests; minor traceability defects only)

## Summary
- 🔴 Critical: 0 issues
- 🟡 Warnings: 2 issues
- 🟢 Observations: 3

---

## 🔴 Critical Issues (Must Fix)
None.

## 🟡 Warnings (Should Fix)

### W1 — Stale `Feature:` header reference in all three `no_napi_dependency.rs` tests

The three new test files all carry a doc-comment header referencing a feature
file that does NOT exist on disk:

```rust
//! Feature: spec/features/wire-codelet-sessions-sessionmanager-into-codelet-fspec-common-build-service-add-dependency-rule-regression-tests.feature
```

The actual matching feature file for each test is:

| Test file | Correct feature |
|---|---|
| `codelet/fspec/tests/no_napi_dependency.rs` | `spec/features/codelet-fspec-no-napi-regression.feature` |
| `codelet/fspec-tui/tests/no_napi_dependency.rs` | `spec/features/codelet-fspec-tui-no-napi-regression.feature` |
| `codelet/sessions/tests/no_napi_dependency.rs` | `spec/features/codelet-sessions-no-napi-regression.feature` |

Per project convention (CLAUDE.md test header rule, plus the review-skill C.3
@step traceability requirement) the feature reference must point at the
actual `.feature` file so a reader can follow the trace from test → spec.

### W2 — `@step` comment paraphrases a Gherkin step in three test files

In `no_codelet_napi_import_in_source` (all three crates) the second `@step`
comment reads:

```rust
// @step When I scan every `.rs` file under codelet/<crate>/src/ for codelet_napi references
```

This does NOT appear verbatim in any feature file. Each scenario contains
exactly one `When` step:

```gherkin
When I run `cargo test -p codelet-<crate> --test no_napi_dependency`
```

The review skill (C.4) and the project test-naming convention require @step
text to match the feature step text exactly — not paraphrased.

---

## 🟢 Observations (Nice to Have)

### O1 — One test covers two scenarios
`build_service_wires_session_manager_into_shared_service` carries `@step`
comments for both:
- Scenario "build_service constructs a real SessionManager and passes it via with_session_manager" (line 32)
- Scenario "build_service returns a SharedFspecService whose session-manager handle has been attached" (line 49)

Coverage is achieved (both scenarios linked to lines 565-668) and the rules
are exercised. Splitting into two test functions would give a cleaner 1:1
scenario→test map, but the assertions are correct as-is.

### O2 — Example [6] expects `Arc::strong_count` check; implementation uses behavior check
Example [6] in the example map says:
> The build_service unit test sees Arc::strong_count of the session_manager
> handle inside SharedFspecService increment.

The actual test instead does a `chunks_tx().send(...)` → `chunks_rx().try_recv(...)`
round-trip to prove the broadcast wiring is live. This is a stronger,
black-box behavior assertion that is more reliable than a strong-count check
(strong counts can fluctuate with internal Arc clones). Acceptable
substitution; no fix needed.

### O3 — `common.rs` is over the 300-line guideline
`codelet/fspec/src/common.rs` is 713 lines. This is pre-existing from
RPC-010 / RPC-011 / RPC-025 and is outside the scope of RPC-044 (which only
added ~25 lines plus tests). Out of scope for this card.

---

## Verification Performed

### Coverage
| Feature | Scenarios | Coverage |
|---|---|---|
| `wire-codelet-sessions-into-fspec-build-service.feature` | 3 | 100% |
| `codelet-fspec-no-napi-regression.feature` | 1 | 100% |
| `codelet-fspec-tui-no-napi-regression.feature` | 1 | 100% |
| `codelet-sessions-no-napi-regression.feature` | 1 | 100% |

### Builds
- `cargo build --workspace` — ✅ PASS
- `cargo build -p codelet-fspec` — ✅ PASS

### Tests
- `cargo test -p codelet-fspec` — ✅ all 4 in-crate unit tests + 2 new dep-rule tests pass
  - `build_service_wires_session_manager_into_shared_service` — ✅
  - `fspec_cargo_toml_declares_sessions_dep_and_not_napi` — ✅
  - `build_service_attaches_workspace_cwd` (RPC-017 regression) — ✅
  - `build_service_initializes_global_data_directory_for_persistence` (RPC-025 regression) — ✅
  - `no_codelet_napi_in_transitive_dependency_graph` — ✅
  - `no_codelet_napi_import_in_source` — ✅
- `cargo test -p codelet-fspec-tui --test no_napi_dependency` — ✅ 2/2 pass
- `cargo test -p codelet-sessions --test no_napi_dependency` — ✅ 2/2 pass
- `cargo test -p codelet-sessions --test skeleton_invariants` — ✅ 6/6 pass (RPC-038 regression stays green)
- `cargo test -p codelet-rpc-embedded --test rpc_006_source_shape` — ✅ 6/6 pass (forbidden `rpc → napi` arrow stays absent)

### Example-map rule traceability
| Rule | Scenario | Test | Status |
|---|---|---|---|
| [0] build_service constructs Arc<dyn SessionManagerHandle> | scen 1 (wire feature) | `build_service_wires_session_manager_into_shared_service` | ✅ |
| [1] Cargo.toml adds codelet-sessions, no codelet-napi | scen 2 (wire feature) | `fspec_cargo_toml_declares_sessions_dep_and_not_napi` | ✅ |
| [2] set_data_directory before SessionManager construction | scen 1 (wire feature) | `build_service_wires_session_manager_into_shared_service` (ordering assertion) | ✅ |
| [3] codelet/fspec/tests/no_napi_dependency.rs | codelet-fspec-no-napi feature | new file | ✅ |
| [4] codelet/fspec-tui/tests/no_napi_dependency.rs | codelet-fspec-tui-no-napi feature | new file | ✅ |
| [5] codelet/sessions/tests/no_napi_dependency.rs | codelet-sessions-no-napi feature | new file | ✅ |
| [6] rpc_006_source_shape.rs stays green | (regression — implicit) | `scenario_codelet_rpc_may_depend_on_codelet_core_but_not_on_codelet_napi` | ✅ |
| [7] cargo build --workspace succeeds | (regression — implicit) | manual `cargo build --workspace` | ✅ |

### Files Reviewed
- `spec/features/wire-codelet-sessions-into-fspec-build-service.feature`
- `spec/features/codelet-fspec-no-napi-regression.feature`
- `spec/features/codelet-fspec-tui-no-napi-regression.feature`
- `spec/features/codelet-sessions-no-napi-regression.feature`
- `codelet/fspec/src/common.rs`
- `codelet/fspec/Cargo.toml`
- `codelet/fspec/tests/no_napi_dependency.rs`
- `codelet/fspec-tui/Cargo.toml`
- `codelet/fspec-tui/tests/no_napi_dependency.rs`
- `codelet/sessions/Cargo.toml`
- `codelet/sessions/tests/no_napi_dependency.rs`
- `codelet/rpc/src/lib.rs` (verified `with_session_manager` constructor present)
- `spec/attachments/RPC-044/wire-into-fspec-binary.md`
- `spec/attachments/RPC-044/ast-research-wiring-targets.md`

---

## Fix Results

### W1 — Stale Feature header
Fixed: replaced the bogus header in all three `no_napi_dependency.rs` test
files with the path to the actual matching feature file.

### W2 — Paraphrased @step comment
Fixed: replaced the paraphrased `@step When I scan every `.rs` file...`
comment with the verbatim Gherkin `When` step in all three test files.

### Final Verification
- All tests pass after fix: ✅
- Build succeeds: ✅
- Coverage complete: ✅ (100% on all four features)
- Feature files valid: ✅
- Tags valid: ✅
