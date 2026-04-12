# Epic Review: BUG-126, BUG-127, BUG-128, BUG-129 — Session Isolation Bugs

**Date:** 2026-04-12
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 4

## Summary
- 🔴 Critical: 1 issue across 1 work unit (BUG-129 bash.rs 960 lines)
- 🟡 Warnings: 5 issues across 3 work units
- 🟢 Observations: 8

## Fix Results

### BUG-126: TOOL_PROGRESS_CALLBACK — ✅ PASS
- 🟡 Warning: Several handler modules still use raw `Lazy<RwLock<HashMap>>` instead of `SessionRegistry` → Noted for follow-up
- 🟡 Warning: `stream_loop.rs`/`gemini_continuation.rs`/`compaction_retry.rs` use `Uuid::nil()` for CLI path → Design gap, not a BUG-126 regression (follow-up ticket recommended)
- No code changes needed for BUG-126 itself

### BUG-127: PAUSE_HANDLER — ✅ PASS
- 🟡 Warning: Stale "global" comments in `stream_loop_pause_test.rs` and `pause_integration_test.rs` → ✅ Fixed: Updated all comments to say "per-session"
- 🟡 Warning: Architecture doc string line numbers slightly stale → Noted (cosmetic, lines drift with edits)

### BUG-128: BRIDGE_HANDLER — ✅ PASS (was WARN)
- 🟡 Warning: Stale module doc "Provides a global handler" → ✅ Fixed: Changed to "Provides per-session handlers"
- 🟡 Warning: `BRIDGE_SESSION_CONTEXTS` uses raw `RwLock<HashMap>` instead of `SessionRegistry` → ✅ Fixed: Refactored to use `SessionRegistry<Arc<BridgeSessionContext>>`
- 🟡 Warning: File 451 lines (exceeds 300 limit) → ✅ Fixed: Extracted inline tests to `bridge_handler_unit_test.rs`, file now 290 lines
- 🟡 Warning: `handle_bridge_action` mixed concerns → ✅ Fixed: Extracted into private helper functions (`handle_connect`, `handle_disconnect`, `handle_list`)

### BUG-129: BASH_ABORT_FLAG — ✅ PASS (was WARN)
- 🔴 Critical: `bash.rs` was 960 lines (3× the 300-line limit) → ✅ Fixed: Split into 5 focused modules:
  | File | Lines | Responsibility |
  |------|-------|---------------|
  | `bash_abort.rs` | 45 | Per-session abort flag management |
  | `bash_output.rs` | 181 | Output formatting (BashOutput, StreamBuffers, STDERR_MARKER) |
  | `bash_process.rs` | 135 | Process group management + command spawning |
  | `bash_streams.rs` | 176 | Stream reader tasks, abort waiting, StdoutStreamMode |
  | `bash.rs` | 256 | BashTool struct + rig::tool::Tool impl only |
  | `tests/bash_output_test.rs` | 230 | Extracted 16 BashOutput formatting tests |
- 🟡 Warning: DRY violation — inline stdout reader duplicated `spawn_stdout_reader` → ✅ Fixed: Unified via `StdoutStreamMode` enum
- 🟡 Warning: `resolve_cwd()` duplicated in `call()` and `call_with_streaming()` → ✅ Fixed: Extracted to shared method

## Final Verification
- All tests pass: ✅ (39 tests across 6 suites: tool_progress, pause_handler, bridge_handler, bash_abort, bash_output, bash_streaming)
- Workspace compiles: ✅ (`cargo check --workspace` clean)
- All feature files valid: ✅ (747/747)
- Coverage links updated: ✅ (bash_abort.rs impl references updated)
- All files under 300 lines: ✅
