# AST Research: Existing Graph Infrastructure — KGRAPH-016

## Singleton Pattern in mod.rs

The current `graph/mod.rs` uses a `lazy_static!` global `Mutex<Option<Database>>` singleton:

```rust
lazy_static::lazy_static! {
    static ref GRAPH_DB: Mutex<Option<Database>> = Mutex::new(None);
}
```

### Public API functions (all operate on the singleton):
- `ensure_graph_db()` — init or open
- `graph_db_stats()` — stats query
- `graph_db_load_jsonl()` — data loading
- `graph_db_query()` — named query execution
- `graph_describe_schema()` — schema inspection
- `is_graph_initialized()` — status check
- `reset_graph_db()` — reset singleton
- `close_graph_db()` — close database

### Problem: Not Reusable
All functions are free-standing and reference a single static `GRAPH_DB`. To support multiple databases (AST + Learnings), we need a `GraphDatabase` struct that encapsulates:
1. The `Database` handle
2. The path
3. The schema source
4. The query source

### Entity Pipeline (entity_pipeline.rs) also uses lazy_static:
```rust
lazy_static::lazy_static! {
    static ref PENDING_ENTITIES: Mutex<Vec<GraphEntity>> = Mutex::new(Vec::new());
}
```
This is specific to the old per-tool-call extraction and should NOT be carried into the new abstraction.

## Existing Data Types (extractors.rs)

The `GraphEntity` enum represents any node/edge:
```rust
pub enum GraphEntity {
    Node { node_type: String, slug: String, properties: serde_json::Map<String, Value> },
    Edge { edge_type: String, from_slug: String, to_slug: String, properties: serde_json::Map<String, Value> },
}
```
This is a good abstraction — reusable for both AST and Learnings graphs.

## Merge Logic (merge.rs)

`entities_to_jsonl()` converts `Vec<GraphEntity>` to JSONL format. Has special merge logic for:
- Concept `mentionCount` (increment)
- Concept `firstSeen`/`lastSeen` (min/max)
- Concept `confidence` (promote only)
- RelatesTo `coOccurrenceCount` (increment)
- RelatesTo `strength` (recalculate)

The JSONL conversion is reusable. The merge logic is specific to agent-memory schema types.

## Schema (agent-memory.pg)

Defines 6 node types and 10 edge types. The AST schema needs entirely different types.

## Key Findings for Refactoring

1. **Extract `GraphDatabase` struct** from `mod.rs` singleton pattern
2. **Keep `GraphEntity`** enum from `extractors.rs` — it's schema-agnostic
3. **Keep JSONL conversion** from `merge.rs` — separate from merge rules
4. **New file: `ast-code.pg`** — new schema for AST nodes/edges
5. **Two singletons** (or a registry): one for AST graph, one for old/learnings graph
6. **Batch load function** on `GraphDatabase` that takes `Vec<GraphEntity>` and converts+loads in one call
