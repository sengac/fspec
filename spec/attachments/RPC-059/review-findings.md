# Epic Review: RPC-059 — Lift loop store into codelet-core::loops; /loop subcommand handler

**Date:** 2026-05-24
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (RPC-059, no children — parent is RPC-030)

## Summary

- 🔴 Critical: 0 issues
- 🟡 Warnings: 1 issue (fixed)
- 🟢 Observations: Several pre-existing, project-wide concerns documented (not in RPC-059 scope)

## Method

Four parallel sub-reviewer agents were spawned, one per feature file linked to RPC-059:

1. `rpc059-loop-store-lift.feature` — lift `loop_store` into `codelet-core::loops`
2. `rpc059-loop-source-shape.feature` — pin RPC surface source shape across the dual-transport stack
3. `rpc059-loop-cross-transport-parity.feature` — embedded + WebSocket transports land identically on the stub
4. `rpc059-loop-dispatch.feature` — `/loop` parser + dispatch wiring

Each reviewer ran the test suite for its feature, read the implementation files, and verified Gherkin/example-map/test/code alignment.

## Work Unit Results

### RPC-059: Lift loop store into codelet-core::loops; /loop subcommand handler — ✅ PASS (after fix)

#### Feature 1: `rpc059-loop-store-lift.feature` — PASS

- 7/7 scenarios covered
- `cargo test loop_store_lift_rpc059`: 7 passed, 0 failed
- `cargo check -p codelet-core`: PASS
- `cargo check -p codelet-napi`: PASS (re-export shim resolves `crate::scheduler::loop_store::…` paths in `session_bindings.rs`)
- `codelet/core/src/loops/mod.rs` is 281 LoC, zero `use napi` / `napi_derive` references
- `codelet/napi/src/scheduler/loop_store.rs` confirmed deleted
- Re-export shim at `codelet/napi/src/scheduler/mod.rs:27-30` preserves legacy absolute paths

**No issues.**

#### Feature 2: `rpc059-loop-source-shape.feature` — PASS

- 8/8 scenarios covered
- `cargo test source_shape_rpc059`: 8 passed, 0 failed
- `RegisteredLoop` wire type declared at `codelet/rpc-types/src/lib.rs:636` with all 7 fields and `#[cfg_attr(feature = "napi", napi_derive::napi(object))]`
- All trait methods (`loop_add`, `loop_cancel`, `loop_list`) declared with safe-default impls at `codelet/core/src/session_manager_handle.rs:705/719/725`
- Stub counters + seeders at `:1110-1144`
- `FspecService` (tarpc) at `codelet/rpc/src/lib.rs:422-433`
- `FspecBackend` trait at `codelet/fspec-tui/src/transport/mod.rs:602-617`
- Both transports forward to tarpc client (`embedded.rs:667-688`, `websocket.rs:1061-1093`)
- `loop_parser` and `dispatch_rpc059` source shapes pinned

**No issues.**

#### Feature 3: `rpc059-loop-cross-transport-parity.feature` — PASS

- 3/3 scenarios covered
- `cargo test rpc059_cross_transport_parity`: 3 passed, 0 failed (0.06s)
- Embedded + WebSocket transports land on the same `StubSessionManagerHandle` with byte-identical payloads and `*_calls` counter == 2 for each of `loop_add` / `loop_cancel` / `loop_list`

**No issues within RPC-059 scope.**

#### Feature 4: `rpc059-loop-dispatch.feature` — PASS (after fix)

- 21/21 scenarios covered
- `cargo test loop_dispatch_rpc059`: 21 passed, 0 failed
- Parser handles bare `/loop`, `list`, `cancel <id>`, leading `Ns/Nm/Nh/Nd`, trailing `every N <unit>`, default 600s, and minimum-1-second clamp
- Dispatch correctly mirrors the RPC-058 pattern: `try_dispatch_rpc059` catch-all arm, `Action::LoopSubcommandParsed` routing, `Action::EmitSessionNotice` for scrollback, graceful no-op on missing session or missing Tokio runtime

**Issue found (now fixed):**

🟡 **Architecture-note drift — `format_loop_help()` helper missing.** Architecture note [3] explicitly states the dispatch file should contain a `format_loop_help()` formatter alongside the other `format_loop_*` helpers. The original implementation inlined `USAGE_TEXT.to_string()` directly inside `handle_slash_loop_help` at `codelet/fspec-tui/src/app/dispatch_rpc059.rs:41`. All sibling formatters (`format_loop_added`, `format_loop_cancelled`, `format_loop_cancel_missing`, `format_loop_list`, `format_loop_error`) were already in the named-function pattern — `format_loop_help` was the lone exception.

**Fix applied:**

- Added `fn format_loop_help() -> String { USAGE_TEXT.to_string() }` next to the other `format_loop_*` helpers at `dispatch_rpc059.rs:236-238`.
- Updated `handle_slash_loop_help` (line 41) to send `format_loop_help()` instead of `USAGE_TEXT.to_string()`.
- File grew from 231 to 238 LoC, still well under the 300-LoC ceiling.
- All 21 `loop_dispatch_rpc059` tests + 8 `source_shape_rpc059` tests still pass.

## Observations — Pre-existing / Out of RPC-059 Scope

These were surfaced by reviewers but are explicitly **NOT** addressed under RPC-059 (per "no scope creep" instruction):

1. `codelet/core/src/session_manager_handle.rs` is 2003 LoC (project-wide ceiling concern, not RPC-059's job to split).
2. `codelet/rpc/src/lib.rs` is 1636 LoC; `codelet/fspec-tui/src/transport/websocket.rs` is 1216 LoC; `codelet/fspec-tui/src/transport/embedded.rs` is 704 LoC — same comment.
3. `codelet/fspec-tui/src/app/dispatch.rs` and `dispatch_rpc020.rs` are both at 299 LoC (one less than the soft ceiling). Future RPC cards adding new arms will need to refactor — out of scope for RPC-059.
4. `StubSessionManagerHandle::loop_list` returns all seeded loops without filtering by `session_id`. `StubSessionManagerHandle::loop_add` returns the first seeded loop without echoing call args. **Rule [3] explicitly says the stub returns "deterministic in-memory snapshots seeded via seed_registered_loop/seed_registered_loops"** — current behaviour matches the rule. No action needed.
5. `loop_parser.rs` uses `.expect()` on compile-time-constant regex initialisation and `.unwrap_or(1)` after a digit-only regex capture. Acceptable Rust convention for compile-time-constant inputs.
6. Source-shape test `both_transports_implement_loop_methods` uses substring matching for `.loop_add(` etc. Coarse-grained but consistent with the rest of the source-shape suite.

## Fix Results

### RPC-059 — Architecture-note drift on `format_loop_help`

- 🟡 Issue: `format_loop_help()` helper missing — architecture note [3] documents it but `dispatch_rpc059.rs` inlined `USAGE_TEXT.to_string()` instead.
- ✅ Fixed: Added `fn format_loop_help() -> String` formatter and routed `handle_slash_loop_help` through it. All sibling `format_loop_*` formatters now form a consistent named-function family.

## Final Verification

- `cargo build -p codelet-fspec-tui`: ✅ PASS
- `cargo test loop_dispatch_rpc059`: ✅ 21/21 passed
- `cargo test loop_store_lift_rpc059`: ✅ 7/7 passed
- `cargo test rpc059_cross_transport_parity`: ✅ 3/3 passed
- `cargo test source_shape_rpc059`: ✅ 8/8 passed
- `Fspec validate`: ✅ All 997 feature files valid
- All RPC-059 scenarios: ✅ 39/39 covered (7 lift + 8 source-shape + 3 parity + 21 dispatch)

## Summary Table

| Feature                              | Scenarios | Tests       | Status   |
| ------------------------------------ | --------- | ----------- | -------- |
| rpc059-loop-store-lift               | 7/7       | 7 passed    | ✅ PASS  |
| rpc059-loop-source-shape             | 8/8       | 8 passed    | ✅ PASS  |
| rpc059-loop-cross-transport-parity   | 3/3       | 3 passed    | ✅ PASS  |
| rpc059-loop-dispatch                 | 21/21     | 21 passed   | ✅ PASS  |
| **TOTAL**                            | **39/39** | **39 / 0**  | **✅ PASS** |
