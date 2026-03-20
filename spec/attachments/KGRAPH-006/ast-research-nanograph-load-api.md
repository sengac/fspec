# AST Research: Nanograph Load & Merge API

## Key API Methods (vendor/nanograph/crates/nanograph/src/store/database/persist.rs)

### `db.load(data_source: &str)` — Auto-detect mode
- Auto-selects `LoadMode::Merge` if schema has `@key` properties
- Auto-selects `LoadMode::Overwrite` otherwise

### `db.load_with_mode(data_source: &str, mode: LoadMode)` — Explicit mode
- Locks writer, applies mutation plan

### `db.apply_merge_mutation(data_source: &str, op_summary: &str)` — Direct merge
- Shortcut for merge-mode load with custom operation summary

### LoadMode enum
```rust
pub enum LoadMode {
    Overwrite,
    Append,
    Merge,
}
```

## JSONL Format (from schema research)

### Node format:
```jsonl
{"type":"Concept","data":{"slug":"jwt-auth","name":"JWT Authentication","category":"pattern","summary":"...","mentionCount":1,"firstSeen":"2026-03-19T...","lastSeen":"2026-03-19T...","confidence":"high"}}
```

### Edge format:
```jsonl
{"edge":"RelatesTo","from":"jwt-auth","to":"session-mgmt","data":{"strength":0.85,"relationType":"uses","firstSeen":"2026-03-19T...","lastSeen":"2026-03-19T...","coOccurrenceCount":1}}
```

## Merge Semantics
- When `@key` field matches existing node → updates properties (full overwrite at Lance level)
- For increment/min/max semantics → need read-before-write pattern in Rust
- `db.run_query()` for reads, compose merged properties, then `db.load_with_mode()` for writes

## Query API
- `db.run(query_source, query_name, params)` → RunResult
- `db.run_json(query_source, query_name, params, mode)` → RunResult  
- `db.prepare_read_query(query)` → PreparedReadQuery → `.execute(params)` → QueryResult

## GraphEntity types (from extractors.rs)
```rust
pub enum GraphEntity {
    Node { node_type: String, slug: String, properties: Map<String, Value> },
    Edge { edge_type: String, from_slug: String, to_slug: String, properties: Map<String, Value> },
}
```
