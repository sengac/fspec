# Review: RPC-046 — `/clear` slash command end-to-end

**Date:** 2026-05-22
**Reviewer:** Claude Code (fspec review-skill)
**Status:** WARN (no critical issues; 2 coverage warnings)

## Scope

Single work-unit review (RPC-046 has no children). Coverage of `spec/features/slash-command-clear.feature` (6 scenarios) against `tests/slash_clear_rpc046.rs` and the implementation in `app/dispatch_rpc020.rs::handle_slash_command` (Clear arm) + `app/dispatch_rpc046.rs::handle_emit_session_notice`.

## Summary

- 🔴 Critical: 0
- 🟡 Warnings: 2
- 🟢 Observations: 0

## Verification Performed

| Check | Result |
|-------|--------|
| `cargo build` (codelet/fspec-tui) | ✅ Clean |
| `cargo test --test slash_clear_rpc046` | ✅ 6/6 pass in 0.11s |
| `Fspec validate` on slash-command-clear.feature | ✅ Valid Gherkin |
| `Fspec list-feature-tags` | ✅ All required tags present (@rpc component, @agent-view/@slash-command/@session-management feature-groups, @RPC-046 work-unit, @done lifecycle) |
| File size ceilings (≤300 LoC) | ✅ `dispatch_rpc020.rs` 206, `dispatch_rpc046.rs` 32, `dispatch.rs` 299 |
| @step comments match Gherkin verbatim | ✅ 6/6 scenarios — exact text match |
| Architecture notes match implementation | ✅ `Action::EmitSessionNotice` exists in `components/mod.rs:400`, routed in `dispatch.rs:290`, handled in `dispatch_rpc046.rs:27` |
| Tokio runtime guard present | ✅ `Handle::try_current().is_err()` short-circuit at `dispatch_rpc020.rs:58` |
| Rules ↔ Examples ↔ Scenarios traceability | ✅ Each of the 6 rules has at least one example and at least one scenario covering it |
| No unanswered questions | ✅ Empty question list on work unit |

## 🟡 Warnings (Should Fix)

### W1 — Coverage impl-file linking is incomplete

The implementation spans TWO files: the spawn-and-await arm in `dispatch_rpc020.rs:33–71` and the per-session notice router in `dispatch_rpc046.rs:23–32`. Scenarios that exercise BOTH paths only link ONE file:

| Scenario | Exercises spawn arm (dispatch_rpc020) | Exercises notice router (dispatch_rpc046) | Currently linked |
|----------|---------------------------------------|--------------------------------------------|------------------|
| /clear resets local scrollback synchronously … | ✅ | ❌ | rpc020 ✅ |
| /clear calls backend.clear_history … | ✅ | ❌ | rpc020 ✅ |
| /clear emits a success notice … on Ok | ✅ | ✅ | **rpc020 only ❌** |
| /clear emits an error notice … on Err | ✅ | ✅ | **rpc020 only ❌** |
| /clear with no current session is a silent no-op | ✅ | ❌ | rpc020 ✅ |
| /clear only affects the focused session — background sessions are untouched | ✅ | ✅ | **rpc046 only ❌** |

Action: re-link scenarios 3, 4, 6 so both impl files appear in the coverage record.

### W2 — Test line ranges drift past test-function boundaries

Coverage line ranges for several scenarios don't precisely match the `#[test]` / `#[tokio::test]` function boundaries:

| Scenario | Test fn span | Current coverage | Drift |
|----------|--------------|------------------|-------|
| 1 (sync reset) | 108–124 | 108–126 | +2 trailing |
| 2 (backend called) | 131–150 | 132–150 | −1 leading |
| 3 (success notice) | 158–179 | 157–180 | ±1 |
| 4 (error notice) | 187–206 | 187–208 | +2 trailing |
| 5 (no-op) | 213–241 | 215–241 | −2 leading |
| 6 (background isolation) | 249–304 | 249–300 | −4 trailing |

Action: re-link with corrected ranges so coverage diagnostics point to actual test code.

## Files Reviewed

- `spec/features/slash-command-clear.feature`
- `spec/attachments/RPC-046/slash-clear.md`
- `spec/attachments/RPC-046/ast-research-slash-clear-wiring.md`
- `codelet/fspec-tui/tests/slash_clear_rpc046.rs`
- `codelet/fspec-tui/tests/common/mod.rs` (MockBackend clear_history wiring)
- `codelet/fspec-tui/src/app/dispatch_rpc020.rs`
- `codelet/fspec-tui/src/app/dispatch_rpc046.rs`
- `codelet/fspec-tui/src/app/dispatch.rs`
- `codelet/fspec-tui/src/components/mod.rs` (Action::EmitSessionNotice)
- `codelet/fspec-tui/src/store/agent_view/session_context.rs`
- `codelet/fspec-tui/src/store/agent_view.rs`
- `codelet/fspec-tui/src/app/state.rs`
- `codelet/fspec-tui/src/transport/{mod,embedded,websocket}.rs`

## Fix Results

### W1 — Coverage impl-file linking → ✅ Fixed

Re-linked the three affected scenarios so both impl files appear in the coverage record:

- Scenario 3 (success notice on Ok): + `dispatch_rpc046.rs:27-31`
- Scenario 4 (error notice on Err): + `dispatch_rpc046.rs:27-31`
- Scenario 6 (background isolation): + `dispatch_rpc020.rs:33-71` (previously only had `dispatch_rpc046.rs`)

### W2 — Test line ranges → ✅ Fixed

All six scenarios now point to the precise `#[test]` / `#[tokio::test]` function boundaries:

| Scenario | Old range | New range |
|----------|-----------|-----------|
| 1 (sync reset) | 108–126 | 107–124 |
| 2 (backend called) | 132–150 | 130–150 |
| 3 (success notice) | 157–180 | 157–179 |
| 4 (error notice) | 187–208 | 186–206 |
| 5 (no-op) | 215–241 | 212–241 |
| 6 (background isolation) | 249–300 | 248–304 |

## Final Verification

- `cargo test --test slash_clear_rpc046` — ✅ 6/6 pass in 0.10s
- `Fspec validate` slash-command-clear.feature — ✅ valid
- `Fspec audit-coverage` — ✅ "All files found (18/18) — All mappings valid"
- `cargo build` (codelet/fspec-tui) — ✅ clean
- Work-unit status: `done → implementing → validating → done` (final state restored)
