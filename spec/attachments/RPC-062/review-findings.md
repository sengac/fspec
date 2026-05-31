# Review: RPC-062 — MCP injection plumbing in extracted SessionManager

**Date:** 2026-05-25
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (no children — leaf story under RPC-030)

## Status: ✅ PASS

## Summary
- 🔴 Critical: 0 issues
- 🟡 Warnings: 0 issues
- 🟢 Observations: 2

---

## 🔴 Critical Issues (Must Fix)

None.

## 🟡 Warnings (Should Fix)

None.

## 🟢 Observations (Nice to Have)

### 1. `cargo test` is paraphrased as `cargo metadata` in the no-napi-dependency scenario
File: `codelet/sessions/tests/mcp_injection_source_shape.rs:349-433`
Feature: `spec/features/rpc-062-mcp-injection-source-shape.feature:61-65`

The Gherkin step says *"When I run cargo test -p codelet-sessions --test no_napi_dependency"* but the test implementation calls `cargo metadata` and walks the resolve graph directly. The test author documents this in a comment (lines 362–366): recursively invoking `cargo test` from inside a cargo test runner risks deadlocking and is slow. The dependency-graph assertion is semantically equivalent to the no_napi_dependency test's `no_codelet_napi_in_transitive_dependency_graph` scenario — both walk the same `resolve.nodes` set from `cargo metadata`.

This is an acceptable pragmatic trade-off and is **not a defect**. Both `no_napi_dependency.rs` tests independently pass (verified) and any regression would fail BOTH locations. No action required.

### 2. Duplicate work-unit tag casing (`@RPC-062` and `@rpc-062`)
Files: both feature files.

Both feature files carry `@RPC-062` (uppercase work-unit identifier) AND `@rpc-062` (lowercase status-tag variant registered in `spec/tags.json`). This dual-tagging pattern matches the convention used by other RPC-* feature files in the repository (e.g. RPC-006, RPC-010, RPC-022, RPC-041, RPC-057–RPC-059), so it is **consistent with the project**, not a regression for this card. No action required.

---

## Coverage Verification

### Feature 1: `spec/features/rpc-062-mcp-injection-lifecycle.feature`
- Feature file: **OK** — Gherkin syntax valid (`fspec validate` clean), 5 scenarios all well-formed.
- Test file: `codelet/sessions/tests/mcp_injection_lifecycle.rs` — **OK**, 5/5 tests pass.
- Impl file(s): `codelet/tools/src/mcp.rs:820-872` (MCP_SESSIONS registry + init/cleanup/get_mcp_connections) — **OK**, all NAPI-free `pub fn` confirmed.
- Scenario coverage: **5/5 scenarios covered (100%)**.

| Scenario | Test fn | Status |
|---|---|---|
| Idempotent re-init replaces the entry without leaking the previous receiver | `scenario_idempotent_reinit_replaces_entry` (111-152) | ✅ |
| cleanup_mcp_session on an unknown uuid is a silent no-op | `scenario_cleanup_on_unknown_uuid_is_silent_noop` (158-178) | ✅ |
| init_mcp_session registers per-session state and get_mcp_connections returns Some | `scenario_init_registers_state_and_get_mcp_connections_returns_some` (44-77) | ✅ |
| cleanup_mcp_session removes per-session state and get_mcp_connections returns None | `scenario_cleanup_removes_state_and_get_mcp_connections_returns_none` (83-105) | ✅ |
| MCP_SESSIONS registry isolates entries per session uuid | `scenario_registry_isolates_entries_per_session_uuid` (184-225) | ✅ |

### Feature 2: `spec/features/rpc-062-mcp-injection-source-shape.feature`
- Feature file: **OK** — Gherkin syntax valid, 7 scenarios all well-formed.
- Test file: `codelet/sessions/tests/mcp_injection_source_shape.rs` — **OK**, 7/7 tests pass.
- Impl file(s):
  - `codelet/sessions/src/session_manager.rs:58` (McpInjection import) — **OK**
  - `codelet/sessions/src/session_manager.rs:600,604` (init + spawn in create_session_with_id) — **OK**
  - `codelet/sessions/src/session_manager.rs:846,848` (init + spawn in create_isolated_session_with_id) — **OK**
  - `codelet/sessions/src/session_manager.rs:935` (cleanup in destroy_session, after sessions.shift_remove at line 929) — **OK**
  - `codelet/sessions/src/session_manager.rs:79-110` (SessionManagerHooks trait with `mcp_injection_rx: mpsc::Receiver<McpInjection>`) — **OK**
  - `codelet/napi/src/agent_loop.rs:307,424` (consumer side: `mut mcp_injection_rx: mpsc::Receiver<McpInjection>` + `mcp_injection_rx.recv()`) — **OK**
  - `codelet/core/src/session_manager_handle.rs`, `codelet/rpc/src/lib.rs`, `codelet/fspec-tui/src/transport/mod.rs` — **OK** (negative-grep clean: zero `init_mcp|cleanup_mcp|mcp_session|mcp_injection` occurrences)
  - `codelet/sessions/Cargo.toml` (no codelet-napi dependency) — **OK**
- Scenario coverage: **7/7 scenarios covered (100%)**.

---

## Compliance Checklist

### A. Feature File Compliance — ✅
- Given/When/Then ordering is correct across all 12 scenarios.
- No placeholder text (`[role]`, `[action]`, `[benefit]`) detected.
- Architecture doc strings present and accurate on both features.
- `@RPC-062` tag present on both files.
- The "And calling cleanup_mcp_session afterwards…" tail step in the idempotent re-init scenario combines a small follow-up action with its assertion. This is a conventional BDD shorthand (single scenario verifying a complete lifecycle) and is intentional, not a defect.

### B. Example Map Alignment — ✅
- All 10 rules in the example map map to scenarios:
  - Rule [0] (McpInjection import) → source-shape scenario 1
  - Rules [1] [2] (init in both create paths + spawn_agent_loop) → source-shape scenario 2
  - Rule [3] (cleanup in destroy_session) → source-shape scenario 3
  - Rule [4] (SessionManagerHooks signature) → source-shape scenario 4
  - Rules [5] [6] (init/cleanup NAPI-free public functions + registry behavior) → lifecycle scenarios 3, 4, 5
  - Rule [7] (singleton-per-session) → lifecycle scenario 1
  - Rule [8] (no MCP in RPC surface) → source-shape scenario 6
  - Rule [9] (four call-site pin) → source-shape scenarios 1–4
- All 10 examples map to corresponding scenarios.
- Zero unanswered questions remain (work unit `rules`/`examples`/`architectureNotes` only; no `questions` field).
- Architecture notes match the implementation approach.

### C. Test Coverage Compliance — ✅
- Every Gherkin scenario has a corresponding test function.
- Every Gherkin step has a `// @step` comment in the test file matching the step text **verbatim** (spot-checked across all 12 scenarios — no paraphrasing).
- Tests verify actual behavior (drop semantics on `rx_first.recv() → None`, registry isolation across two uuids, etc.) — no trivial assertions.
- Coverage links verified via `fspec show-coverage` — all line ranges point to live test/impl code.

### D. Implementation Quality — ✅
- **SOLID:** No production-code edits in this card (per Architecture Note [F] — this is an audit-only landing). Existing call sites already satisfy single-responsibility.
- **DRY:** Source-shape test helpers (`strip_rust_comments`, `read`, `extract_fn_body`) mirror the pattern from `handle_impl.rs` per Architecture Note [C]. The pattern is duplicated rather than extracted to a shared helper — this is consistent with RPC-042/RPC-044 source-shape tests in the same crate (intentional, isolated per-test pattern).
- **No TODO/FIXME/HACK/unimplemented!/todo!:** grep clean across the two test files.
- **Wired up end-to-end:** `init_mcp_session` is called in both create paths, the receiver is forwarded via `spawn_agent_loop`, and the NAPI side's `agent_loop` selects on `mcp_injection_rx.recv()` (verified at `codelet/napi/src/agent_loop.rs:307,424`).
- **Type safety:** Rust code; `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` scoped to test files (acceptable for source-shape and lifecycle tests where panics are the failure mode).
- **Error handling:** Lifecycle test uses `tokio::time::timeout` to bound `rx_first.recv()` so the test cannot hang; cleanup is unconditionally called at the end of each test to keep the process-global `MCP_SESSIONS` registry hygienic.
- **File size:** 228 lines (lifecycle) + 434 lines (source-shape). Source-shape file exceeds the 300-line TypeScript guideline, but the AGENT_GUIDELINES rule explicitly applies to TypeScript ("Keep files under 300 lines"); Rust integration tests in this crate routinely exceed this (`background_session_shape.rs` is 49926 bytes, `session_manager_shape.rs` is 77891 bytes, `handle_impl.rs` is 32992 bytes), and the size here is driven by exhaustive scenario coverage. No refactor required.

### E. Build & Test Verification — ✅
- `cargo test -p codelet-sessions --test mcp_injection_lifecycle --test mcp_injection_source_shape` → 12/12 pass (`/tmp/rpc062-test.log`).
- `cargo test -p codelet-sessions --test no_napi_dependency` → 2/2 pass (`/tmp/rpc062-no-napi.log`) — the companion test the scenario 7 in source-shape pins.
- `cargo build -p codelet-tools -p codelet-sessions -p codelet-napi -p codelet-core` → clean (`/tmp/rpc062-fullbuild.log`).
- `fspec validate` → 1003/1003 feature files valid.

### F. Cross-Cutting Concerns — ✅
- No security concerns (no untrusted input handling).
- No performance concerns (lifecycle tests bounded by `Duration::from_millis(20)` and `100`).
- The negative-grep scenario actively prevents future leakage of MCP methods into the RPC surface — this is a regression-protection win.
- The `cargo metadata` walk in scenario 7 actively prevents NAPI dependency creep into `codelet-sessions`.

---

## Files Reviewed
- `spec/features/rpc-062-mcp-injection-lifecycle.feature`
- `spec/features/rpc-062-mcp-injection-source-shape.feature`
- `codelet/sessions/tests/mcp_injection_lifecycle.rs`
- `codelet/sessions/tests/mcp_injection_source_shape.rs`
- `codelet/sessions/src/session_manager.rs` (lines 58, 79–110, 117–124, 600–604, 846–848, 920–940)
- `codelet/tools/src/mcp.rs` (lines 820–897)
- `codelet/napi/src/agent_loop.rs` (lines 307, 424, function signatures)
- `codelet/sessions/tests/no_napi_dependency.rs` (companion verification)
- `spec/attachments/RPC-062/mcp-injection.md`
- `spec/attachments/RPC-062/ast-research-mcp-injection-wiring.md`

---

## Fix Results

**No fixes required.** RPC-062 passes all ACDD compliance checks. The card was a pure audit + test-landing (Architecture Note [F]) and the audit objectives are fully satisfied:

1. ✅ MCP injection wiring is correctly preserved through the SessionManager extraction.
2. ✅ Four call sites pinned by source-shape tests (import + 2 init + 1 cleanup + trait signature + consumer signature).
3. ✅ Five lifecycle behaviors pinned by runtime tests against the process-global `MCP_SESSIONS` registry.
4. ✅ Zero MCP leakage into the RPC surface (handle, service, backend) — actively enforced by negative-grep.
5. ✅ codelet-sessions has zero transitive codelet-napi dependency — actively enforced by cargo-metadata walk.

## Final Verification
- All tests pass: ✅ (12/12 RPC-062 tests + 2/2 companion no_napi_dependency tests)
- Build succeeds: ✅ (`cargo build` clean across tools/sessions/napi/core)
- Coverage complete: ✅ (5/5 + 7/7 = 100%)
- Feature files valid: ✅ (`fspec validate` clean across 1003 files)
- Tags valid: ✅ (all 16 tags across both features are registered in `spec/tags.json`)
- Work unit status: **done** (no transition needed)
