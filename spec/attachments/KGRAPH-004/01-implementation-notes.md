# KGRAPH-004: Structural Extractors — Implementation Notes

## Overview

Structural extractors parse messages deterministically (no LLM) to extract
graph entities from tool calls that are already structured data.

## Three Extractors

### 1. Fspec Tool Call Extractor

Detects fspec tool invocations and extracts WorkUnit nodes + status edges:

**Trigger:** Message contains fspec tool call results (command patterns)

**Extracts:**
- `WorkUnit` nodes from `create-story`, `create-bug`, `create-task` calls
- `WorksOn` edges from `update-work-unit-status` calls (Session → WorkUnit)
- Status updates on existing WorkUnit nodes

**Pattern matching:**
```
Tool: Fspec → args.command matches:
  "create-story"  → Insert WorkUnit { slug, title, workType: "story" }
  "create-bug"    → Insert WorkUnit { slug, title, workType: "bug" }
  "create-task"   → Insert WorkUnit { slug, title, workType: "task" }
  "update-work-unit-status" → Update WorkUnit { status }
                             + Insert WorksOn edge (Session → WorkUnit)
```

### 2. File Modification Extractor

Detects Write/Edit tool calls and extracts CodeEntity nodes:

**Trigger:** Message contains Write or Edit tool call

**Extracts:**
- `CodeEntity` nodes with file path, entity type, language (inferred from extension)
- `Modifies` edges (Turn → CodeEntity) with operation type

**Pattern matching:**
```
Tool: Write → file_path
  → Insert CodeEntity { slug: path, entityType: "file", operation: "created" }
  → Insert Modifies edge (Turn → CodeEntity)

Tool: Edit → file_path
  → Insert CodeEntity { slug: path, entityType: "file", operation: "modified" }
  → Insert Modifies edge (Turn → CodeEntity)
```

### 3. Error Resolution Extractor

Detects failure→success patterns (reuses existing annotation_detector.rs logic):

**Trigger:** A tool call fails, then a subsequent tool call of same type succeeds

**Extracts:**
- Lightweight `Decision` node (slug: "resolve-{error-hash}", domain: "implementation")

## Integration Point

Wire into the existing `AnnotationDetector` in `session_manager.rs`:

```rust
// After structural annotations are detected, also extract graph entities
if let Some(annotations) = detect_annotations(&turn) {
    if graph_enabled {
        let entities = structural_extract(&annotations, &session_id, turn_index);
        queue_graph_upsert(entities);
    }
}
```

The `queue_graph_upsert` function batches entities and writes them to the
graph DB periodically (not on every single turn — amortize I/O).

## Batch Queue

```rust
struct GraphEntityQueue {
    nodes: Vec<JsonlEntry>,
    edges: Vec<JsonlEntry>,
    flush_threshold: usize, // default: 50
}
```

Flush happens when:
- Queue hits threshold (50 entries)
- Session goes idle
- Process exit cleanup
