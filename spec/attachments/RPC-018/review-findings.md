# Epic Review: RPC-018 — AgentView SessionHeader + SessionFooter widgets

**Date:** 2026-05-15
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (RPC-018, no children)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 9 across 1 work unit — **9 fixed**
- 🟢 Observations: 5 — out-of-scope (deferred or RPC-022)

All 37 RPC-018 tests pass after fixes (1 redundant test removed). No compile warnings.

---

## Findings — Worker A (Feature / Test Compliance)

### 🟡 Warnings

1. **Coverage test line ranges are systematically off in `app_bootstrap_rpc018.rs`.** Multiple scenarios in `spec/features/rpc018-app-bootstrap.feature.coverage` point at test ranges that don't enclose the actual `#[test]` / `#[tokio::test]` blocks (drift of 10–50 lines). → ✅ **Fixed**: re-linked all 7 scenarios with correct ranges; `audit-coverage` now reports `All mappings valid`.

2. **Coverage test line ranges drift in `view_agent_unit_rpc018.rs`** (off by 2–10 lines for every scenario in `rpc018-agent-chrome.feature.coverage`). → ✅ **Fixed**: re-linked all 9 scenarios.

3. **Coverage test line ranges drift in `agent_chrome_parity_rpc018.rs`** (off by ~5 lines per scenario). → ✅ **Fixed**: re-linked all 9 scenarios.

4. **Feature/test/impl drift on `codelet_git::status::get_current_branch`.** Feature `rpc018-source-shape.feature:70` and architecture note [1] specify the full `codelet_git::status::get_current_branch` path, but `codelet/rpc/src/lib.rs:521` called the short re-exported form and the test at `source_shape_rpc018.rs:96` was loosened to match. → ✅ **Fixed**: updated `rpc/src/lib.rs:521` to use the full path, tightened the test assertion to require the exact substring from the feature step. Source-shape regression test still passes.

5. **Test behavior contradicts the Given clause in `view_agent_unit_rpc018.rs:60-66`.** "Empty AgentViewStore paints placeholder header and bare-cwd footer" said `Given … no workspace snapshot`, but the test set a workspace anyway. → ✅ **Fixed**: removed the `store.set_workspace(...)` call so the test matches the Given. Test still passes (the empty footer assertions don't require a workspace).

6. **Orphan test in `app_bootstrap_rpc018.rs:215-222`.** `thinking_level_loaded_populates_by_session_map` had no corresponding scenario and duplicated coverage already provided by "Action::SessionCreated spawns get_model_info + get_thinking_level fetches". → ✅ **Fixed**: removed the orphan test.

### 🟢 Observations
- All four feature files have a proper triple-quoted doc string, `@RPC-018` tag, and a `Background: User Story` block.
- All RPC-018 tags pass the registry validator (`validate-tags`).
- Three scenarios chain a second `When/Then` cycle within a single scenario. Acceptable Gherkin; not changed.

---

## Findings — Worker B (Rust Implementation Quality)

### 🟡 Warnings

7. **`header.rs:121–159` — `paint_two_columns` carries dead bookkeeping for `budget_left`.** Declared `mut`, mutated after final use, then discarded with `let _ = budget_left;`. → ✅ **Fixed**: dropped the `mut` and deleted the now-redundant final mutate/discard pair.

8. **`footer.rs:73` — `cwd.starts_with(home_str.as_ref())` is a string-prefix check, not a path-segment check.** Would falsely match `$HOME=/Users/rq` against `cwd=/Users/rquast/...`. → ✅ **Fixed**: only substitute `~` when the suffix is empty or starts with `/`.

9. **`agent_view.rs:58` — `context_fill_pct: info.fill_percentage.min(255) as u8`** clamped to `255` instead of the percentage bound. → ✅ **Fixed**: clamped to `100`.

### 🟢 Observations
- `dispatch.rs` does not spawn `get_model_info` / `get_thinking_level` on `Action::EnterWorkUnit` when `current_session` is already set, even though architecture note [7] mentions that path. No feature scenario tests this path; out of strict RPC-018 scope to add untested behaviour.
- `napi/src/session_manager.rs:8642` — `get_model_info` returns `ModelInfo::default()` directly rather than routing through `SessionManagerHandle::get_model_info`. SessionManager doesn't implement the trait; wiring that up is RPC-022 work (per arch note [4]). Deferred.
- `footer.rs` — `shorten_with_home` re-derives `$HOME` on every render frame. Minor perf only; not changed.
- `header.rs:130` — comment says "paint the left in dim grey" but `left_style` uses `Color::White`. Cosmetic; not changed.

---

## Findings — Worker C (Build & Test Verification)

### 🟡 Warning

10. **`view_agent_unit_rpc018.rs:47` — `render_full_buffer` is never used.** Test that needs joined buffer builds it inline. → ✅ **Fixed**: deleted the dead helper. `dead_code` warning gone.

---

## Fix Results

| Issue | File / Location | Resolution |
|-------|-----------------|------------|
| Coverage drift (app-bootstrap)   | `spec/features/rpc018-app-bootstrap.feature.coverage` | 7 scenarios re-linked |
| Coverage drift (agent-chrome)    | `spec/features/rpc018-agent-chrome.feature.coverage`   | 9 scenarios re-linked |
| Coverage drift (parity)          | `spec/features/rpc018-cross-transport-parity.feature.coverage` | 9 scenarios re-linked |
| codelet_git path drift           | `codelet/rpc/src/lib.rs:521` + `source_shape_rpc018.rs:96` | Both use full `status::get_current_branch` path |
| Empty-store test workspace       | `view_agent_unit_rpc018.rs:60-66` | Removed `set_workspace(...)` |
| Orphan thinking_level test       | `app_bootstrap_rpc018.rs:215-222` | Deleted |
| Header `budget_left` dead code   | `views/agent/header.rs:121-159` | `let` instead of `let mut`; final discard removed |
| Footer home prefix path bug      | `views/agent/footer.rs:69-85` | Path-boundary guard added |
| Context-fill clamp to 255        | `store/agent_view.rs:58` | Now clamps to `100` |
| Unused `render_full_buffer`      | `view_agent_unit_rpc018.rs:47-50` | Deleted |

## Final Verification

- `cargo build -p codelet-rpc -p codelet-fspec-tui`: ✅ Clean (no warnings)
- `cargo check -p codelet-rpc -p codelet-rpc-server -p codelet-rpc-embedded -p codelet-fspec-tui --tests`: ✅ Clean
- `cargo test -p codelet-fspec-tui --test view_agent_unit_rpc018 --test app_bootstrap_rpc018 --test agent_chrome_parity_rpc018 --test source_shape_rpc018`: ✅ **37/37 passed**
- `Fspec validate`: ✅ All 903 feature files valid
- `Fspec audit-coverage rpc018-agent-chrome`: ✅ All files found (18/18), all mappings valid
- `Fspec audit-coverage rpc018-app-bootstrap`: ✅ All files found (14/14), all mappings valid
- `Fspec audit-coverage rpc018-cross-transport-parity`: ✅ All files found (18/18), all mappings valid

## Out-of-Scope Items (Logged for Future Cards)

- **EnterWorkUnit with existing current_session** — Arch note [7] mentions spawning `get_model_info` + `get_thinking_level` on this path. No scenario tests it. Defer to a follow-up card that adds the scenario + behavior together.
- **NAPI `get_model_info` route-through-trait** — Arch note [10] aspirational; full wiring requires implementing `SessionManagerHandle for SessionManager` in `codelet/napi`. Deferred to RPC-022 (which already covers ModelSelector + ThinkingLevel modals per arch note [4]).
