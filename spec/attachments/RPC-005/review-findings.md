# Review: RPC-005 — Foundation: dual-transport tarpc service (embedded + WebSocket) sharing one impl with NAPI

**Date:** 2026-05-08 (fresh independent re-review)
**Reviewer:** Claude Code (fspec review skill)
**Parent epic:** RPC-002 (Rust ratatui frontend with dual transport over tarpc)
**Work units reviewed:** 1 (RPC-005 — leaf, no children)

This review was performed from scratch with no reference to prior review findings, focused on whether the work conclusively establishes the architectural foundation that RPC-002 depends on.

## Status: PASS

No critical issues. No warnings. No fixes required.

## RPC-002 alignment audit

The parent epic RPC-002 establishes nine architecture invariants (resolved in 2026-05-08). RPC-005 is the FIRST card under it and is responsible for proving these invariants are reachable end-to-end. Independent verification:

| RPC-002 invariant | Mechanical evidence in RPC-005 |
|---|---|
| ONE source of truth for shared types (`rpc-types`) | `WorkUnitInfo` defined exactly once at `codelet/rpc-types/src/lib.rs:27`. `codelet/napi/src/types.rs:185` is a `pub use codelet_rpc_types::WorkUnitInfo;` re-export. Source-shape test `scenario_8_*` scans every `.rs` file under `codelet/napi/src/` and fails the build if a local `WorkUnitInfo` redefinition appears. |
| ONE source of truth for business logic (shared service impl) | `FspecServiceImpl` defined exactly once in `codelet/rpc/src/lib.rs:67-82`. Both transports clone an `Arc<SharedFspecService>` and pass it to `tarpc::server::BaseChannel::execute`; neither inlines its own copy. Counter test `scenario_4_*` proves both transports hit the same `Arc` instance. |
| Embedded transport accepts host runtime Handle, never spawns its own | `EmbeddedTransport::new(handle, service)` with non-defaulted `tokio::runtime::Handle` argument at `codelet/rpc-embedded/src/lib.rs:42`. Source-shape test `scenario_7_*` scans for forbidden `tokio::runtime::Builder` / `Runtime::new` and fails if any appear. |
| Wire format default = bincode | Wire-tap proxy test `scenario_5_*` records every byte of every binary WS frame during a real `list_work_units` round-trip and asserts each captured frame bincode-decodes as `Envelope::Rpc(_)` and is NOT valid UTF-8 JSON. |
| Reserved envelope variants rejected | `scenario_6_*` injects all 5 reserved variants (Event, LogEvent, WorkUnitsUpdate, CmdReq, CmdRes); `ServerStats::rejected_variants` log captures each name in arrival order; counter `rejected_envelopes` reaches 5; service counter remains 0 (no FspecService method invoked). |
| Both transports reach feature parity | Single RPC `list_work_units` covered by 4 distinct integration tests (embedded happy path, ws happy path, parity equality, parity counter). |
| Both transports produce identical results | `scenario_3_*` calls `list_work_units` through both clients against the SAME `Arc<SharedFspecService>` and asserts `assert_eq!(embedded_result, ws_result)` under `PartialEq`. |
| TS frontend unchanged | Vitest smoke test `src/__tests__/napi-workunitinfo-shape.test.ts` invokes `getAllWorkUnits()` from `codelet/napi/index.js` and asserts camelCase `workType` plus the seven canonical keys. Codelet-napi rebuilds clean (`cargo build -p codelet-napi` succeeds). |
| Test-only fixture; real watcher deferred (RPC-005 rule [10]) | `default_fixture()` lives in the shared service crate `codelet/rpc/src/lib.rs:93-114`. Source-shape test `scenario_10_*` proves `codelet/rpc/Cargo.toml` declares no `codelet-core` or `codelet-napi` dep AND `codelet/rpc/src/` contains no `use codelet_core` / `use codelet_napi` / `work_units_watcher` import. |
| Spike binary minimal (RPC-005 rule [11]) | `codelet/rpc-server/src/main.rs` binds `127.0.0.1:0`, prints port to stdout (single line, flushed), tracing-to-stderr, ctrl_c shutdown only. Exercised end-to-end by `scenario_2_*` which spawns the actual binary via `CARGO_BIN_EXE_codelet-rpc-server`. Source-shape test `scenario_11_*` forbids non-loopback bind literals (`0.0.0.0`, `[::]`, `::0`). |

Every invariant is enforced by either a runtime integration test or a source-shape regression test. No invariant is declarative-only.

## RPC-005 rules audit (rules [0]–[15])

Each rule mapped to its enforcement:

- [0] Four crates as workspace members → `codelet/Cargo.toml` lines 9–13 ✓
- [1] Shared serde types only in rpc-types → enforced by source-shape `scenario_8_*` ✓
- [2] FspecService trait only in rpc → defined once at `codelet/rpc/src/lib.rs:25-28` ✓
- [3] Service impl written ONCE → enforced by `FspecServiceImpl` being the only `impl FspecService for X` in the workspace ✓
- [4] Embedded accepts Handle, no own runtime → enforced by `scenario_7_*` ✓
- [5] WS uses bincode default → enforced by `scenario_5_*` ✓
- [6] Envelope reserved variants → defined in `codelet/rpc-server/src/envelope.rs:26-41`, rejected by `scenario_6_*` ✓
- [7] Both transports tested for every method → 4 integration tests cover the single `list_work_units` method across both paths ✓
- [8] Both produce identical results → `scenario_3_*` ✓
- [9] TS frontend unchanged → Vitest smoke test passes ✓
- [10] Test-only in-memory state → enforced by `scenario_10_*` ✓
- [11] Minimal test-spawnable binary → exercised by `scenario_2_*` ✓
- [12] Lift WorkUnitInfo + define list_work_units → both done with exact field set ✓
- [13] TCP loopback only → enforced by `scenario_11_*` ✓
- [14] Cancellation deferred → no cancellation code present (correct for spike) ✓
- [15] Vitest smoke test → present at `src/__tests__/napi-workunitinfo-shape.test.ts` ✓

## Code quality audit

| Check | Result |
|---|---|
| Production code (`codelet/rpc{,-types,-embedded,-server}/src/`) free of `unwrap()`/`expect()`/`panic!`/`todo!()`/`unimplemented!()` | ✅ zero hits |
| Production code free of TODO/FIXME/HACK/XXX | ✅ zero hits |
| All files under 300-line project limit | ✅ max is `websocket_transport.rs` at 266 lines (test file) |
| Production source files | All ≤120 lines (largest: `pump.rs` at 120) |
| `cargo clippy --all-targets -- -D warnings` for the four crates | ✅ clean |
| Workspace-wide `cargo build -p codelet-napi` after the lift | ✅ clean |
| All `// @step` comments match the corresponding Gherkin step text verbatim | ✅ 31 @step comments cross-checked against 5 feature files |
| Coverage line ranges point at actual code | ✅ verified via `show-coverage` for all 5 features |

## Test verification

```
cargo test -p codelet-rpc-types -p codelet-rpc -p codelet-rpc-embedded -p codelet-rpc-server
  → 11 tests pass:
     - 5 architecture_invariants (scenarios 7–11)
     - 1 embedded_happy_path (scenario 1)
     - 2 parity (scenarios 3–4)
     - 3 websocket_transport (scenarios 2, 5, 6)
npx vitest run src/__tests__/napi-workunitinfo-shape.test.ts
  → 1 test passes (scenario 12)
```

12 scenarios across 5 feature files, 100% coverage, 12 tests, all pass.

## Architectural observations (not issues)

1. **Architecture-invariants tests live in `codelet/rpc-embedded/tests/`** rather than a workspace-level test target. This is unusual organisationally because the file inspects sources of `rpc-server`, `rpc-types`, `napi`, and `rpc` — but it works correctly via `workspace_root()` traversal, the test target is logical (rpc-embedded already has a test crate), and moving it would only add churn. Acceptable.

2. **Double-bincode framing.** The wire layout is `bincode(Envelope::Rpc(bincode(tarpc_message)))` — two layers of bincode, one for the envelope multiplex, one for the tarpc protocol. This is theoretically slightly redundant but cleanly separates envelope multiplexing from RPC framing, which is the right call for the foundation. No fix.

3. **`default_fixture()` is `pub` and used by the rpc-server binary.** The doc-string calls it "test-only" but it's compiled into the production binary. The wording captures intent (rule [10]: test-only-until-real-watcher-is-wired-in) clearly enough; a follow-up card will replace it with the real watcher per rule [10]. No fix needed.

4. **`connect_with_retry` in the test helper is unbounded.** If the listener never accepts, the test would hang until cargo's outer timeout. In practice the listener accepts within milliseconds, so this is benign for now. Could be capped in a future test-helpers card.

None of the above rise to the level of warnings — they are observations of intentional design choices.

## Coverage Verification

- `embedded-transport-rpc.feature` — 1/1 covered
- `dual-transport-parity.feature` — 2/2 covered
- `websocket-transport-rpc.feature` — 3/3 covered
- `rpc-architecture-invariants.feature` — 5/5 covered
- `napi-workunitinfo-shape.feature` — 1/1 covered

**Total: 12/12 scenarios fully covered.**

## Files reviewed (this pass)

Source:
- codelet/Cargo.toml
- codelet/rpc-types/{Cargo.toml,src/lib.rs}
- codelet/rpc/{Cargo.toml,src/lib.rs}
- codelet/rpc-embedded/{Cargo.toml,src/lib.rs}
- codelet/rpc-server/{Cargo.toml,src/{lib.rs,main.rs,envelope.rs,transport.rs,server.rs,client.rs,pump.rs}}
- codelet/napi/{Cargo.toml,src/types.rs (175–186),src/work_units_watcher.rs (1–60)}

Tests:
- codelet/rpc-embedded/tests/{embedded_happy_path.rs,architecture_invariants.rs,source_helpers/mod.rs}
- codelet/rpc-server/tests/{parity.rs,websocket_transport.rs,common/mod.rs}
- src/__tests__/napi-workunitinfo-shape.test.ts

Specs:
- spec/features/{embedded-transport-rpc,dual-transport-parity,websocket-transport-rpc,rpc-architecture-invariants,napi-workunitinfo-shape}.feature
- spec/attachments/RPC-005/rpc-002-feasibility.md (referenced sections 4, 5, 6, 9)
- Work unit definition for RPC-005 (rules [0]–[15], examples [0]–[7], architecture notes [0]–[4])
- Work unit definition for RPC-002 (parent epic, resolved Q1/Q2/Q3/Q5/Q9/Q10)

## Conclusion

RPC-005 conclusively establishes every architectural invariant RPC-002 depends on, with mechanical (test or source-shape) evidence for each. The four-crate dual-transport tarpc architecture is in place, the WorkUnitInfo lift is complete and the TS frontend is preserved, and the spike's deliberate constraints (test-only fixture, loopback-only bind, single RPC, ctrl_c-only shutdown, no streaming, no cancellation) are correctly observed and codified as regression tests so follow-up cards cannot silently regress them.

No fixes applied because none are warranted.
