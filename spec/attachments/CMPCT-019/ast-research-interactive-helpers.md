# AST Research: CMPCT-019 Incremental DAG Condensation

## Scope
- Primary file: `codelet/cli/src/interactive_helpers.rs`
- Related: `codelet/core/src/compaction/model.rs`, `codelet/napi/src/inject_summary_handler.rs`

## Findings

### 1. execute_compaction — Current Implementation
**Location:** `codelet/cli/src/interactive_helpers.rs:305:1`
- Single async function that always uses `COMPACTION_SYSTEM_INSTRUCTION`
- Calls `reset_session_to_reminders()` which clears messages
- Detection of existing DAG must happen BEFORE reset

### 2. COMPACTION_SYSTEM_INSTRUCTION Constant
**Location:** `codelet/cli/src/interactive_helpers.rs:245:1`
- Single constant — needs to be split into FRESH and INCREMENTAL variants
- Referenced by tests in `inject_summary_handler.rs` (lines 315-358)

### 3. detect_existing_dag — Does Not Exist Yet
- No function matching `detect_existing_dag` found in codebase
- Must be created as new public function in `interactive_helpers.rs`

### 4. DAG Content Detection
- Compaction-dag marker: `<!-- type:compaction-dag -->`
- Wrapped by `wrap_dag_content()` in `inject_summary_handler.rs:95-99`
- NOT a standard `SystemReminderType` — uses raw string matching
- `partition_for_compaction` treats it as a regular user message (not a named reminder type)
- BUT `apply_pending_dag` in inject_summary_handler re-injects it after clearing

### 5. parse_dag_nodes — Available from CMPCT-017
**Location:** `codelet/core/src/compaction/model.rs:368`
- `pub fn parse_dag_nodes(dag_content: &str, message_count: Option<usize>) -> Vec<DagNodeMeta>`
- Can extract `turn_end` from parsed nodes to determine `max_turn_end`
- Returns nodes sorted by `turn_start` ascending, so last node's `turn_end` is max

### 6. Tests Referencing COMPACTION_SYSTEM_INSTRUCTION
- `inject_summary_handler.rs:315` — `test_compaction_instruction_content`
- `inject_summary_handler.rs:330` — `test_compaction_instruction_specifies_dag_node_format`
- Both import `COMPACTION_SYSTEM_INSTRUCTION` — need updating to new constant names
