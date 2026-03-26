# KGRAPH-061: Transitive Callers / Callees (Multi-Hop Traversal)

## Problem

Our `ast_neighbors` action declares a `depth` parameter but it's documented as "reserved" and unimplemented. All neighbor queries are strictly single-hop. Agents can't answer "what will break if I change this function?" without knowing ALL transitive callers.

## CGC Reference Implementation

### find_all_callers — `code_finder.py` lines 576–604

```python
def find_all_callers(self, function_name: str, path=None, repo_path=None) -> List[Dict]:
    """Find all direct and indirect callers of a specific function."""
    # Uses variable-length path traversal [:CALLS*]
    query = f"""
        MATCH p = (f:Function)-[:CALLS*]->()
        WITH f, p, nodes(p) as path_nodes
        WITH f, path_nodes, list_extract(path_nodes, size(path_nodes)) as target
        WHERE target.name = $function_name AND target.path = $path {repo_filter}
        RETURN DISTINCT f.name AS caller_name, f.path AS caller_file_path,
               f.line_number AS caller_line_number, f.is_dependency AS caller_is_dependency
        ORDER BY caller_is_dependency ASC, caller_file_path, caller_line_number
        LIMIT 50
    """
```

### find_all_callees — `code_finder.py` lines 606–636

```python
def find_all_callees(self, function_name: str, path=None, repo_path=None) -> List[Dict]:
    """Find all direct and indirect callees of a specific function."""
    query = f"""
        MATCH (caller:Function {{name: $function_name}})
        MATCH p = (caller)-[:CALLS*]->()
        WITH p, nodes(p) as path_nodes
        WITH list_extract(path_nodes, size(path_nodes)) as f
        {repo_filter}
        RETURN DISTINCT f.name AS callee_name, f.path AS callee_file_path, ...
        LIMIT 50
    """
```

### MCP Integration — `code_finder.py` lines 899–911

Both are exposed via the unified `analyze_code_relationships` tool:
```python
elif query_type == "find_all_callers":
    results = self.find_all_callers(target, context, repo_path=repo_path)
    return {
        "summary": f"Found {len(results)} direct and indirect callers of '{target}'"
    }
elif query_type == "find_all_callees":
    results = self.find_all_callees(target, context, repo_path=repo_path)
```

## What We Need to Implement

### Option A: Implement `depth` on `ast_neighbors`

Our existing `ast_neighbors` in `codelet/napi/src/graph_search_handler.rs` runs 12 separate per-edge-type queries. To support depth > 1:

1. For each edge type, run iterative BFS up to `depth` hops
2. Accumulate results with hop distance annotation
3. Deduplicate across hops

This is general-purpose but expensive for large depths.

### Option B: Dedicated `ast_transitive_callers` / `ast_transitive_callees` actions

More targeted — only traverses `Calls` edges (the most useful case).

Add to `codelet/tools/src/graph_search/types.rs`:

```rust
AstTransitiveCallers {
    node_id: String,     // function slug
    max_depth: Option<u32>, // default: unlimited (or 10)
    limit: Option<u32>,     // default: 50
}

AstTransitiveCallees {
    node_id: String,
    max_depth: Option<u32>,
    limit: Option<u32>,
}
```

### Nanograph query needs

If nanograph supports variable-length patterns like `[:Calls*]`, this maps directly. If not, implement iterative BFS in Rust:

```rust
fn transitive_callers(db: &GraphDatabase, start_slug: &str, max_depth: u32) -> Vec<Entity> {
    let mut visited = HashSet::new();
    let mut frontier = vec![start_slug.to_string()];
    let mut results = vec![];
    
    for depth in 0..max_depth {
        let mut next_frontier = vec![];
        for slug in &frontier {
            // Run single-hop callers query
            let callers = db.query("callers_of", &[("slug", slug)])?;
            for caller in callers {
                if visited.insert(caller.slug.clone()) {
                    results.push(caller.with_depth(depth + 1));
                    next_frontier.push(caller.slug.clone());
                }
            }
        }
        frontier = next_frontier;
        if frontier.is_empty() { break; }
    }
    results
}
```

### Relationship to KGRAPH-060

KGRAPH-060 (call chain) finds a path **between two specific functions**. This card finds **all** functions reachable from one function in a given direction. They share the need for multi-hop Calls traversal but serve different purposes:
- Call chain: "How does A reach B?" (path finding)
- Transitive: "What all depends on A?" (reachability)

### Files to modify

| File | Change |
|------|--------|
| `codelet/tools/src/graph_search/types.rs` | Add action variants or implement `depth` |
| `codelet/napi/src/graph_search_handler.rs` | Add dispatch + BFS logic |
| `codelet/napi/src/graph/` | Add nanograph queries or Rust-side BFS |

### Our existing infrastructure

- `Calls` edges: Already populated by KGRAPH-041–054 across all 14 languages
- `ast_neighbors` already has the `CalledBy` single-hop query — this is the building block for BFS
- `ast_dead_code` already uses `not { $caller calls $fn }` — proving our Calls edges work
