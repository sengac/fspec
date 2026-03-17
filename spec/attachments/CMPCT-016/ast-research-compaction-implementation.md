# AST Research: Compaction Implementation Analysis

## Watchdog Retry Logic (session_manager.rs)
- `compaction_in_progress` flag checked after each `run_with_provider` call
- 3-attempt escalation: normal → escalation message → force-inject
- `force_inject_fallback_dag()` extracts partial dag-nodes or creates minimal fallback

## Structured DAG Format (compaction_dag.rs)
- `wrap_dag_content()` / `parse_dag_nodes()` in codelet-core
- `<dag-node depth="Dx" turns="N-M" label="...">` XML blocks
- `<dag-files>` block for file ID propagation

## Incremental Condensation (interactive_helpers.rs)
- `COMPACTION_INSTRUCTION_FRESH` vs `COMPACTION_INSTRUCTION_INCREMENTAL`
- `execute_compaction()` detects existing DAG via compaction-dag system-reminder

## SessionSearch Turn Ranges
- `start_turn` / `end_turn` parameters on show and search actions
- Reuses `resolve_turns_context()` from AgentManager

## Note
This is a decomposed parent card. All implementation delivered through children:
CMPCT-017, CMPCT-018, CMPCT-019, CMPCT-020, CMPCT-021 (all done, 100% coverage).
