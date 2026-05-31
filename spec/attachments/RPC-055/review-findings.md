# Review: RPC-055 — /debug debug-capture wiring

**Date:** 2026-05-23
**Reviewer:** Claude Code (fspec review skill)
**Status:** ✅ PASS (no critical issues)
**Scope:** Single work unit (RPC-055 only — not the full RPC-030 epic)

---

## Summary

- 🔴 Critical: **0**
- 🟡 Warnings: **1**
- 🟢 Observations: **1**

All 16 tests across the 3 feature files pass. The TypeScript-free Rust workspace builds clean. Coverage is 100% (16/16 scenarios linked to test + implementation). All 9 example-map rules are reflected in scenarios.

---

## Files Reviewed

### Feature files
- `spec/features/rpc055-slash-debug-dispatch.feature` (9 scenarios)
- `spec/features/rpc055-slash-debug-source-shape.feature` (5 scenarios)
- `spec/features/rpc055-slash-debug-cross-transport-parity.feature` (2 scenarios)

### Test files
- `codelet/fspec-tui/tests/slash_debug_rpc055.rs` (458 LoC)
- `codelet/fspec-tui/tests/source_shape_rpc055.rs` (135 LoC)
- `codelet/fspec-tui/tests/rpc055_cross_transport_parity.rs` (140 LoC)
- `codelet/fspec-tui/tests/common/mod.rs` (MockBackend additions at L471-477, L606-608, L1369-1376, L2091-2106)

### Implementation files
- `codelet/fspec-tui/src/app/dispatch_rpc055.rs` (68 LoC — NEW)
- `codelet/fspec-tui/src/app/dispatch_rpc020.rs` (L116-119 — Debug arm rewired to `handle_slash_debug`)
- `codelet/fspec-tui/src/app/mod.rs` (L34 — `pub mod dispatch_rpc055;`)
- `codelet/fspec-tui/src/views/agent.rs` (L247 — `is_debug_enabled` now reads from store)
- `codelet/fspec-tui/src/transport/mod.rs` (L406-411 — `FspecBackend::set_debug_directory` default impl)
- `codelet/fspec-tui/src/transport/embedded.rs` (L439-444 — Embedded forwarder)
- `codelet/fspec-tui/src/transport/websocket.rs` (L757-765 — WebSocket forwarder)
- `codelet/rpc/src/lib.rs` (L296-299 service decl + L1203-1212 impl routing)
- `codelet/core/src/session_manager_handle.rs` (L399-410 trait default + L626/L696/L756-759 stub counter + L1154-1158 stub override)
- `codelet/fspec-tui/src/app/dispatch_rpc045.rs` (L90-93 — existing DebugStateChange handler, reused)

---

## A. Feature File Compliance — ✅ OK

All three feature files:
- Carry the `@RPC-055` tag.
- Have Background blocks with `As a … I want … So that …` user stories.
- Have architecture doc strings explaining intent and TS-parity references.
- Have correctly ordered Given/When/Then steps (no precondition leaks into Then/And clauses).
- Carry no `[role]`, `[action]`, `[benefit]`, or other prefill placeholders.

---

## B. Example Map Alignment — ✅ OK

Every one of the 9 rules in the example map maps to a Gherkin scenario:

| Rule | Scenario |
|------|----------|
| [0] SessionManagerHandle exposes default `set_debug_directory` | source-shape: "SessionManagerHandle declares set_debug_directory" |
| [1] StubSessionManagerHandle override + per-call counter | cross-transport-parity: both scenarios |
| [2] FspecService declares `set_debug_directory(String)` + routes through `self.inner.session_manager()` | source-shape: "FspecService declares set_debug_directory" + cross-transport-parity scenarios |
| [3] FspecBackend trait + both transports forward | source-shape: "FspecBackend declares" + "Both transports implement" |
| [4] `SlashCommandAction::Debug` dispatch path | dispatch: scenarios 1-3 |
| [5] No current session is silent no-op | dispatch: "/debug with no current session is a silent no-op" |
| [6] FSPEC_DEBUG_DIR env var resolution | dispatch: "/debug honours the FSPEC_DEBUG_DIR environment variable" |
| [7] SessionHeader `[DEBUG]` badge reads from store | dispatch: badge appears/disappears scenarios + DebugStateChange scenario |
| [8] Cross-transport parity | cross-transport-parity: both scenarios |

No unanswered questions remain. Architecture notes [0]–[3] accurately describe the implementation (verified against the source).

---

## C. Test Coverage Compliance — ✅ OK

Coverage report: 16/16 scenarios linked. Every test has `@step` comments that match the Gherkin step text verbatim (including the `→` arrow as `\u{2192}` in success-notice assertions).

Sampled verifications:
- `debug_emits_success_notice_with_resolved_file_path_on_ok` asserts the exact line `[debug] capture toggled → /tmp/debug/s-1/session-x.jsonl` matches what `dispatch_rpc055.rs:49` produces (`format!("[debug] capture toggled \u{2192} {path}")`).
- `debug_emits_error_notice_on_err` asserts `[error] /debug failed: disk full` matches `dispatch_rpc055.rs:50` (`format!("[error] /debug failed: {e}")`).
- `debug_calls_backend_toggle_debug_for_focused_session` asserts `debug_dir == ".fspec/debug"` matches the fallback at `dispatch_rpc055.rs:42-43`.
- `debug_honours_fspec_debug_dir_env_var` flips the env var and asserts `debug_dir == "/custom/path"` reaches the backend, exercising rule [6].
- Cross-transport parity tests use the **same** `Arc<StubSessionManagerHandle>` mounted behind both `EmbeddedFspecBackend` and `WebSocketFspecBackend` (via `bind_and_serve` on `127.0.0.1:0`), then assert the per-stub counter increments by exactly 2 after one call per transport.

Tests are race-protected: the two FSPEC_DEBUG_DIR-sensitive tests serialise on a shared `static FSPEC_DEBUG_DIR_LOCK` mutex (slash_debug_rpc055.rs:41).

---

## D. Implementation Quality — ✅ OK (with 1 observation)

### File sizes
| File | LoC | Limit | Status |
|------|-----|-------|--------|
| `dispatch_rpc055.rs` | 68 | 300 | ✅ |
| `dispatch_rpc020.rs` (post-change) | 267 | 300 | ✅ |
| `slash_debug_rpc055.rs` (test) | 458 | n/a | ℹ️ test file, not a 300-LoC target |

### Pattern parity with RPC-046 / RPC-054
`handle_slash_debug` (dispatch_rpc055.rs:35-55) faithfully follows the established pattern:
1. Early-return if `current_session()` is `None` → no-op (rule [5]).
2. Defensive return if no tokio runtime (mirrors `/clear`).
3. Resolve `debug_dir` from env, falling back to `.fspec/debug`.
4. Clone backend + action_tx into a spawned tokio task.
5. Map `Ok(path)` → `[debug]` notice, `Err(e)` → `[error]` notice.
6. Route through `Action::EmitSessionNotice` → `handle_emit_session_notice` (already wired in RPC-046).
7. Track JoinHandle in `self.pending_tasks`.

### Rust standards
- No `unwrap()` / `expect()` in production code (`dispatch_rpc055.rs` uses `let _ =` for `action_tx.send`).
- No `todo!()` / `unimplemented!()`.
- Type signatures match across layers (`PathBuf` at trait boundary, `String` over the wire — documented in architecture note [0] as a `napi(object)` compatibility constraint).
- `set_debug_directory_calls` uses `AtomicU64` with `Ordering::SeqCst` for counter parity tests.

### TS parity
`dispatch_rpc055.rs:11-16` cites `AgentView.tsx:2643` and acknowledges that the pre-session global toggle path (TS calls `toggleDebug(debugDir)` with no session) is reachable via the new `backend.set_debug_directory(path)` RPC but no slash command currently wires it — explicitly marked out of scope. This matches the description on the work unit.

### Badge rendering
`views/agent.rs:247` now reads `is_debug_enabled` from `store.debug_enabled_for(s).unwrap_or(false)`, replacing the hardcoded `false`. The existing `DebugStateChange` chunk handler at `dispatch_rpc045.rs:90-93` updates the store on every toggle — no new chunk handler was needed. Test `debug_state_change_chunk_updates_badge_state_for_focused_session` (slash_debug_rpc055.rs:421-457) verifies the end-to-end flow.

---

## E. Build & Test Verification — ✅ OK

```
$ cargo build -p codelet-fspec-tui -p codelet-core -p codelet-rpc
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 34.44s

$ cargo test -p codelet-fspec-tui --test slash_debug_rpc055 \
    --test source_shape_rpc055 --test rpc055_cross_transport_parity
test result: ok. 2 passed; 0 failed   (rpc055_cross_transport_parity)
test result: ok. 9 passed; 0 failed   (slash_debug_rpc055)
test result: ok. 5 passed; 0 failed   (source_shape_rpc055)
```

16/16 tests pass clean.

---

## F. Cross-Cutting Concerns

- **DRY:** `handle_slash_debug` shares structure with `handle_slash_clear` and `handle_slash_provider`, but the bodies differ enough (no scrollback reset; different notice texts; different backend method) that factoring would be premature.
- **Security:** `FSPEC_DEBUG_DIR` is read once at slash-command invocation and passed as-is to the backend; the path is not normalised or sandboxed here. The DebugCaptureManager (`codelet-common::debug_capture`) is responsible for sanitising and creating directories. Acceptable separation of concerns — flagged for the manager review, not for RPC-055.
- **Performance:** spawned task is bounded by one round-trip; `pending_tasks` is drained on every dispatch loop iteration. No unbounded growth.

---

## 🟡 Warnings (Should Fix)

### W1. Weak assertion in the no-op scenario test

**File:** `codelet/fspec-tui/tests/slash_debug_rpc055.rs:211-216`

The Gherkin step is:

```gherkin
And no scrollback chunk is appended to any session
```

The test currently asserts only that no sessions exist:

```rust
// @step And no scrollback chunk is appended to any session
assert_eq!(
    app.agent_view_store().open_sessions().len(),
    0,
    "no sessions should exist for the no-op assertion",
);
```

The assertion is functionally correct (with zero sessions, no chunk can be appended), but the step text and the assertion don't match in intent. If a future refactor made `handle_slash_debug` create a phantom session before discovering it shouldn't proceed, the bug would be caught by `open_sessions().len() == 0` rather than by a chunk-count check — but a stronger assertion would iterate any sessions and verify each session's scrollback chunk count is 0. **Severity: low** — does not affect current correctness, and the test does fail loudly on any phantom-session bug.

---

## 🟢 Observations (Nice to Have)

### O1. `try_dispatch_rpc055` is intentional scaffolding

**File:** `codelet/fspec-tui/src/app/dispatch_rpc055.rs:57-67`

```rust
#[allow(dead_code)]
pub(crate) fn try_dispatch_rpc055(&mut self, _action: &Action) -> bool {
    false
}
```

The function exists only to satisfy the source-shape scenario "`/debug` slash command wiring lives in dispatch_rpc055.rs" (rule "And it declares a method named `try_dispatch_rpc055`"). Unlike `try_dispatch_rpc054`, this hook is NOT called from `dispatch.rs` (verified by grep). The rustdoc explicitly documents this as "preserved for symmetry with RPC-054's `try_dispatch_rpc054`".

This is a documented architectural decision — the source-shape test pins the symmetry contract so a future slice that adds dedicated `Action::Debug*` variants has a stable hook to extend. Acceptable as-is.

---

## Coverage Verification

| Feature file | Scenarios | Linked | Status |
|--------------|-----------|--------|--------|
| `rpc055-slash-debug-dispatch.feature` | 9 | 9 | ✅ 100% |
| `rpc055-slash-debug-source-shape.feature` | 5 | 5 | ✅ 100% |
| `rpc055-slash-debug-cross-transport-parity.feature` | 2 | 2 | ✅ 100% |
| **Total** | **16** | **16** | **✅ 100%** |

All linked test line ranges resolve to actual `#[test]` / `#[tokio::test]` functions. All linked implementation line ranges resolve to live (non-comment) code spans.

---

## Verdict

**RPC-055 PASSES review with no critical or blocking issues.** The single 🟡 warning (W1) is a minor test-quality nit that did not affect correctness. The 🟢 observation (O1) is a documented architectural scaffolding decision.

---

## Fix Results

### W1 — Strengthened the no-op test assertion ✅ Fixed

**File:** `codelet/fspec-tui/tests/slash_debug_rpc055.rs:211-233`

Replaced the single weak assertion (`open_sessions().len() == 0`) with a two-step assertion that:
1. Asserts no sessions exist (preserves the phantom-session check).
2. Iterates any open sessions and asserts `session_chunk_count(session_id) == 0` for each, literally matching the Gherkin step "no scrollback chunk is appended to any session".

Verified: all 9 tests in `slash_debug_rpc055`, 5 tests in `source_shape_rpc055`, and 2 tests in `rpc055_cross_transport_parity` continue to pass.

### Coverage line-range refresh ✅ Fixed (collateral)

The test-file edit added 12 lines, shifting every test below the no-op scenario downward. Refreshed all 9 dispatch-feature coverage links to point to the correct test function bodies:

| Scenario | Old range | New range |
|----------|-----------|-----------|
| `/debug calls backend.toggle_debug…` | 95-127 | **100-131** |
| `/debug emits a success notice…` | 129-155 | **137-158** |
| `/debug emits an error notice…` | 157-181 | **164-184** |
| `/debug with no current session…` | 183-216 | **191-234** |
| `/debug only affects the focused session…` | 218-273 | **241-292** |
| `/debug honours the FSPEC_DEBUG_DIR…` | 275-308 | **303-337** |
| `SessionHeader [DEBUG] badge reflects…` | 353-381 | **380-409** |
| `SessionHeader [DEBUG] badge disappears…` | 383-404 | **411-432** |
| `DebugStateChange chunk…` | 410-449 | **438-475** |

`fspec audit-coverage` reports `All mappings valid` for all 3 feature files.

---

## Final Verification

| Check | Result |
|-------|--------|
| `cargo build -p codelet-fspec-tui -p codelet-core -p codelet-rpc` | ✅ Clean |
| `cargo test --test slash_debug_rpc055` | ✅ 9/9 pass |
| `cargo test --test source_shape_rpc055` | ✅ 5/5 pass |
| `cargo test --test rpc055_cross_transport_parity` | ✅ 2/2 pass |
| `fspec validate` | ✅ All 983 feature files valid |
| `fspec audit-coverage rpc055-slash-debug-dispatch` | ✅ All mappings valid (18/18) |
| `fspec audit-coverage rpc055-slash-debug-source-shape` | ✅ All mappings valid (10/10) |
| `fspec audit-coverage rpc055-slash-debug-cross-transport-parity` | ✅ All mappings valid (4/4) |

The work unit returned to `done` status with all post-review fixes applied.
