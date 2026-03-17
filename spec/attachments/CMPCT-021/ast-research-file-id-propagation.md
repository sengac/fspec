# AST Research: CMPCT-021 File ID Propagation Through DAG

## Key Types and Functions

### FileOp enum (codelet/core/src/compaction/model.rs:276)
- `Created`, `Modified`, `Deleted` variants
- Derives: Debug, Clone, PartialEq, Eq, Serialize, Deserialize

### StructuralAnnotation enum (codelet/core/src/compaction/model.rs:299)
- `FileModification { path: String, operation: FileOp }` variant
- Also has FspecMilestone, ErrorResolution variants

### detect_file_modifications (codelet/core/src/compaction/annotation_detector.rs:110)
- Detects FileModification annotations from Write/Edit tool calls
- Already implemented and tested

### apply_pending_dag (codelet/napi/src/inject_summary_handler.rs:170)
- Takes session + pending_dag Arc<Mutex<Option<String>>>
- Returns Option<Vec<DagNodeMeta>>
- Currently: takes pending DAG, resets session, appends wrapped DAG, recalculates tokens
- **MODIFICATION POINT**: After storing agent's DAG, check for dag-files block and auto-append if missing

### detect_existing_dag (codelet/cli/src/interactive_helpers.rs:328)
- Scans messages for compaction-dag marker
- Returns Option<(String, usize)> — (dag_content, max_turn_end)
- The dag_content includes the full existing DAG which will have <dag-files> if previously appended

### COMPACTION_INSTRUCTION_INCREMENTAL (codelet/cli/src/interactive_helpers.rs:300)
- Template with {existing_dag_content} and {last_compacted_turn} placeholders
- **MODIFICATION POINT**: Add rule #6 about preserving/updating dag-files section

### COMPACTION_INSTRUCTION_FRESH (codelet/cli/src/interactive_helpers.rs:245)
- No placeholders, used for first compaction
- **MODIFICATION POINT**: Add guidance about Active Files / dag-files section

### persist_pending_annotations (codelet/napi/src/session_manager.rs:3772)
- Persists annotations to StoredMessage.metadata["annotations"]
- Called after each stream completion

## Data Flow

1. Write/Edit tool calls → annotation_detector → FileModification annotations
2. persist_pending_annotations → stored in session message metadata
3. inject_summary → stores DAG in pending_dag
4. apply_pending_dag → clears session, injects DAG (HERE: auto-append dag-files)
5. Next compaction → detect_existing_dag → dag-files block flows through
