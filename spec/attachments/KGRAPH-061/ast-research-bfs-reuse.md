# AST Research: BFS Infrastructure Reuse for Transitive Callers/Callees

## Existing BFS Infrastructure (KGRAPH-060)

### Files and Structure

```
codelet/napi/src/graph/ast_call_chain/
├── mod.rs       — dispatch_ast_call_chain(), build_adjacency_list(), build_structured_chains()
├── bfs.rs       — find_paths() BFS path finder, AdjEntry, CallEdgeInfo
└── snapshot.rs  — GraphSnapshot (pre-fetched function data for O(1) lookups)
```

### Key Components

#### GraphSnapshot (snapshot.rs)
- Pre-fetches all functions via `all_functions` query
- Provides `function_exists()` for O(1) validation
- Provides `known_slugs()` for iteration
- Provides `get_metadata()` for enriching results
- **Fully reusable** for transitive callers/callees

#### build_adjacency_list (mod.rs:98-130)
- Iterates all known slugs
- For each slug, queries `function_calls` to get outgoing Calls edges
- Returns `HashMap<String, Vec<AdjEntry>>` (callee direction)
- **For callers**: Need reverse adjacency — can either:
  - (A) Use `function_callers` query to build reverse adj, or
  - (B) Invert the existing callees adj list
  - Option B is simpler: iterate adj entries and build reverse map

#### BFS (bfs.rs)
- `find_paths()` finds all shortest paths from A to B
- Not directly reusable for "find all reachable" — it targets a specific destination
- **Need new BFS function**: `find_all_reachable()` that collects all nodes reachable from source with depth annotation

### Design Decision: Shared Module Approach

Rather than duplicating code, the plan is:
1. **Reuse** `GraphSnapshot` and `build_adjacency_list` from `ast_call_chain`
2. **Add** a new `find_all_reachable()` to `bfs.rs` (complement to `find_paths()`)
3. **Create** new `ast_transitive/` module for the dispatch functions
4. For callers direction: build a **reverse adjacency list** by inverting the forward one

### Forward vs Reverse Adjacency

Forward (callees): `A→B` means adj[A] contains B
Reverse (callers): `A→B` means reverse_adj[B] contains A

```rust
fn reverse_adjacency_list(adj: &HashMap<String, Vec<AdjEntry>>) -> HashMap<String, Vec<AdjEntry>> {
    // For each entry A→B, create reverse entry B→A
}
```

### Dispatch Pattern (from graph_search_handler.rs)

```rust
GraphSearchAction::AstCallChain { from, to, max_depth } => {
    let db = match get_graph_or_err(...).await { ... };
    graph::ast_call_chain::dispatch_ast_call_chain(&db, &from, &to, max_depth).await
}
```

New entries will follow the same pattern:
```rust
GraphSearchAction::AstCallers { node_id, max_depth, limit } => { ... }
GraphSearchAction::AstCallees { node_id, max_depth, limit } => { ... }
```

### Tool Definition Parameters (from mod.rs)

The `node_id` parameter is already documented for `ast_neighbors`. New actions will share:
- `node_id`: function slug (required)
- `max_depth`: BFS depth limit (optional, default 5)
- `limit`: max results (optional, default 50)

### CGC Output Format Reference

CGC returns flat lists with:
- `caller_name` / `callee_name`
- `caller_file_path` / `callee_file_path`
- `caller_line_number` / `callee_line_number`
- `caller_is_dependency` / `callee_is_dependency`
- Ordered by: is_dependency ASC, file_path, line_number

Our equivalent enriched from GraphSnapshot:
- `slug`, `name`, `qualifiedName`
- `lineStart`, `lineEnd`
- `isAsync`, `isPublic`, `paramCount`
- `depth` (hop distance from source — CGC doesn't have this but our rules require it)

### Summary Format (CGC)

```
"Found {len(results)} direct and indirect callers of '{target}'"
"Found {len(results)} direct and indirect callees of '{target}'"
```
