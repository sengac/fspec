# AST Research: GraphSearch Integration for AST Queries

## Existing GraphSearch Architecture

### Types (`tools/src/graph_search/types.rs`)
- `GraphSearchAction` enum with 8 variants: Search, Neighbors, Path, Related, Decisions, History, Stats, Index
- `GraphSearchArgs` struct wrapping the action

### Handler (`tools/src/graph_search/handler.rs`)
- Per-session handler map (`HashMap<Uuid, GraphSearchHandler>`)
- `execute_graph_search(session_id, action)` dispatches to registered handler
- Handler type: `Arc<dyn Fn(GraphSearchAction, Uuid) -> String>`

### Dispatch (`napi/src/graph/dispatch.rs`)
- One function per action: `dispatch_search`, `dispatch_neighbors`, `dispatch_path`, etc.
- All use `graph::graph_db_query(GRAPH_QUERIES, query_name, params)` which routes to agent-memory graph
- Queries bundled via `GRAPH_QUERIES` constant

## Integration Plan

### New Types (modify `tools/src/graph_search/types.rs`)
Add 3 new variants to `GraphSearchAction`:
- `AstSearch { query, entity_type, limit }` — search code entities
- `AstNeighbors { node_id, depth, edge_types }` — traverse AST graph  
- `AstStats` — codebase statistics

### New Files
- `napi/schemas/ast-queries.pg` — PG query definitions for AST graph
- `napi/src/graph/ast_dispatch.rs` — dispatch functions for AST actions

### Handler Routing
The main handler in NAPI currently matches all actions against agent-memory dispatch.
Need to match AST-prefixed actions and route to `ast_dispatch` instead.
The handler is registered in `napi/src/graph/` module — will need to handle the routing there.

### AST Queries Needed
```
query search_functions(query: String) — partial name match on Function.name
query search_files(query: String) — partial path match on File.path
query search_types(query: String) — partial name match on Type.name
query ast_neighbors(slug: String) — get all edges from/to a node
query ast_stats() — count nodes and edges by type
```
