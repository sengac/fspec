# KGRAPH-026: AST Extraction Pipeline Duplicate File Entity Fix

## Bug Summary

The `GraphSearch ast_index` command fails with:
```
@unique constraint violation on File.path: duplicate value 'bridge/telegram-slash-commands.ts' at rows 10 and 13
```

## Root Cause Analysis

### Two Code Paths Create File Nodes

The AST extraction pipeline creates `File` graph nodes from **two independent code paths**:

#### Path 1 — Direct File Walk (Full Node)
`walk_and_extract()` → `extract_file()` → `extract_typescript()` → `helpers::build_file_node()`

Creates a **full** File node with all properties:
```json
{
  "type": "File",
  "data": {
    "slug": "bridge-telegram-slash-commands-ts",
    "path": "bridge/telegram-slash-commands.ts",
    "language": "typescript",
    "lineCount": 247,
    "isTest": false
  }
}
```

**Location**: `ast_ts_extractor.rs:48-50`

#### Path 2 — Import Resolution (Stub Node)
`extract_typescript()` → `extract_imports()` → creates inline stub

Creates a **stub** File node with only slug + path:
```json
{
  "type": "File",
  "data": {
    "slug": "bridge-telegram-slash-commands-ts",
    "path": "bridge/telegram-slash-commands.ts"
  }
}
```

**Location**: `ast_ts_extractor.rs:128-136`

### Why It Fails

1. `bridge/telegram-endpoint.ts` (line 24) has:
   ```typescript
   import { isSlashCommand, handleSlashCommand } from './telegram-slash-commands';
   ```

2. When processing `bridge/telegram-endpoint.ts`, the import extractor calls `resolve_import_path("bridge/telegram-endpoint.ts", "./telegram-slash-commands")` which returns `"bridge/telegram-slash-commands.ts"` and creates a stub File node.

3. The file walker **also** directly processes `bridge/telegram-slash-commands.ts` and creates a full File node.

4. Both nodes land in the same `Vec<GraphEntity>` via `all_entities.extend(entities)` — **no deduplication**.

5. `entities_to_jsonl()` serializes both nodes as separate JSONL lines.

6. The nanograph schema declares:
   ```
   node File {
       slug: String @key
       path: String @unique
   }
   ```

7. Nanograph's JSONL loader enforces `@unique` within a single batch load → **crash**.

### Why Existing Tests Didn't Catch It

The existing `test_walk_project_directory_with_gitignore_and_batch_load` test uses simple fixture files (`src/main.ts`, `src/lib.rs`) that have **no cross-imports**. The duplicate only occurs when:
- File A imports File B
- Both A and B are in the walked project directory

## Fix Strategy

### Location: `walk_and_extract()` in `codelet/napi/src/graph/ast_pipeline/mod.rs`

After collecting all entities from all files, deduplicate Node entities by `(node_type, slug)`:

1. Separate entities into nodes and edges
2. Build a `HashMap<(node_type, slug), usize>` tracking seen node indices
3. When a duplicate is found, keep the node with **more properties** (full > stub)
4. Reassemble: deduplicated nodes + all edges (edges are never deduplicated)

### Why This Location

- Deduplicating in `walk_and_extract()` is the **single point** before JSONL serialization
- Putting dedup in individual extractors would require cross-file coordination
- Putting dedup in `entities_to_jsonl()` would add a surprising side effect to a serialization function

### Implementation Pseudocode

```rust
pub fn deduplicate_entities(entities: Vec<GraphEntity>) -> Vec<GraphEntity> {
    let mut node_map: HashMap<(String, String), usize> = HashMap::new();
    let mut deduped_nodes: Vec<GraphEntity> = Vec::new();
    let mut edges: Vec<GraphEntity> = Vec::new();

    for entity in entities {
        match entity {
            GraphEntity::Node { ref node_type, ref slug, ref properties } => {
                let key = (node_type.clone(), slug.clone());
                if let Some(&existing_idx) = node_map.get(&key) {
                    // Keep the one with more properties
                    if let GraphEntity::Node { properties: ref existing_props, .. } = deduped_nodes[existing_idx] {
                        if properties.len() > existing_props.len() {
                            deduped_nodes[existing_idx] = entity;
                        }
                    }
                } else {
                    node_map.insert(key, deduped_nodes.len());
                    deduped_nodes.push(entity);
                }
            }
            GraphEntity::Edge { .. } => {
                edges.push(entity);
            }
        }
    }

    deduped_nodes.extend(edges);
    deduped_nodes
}
```

## Test Plan

### Rust Integration Tests (codelet/napi/tests/)

1. **Test: Dedup when import target is also walked**
   - Two TS files, one imports the other
   - Run `walk_and_extract()` → assert single File node for imported file
   - Assert File node has full properties (language, lineCount, isTest)

2. **Test: External import targets preserved**
   - TS file imports from "express" (external package)
   - Run extractor → stub File node exists, no collision

3. **Test: Multiple importers → single target node**
   - Three TS files all import same file
   - Run `walk_and_extract()` → assert exactly 1 File node for target

4. **Test: Full graph load succeeds after dedup**
   - Set up fixture with cross-imports
   - Run `walk_and_extract()` → load into nanograph → assert no error

### E2E Tests (tui-test)

5. **Test: ast_index via GraphSearch tool doesn't error**
   - Boot app → enter session → trigger ast_index → verify no error messages in TUI

## Files Changed

| File | Change |
|------|--------|
| `codelet/napi/src/graph/ast_pipeline/mod.rs` | Add `deduplicate_entities()` function; call it at end of `walk_and_extract()` |
| `codelet/napi/tests/ast_extraction_pipeline_test.rs` | Add 4 new integration test scenarios |
| `e2e/ast-dedup.test.ts` | New E2E test for ast_index via TUI |
| `spec/features/ast-entity-deduplication.feature` | New feature file with scenarios |
