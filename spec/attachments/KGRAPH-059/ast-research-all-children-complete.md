# KGRAPH-059: AST Research — All Children Complete

## Summary

All 10 child cards (KGRAPH-060 through KGRAPH-069) are complete. This parent card tracks the overall GraphSearch Enhancement feature parity with CodeGraphContext.

## Dispatch Functions Implemented

From `codelet/napi/src/graph/`:

| Function | Module | Child Card |
|----------|--------|------------|
| `dispatch_ast_call_chain` | ast_call_chain/mod.rs | KGRAPH-060 |
| `dispatch_ast_callers` | ast_transitive.rs | KGRAPH-061 |
| `dispatch_ast_callees` | ast_transitive.rs | KGRAPH-061 |
| `dispatch_ast_complexity` | ast_complexity.rs | KGRAPH-062 |
| `dispatch_ast_search` (metadata+fulltext+decorators) | ast_dispatch.rs | KGRAPH-063/067/068 |
| `dispatch_ast_neighbors` | ast_dispatch.rs | Existing |
| `dispatch_ast_stats` (variable counts) | ast_dispatch.rs | KGRAPH-066 |
| `dispatch_ast_dead_code` | ast_dead_code.rs | Existing |
| `dispatch_ast_index` (incremental support) | ast_index.rs | KGRAPH-065 |
| Export/Import in database.rs | database.rs | KGRAPH-069 |

## CGC Parity Coverage

Original CGC code_finder.py had ~30 methods. Our implementation covers 15+ query patterns:
- ✅ find_function_call_chain → ast_call_chain
- ✅ find_all_callers → ast_callers  
- ✅ find_all_callees → ast_callees
- ✅ find_class_hierarchy → ast_neighbors (type_extends/type_implements)
- ✅ get_cyclomatic_complexity → ast_complexity
- ✅ find_most_complex_functions → ast_complexity (top-N)
- ✅ find_by_content → ast_search search_mode=content
- ✅ find_functions_by_decorator → ast_search decorator filter
- ✅ find_by_variable_name → ast_search entity_type=Variable
- ✅ find_dead_code → ast_dead_code
- ✅ Source/docstring/metadata storage → metadata pipeline
- ✅ Variable/symbol tracking → Variable node type
- ✅ Incremental re-indexing → ast_index incremental
- ✅ Portable bundles → ast_export/ast_import
