# AST Research — KGRAPH-027 Dead Code Detection

## Current Dispatch Functions (ast_dispatch.rs)
- `dispatch_ast_search()` — entity search by name/pattern
- `dispatch_ast_neighbors()` — per-node neighbor traversal
- `dispatch_ast_stats()` — node/edge counts
- `dispatch_ast_index()` — walk + extract + load

## GraphSearchAction Enum (types.rs)
- AstSearch, AstNeighbors, AstStats, AstIndex
- Need to add: `AstDeadCode { entity_type: Option<String> }`

## AST Extractor Functions
- `extract_typescript()` in ast_ts_extractor.rs — currently emits File, Function, Contains, Imports
- `extract_rust()` in ast_rust_extractor.rs — emits File, Function, Type, Contains, ContainsType
- `walk_and_extract()` in mod.rs — orchestrates extraction with dedup

## Schema (ast-code.pg) — Edges already defined but unpopulated:
- `Calls: Function -> Function` — 0 edges currently
- `TypeRef: Function -> Type` — 0 edges currently
- `Imports: File -> File` — 5,415 edges (working)

## Nanograph Anti-Join Support
- `not { }` clause → `Clause::Negation` → `IROp::AntiJoin` → `AntiJoinExec`
- Fully implemented in nanograph 1.0.0, confirmed in parser.rs, lower.rs, planner.rs
- Test case: `not { $p worksAt $_ }` — anti-join with wildcard

## Implementation Plan
1. Add `extract_calls()` to ast_ts_extractor.rs — parse function bodies for call expressions
2. Add `extract_type_refs()` to ast_ts_extractor.rs — parse function signatures for type annotations
3. Add orphan/dead code queries to ast-queries.gq using `not { }` anti-join
4. Add `dispatch_ast_dead_code()` to ast_dispatch.rs
5. Add `AstDeadCode` variant to GraphSearchAction enum
6. Wire through graph_search_handler.rs dispatch
