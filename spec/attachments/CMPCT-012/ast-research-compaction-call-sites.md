# AST Research: Emergency Threshold Compaction Call Sites

## execute_compaction_legacy() Call Sites (3 total)

### 1. Pre-prompt compaction (stream_loop.rs:464)
```
codelet/cli/src/interactive/stream_loop.rs:464:15 — execute_compaction_legacy(session)
```
Context: Called when `estimated_total > threshold && has_turns_to_compact`.
Returns `(CompactionMetrics, Option<AnchorPoint>)`.
Callers use metrics for UX events (`emit_compaction_complete`), then call `session.token_tracker.reset_after_compaction()`.

### 2. Post-loop hook-triggered compaction (stream_loop.rs:1530)
```
codelet/cli/src/interactive/stream_loop.rs:1530:15 — execute_compaction_legacy(session)
```
Context: Called when `compaction_needed && !is_interrupted`.
After compaction, re-adds original prompt to session.messages and starts a retry stream via `prompt_streaming_with_history_and_hook`.

### 3. /compact slash command (repl_loop.rs:88)
```
codelet/cli/src/interactive/repl_loop.rs:88:19 — execute_compaction_legacy(session)
```
Context: Manual compaction triggered by user typing `/compact`.
Uses metrics for CLI output (compression percentage, turns summarized/kept).

## execute_compaction() — New CMPCT-011 Function

```
codelet/cli/src/interactive_helpers.rs:261:1 — pub async fn execute_compaction(session, compaction_in_progress: Arc<AtomicBool>) -> Result<()>
```
Signature: `(session: &mut Session, compaction_in_progress: Arc<AtomicBool>) -> Result<()>`
Returns `Ok(())` instead of metrics — callers must capture pre-compaction tokens before calling.

## NAPI session_compact() — Already Migrated

```
codelet/napi/src/session_manager.rs:7418 — execute_compaction(&mut inner, session.compaction_in_progress.clone())
```
Already uses new flow (done in CMPCT-011). Captures original_tokens before calling.

## annotation_detector — Detection API

```
codelet/core/src/compaction/annotation_detector.rs:55:1 — pub fn detect_annotations(ctx: &TurnContext<'_>) -> Vec<StructuralAnnotation>
```
Input: `TurnContext { current_tool_calls, previous_tool_calls }`
Output: `Vec<StructuralAnnotation>` (FspecMilestone, FileModification, ErrorResolution)

## run_agent_stream Call Chain (needs compaction_in_progress threading)

- `run_agent_stream_internal` (line 385) — core generic loop, does NOT have compaction_in_progress
- `run_agent_stream` (line 254) — NAPI entry point, calls internal
- `run_agent_stream_with_images` (line 285) — NAPI multimodal, calls internal
- `run_agent_stream_with_interruption` (line 219) — CLI entry point, calls internal

NAPI macro `run_with_provider!` calls `run_agent_stream_with_images` at line 5022.

## BackgroundSession compaction_in_progress Field

```
codelet/napi/src/session_manager.rs:1007 — pub compaction_in_progress: Arc<AtomicBool>
codelet/napi/src/session_manager.rs:1073 — compaction_in_progress: Arc::new(AtomicBool::new(false))
```
Already exists on NAPI BackgroundSession. CLI Session does NOT have this field — CLI callers must create a local Arc.
