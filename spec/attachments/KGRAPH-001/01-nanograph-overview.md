# Nanograph Overview — Deep Research Findings

## What Is Nanograph?

**Nanograph is an embedded, local-first, typed property graph database written in Rust.** Think "SQLite for graphs." It runs entirely on-device with zero server infrastructure — no Docker, no cloud, no setup. It stores data in a single `.nano/` directory, supports ACID transactions with time-travel, and enforces a schema at compile time through two custom DSLs.

**Core tagline:** *On-device typed property graph DB for agents and humans. One CLI. One folder. Schema-as-code. No server.*

**Built on:** Rust, Apache Arrow (columnar in-memory format), Lance (columnar storage engine), and DataFusion (query execution engine).

**Target use cases:**
- Context graphs and decision traces for AI agents
- Agentic memory with typed, sub-100ms local queries
- Personal knowledge graphs with schema enforcement
- Dependency and lineage modeling
- Feature generation for ML pipelines

## Why Nanograph Is an Ideal Fit

1. **Already has a Node.js SDK** (`nanograph-ts` via napi-rs, npm package `nanograph-db`)
2. **In-memory mode** — `Database.openInMemory(schema)` for ephemeral/test use
3. **File-backed persistence** — Lance-based `.nano/` directory for durable storage
4. **Typed schema** — Schema-as-code enforced at query time (21 type rules)
5. **Graph traversals** — Datalog-flavored query language with bounded expansion
6. **Vector search** — `@embed` annotations, `nearest()` for cosine similarity
7. **CDC (Change Data Capture)** — Built-in audit trail for all mutations
8. **ACID transactions** with time-travel via Lance versioning
9. **C FFI + Swift wrapper** — Already exists for cross-platform embedding
10. **Sub-millisecond opens** — Lance storage optimized for local access

## Storage Layout

```
<name>.nano/
├── schema.pg              # Source schema (human-readable)
├── schema.ir.json         # Compiled SchemaIR (serialized, deterministic)
├── graph.manifest.json    # Dataset inventory (which types have data)
├── _tx_catalog.jsonl      # Transaction log (append-only)
├── _cdc_log.jsonl         # CDC event log (append-only)
├── _embedding_cache.jsonl # Content-hashed embedding cache
├── nodes/<type_id_hex>/   # Lance dataset per node type
└── edges/<type_id_hex>/   # Lance dataset per edge type
```

**Type IDs:** FNV-1a hash of `"node:TypeName"` or `"edge:TypeName"` → u32, rendered as 8-digit hex directory names.

## Performance Characteristics

- **Lance columnar storage** — Sub-millisecond dataset opens, ACID, versioned
- **Arrow in-memory format** — Zero-copy, columnar batch processing
- **DataFusion query engine** — Optimized physical plans with filter pushdown
- **CSR/CSC adjacency indices** — Compressed Sparse Row/Column for O(1) neighbor access
- **Scalar indexes** on `@index`/`@unique` annotated properties
- **Embedding cache** — Content-hashed, avoids redundant API calls
