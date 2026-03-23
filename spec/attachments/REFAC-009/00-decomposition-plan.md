# Decomposition Plan: `stream_loop.rs` (2,331 lines → 6 modules)

**Source**: `codelet/cli/src/interactive/stream_loop.rs`  
**Generated**: 2026-03-23  
**Method**: GraphSearch ast_neighbors + ast_search for dependency/call analysis  
**Work Unit**: REFAC-009

---

## Current State (from AST Graph)

**File**: `codelet-cli-src-interactive-stream_loop-rs` — 2,331 lines  
**Contains 14 functions + 1 type**:

| Function | Lines | Public | Concern |
|----------|-------|--------|---------|
| `process_turn_annotations` | 49–71 | no | Annotation |
| `is_prompt_too_long_error` | 78–94 | yes | Error classification |
| `is_image_content_error` | 102–115 | yes | Error classification |
| `sanitize_image_content` | 125–221 | yes | Image recovery |
| `is_truncated_tool_call_error` | 235–237 | yes | Error classification |
| `build_truncation_recovery_message` | 245–272 | yes | Truncation recovery |
| `build_truncation_budget_exhausted_message` | 277–285 | yes | Truncation recovery |
| `is_thinking_exhaustion` | 322–348 | yes | Error classification |
| `build_thinking_exhaustion_recovery_message` | 358–384 | yes | Thinking recovery |
| `build_thinking_budget_exhausted_message` | 389–395 | yes | Thinking recovery |
| `downgrade_thinking_level` | 403–411 | yes | Thinking recovery |
| `is_compaction_cancelled` | 415–417 | no | Compaction |
| `signal_compaction_needed` | 421–430 | no | Compaction |
| `build_user_content_with_images` | 554–601 | yes | Multimodal content |
| **Type**: `BridgeImage` | 541–546 | yes | Multimodal content |

**Plus 4 async functions** (not extracted by AST — they use generics):
| Function | Lines | Visibility | Concern |
|----------|-------|------------|---------|
| `run_agent_stream_with_interruption` | 437–465 | pub(super) | Entry point (CLI) |
| `run_agent_stream` | 475–502 | pub | Entry point (NAPI) |
| `run_agent_stream_with_images` | 509–537 | pub | Entry point (NAPI+images) |
| `run_agent_stream_internal` | 611–2331 | private | **THE MONOLITH** (1,720 lines) |

**Constants** (4):
- `MAX_TRUNCATION_RETRIES` (line 226)
- `MAX_THINKING_EXHAUSTION_RETRIES` (line 294)
- `THINKING_EXHAUSTION_OUTPUT_THRESHOLD` (line 300)
- `THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD` (line 306)

---

## Decomposition Plan: 6 New Modules

### Summary

| # | New Module | Lines (est.) | Functions Moved | Responsibility |
|---|-----------|-------------|----------------|----------------|
| 1 | `error_classifiers.rs` | ~80 | 4 functions | Pure error string → bool classification |
| 2 | `recovery_truncation.rs` | ~80 | 3 functions + 1 const | PROV-040 truncation recovery |
| 3 | `recovery_thinking.rs` | ~120 | 4 functions + 3 consts | PROV-041 thinking exhaustion recovery |
| 4 | `recovery_image.rs` | ~120 | 2 functions | EXT-016 image sanitization/recovery |
| 5 | `multimodal.rs` | ~80 | 1 function + 1 struct | BridgeImage + content building |
| 6 | `stream_loop.rs` (slimmed) | ~300 | Entry points + core loop | Orchestration only |
| — | **Total** | ~780 | | vs 2,331 original |

The remaining ~1,550 lines of `run_agent_stream_internal` need further structural decomposition (Gemini continuation sub-loop, post-loop compaction, retry-after-compaction sub-loop). Those are covered in the `stream_loop.rs` slimmed document.

---

## Existing Test Files (from AST Graph)

These test files `use codelet_cli::interactive::*` — import paths must be re-exported from `mod.rs`:

| Test File | Lines | Tests |
|-----------|-------|-------|
| `codelet/cli/tests/prompt_too_long_recovery_test.rs` | 416 | `is_prompt_too_long_error` |
| `codelet/cli/tests/truncation_recovery_test.rs` | 359 | `is_truncated_tool_call_error`, `build_truncation_*` |
| `codelet/cli/tests/thinking_exhaustion_recovery_test.rs` | 529 | `is_thinking_exhaustion`, `build_thinking_*`, `downgrade_thinking_level` |
| `codelet/cli/tests/image_content_recovery_test.rs` | 456 | `is_image_content_error`, `sanitize_image_content` |
| `codelet/cli/tests/stream_loop_pause_test.rs` | 250 | Stream loop pause behavior |

**Key constraint**: All `pub` items currently re-exported via `interactive/mod.rs` must remain re-exported after decomposition. Tests import from `codelet_cli::interactive::*`.

---

## Module Dependency Diagram

```
stream_loop.rs (orchestrator)
  ├── uses error_classifiers.rs (pure functions, no deps)
  ├── uses recovery_truncation.rs (pure functions, no deps)
  ├── uses recovery_thinking.rs (pure functions + ThinkingLevel dep)
  ├── uses recovery_image.rs (depends on rig::message types)
  ├── uses multimodal.rs (depends on rig + codelet_tools::image_dimensions)
  ├── uses stream_handlers.rs (existing, unchanged)
  └── uses interactive_helpers.rs (existing, unchanged)
```
