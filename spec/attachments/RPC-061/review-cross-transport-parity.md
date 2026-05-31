# Review: RPC-061 (Cross-Transport Parity) — Supervisor / subordinate links surface

## Status: PASS (with warnings)

## 🔴 Critical Issues (Must Fix)
None — All 7 tests pass, all 7 scenarios mapped, build is clean, no `todo!`/`unimplemented!`/`unwrap()` in production paths (embedded.rs / websocket.rs / session_manager_handle.rs production paths are all clean), BFS cycle detection is implemented correctly (codelet/core/src/session_manager_handle.rs:1655-1674), per-method call counters increment in every method (lines 1627, 1640, 1692, 1718, 1726, 1739), and both transports share the same `Arc<StubSessionManagerHandle>` via `SharedFspecService::with_session_manager` (tests/rpc061_cross_transport_parity.rs:40-51).

## 🟡 Warnings (Should Fix)

1. **Test file header references the wrong feature file** — `codelet/fspec-tui/tests/rpc061_cross_transport_parity.rs:4` states `Feature: spec/features/rpc061-supervisor-links.feature`, but the scenarios it actually tests are in `spec/features/rpc061-cross-transport-parity.feature`. The companion `supervisor-links.feature` is a separate file driving App + MockBackend tests. The header should read `Feature: spec/features/rpc061-cross-transport-parity.feature`.

2. **`get_subordinates` "same order" claim is under-asserted** — `tests/rpc061_cross_transport_parity.rs:181-188` claims via @step comment "Then both calls return [SessionId(\"sub-a\"), SessionId(\"sub-b\")] (same order)" but only asserts `em.len() == 2` and `em == ws`. It does NOT verify the contents are actually `[sub-a, sub-b]` nor the seeded order. A `sub-a` / `sub-b` swap by the stub would silently pass. Add `assert_eq!(em, vec![SessionId::new("sub-a"), SessionId::new("sub-b")])`.

3. **`add_supervisor` "two subordinates" assertion does not test order/contents strictly** — `tests/rpc061_cross_transport_parity.rs:111-115` uses `contains` on a `Vec<String>` derived from `SessionId.value`. Functional but less robust than asserting against a sorted/set comparison of `SessionId`s directly.

4. **`@step` comments span multiple lines (continuation lines without `@step` prefix)** — e.g. `tests/rpc061_cross_transport_parity.rs:89-90` and 96-97 wrap the step text onto continuation `//` lines. fspec link-coverage matches only the line containing `@step`. The current step extraction will only capture the first line of the step, dropping the rest of the step text. Either fold each step onto a single `// @step ...` line or accept slight semantic drift between the feature step and the @step comment.

5. **`remove_supervisor` "no subordinates" assertion is only verified between the two calls, not after the WebSocket call** — `tests/rpc061_cross_transport_parity.rs:294-303`: the empty-state assertion at line 295 runs before the WebSocket `remove_supervisor` call; the second (idempotent) call only asserts the call counter. The feature says nothing wrong is happening, but a stronger guarantee would re-assert emptiness after the WebSocket call.

6. **`get_subordinate` scenario has no "And the stub's get_subordinate_calls counter increased by 2" Gherkin step**, yet the test asserts that counter at lines 218-222. This is correct behaviour but the Gherkin scenario at `spec/features/rpc061-cross-transport-parity.feature:43-46` is missing the counter assertion step — recommend adding it for parity with the other scenarios.

## 🟢 Observations (Nice to Have)

1. **`session_manager_handle.rs` is 2240 lines** (well over the 300-LoC ceiling stated in `ClaudeMd`). Pre-existing oversize file, RPC-061 added ~80 lines (873-883 struct fields, 985-994 init, 1225-1260 accessors, 1626-1745 trait impl). Consider splitting `StubSessionManagerHandle` into a sub-module like `stub_session_manager_handle.rs`. The 300-LoC rule in CLAUDE.md is a TypeScript-targeted guideline, but the policy is project-wide.

2. **`embedded.rs` is 754 lines and `websocket.rs` is 1279 lines** — same oversize concern. RPC-061 contributed lines 416-463 and 726-786 respectively, all clean and following the established RPC-037 pattern.

3. **Architecture note `[12]` says MockBackend additions in `codelet/fspec-tui/tests/common/mod.rs`** — out of scope for this feature file but worth flagging that `rpc061-cross-transport-parity.feature` does not use MockBackend (it uses the real `Stub` through both transports), which is exactly correct. The MockBackend additions are exercised by `supervisor_links_rpc061.rs` (companion).

4. **`Aborting on poisoned mutex` semantics** — `session_manager_handle.rs:1741` silently ignores a poisoned `recorded_incoming_messages` lock (`if let Ok(...)`). The receive call still returns `Ok(())` even when nothing was recorded. Test code, low impact, but inconsistent with the `add_supervisor`/`remove_supervisor` `.map_err("... poisoned")` style used in this same file.

5. **`circular_add_supervisor` scenario only asserts the error MESSAGE matches** — does not assert the stub state is unchanged after the rejected call. A regression that mutates state but still returns Err would pass.

## Coverage Verification
- Feature file: `spec/features/rpc061-cross-transport-parity.feature` — **OK** (Given/When/Then ordering is valid; architecture doc string present; `@RPC-061` tag present at line 2; no prefill placeholders)
- Test file: `codelet/fspec-tui/tests/rpc061_cross_transport_parity.rs` — **OK** with WARN: header references wrong feature file (line 4); 7 `#[tokio::test]` functions match 7 scenarios 1:1; all tests pass; @step comments present
- Impl files: `codelet/fspec-tui/src/transport/embedded.rs:416-463`, `codelet/fspec-tui/src/transport/websocket.rs:726-786`, `codelet/core/src/session_manager_handle.rs:871-994 + 1225-1260 + 1626-1745`, `codelet/rpc-types/src/lib.rs:711-725`, `codelet/rpc/src/lib.rs:283-321 + 1262-1325` — **OK** all required additions present
- Scenario coverage: **7/7 scenarios covered**, all line ranges in `rpc061-cross-transport-parity.feature.coverage` verified against actual file content

## Build/Test Output
```
$ cargo test -p codelet-fspec-tui --test rpc061_cross_transport_parity
    Blocking waiting for file lock on artifact directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.84s
     Running tests/rpc061_cross_transport_parity.rs (target/debug/deps/rpc061_cross_transport_parity-f1cf2a77b0e3afd6)

running 7 tests
test receive_incoming_message_round_trips_identically_across_transports ... ok
test get_subordinates_round_trips_identically_across_transports ... ok
test add_supervisor_round_trips_identically_across_transports ... ok
test get_subordinate_round_trips_identically_across_transports ... ok
test get_supervisors_round_trips_identically_across_transports ... ok
test circular_add_supervisor_is_rejected_identically_across_transports ... ok
test remove_supervisor_round_trips_identically_across_transports ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

$ cargo build -p codelet-fspec-tui
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.28s
```

## Files Reviewed
- `/Users/rquast/projects/fspec/spec/features/rpc061-cross-transport-parity.feature`
- `/Users/rquast/projects/fspec/spec/features/rpc061-cross-transport-parity.feature.coverage`
- `/Users/rquast/projects/fspec/codelet/fspec-tui/tests/rpc061_cross_transport_parity.rs`
- `/Users/rquast/projects/fspec/codelet/fspec-tui/src/transport/embedded.rs` (lines 400-499)
- `/Users/rquast/projects/fspec/codelet/fspec-tui/src/transport/websocket.rs` (lines 700-820)
- `/Users/rquast/projects/fspec/codelet/core/src/session_manager_handle.rs` (lines 370-470, 850-1010, 1620-1779)
- `/Users/rquast/projects/fspec/codelet/rpc-types/src/lib.rs` (lines 700-749)
- `/Users/rquast/projects/fspec/codelet/rpc/src/lib.rs` (lines 283-321, 1262-1325)
- RPC-061 work unit metadata via fspec show-work-unit
