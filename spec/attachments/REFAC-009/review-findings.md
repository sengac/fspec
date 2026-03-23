# Epic Review: REFAC-009 — Decompose stream_loop.rs

**Date:** 2026-03-23
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 8 (REFAC-010 through REFAC-017)

## Summary
- 🔴 Critical: 0 issues
- 🟡 Warnings: 5 issues
- 🟢 Observations: 9 (all positive)

## Overall Status: WARN

---

## 🔴 Critical Issues (Must Fix)
None

## 🟡 Warnings (Should Fix)

1. **`gemini_continuation.rs` at 406 lines exceeds 300-line limit** — `run_continuation_loop` (~206 lines) contains nested continuation handling (lines 265–312) that could be extracted into `handle_nested_continuation()` to bring the file under 300 lines.

2. **`compaction_retry.rs` at 320 lines exceeds 300-line limit** — `run_retry_stream` contains ~80 lines of debug capture boilerplate that could be extracted into helper functions.

3. **`stream_loop.rs` still at 1,315 lines** — While reduced from 2,331, the main `run_agent_stream_internal` function spans ~1,072 lines. Pre-prompt compaction check, debug capture setup, thinking exhaustion recovery, and truncation recovery are candidates for further extraction.

4. **Structural duplication across 3 stream processing loops** — Three separate `match` loops process `MultiTurnStreamItem` variants in nearly identical patterns across `stream_loop.rs`, `compaction_retry.rs`, and `gemini_continuation.rs`. A `StreamProcessor` struct or `process_stream_item()` helper could eliminate ~120–140 lines of duplication.

5. **Unused `use anyhow;` import style in `error_classifiers.rs`** — The bare `use anyhow;` is technically valid but inconsistent with codebase patterns. More idiomatic: inline `anyhow::Error` or `use anyhow::Error`.

## 🟢 Observations (Nice to Have)

1. All module-level doc comments present and descriptive
2. No `unwrap()` in production code — all safe fallbacks
3. No TODO/FIXME/HACK/XXX/todo!()/unimplemented!() found
4. Zero compiler warnings
5. All 265+ tests pass
6. Re-exports in mod.rs are complete — all external consumers work
7. `signal_compaction_needed` and `emit_context_fill_from_usage` correctly `pub(super)`
8. No duplicate function definitions — all originals replaced with imports
9. SRP well-maintained across all extracted modules

## Decomposition Results

| Module | Lines | Status | Purpose |
|--------|-------|--------|---------|
| `error_classifiers.rs` | 67 | ✅ | 4 pure error classification functions |
| `recovery_truncation.rs` | 57 | ✅ | PROV-040 truncation recovery |
| `recovery_thinking.rs` | 127 | ✅ | PROV-041 thinking exhaustion |
| `recovery_image.rs` | 113 | ✅ | EXT-016 image sanitization |
| `multimodal.rs` | 70 | ✅ | BridgeImage + content building |
| `gemini_continuation.rs` | 406 | ⚠️ | Gemini continuation sub-loop |
| `compaction_retry.rs` | 320 | ⚠️ | Post-loop compaction retry |
| `stream_loop.rs` (slimmed) | 1,315 | ⚠️ | Down from 2,331 (44% reduction) |

**Total extracted**: ~1,160 lines into 7 new modules
**Original**: 2,331 lines → now 1,315 lines in stream_loop.rs + 1,160 in new modules = 2,475 total (slight increase due to module overhead/doc comments)
