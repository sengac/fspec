# AST Research: Graph Types for LLM Extraction

## GraphEntity enum (extractors.rs:13)
```rust
pub enum GraphEntity {
    Node { node_type: String, slug: String, properties: Map<String, Value> },
    Edge { edge_type: String, from_slug: String, to_slug: String, properties: Map<String, Value> },
}
```

## EntityQueue (extractors.rs:164)
```rust
pub struct EntityQueue { buffer: Vec<GraphEntity>, threshold: usize }
```

## Key integration points:
- LLM extraction output MUST produce Vec<GraphEntity> to be compatible with structural extractors
- Properties are serde_json::Map<String, Value> — flexible for any schema fields
- Node slug is the primary key for upsert logic (KGRAPH-006)
- Edge types must match schema: Mentions, Discusses, RelatesTo, etc.
