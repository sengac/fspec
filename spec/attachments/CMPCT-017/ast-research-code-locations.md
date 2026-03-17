# AST Research: CMPCT-017 Code Locations

## Purpose
Identify exact code locations for CMPCT-017 implementation: structured DAG node format and engine parsing.

## Findings

### 1. COMPACTION_SYSTEM_INSTRUCTION
- **File:** `codelet/cli/src/interactive_helpers.rs`
- **Line:** 245
- **Type:** `pub const &str` — the system instruction injected during compaction
- **Usage:** Lines 308 (with user prompt) and 310 (standalone) in `execute_compaction()`
- **Change needed:** Replace free-form markdown format with structured `<dag-node>` XML format

### 2. apply_pending_dag()
- **File:** `codelet/napi/src/inject_summary_handler.rs`
- **Line:** 168
- **Signature:** `pub fn apply_pending_dag(session, pending_dag: &Arc<Mutex<Option<String>>>) -> bool`
- **Change needed:** After storing DAG content, parse `<dag-node>` blocks via regex, extract `Vec<DagNodeMeta>`, store alongside raw content

### 3. StructuralAnnotation enum
- **File:** `codelet/core/src/compaction/model.rs`
- **Line:** 299
- **Variants:** `FspecMilestone`, `ErrorResolution`, `FileModification`
- **Companion:** `FileOp` enum at line 276 (Created, Modified, Deleted)
- **Change needed:** Add `DagNodeMeta` struct and `DagDepth` enum after existing types

### 4. DagDepth / DagNodeMeta
- **Status:** Not yet implemented (no matches found)
- **Change needed:** Create `DagDepth` enum (D0, D1, D2) and `DagNodeMeta` struct in `model.rs`

### 5. State Management
- **Current:** `pending_dag: Arc<Mutex<Option<String>>>` — bare string
- **Change needed:** Extend to `InjectSummaryState` struct with `dag_content: String` and `dag_nodes: Vec<DagNodeMeta>`

## Re-export Path
- `codelet/core/src/compaction/mod.rs` line 33 re-exports model types
- Must add `DagDepth`, `DagNodeMeta` to the re-export list
