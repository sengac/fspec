# KGRAPH-069: Portable Graph Bundles — Export/Import

## Problem

Our AST graph is tied to the local `.fspec/graph/ast-code.nano/` directory. There's no way to share a pre-built graph, load one from a teammate, or pre-compute graphs for common libraries. Every new session on a large codebase requires a full re-index.

## CGC Reference Implementation

### Bundle format — `core/cgc_bundle.py`

A `.cgc` file is a ZIP archive containing:
- `metadata.json` — Repository and indexing metadata
- `schema.json` — Graph schema definition
- `nodes.jsonl` — All nodes (one JSON per line)
- `edges.jsonl` — All relationships (one JSON per line)
- `stats.json` — Graph statistics
- `README.md` — Human-readable description

CGC export queries all nodes/edges from Neo4j/FalkorDB, serializes to JSONL, packages as ZIP.
CGC import extracts ZIP, validates, MERGEs nodes/edges into the database.
CGC also has a BundleRegistry for downloading pre-indexed libraries from GitHub Releases.

### Key CGC features implemented:
1. **Export**: Query all nodes → JSONL + edges → JSONL + metadata → ZIP
2. **Import**: Unzip → validate → MERGE into graph (with optional clear_existing)
3. **Zip Slip protection**: Validates entry paths don't escape target directory
4. **Duplicate check**: Rejects import if repository already exists (unless clear_existing=True)

## Our Architecture

### Bundle format (`.astbundle`)

```
my-project.astbundle (ZIP archive)
├── metadata.json       # version, timestamp, languages, entity counts
├── schema.pg           # nanograph schema source for compatibility check
├── entities.jsonl      # all nodes and edges in nanograph JSONL format
```

### Why this format

1. **entities.jsonl** reuses our existing `entities_to_jsonl()` format — nodes as `{"type":"NodeType","data":{...}}`, edges as `{"edge":"EdgeType","from":"slug","to":"slug","data":{...}}`
2. **schema.pg** enables compatibility check — SHA-256 hash comparison before import
3. **metadata.json** provides human-readable context without parsing JSONL

### Export: `export_all_entities()` on GraphDatabase

Read all data from the graph snapshot via direct Arrow storage iteration:

1. **Build node id→slug map**: Iterate all node segments, extract `id` (column 0) and `slug` values
2. **Export nodes**: For each node type → each batch → each row: skip `id` column, read all property columns → `GraphEntity::Node`
3. **Export edges**: For each edge type → use `edge_batch_for_save()` which produces a combined RecordBatch `[id, src, dst, ...props]` → resolve `src`/`dst` IDs to slugs via map → `GraphEntity::Edge`

Arrow column type handling:
- `Utf8/LargeUtf8` → `Value::String`
- `Int32` → `Value::Number`
- `UInt64` → `Value::Number`
- `Boolean` → `Value::Bool`
- `Date64` → `Value::String` (ISO format)
- Null values → skip (don't include in properties map)

### Import: `jsonl_to_entities()` + `load_entities()`

1. Unzip archive to temp directory
2. Validate: check metadata.json exists, compare schema.pg hash
3. Parse `entities.jsonl` → `Vec<GraphEntity>` via new `jsonl_to_entities()` function
4. Load via `load_entities_overwrite()` (default) or `load_entities()` (merge mode)

### JSONL round-trip

The `entities_to_jsonl()` function already serializes `GraphEntity` to JSONL. We need the inverse: `jsonl_to_entities()` that parses JSONL lines back to `Vec<GraphEntity>`.

Node format: `{"type":"Function","data":{"slug":"...","name":"...","isAsync":true}}`
Edge format: `{"edge":"Contains","from":"file_slug","to":"fn_slug","data":{"lineStart":42}}`

### Files to modify

| File | Change |
|------|--------|
| `codelet/napi/src/graph/graph_entities.rs` | Add `export_all_entities()`, `jsonl_to_entities()`, Arrow value reader |
| `codelet/napi/src/graph/database.rs` | Add `export_all_entities()` method |
| `codelet/napi/src/graph_search_handler.rs` | Add `AstExport`/`AstImport` dispatch |
| `codelet/tools/src/graph_search/types.rs` | Add `AstExport`/`AstImport` enum variants |
| `codelet/tools/src/graph_search/mod.rs` | Update tool definition docs + JSON schema |
| `codelet/napi/Cargo.toml` | Add `zip` crate dependency |

### ZIP dependency

Use the `zip` crate (pure Rust) for creating/extracting ZIP archives. It handles DEFLATE compression and path safety.

### Use cases

1. **Team sharing**: Senior dev indexes a large monorepo, exports bundle, teammates load it instantly
2. **CI integration**: Build pipeline exports graph bundle as artifact, agents load it in PR review
3. **Library pre-indexing**: Pre-build graphs for popular frameworks (React, Express, Django)
4. **Backup/restore**: Save graph state before experimental re-indexing
