# CMPCT-017: Structured DAG Node Format and Engine Parsing

## What This Card Does

Upgrades the compaction system from free-form markdown DAGs to **structured XML-annotated DAG nodes** that the engine can parse and validate. This is the foundation that all other CMPCT-016 children depend on.

## Three Deliverables

### 1. Update Compaction Instruction (Agent-Side)

**File:** `codelet/cli/src/interactive_helpers.rs`  
**Location:** `COMPACTION_SYSTEM_INSTRUCTION` constant (lines ~245–263)

**Current instruction** tells the agent to write a "structured summary with depth levels: D2, D1, D0" as plain markdown.

**New instruction** must specify structured XML format:

```xml
<dag-node depth="D2" turns="0-45" label="Architecture: JWT auth system">
  Durable summary content here...
</dag-node>

<dag-node depth="D1" turns="46-82" label="Implementing login endpoint">
  Arc summary content here...
</dag-node>

<dag-node depth="D0" turns="83-95" label="Fixing test failures in auth.test.ts">
  Detailed summary content here...
  [SessionSearch: turns 88-92]
</dag-node>
```

**Key attributes:**
- `depth` — Required. One of `D0`, `D1`, `D2`
- `turns` — Required. Format `N-M` (inclusive range of turn indices this node summarizes)
- `label` — Required. Short identifier (max ~80 chars)

**Instruction must explain:**
- D2 = durable decisions/milestones that survive multiple compactions
- D1 = current work arc, promoted from D0 on re-compaction
- D0 = detailed recent work, most granular
- Turn ranges should be non-overlapping and collectively cover the session
- `[SessionSearch: turns X-Y]` references inside nodes enable future drilldown

### 2. DagNodeMeta Data Model

**File:** `codelet/core/src/compaction/model.rs`  
**After:** `StructuralAnnotation` enum (line ~321)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNodeMeta {
    pub depth: DagDepth,
    pub turn_start: usize,
    pub turn_end: usize,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DagDepth {
    D0,  // Detailed — recent work
    D1,  // Arc — current work state
    D2,  // Durable — architecture decisions, milestones
}
```

### 3. Engine-Side Parsing in apply_pending_dag

**File:** `codelet/napi/src/inject_summary_handler.rs`  
**Function:** `apply_pending_dag()` (lines ~162–198)

After the DAG content is stored, parse out `<dag-node>` blocks:

```rust
fn parse_dag_nodes(dag_content: &str) -> Vec<DagNodeMeta> {
    // Regex: <dag-node depth="(D[012])" turns="(\d+)-(\d+)" label="([^"]+)">
    // Extract all matches
    // Validate: turn_end >= turn_start
    // Validate: depth is one of D0, D1, D2
    // Sort by turn_start ascending
    // Return Vec<DagNodeMeta>
}
```

**Validation rules:**
- If `turn_end > persisted_message_count`, clamp to message count (don't fail)
- If no `<dag-node>` blocks found, that's OK (backward compat with plain markdown DAGs)
- Log a warning if turn ranges overlap
- Store `Vec<DagNodeMeta>` alongside the DAG content for use by CMPCT-019 and CMPCT-021

**Storage:** Add to `InjectSummaryState`:
```rust
pub dag_nodes: Option<Vec<DagNodeMeta>>,
```

## Testing Strategy

- Unit tests for `parse_dag_nodes()` — valid XML, malformed, missing attributes, overlapping ranges
- Unit tests for `DagNodeMeta` serialization round-trip
- Integration test: inject a DAG with structured nodes, verify `dag_nodes` populated
- Backward compat test: inject a plain markdown DAG (no `<dag-node>` tags), verify no crash

## Dependencies

- None (this is the root of the CMPCT-016 dependency chain)

## Depends-On (Downstream)

- CMPCT-018 uses DagNodeMeta turn ranges for scoped queries
- CMPCT-019 uses dag_nodes to detect existing DAG for incremental mode
- CMPCT-020 extracts dag-node blocks for Level 3 fallback
- CMPCT-021 uses dag_nodes for file propagation
