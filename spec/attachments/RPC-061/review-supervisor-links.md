# Review: RPC-061 (Supervisor Links) — Supervisor / subordinate links surface

## Status: WARN

## 🔴 Critical Issues (Must Fix)

1. **Rule [9] partially unimplemented: `Action::AttachToSession` does NOT trigger `spawn_load_supervisors`.**
   Rule [9] explicitly says: *"On Action::SessionCreated **and on session activation (Action::AttachToSession or session cycle)**, App::dispatch spawns a backend.get_supervisors task…"*. The implementation only wires it from `Action::SessionCreated` (`codelet/fspec-tui/src/app/dispatch.rs:142`). `handle_attach_to_session` in `codelet/fspec-tui/src/app/dispatch_rpc026.rs:65-117` refreshes chrome (line 89) and hydrates pending input (line 93) but never calls `self.spawn_load_supervisors(session.clone())`. `Action::SessionPrev` / `Action::SessionNext` (`dispatch.rs:229-238`) also do not refresh supervisors. Consequence: re-attaching to an existing session via `/resume` or the resume view will paint a stale (possibly empty) `[Subordinate of: …]` badge.

2. **Example [8] (supervisor chip wins over compaction chip) has NO corresponding scenario.**
   The behaviour is implemented (`codelet/fspec-tui/src/views/agent/footer.rs:63-72` — `if/else if` chain), but `spec/features/rpc061-supervisor-links.feature` has zero scenarios asserting it. This is a Bridge-the-Gap concern explicitly called out in the example map.

3. **Rule [7] (SessionHeader subordinate badge) has NO corresponding scenario.**
   The badge IS rendered (`header_build.rs:97-104`), but no scenario in `rpc061-supervisor-links.feature` exercises the rendered output (e.g., `[Subordinate of: <id>]` text). The closest is the `supervisors_loaded_writes_into_agent_view_store` test which only asserts store state, not rendering. Same for example [0] which describes "the next render shows `[Subordinate of: <s-sup-short>]`" — no render-level scenario.

4. **Rule [8] (SessionFooter pending chip rendering) has NO corresponding scenario.**
   Scenarios only assert `store.supervisor_pending_count_for(…)` returns N; none assert the footer actually paints `[N pending from supervisor]` in yellow. Implementation lives at `footer.rs:81-87` but is unverified by RPC-061 acceptance criteria.

5. **Example [7] (multi-supervisor `+N` format) is NOT covered by any scenario.**
   `format_subordinate_label` in `header_build.rs:113-124` produces `"first8+N"`, but no scenario in `rpc061-supervisor-links.feature` asserts this. Only an inline header-build unit test (if any) might cover it.

6. **Example [1] (no supervisor → no badge) is NOT covered by any scenario.**
   The negative case (`supervisors_for(session) == empty → no badge rendered`) is unverified by acceptance criteria.

## 🟡 Warnings (Should Fix)

1. **Test `send_to_subordinate_err_without_open_session_is_silent_no_op` does not actually assert "no EmitSessionNotice observed".**
   The scenario at `rpc061-supervisor-links.feature:42-46` says *"And no Action::EmitSessionNotice is observed on the action bus"*. The test (`tests/supervisor_links_rpc061.rs:200-209`) only asserts `mock.receive_incoming_message_calls() == 1`. The test even comments at line 208: *"(no open session means no scrollback to emit into; assertion is via call counts)"* — this is **not** a faithful encoding of the @step. There is no probe of `app.try_recv_action()` filtering for `EmitSessionNotice` variants. Coverage line range claim is therefore misleading.

2. **`set_supervisor_pending_count` is effectively dead code in the production path.**
   `supervisor_state.rs:59-65` exposes a setter advertised in the docstring (lines 19-20) as the reset hook for "a fresh count chunk", but `dispatch_rpc045.rs:116-119` (the only chunk handler) ONLY calls `apply_supervisor_pending_injection` (monotonic increment). No production code-path or test calls `set_supervisor_pending_count`. Either wire a reset chunk variant or remove the public method per DRY/YAGNI.

3. **Feature file is missing scenarios for renderable acceptance criteria — feature is dispatcher-only.**
   `rpc061-supervisor-links.feature` (7 scenarios) covers only store/dispatch invariants. The work unit's UX-flavoured examples ([0] last sentence, [2] last sentence, [7], [8]) all describe rendered output but no scenario uses `render_one_frame` / buffer assertions. Compare with `rpc060` or `rpc029` features which do snapshot render output.

4. **`handle_send_to_subordinate` silently swallows the runtime-absence path.**
   `dispatch_rpc061.rs:50-52`: when `tokio::runtime::Handle::try_current().is_err()`, the method returns early WITHOUT calling backend OR emitting a notice. The "silent no-op without an open session" scenario *requires* a tokio runtime (test is `#[tokio::test]`). There is no scenario or test verifying the non-runtime branch, and the early-return is invisible in coverage.

5. **`app/dispatch.rs` is at exactly 298 lines — only 2 lines of headroom.**
   Adding another `try_dispatch_rpcXXX` row in the catch-all OR a single new top-level arm would push it over. Rule [11] (and `rpc024-source-shape.feature`) is satisfied today but fragile. Consider extracting an early helper.

6. **`agent_view.rs` is at 294 lines — also 6 lines from the ceiling.**
   No immediate violation but worth noting since the `supervisors_by_session` + `supervisor_pending_count_by_session` fields (lines 120-121) were added without offsetting any prior fields.

## 🟢 Observations (Nice to Have)

1. **`#![allow(clippy::unwrap_used, …)]` in test file is acceptable** (`tests/supervisor_links_rpc061.rs:14`) — tests are allowed `.unwrap`/`.expect`, but the supervisor scrollback extraction helper at line 64-79 uses `.unwrap_or_default()` cleanly.

2. **`handle_supervisors_loaded` is a tiny shim** (`dispatch_rpc061.rs:33-40`). Could be inlined into `try_dispatch_rpc061`, but factoring is fine for testability.

3. **`spawn_load_supervisors` swallows Err from `backend.get_supervisors`** (`dispatch_rpc061.rs:83`: `if let Ok(supervisors) = …`). On Err it neither logs nor emits a notice. This is an intentional silent-fail per RPC-061's "background snapshot" semantics, but worth a tracing::debug for diagnostics parity with `move_work_unit_*` (dispatch.rs:184).

4. **`format_subordinate_label` correctly handles UTF-8 short IDs** via `.chars().take(8)` (`header_build.rs:118`) — good defensive coding.

5. **`@step` comments in tests track step text faithfully** for the 7 covered scenarios, with one minor caveat: in `send_to_subordinate_err_without_open_session_is_silent_no_op`, the `@step And no Action::EmitSessionNotice is observed on the action bus` is recorded as a comment at line 207 but the assertion immediately below (line 209) only re-asserts the receive count — see Warning 1.

6. **`drain_pending` re-dispatch loop is correctly bounded** (`tests/supervisor_links_rpc061.rs:39-49`): drains `pending_tasks` then drains `try_recv_action`, then re-drains spawned tasks. Good pattern.

## Coverage Verification
- Feature file: `spec/features/rpc061-supervisor-links.feature` — **ISSUE: missing scenarios for examples [1], [7], [8]; rules [7] and [8] (UI rendering) have no render-level scenarios.**
- Test file: `codelet/fspec-tui/tests/supervisor_links_rpc061.rs` — **ISSUE: `send_to_subordinate_err_without_open_session_is_silent_no_op` does not assert "no EmitSessionNotice observed" as the scenario text requires.**
- Impl files: dispatch_rpc061.rs, supervisor_state.rs, dispatch_rpc045.rs (chunk handler), header.rs, header_build.rs, footer.rs, agent_view.rs — **ISSUE: `Action::AttachToSession` and session-cycle paths do not call `spawn_load_supervisors` per rule [9]; `set_supervisor_pending_count` reset path is dead code.**
- Scenario coverage: **7/7 declared scenarios covered by tests** (all pass), but **4 work-unit examples ([1], [7], [8]) + 2 rules ([7] UI, [8] UI) have no scenarios** at all.

## Example Map Gap Analysis
| Example | Gap |
|--------|------|
| [1] No supervisor → no badge | No scenario, no test |
| [7] Multi-supervisor `+N` format `[Subordinate of: s-sup-aa+2]` | No scenario, no integration test (only `format_subordinate_label` unit-testable in header_build) |
| [8] Supervisor chip wins over compaction chip | No scenario, behaviour exists in `footer.rs:63-72` but unverified at acceptance level |
| Rule [7] SessionHeader rendering | No scenario (only store-state scenarios) |
| Rule [8] SessionFooter chip rendering | No scenario (only store-state scenarios) |
| Rule [9] AttachToSession + session-cycle re-load | **Not wired in production code**, no scenario |

(Examples [3], [4], [5], [6] are split across `rpc061-cross-transport-parity.feature` and the source-shape feature — they are out of scope of this review's feature file but in scope of the work unit.)

## File-size Audit
```
     298 codelet/fspec-tui/src/app/dispatch.rs            ← 2 LoC headroom
     107 codelet/fspec-tui/src/app/dispatch_rpc061.rs
     294 codelet/fspec-tui/src/store/agent_view.rs        ← 6 LoC headroom
      77 codelet/fspec-tui/src/store/agent_view/supervisor_state.rs
     178 codelet/fspec-tui/src/views/agent/header.rs
     207 codelet/fspec-tui/src/views/agent/header_build.rs
     264 codelet/fspec-tui/src/views/agent/footer.rs
     297 codelet/fspec-tui/src/views/agent.rs              ← 3 LoC headroom
```
All under 300, but `dispatch.rs`, `agent_view.rs`, and `agent.rs` are uncomfortably close to the ceiling.

## Build/Test Output
```
running 7 tests
test supervisors_loaded_writes_into_agent_view_store ... ok
test two_supervisor_pending_injection_chunks_bump_count_to_two ... ok
test supervisor_pending_injection_chunk_bumps_pending_count ... ok
test send_to_subordinate_err_path_emits_error_notice ... ok
test session_created_spawns_get_supervisors_and_fires_supervisors_loaded ... ok
test send_to_subordinate_spawns_receive_incoming_message ... ok
test send_to_subordinate_err_without_open_session_is_silent_no_op ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```
```
cargo build -p codelet-fspec-tui: Finished `dev` profile [unoptimized + debuginfo] target(s)
```

## Files Reviewed
- spec/features/rpc061-supervisor-links.feature
- codelet/fspec-tui/tests/supervisor_links_rpc061.rs
- codelet/fspec-tui/tests/common/mod.rs (MockBackend supervisor surface @ 1765-1819, 2769-2827)
- codelet/fspec-tui/src/app/dispatch_rpc061.rs (full)
- codelet/fspec-tui/src/app/dispatch.rs (full)
- codelet/fspec-tui/src/app/dispatch_rpc045.rs (chunk handler @ 100-137)
- codelet/fspec-tui/src/app/dispatch_rpc026.rs (handle_attach_to_session @ 60-117)
- codelet/fspec-tui/src/store/agent_view.rs (full)
- codelet/fspec-tui/src/store/agent_view/supervisor_state.rs (full)
- codelet/fspec-tui/src/views/agent/header.rs (full)
- codelet/fspec-tui/src/views/agent/header_build.rs (subordinate_label + format_subordinate_label @ 24, 97-124)
- codelet/fspec-tui/src/views/agent/footer.rs (full)
