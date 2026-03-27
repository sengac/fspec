# AST Research: Class Hierarchy Traversal Infrastructure

## Existing Edge Types (ast-queries.gq)

| Edge | Direction | Query |
|------|-----------|-------|
| `Extends` | Type → Type | `type_extends($slug)` — finds parents |
| `Implements` | Type → Type | `type_implements($slug)` — finds interfaces |
| `ContainsType` | File → Type | `type_container($slug)` — finds containing file |
| `Contains` | File → Function | `file_functions($slug)` — finds functions in file |

## Gap: No Type→Function containment

CGC has `(Class)-[:CONTAINS]->(Function)` for methods. Our schema only has:
- `File→Contains→Function` (all functions in file)
- `File→ContainsType→Type` (types in file)

**Workaround**: For method listing, find the file containing the type, then return all functions in that file. This is approximate (works for single-class files, which is common). Noted as limitation in response.

## New Queries Needed

1. `type_extended_by($slug)` — reverse Extends: find types that extend a given type (children)
2. `type_implemented_by($slug)` — reverse Implements: find types that implement a given interface

## BFS Approach

Same iterative BFS pattern as KGRAPH-060/061 but over Extends edges instead of Calls:
- **Parents**: BFS up via `type_extends` queries (each hop finds parent, then recurse)
- **Children**: BFS down via `type_extended_by` queries (each hop finds children, then recurse)
- Depth annotation on each node

## Implementation Plan

1. Add `type_extended_by` and `type_implemented_by` queries to `ast-queries.gq`
2. Add `AstHierarchy` variant to `GraphSearchAction` in types.rs
3. Create `ast_hierarchy.rs` with `dispatch_ast_hierarchy` function
4. Wire dispatch in `graph_search_handler.rs`
5. Update tool definition in `mod.rs`

## Key Files

| File | Role |
|------|------|
| `codelet/tools/src/graph_search/types.rs` | Action enum |
| `codelet/napi/src/graph_search_handler.rs` | Dispatch routing |
| `codelet/napi/schemas/ast-queries.gq` | Nanograph queries |
| `codelet/napi/src/graph/ast_dispatch.rs` | Existing neighbor queries (reference) |
| `codelet/napi/src/graph/ast_transitive.rs` | Similar BFS pattern (reference) |
