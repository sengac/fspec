# AST Research: Export/Import Infrastructure

## Findings

### entities_to_jsonl format
- Nodes: `{"type":"NodeType","data":{...properties including slug...}}`
- Edges: `{"edge":"EdgeType","from":"slug","to":"slug","data":{...}}`
- The slug field is NOT serialized separately — it lives inside data
- This is nanograph's native JSONL ingest format

### GraphDatabase loading methods
- `load_entities(&[GraphEntity])` — Merge mode (upserts by @key slug)
- `load_entities_overwrite(&[GraphEntity])` — Overwrite mode (full replacement)
- Both go through: entities_to_jsonl() → db.load() / db.load_with_mode()

### Arrow type mapping for export
- String/String? → Utf8 (nullable if ?)
- I32/I32? → Int32
- Bool/Bool? → Boolean
- DateTime → Date64
- enum(...) → Utf8 (stored as string)
- Internal `id` column → UInt64 (prepended by nanograph, skip during export)

### Graph storage structure
- `storage.node_segments[type_name].batches` — Arrow RecordBatches with [id, ...props]
- `storage.edge_segments[type_name]` — flat src_ids/dst_ids vectors + property batches
- `edge_batch_for_save(type_name)` — produces combined [id, src, dst, ...props] batch

### Registry pattern
- `get_graph("ast-code")` → lazy init singleton
- Schema check via SHA-256 hash of schema.pg source
- `delete_graph_data()` → removes from HashMap + deletes .nano/ directory

### Dispatch pattern
- Each action delegates to a `dispatch_*` function in a dedicated module
- `get_graph_or_err()` helper resolves named graphs, wrapping errors
- AstIndex is special (manages lifecycle before loading)
