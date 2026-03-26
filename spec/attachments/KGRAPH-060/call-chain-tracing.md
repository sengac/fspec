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
            [node in func_nodes | {{name: node.name, path: node.path, ...}}] as function_chain,
            [rel in call_rels | {{call_line: rel.line_number, ...}}] as call_details,
            length(path) as chain_length
        ORDER BY chain_length ASC
        LIMIT 20
    """
```

**Key design decisions in CGC:**
- Returns the **shortest** chains first (ORDER BY chain_length ASC)
- Returns the **full chain** as an array of function objects, not just start/end
- Includes **call details** (line number, args, full call name) on each hop
- Optional file path disambiguation for both start and end functions
- Default max_depth of 5, configurable up to higher values
- LIMIT 20 prevents explosion on deeply connected graphs

**MCP tool schema** (`tool_definitions.py` line 40–49):
```python
"analyze_code_relationships": {
    "query_type": {"enum": [..., "call_chain", ...]},
    "target": {"description": "For call_chain: use 'start_function->end_function' format"},
    "context": {"description": "Optional: max_depth as string"}
}
```

## What We Need to Implement

### New nanograph query (`.gq` file)

We need a variable-length path traversal query in nanograph's query language. Our current queries are all single-hop `match` clauses. We need something equivalent to:

```
match path {
  $start: Function { slug: $from_slug }
  ($start)-[calls: Calls*1..5]->($end: Function { slug: $to_slug })
}
return path
```

### New GraphSearch action

Add `ast_call_chain` to the `GraphSearchAction` enum in `codelet/tools/src/graph_search/types.rs`:

```rust
AstCallChain {
    from: String,        // source function slug
    to: String,          // target function slug  
    max_depth: Option<u32>, // default 5
}
```

### Dispatch in `codelet/napi/src/graph_search_handler.rs`

Route to a new `ast_call_chain_dispatch()` function that:
1. Resolves both slugs to node IDs
2. Runs the variable-length path query
3. Returns ordered list of chains with intermediate hops

### Our existing edge infrastructure

We already have `Calls` edges populated by all 14 language extractors (KGRAPH-041 through KGRAPH-054). The data is there — we just can't traverse it beyond one hop.

### Files to modify

| File | Change |
|------|--------|
| `codelet/tools/src/graph_search/types.rs` | Add `AstCallChain` variant |
| `codelet/napi/src/graph_search_handler.rs` | Add dispatch case |
| `codelet/napi/src/graph/` | Add new `.gq` query file for path traversal |
| `codelet/napi/src/graph/dispatch_helpers.rs` | Add chain formatting helper |

### Open questions

1. Does nanograph support variable-length path traversal? If not, we may need BFS/DFS in Rust over single-hop queries.
2. Should we return all paths up to max_depth, or just the shortest K?
3. Should we support cross-file chains only, or also intra-file chains?
