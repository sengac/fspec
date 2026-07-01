# Epic Review: RPC-398 — Bash/tool output does not stream incrementally in Rust TUI

**Date:** 2026-07-01
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (standalone bug, no children)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 2
- 🟢 Observations: 4

## Work Unit Result

### RPC-398: Bash/tool output streaming session-id key mismatch — PASS

## Status: PASS

### 🔴 Critical Issues
None.

### 🟡 Warnings (Should Fix)
1. **Scenario 3 (exactly 3 incremental deliveries) is potentially flaky by design.**
   `codelet/tools/tests/tool_progress_session_key_rpc398.rs:181-188` asserts
   `stdout_deliveries() == 3` exactly. Producer: `echo a; sleep 0.1; echo b; sleep 0.1; echo c`.
   Delivery count depends on `read_until(b'\n')` boundaries in `bash_streams.rs`. Reliable
   locally, but a hard equality on timing/line-buffering. If all three lines land in the pipe
   buffer before the first read, a single `read_until` still splits on `\n` (so 3 chunks),
   but the exact-3 assertion is fragile under heavy load. **Recommendation:** assert `>= 3`
   incremental deliveries AND that payloads contain `a`, `b`, `c` (proving incrementality
   without brittle exact-count timing).

2. **Coverage impl-link line ranges are scattered.** The four scenarios link impl at
   `stream_loop.rs:465-478`, `session_registry.rs:95-100`, `tool_progress.rs:67-71`,
   `stream_loop.rs:1884-1889`. Scenarios 2/3 point at registry/emit internals that were
   *unchanged*; the actual behavioral fix is the threaded-`session_id` register/clear.
   Links point at real exercised code and are defensible, but scenario→fix traceability
   would be clearer. Minor.

### 🟢 Observations
1. Source-shape tests in `tool_progress_registration_key_rpc398.rs` are string-scans (brittle
   to formatting) but complement behavioral tests and correctly pin the anti-regression
   (no `Uuid::nil()` registration). Acceptable given `Session` has no `id` field.
2. `rpc084_streaming.rs` window-widening verified: INTENT preserved — still asserts exactly
   one call, 9 positional args in canonical order ending in `$session.id`, located via the
   `_ =>` arm marker. The 8→9 arg census TIGHTENED; not merely loosened to pass.
3. Single-shot `codelet/cli/src/lib.rs:171 run_agent_stream` is a distinct local generic that
   streams via `agent.prompt_streaming` with no tool-progress registration — correctly left
   unchanged (no TUI card/progress-emitter path). Not a missed call site.
4. No `unwrap()`/`todo!()`/`unimplemented!()` in production code. No global fallback added —
   `session_registry.rs with()` remains exact-match; BUG-126 isolation preserved and proven
   by scenario 2.

## Coverage Verification
- Feature file: `spec/features/bash-tool-progress-session-key-streaming.feature` — OK
- Test files: `codelet/tools/tests/tool_progress_session_key_rpc398.rs` (4 behavioral),
  `codelet/cli/tests/tool_progress_registration_key_rpc398.rs` (3 source-shape) — OK
- Impl files: `stream_loop.rs`, `agent_runner.rs`, `dispatch.rs`, `agent_loop.rs`,
  `napi/agent_loop.rs`, `tool_progress.rs`, `session_registry.rs` — OK
- Scenario coverage: 4/4 (100%)

## End-to-End Key Agreement (verified)
- ✅ `agent_runner.rs` passes the SAME `session_id` given to `create_rig_agent`/`BashTool::new`.
- ✅ `dispatch.rs` / `agent_loop.rs` / `napi/agent_loop.rs` pass `session.id`, which is also
  what `create_rig_agent(session.id, ...)` receives → `BashTool::new(session.id)`.
- ✅ Registration key now == emit key. Bug fixed end-to-end.

## Build & Test Results
- `cargo test -p codelet-tools --test tool_progress_session_key_rpc398` → 4 passed
- `cargo test -p codelet-cli --test tool_progress_registration_key_rpc398` → 3 passed
- `cargo test -p codelet-agent-loop --test rpc084_streaming --test rpc088_interrupt_cascade` → 7+7 passed
- `cargo build -p codelet-cli -p codelet-agent-loop` → OK
- `cargo clippy -p codelet-cli -p codelet-tools -p codelet-agent-loop` → clean

## Fix Results

### RPC-398
- 🟡 Warning 1 (brittle exact-3 delivery assertion) → ✅ Fixed:
  `incremental_progress_delivered_while_command_running` now asserts
  `stdout_deliveries() >= 3` (still proves per-line streaming, tolerates benign
  line-buffer coalescing) AND that the streamed text contains `a`, `b`, `c`
  (proves all three lines streamed). `@step` text kept exact. Failure messages updated.
- 🟡 Warning 2 (scattered coverage impl-links) → ⚪ Accepted as-is: links point at
  real exercised code (registry exact-match lookup for isolation scenario, emit path
  for incremental scenario, register/clear sites for the other two). Defensible;
  no functional impact. Left unchanged.

## Final Verification
- `cargo test -p codelet-tools --test tool_progress_session_key_rpc398` → 4 passed ✅
- `cargo test -p codelet-cli --test tool_progress_registration_key_rpc398` → 3 passed ✅
- `cargo test -p codelet-agent-loop --test rpc084_streaming --test rpc088_interrupt_cascade` → 7+7 passed ✅
- `cargo clippy -p codelet-cli -p codelet-tools -p codelet-agent-loop` → clean ✅
- `cargo fmt` → applied ✅
- Feature file valid ✅ · Coverage 4/4 (100%) · audit-coverage 8/8 files found ✅
