# AST Research: GraphSearch dispatch functions and query infrastructure

## Graph dispatch functions (all return String, async)

All AST dispatch functions in `codelet/napi/src/graph/`:
- `dispatch_ast_index(custom_path, reset)` → ast_index.rs
- `dispatch_ast_search(db, query, entity_type, limit, path_pattern)` → ast_dispatch.rs
- `dispatch_ast_neighbors(db, node_id, depth, edge_types)` → ast_dispatch.rs
- `dispatch_ast_stats(db)` → ast_dispatch.rs
- `dispatch_ast_dead_code(db, entity_type, limit, path)` → ast_dead_code.rs

## Pattern for adding new actions:
1. Add variant to `GraphSearchAction` enum in `codelet/tools/src/graph_search/types.rs`
2. Add dispatch case in `graph_search_handler.rs` → `dispatch_action()`
3. Create new `ast_<action>.rs` module under `codelet/napi/src/graph/`
4. Add `.gq` queries to `schemas/ast-queries.gq` if needed
5. Use `get_graph_or_err()` helper for graph access

## Existing .gq queries available for reuse:
- `function_calls($slug)` — outgoing Calls from function
- `function_callers($slug)` — incoming Calls to function
- `type_extends($slug)` — outgoing Extends from type
- `type_implements($slug)` — outgoing Implements from type
- `type_referencing_functions($slug)` — incoming TypeRef to type

## BFS approach needed:
Nanograph doesn't support variable-length paths. All multi-hop traversals must use
iterative BFS in Rust, calling single-hop .gq queries at each level.
