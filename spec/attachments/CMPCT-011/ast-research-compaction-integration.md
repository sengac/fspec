# AST Research: In-View DAG Construction Compaction Flow (CMPCT-011)

## 1. execute_compaction() — Current Signature & Callers

### Function Definition
```
codelet/cli/src/interactive_helpers.rs:171:1
pub async fn execute_compaction(session: &mut Session) -> Result<(CompactionMetrics, Option<AnchorPoint>)>
```

### All Call Sites (4 total)
```
codelet/cli/src/interactive/stream_loop.rs:464:15  — pre-prompt compaction check
codelet/cli/src/interactive/stream_loop.rs:1530:15 — hook-triggered compaction
codelet/cli/src/interactive/repl_loop.rs:88:19     — manual /compact command (CLI mode)
codelet/napi/src/session_manager.rs:7417:35        — session_compact() NAPI binding
```

**Impact**: ALL 4 callers must be updated when signature and return type change.
- stream_loop.rs: 2 call sites, needs Arc<AtomicBool> threaded through function chain
- repl_loop.rs: 1 call site, manual /compact in CLI mode
- session_manager.rs: 1 call site, has direct access to BackgroundSession.compaction_in_progress

## 2. inject_summary_handler — Current Signature

```
codelet/napi/src/inject_summary_handler.rs:38:1
pub fn create_handler(session: Arc<Mutex<Session>>, context_window: u64) -> InjectSummaryHandler
```

**Impact**: Needs additional `compaction_in_progress: Arc<AtomicBool>` parameter.
Handler must clear flag atomically after injection completes (CMPCT-011 rule [14]).

## 3. session_search_handler — create_handler Already Extended

```
codelet/napi/src/session_search_handler.rs:38:1
pub fn create_handler(project_path: PathBuf, compaction_trimming: Arc<AtomicBool>) -> SessionSearchHandler
```

Already accepts `compaction_trimming: Arc<AtomicBool>` (CMPCT-010 done).
Registration at session_manager.rs:5375 passes `session.compaction_in_progress.clone()`.

## 4. BackgroundSession.compaction_in_progress — Already Exists

```
codelet/napi/src/session_manager.rs:1007
pub compaction_in_progress: Arc<AtomicBool>
```

Initialized to false at session_manager.rs:1073.
Already passed to session_search_handler at session_manager.rs:5375.

## 5. inject_summary Handler Registration

```
codelet/napi/src/session_manager.rs:5379-5392 — Registration (CMPCT-009)
codelet/napi/src/session_manager.rs:5602       — Cleanup on teardown
```

Currently creates handler with `(session.inner.clone(), context_window)`.
Needs `session.compaction_in_progress.clone()` as third arg.

## 6. stream_loop Call Chain (flag propagation path)

```
agent_loop(session: Arc<BackgroundSession>, ...) [session_manager.rs:5050]
  → run_with_provider! macro [session_manager.rs:5016]
    → run_agent_stream_with_images(agent, input, images, inner, ...) [stream_loop.rs:285]
      → run_agent_stream_internal(agent, prompt, images, session, ...) [stream_loop.rs:385]
        → execute_compaction(session) [stream_loop.rs:464, 1530]
```

**Design choices for flag propagation:**
A) Thread Arc<AtomicBool> through run_agent_stream → run_agent_stream_internal → execute_compaction
B) Use codelet_tools session-scoped registry (like set_session_search_handler pattern)
C) Add field to Session struct itself

## 7. StructuralAnnotation — Already Exists (CMPCT-007)

```
codelet/core/src/compaction/model.rs:299
pub enum StructuralAnnotation { FspecMilestone, ErrorResolution, FileModification }
```

Re-exported at codelet/core/src/compaction/mod.rs:37.

## 8. annotation_detector.rs — Does Not Exist Yet

No file at `codelet/core/src/compaction/annotation_detector.rs`.
Must be created as new module in the compaction crate.

## 9. clear_history() — BackgroundSession Method

```
codelet/napi/src/session_manager.rs:1582
pub fn clear_history(&self) { ... }
```

This is on BackgroundSession, NOT on Session. execute_compaction() takes &mut Session.
The new execute_compaction() must perform equivalent operations directly:
- session.messages.clear() + restore via partition_for_compaction()
- session.turns.clear()
- session.token_tracker = TokenTracker::default()

## Summary of Files To Modify

1. `codelet/cli/src/interactive_helpers.rs` — Rewrite execute_compaction()
2. `codelet/cli/src/interactive/stream_loop.rs` — Update 2 call sites + add annotation detection
3. `codelet/cli/src/interactive/repl_loop.rs` — Update 1 call site
4. `codelet/napi/src/session_manager.rs` — Update session_compact() + inject_summary registration
5. `codelet/napi/src/inject_summary_handler.rs` — Add Arc<AtomicBool> param, clear flag

## File To Create

1. `codelet/core/src/compaction/annotation_detector.rs` — Per-turn annotation detection
2. `codelet/core/src/compaction/mod.rs` — Add module declaration + re-exports
