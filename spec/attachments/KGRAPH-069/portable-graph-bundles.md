# KGRAPH-069: Portable Graph Bundles — Export/Import

## Problem

Our AST graph is tied to the local `.fspec/graph/ast-code.nano/` directory. There's no way to share a pre-built graph, load one from a teammate, or pre-compute graphs for common libraries. Every new session on a large codebase requires a full re-index.

## CGC Reference Implementation

### Bundle format — `docs/BUNDLES.md`

A `.cgc` file is a ZIP archive:
```
numpy.cgc
├── metadata.json       # Repository and indexing metadata
├── schema.json         # Graph schema definition
├── nodes.jsonl         # All nodes (one JSON per line)
├── edges.jsonl         # All relationships (one JSON per line)
├── stats.json          # Graph statistics
└── README.md           # Human-readable description
```

### metadata.json format

```json
{
  "cgc_version": "0.1.0",
  "exported_at": "2026-01-13T22:00:00",
  "repo": "numpy/numpy",
  "commit": "a1b2c3d4",
  "languages": ["python", "c"],
  "format_version": "1.0"
}
```

### Bundle implementation — `core/cgc_bundle.py` (~31K chars, entire file)

The bundle system handles:
1. **Export**: Query all nodes/edges from graph DB → serialize to JSONL → package as ZIP
2. **Import**: Unzip → parse JSONL → MERGE into graph DB
3. **Load from registry**: Check local cache → download from GitHub Releases → import

Key classes:
- `CGCBundleExporter` — walks graph, serializes nodes/edges, writes ZIP
- `CGCBundleImporter` — reads ZIP, deserializes, MERGEs into DB
- `BundleRegistry` — searches/downloads from remote registry

### Export implementation — `cgc_bundle.py` (export section)

```python
def export_bundle(self, output_path, repo_path=None, include_stats=True):
    # 1. Query all nodes
    nodes = session.run("MATCH (n) RETURN n, labels(n) as labels")
    
    # 2. Write to JSONL
    with open(nodes_file, 'w') as f:
        for record in nodes:
            node_data = dict(record['n'])
            node_data['_labels'] = record['labels']
            node_data['_id'] = record['n'].element_id
            f.write(json.dumps(node_data) + '\n')
    
    # 3. Query all edges
    edges = session.run("MATCH (a)-[r]->(b) RETURN ...")
    
    # 4. Package as ZIP
    with zipfile.ZipFile(output_path, 'w', zipfile.ZIP_DEFLATED) as zf:
        zf.write(nodes_file, 'nodes.jsonl')
        zf.write(edges_file, 'edges.jsonl')
        zf.write(metadata_file, 'metadata.json')
```

### Import implementation — `cgc_bundle.py` (import section)

```python
def import_bundle(self, bundle_path, clear_existing=False):
    if clear_existing:
        session.run("MATCH (n) DETACH DELETE n")
    
    # Read nodes.jsonl, create nodes via MERGE
    for line in nodes_file:
        node = json.loads(line)
        labels = ':'.join(node['_labels'])
        session.run(f"MERGE (n:{labels} {{name: $name, path: $path}}) SET n += $props",
                    **node)
    
    # Read edges.jsonl, create relationships
    for line in edges_file:
        edge = json.loads(line)
        session.run(f"MATCH (a {{...}}), (b {{...}}) MERGE (a)-[:{edge['type']}]->(b)")
```

### Bundle registry — `core/bundle_registry.py` (7.4K chars)

```python
class BundleRegistry:
    REGISTRY_URL = "https://api.github.com/repos/CodeGraphContext/CodeGraphContext/releases"
    CACHE_DIR = "~/.codegraphcontext/bundles/"
    
    def search(self, query):
        """Search available bundles by name/description."""
        
    def download(self, bundle_name, output_dir=None):
        """Download bundle from GitHub Releases."""
        
    def list_available(self, unique_only=False):
        """List all available bundles in registry."""
```

### MCP tools — `tool_definitions.py` lines 166–186

```python
"load_bundle": {
    "description": "Load a pre-indexed .cgc bundle into the database.",
    "inputSchema": {
        "properties": {
            "bundle_name": {"type": "string"},
            "clear_existing": {"type": "boolean", "default": False}
        }
    }
}

"search_registry_bundles": {
    "description": "Search for available pre-indexed bundles in the registry.",
    "inputSchema": {
        "properties": {
            "query": {"type": "string"},
            "unique_only": {"type": "boolean", "default": False}
        }
    }
}
```

## What We Need to Implement

### Phase 1: Local export/import

#### Export

New GraphSearch action `ast_export`:

```rust
AstExport {
    output_path: String,     // e.g., "my-project.astgraph"
    include_stats: Option<bool>,
}
```

Implementation:
1. Use `GraphDatabase::query()` to fetch all nodes and edges
2. Serialize to JSONL (we already have `GraphEntity` enum + JSONL serialization in `graph_entities.rs`)
3. Include nanograph schema IR
4. Package as ZIP with metadata

#### Import

New GraphSearch action `ast_import`:

```rust
AstImport {
    input_path: String,
    merge_mode: Option<String>,  // "merge" or "overwrite"
}
```

Implementation:
1. Unzip archive
2. Validate schema compatibility
3. Use `GraphDatabase::load_entities()` or `load_entities_overwrite()` to ingest

### Phase 2: Registry (future)

A registry of pre-indexed popular libraries. Could be:
- GitHub Releases (like CGC)
- A simple JSON manifest file hosted on a CDN
- npm packages containing `.astgraph` bundles

### Our existing infrastructure that helps

- `graph_entities.rs` already has `GraphEntity` enum with Node/Edge variants
- `GraphEntity` already has JSONL serialization
- `GraphDatabase::load_entities()` supports both Merge and Overwrite modes
- `GraphDatabase::stats()` gives node/edge counts for stats.json

### Bundle format proposal (`.astgraph`)

```
my-project.astgraph
├── metadata.json       # version, repo, commit, languages, timestamp
├── schema.ir.json      # nanograph schema IR for compatibility check
├── entities.jsonl      # all nodes and edges (same format as load_entities)
└── stats.json          # node/edge type counts
```

We can reuse the existing JSONL format from `graph_entities.rs` directly — no new serialization needed.

### Files to modify

| File | Change |
|------|--------|
| `codelet/tools/src/graph_search/types.rs` | Add `AstExport` and `AstImport` variants |
| `codelet/napi/src/graph_search_handler.rs` | Add dispatch |
| `codelet/napi/src/graph/database.rs` | Add export method (query all → JSONL) |
| `codelet/napi/src/graph/graph_entities.rs` | Ensure JSONL round-trips correctly |

### Effort estimate

**Medium** — Export is straightforward (query + serialize + zip). Import already works via `load_entities()`. The main work is the ZIP packaging, metadata format, and schema compatibility validation.

### Use cases

1. **Team sharing**: Senior dev indexes a large monorepo, exports bundle, teammates load it instantly
2. **CI integration**: Build pipeline exports graph bundle as artifact, agents load it in PR review
3. **Library pre-indexing**: Pre-build graphs for popular frameworks (React, Express, Django, etc.)
4. **Backup/restore**: Save graph state before experimental re-indexing
