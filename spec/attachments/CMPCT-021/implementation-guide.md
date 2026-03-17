# CMPCT-021: File ID Propagation Through DAG

## What This Card Does

Ensures file references survive across multiple compaction cycles by:
1. **Engine-side:** Automatically appending a `<dag-files>` block to the DAG after inject_summary, built from FileModification annotations
2. **Agent-side:** Including the previous `<dag-files>` block in the incremental compaction instruction so the agent carries file awareness forward

## The Problem

After compaction, the agent loses awareness of which files it modified earlier in the session. While it can re-discover files using Glob/Grep/Read tools, this wastes turns and tokens. LCM solves this by propagating "file IDs" through summary nodes.

## Two Deliverables

### 1. Engine-Side: Auto-Append dag-files Block

**File:** `codelet/napi/src/inject_summary_handler.rs`  
**Function:** `apply_pending_dag()` (lines ~162–198)

After the agent's DAG is stored, scan structural annotations from the compacted turns and append a `<dag-files>` block:

```rust
fn build_dag_files_block(
    session_id: &str,
    compacted_turn_range: (usize, usize),
    existing_dag_files: Option<&str>,
) -> Option<String> {
    // 1. Load FileModification annotations from persisted messages
    //    within the compacted turn range
    let file_mods = load_file_annotations(session_id, compacted_turn_range);
    
    // 2. Merge with any existing dag-files from previous compaction
    let mut all_files: BTreeMap<String, FileOp> = BTreeMap::new();
    
    // Parse existing dag-files block if present
    if let Some(existing) = existing_dag_files {
        for line in existing.lines() {
            // Parse: "- path/to/file.rs (Created|Modified|Deleted)"
            if let Some((path, op)) = parse_dag_file_line(line) {
                all_files.insert(path, op);
            }
        }
    }
    
    // Add new annotations (newer ops override older)
    for annotation in &file_mods {
        if let StructuralAnnotation::FileModification { path, operation } = annotation {
            all_files.insert(path.clone(), operation.clone());
        }
    }
    
    if all_files.is_empty() {
        return None;
    }
    
    // 3. Build the block
    let mut block = String::from("<dag-files>\n");
    for (path, op) in &all_files {
        block.push_str(&format!("- {} ({:?})\n", path, op));
    }
    block.push_str("</dag-files>");
    
    Some(block)
}
```

**In `apply_pending_dag()`:**

```rust
// After storing the agent's DAG content
let dag_content = pending_dag.take().unwrap();

// Check if agent already included a <dag-files> block
if !dag_content.contains("<dag-files>") {
    // Auto-append from annotations
    if let Some(files_block) = build_dag_files_block(
        session_id,
        (0, last_turn),
        None, // First compaction — no existing dag-files
    ) {
        dag_content.push_str("\n\n");
        dag_content.push_str(&files_block);
    }
}
```

### 2. Agent-Side: Include dag-files in Incremental Instruction

**File:** `codelet/cli/src/interactive_helpers.rs`  
**Location:** `COMPACTION_INSTRUCTION_INCREMENTAL` (from CMPCT-019)

The incremental instruction template already includes `{existing_dag_content}`. The `<dag-files>` block will be part of that content since it's appended to the DAG.

Additionally, the instruction should explicitly mention:

```
6. PRESERVE the <dag-files> section — update it with any new file modifications from fresh turns.
   Remove entries for files that were deleted. Add entries for newly created/modified files.
```

### How dag-files Flows Across Compactions

```
Compaction 1 (FRESH):
  Agent builds DAG → inject_summary
  Engine appends: <dag-files>
    - src/auth.rs (Created)
    - src/db.rs (Modified)
  </dag-files>

Compaction 2 (INCREMENTAL):
  Instruction includes existing DAG with dag-files block
  Agent updates DAG → inject_summary  
  Engine merges: new annotations + existing dag-files
  Result: <dag-files>
    - src/auth.rs (Modified)     ← updated: was Created, now Modified again
    - src/db.rs (Modified)       ← carried forward
    - src/api.rs (Created)       ← NEW from fresh turns
  </dag-files>

Compaction 3 (INCREMENTAL):
  Same flow — dag-files block grows/updates across cycles
```

## Data Sources

### FileModification Annotations
- **Detection:** `codelet/core/src/compaction/annotation_detector.rs:106–131`
  - Write tool → `FileModification { path, operation: Created }`
  - Edit tool → `FileModification { path, operation: Modified }`
- **Storage:** `codelet/napi/src/session_manager.rs:3772–3847` — `persist_pending_annotations()`
  - Stored in `StoredMessage.metadata["annotations"]` as JSON
- **Retrieval:** Load from persisted session, deserialize `StructuralAnnotation` from metadata

### Existing dag-files Extraction
- Parse from the existing DAG content (string between `<dag-files>` and `</dag-files>` tags)
- Simple line-by-line parsing: `- path/to/file (Operation)`

## Edge Cases

- Agent already includes a `<dag-files>` block → don't duplicate, use the agent's version (they may have intentionally removed stale entries)
- No FileModification annotations exist → no `<dag-files>` block appended (don't create empty block)
- File deleted then re-created → last operation wins (BTreeMap insert overwrites)
- Very large number of files → consider truncating to most recent 50 files with a note

## Testing Strategy

- Unit test: `build_dag_files_block()` with various annotation sets
- Unit test: Merge existing dag-files with new annotations
- Unit test: Parse dag-files block from DAG content
- Unit test: Agent-provided dag-files not duplicated
- Integration test: Two compaction cycles, verify files propagate

## Dependencies

- **CMPCT-017** — Structured DAG format (`<dag-node>` blocks) alongside which `<dag-files>` sits
- **CMPCT-019** — Incremental compaction instruction (dag-files block is included in the existing DAG passed to incremental instruction)
