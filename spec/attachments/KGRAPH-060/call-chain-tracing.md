# KGRAPH-060: Call Chain / Path Tracing Between Two Functions

## Problem

Our `ast_neighbors` action only returns single-hop neighbors. There is no way to ask "how does function A reach function B?" — the most common question during impact analysis or debugging unfamiliar code.

## CGC Reference Implementation

**File**: `src/codegraphcontext/tools/code_finder.py` lines 638–682

```python
def find_function_call_chain(self, start_function: str, end_function: str, 
                              max_depth: int = 5, start_file=None, end_file=None,
                              repo_path=None) -> List[Dict]:
    """Find call chains between two functions"""
    # Uses Cypher variable-length path: [:CALLS*1..{max_depth}]
    query = f"""
        MATCH (start:Function {start_props}), (end_target:Function {end_props})
        WITH start, end_target
        MATCH path = (start)-[:CALLS*1..{max_depth}]->()
        WITH path, end_target, nodes(path) as func_nodes, relationships(path) as call_rels
        WITH path, func_nodes, call_rels, list_extract(func_nodes, size(func_nodes)) as path_end
        WHERE path_end.name = end_target.name
        RETURN 
            [node in func_nodes | {{
                name: node.name,
                path: node.path,
                line_number: node.line_number,
                is_dependency: node.is_dependency
            }}] as function_chain,
            [rel in call_rels | {{
                call_line: rel.line_number,
                args: rel.args,
                full_call_name: rel.full_call_name
            }}] as call_details,
            length(path) as chain_length
        ORDER BY chain_length ASC
        LIMIT 20
    """
```

### CGC Key Design Decisions
- Returns the **shortest** chains first (`ORDER BY chain_length ASC`)
- Returns the **full chain** as an array of function objects, not just start/end
- **Each chain has TWO arrays**: `function_chain` (node metadata) AND `call_details` (edge metadata per hop)
- Call details include: `call_line` (line number of the call site), `args`, `full_call_name`
- Default max_depth of 5, configurable up to higher values
- `LIMIT 20` prevents explosion on deeply connected graphs

### CGC Handler Response Format
**File**: `src/codegraphcontext/tools/handlers/analysis_handlers.py` line 61–89
**File**: `src/codegraphcontext/tools/code_finder.py` lines 913–922

```python
# Handler wraps result in standard envelope:
return {
    "success": True,
    "query_type": "call_chain",
    "target": target,
    "results": results,  # list of dicts from find_function_call_chain
}

# analyze_code_relationships adds summary:
return {
    "query_type": "call_chain",
    "target": target,
    "results": results,
    "summary": f"Found {len(results)} call chains from '{start_func}' to '{end_func}' (max depth: {max_depth})"
}
```

### CGC MCP Tool Schema
**File**: `tool_definitions.py` lines 40–49

```python
"analyze_code_relationships": {
    "query_type": {"enum": [..., "call_chain", ...]},
    "target": {"description": "For call_chain: use 'start_function->end_function' format"},
    "context": {"description": "Optional: max_depth as string"}
}
```

## Our Implementation Mapping

### CGC → fspec Mapping Table

| CGC Concept | fspec Equivalent | Notes |
|-------------|------------------|-------|
| `start_function` / `end_function` | `from` / `to` slugs | We use slugs instead of names |
| `max_depth` (default 5) | `max_depth: Option<u32>` (default 5) | Same semantics |
| `start_file` / `end_file` | Not needed | Slugs are already file-qualified |
| `repo_path` | Not needed | Single-graph-per-project model |
| `function_chain` array | `function_chain` in each chain object | Node metadata per hop |
| `call_details` array | `call_details` in each chain object | Edge metadata per hop |
| `chain_length` | `chain_length` integer | Number of hops |
| `ORDER BY chain_length ASC` | `sort_by_key(\|p\| p.len())` | Shortest first |
| `LIMIT 20` | `MAX_CHAINS = 20` | Same cap |
| `summary` string | `summary` field | Human-readable count |
| `[:CALLS*1..{max_depth}]` | BFS in Rust | nanograph lacks variable-length paths |

### Schema: Our Calls Edge Properties

```
edge Calls: Function -> Function {
    callCount: I32?
    isConditional: Bool?
}
```

CGC has `line_number`, `args`, `full_call_name` on edges. We have `callCount` and `isConditional`. Our `call_details` returns what our schema supports.

### Files Modified

| File | Change |
|------|--------|
| `codelet/tools/src/graph_search/types.rs` | `AstCallChain` variant with `from`, `to`, `max_depth` |
| `codelet/napi/src/graph_search_handler.rs` | Dispatch case for `AstCallChain` |
| `codelet/napi/src/graph/ast_call_chain.rs` | BFS implementation with `GraphSnapshot` |
| `codelet/napi/src/graph/mod.rs` | `pub mod ast_call_chain` |
| `codelet/tools/src/graph_search/mod.rs` | Tool description with `from`, `to`, `max_depth` params |

### Architecture: BFS with GraphSnapshot

nanograph does not support variable-length path traversal (Cypher `[:CALLS*1..N]`). We implement BFS in Rust:
1. `GraphSnapshot::load()` — single `all_functions` query builds `HashSet` + metadata map
2. `build_adjacency_list()` — per-function `function_calls` queries (N queries, unavoidable)
3. `bfs_find_paths()` — pure BFS over adjacency map, returns slug-only chains
4. `enrich_chains()` — maps slugs to full metadata from snapshot
