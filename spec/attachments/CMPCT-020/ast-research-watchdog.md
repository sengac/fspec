# AST Research: CMPCT-020 Compaction Convergence Watchdog

## Scope
- Primary: `codelet/cli/src/interactive_helpers.rs` (new functions)
- Integration: `codelet/napi/src/session_manager.rs` (agent_loop modification)
- Related: `codelet/napi/src/inject_summary_handler.rs` (wrap_dag_content, apply_pending_dag)

## Findings

### 1. agent_loop in session_manager.rs
**Location:** `codelet/napi/src/session_manager.rs:3944`
- Main loop: waits for input, runs stream, applies DAG
- After `run_with_provider!`, calls `apply_pending_dag` (line 4569)
- Already has safety net clearing compaction_in_progress (line 4588)
- Watchdog retry logic goes here: check if compaction was active but no DAG produced

### 2. compaction_in_progress tracking
- `Arc<AtomicBool>` shared between stream_loop and inject_summary handler
- Set to true by `execute_compaction()` 
- Cleared by inject_summary handler when agent calls inject_summary
- Also cleared as safety net after stream ends (session_manager line 4588)
- Watchdog can check: if was_compacting && pending_dag.is_none() → agent failed

### 3. Existing helpers to reuse
- `reset_session_to_reminders()` — clears messages, preserves system reminders
- `recalculate_token_tracker()` — updates token counts from current messages
- `wrap_dag_content()` — wraps DAG in system-reminder compaction-dag tags
- `parse_dag_nodes()` — extracts <dag-node> blocks from content

### 4. force_inject pattern
The force-inject needs to:
1. Extract partial <dag-node> blocks from recent messages (reuse parse_dag_nodes)
2. Or create minimal fallback DAG
3. Call `reset_session_to_reminders(session)` 
4. Push wrapped DAG as user message
5. Call `recalculate_token_tracker(session)`
6. Clear `compaction_in_progress`
This mirrors `apply_pending_dag` but operates directly on session

### 5. Where retry input comes from
The agent_loop uses `tokio::select!` to wait for user/supervisor input.
For watchdog retries, we inject a synthetic "Continue" input before the select.
Pattern: `compaction_retry_input: Option<InputWithImages>` checked at loop top.
