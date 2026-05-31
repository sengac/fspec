# RPC-092 — AST Research: NAPI Coupling Audit

**Generated:** 2026-05-28
**Tool:** AstGrep over `codelet/napi/src/graph/`

## Query 1: Search for `#[napi]` decorations

```
AstGrep(language='rust', pattern='#[napi]', path='codelet/napi/src/graph')
```

**Result:** No matches found.

**Implication:** The graph module has **zero** `#[napi]` macro decorations. It is
already structurally NAPI-free at the source level. The only thing keeping it
captive inside the `codelet-napi` crate is its module path (`crate::graph::*`),
not any actual NAPI coupling.

## Query 2: Public function inventory

```
AstGrep(language='rust', pattern='pub fn $NAME($$$ARGS) -> $RET { $$$BODY }',
        path='codelet/napi/src/graph')
```

**Result:** 90+ public functions across 52 files. None take or return NAPI types
(`JsBuffer`, `NapiResult`, `AsyncTask`, etc.). All signatures use stdlib
types, `serde_json::Value`, `String`, `Result<T, String>`, or types from
`nanograph` / `super::graph_entities::GraphEntity`.

**Sample (selected from each subdir):**

- `ast_call_chain/snapshot.rs`: `function_exists`, `known_slugs`, `get_metadata`, `get_file_path`
- `ast_call_chain/bfs.rs`: `find_paths`, `find_all_reachable`, `reverse_adjacency`
- `graph_entities.rs`: `entities_to_jsonl`, `jsonl_to_entities`
- `registry.rs`: `is_graph_initialized`, `delete_graph_data`, `resolve_graph_config`
- `database.rs`: `schema_hash`, `path`, `has_node_type`, `stats`, `describe_schema`
- `dispatch_helpers.rs`: `format_graph_stats`, `matches_fields`, `matches_decorator`, `matches_parameter`
- `ast_pipeline/helpers.rs`: `slugify_path`, `build_file_node`, `build_function_node`, `count_params`
- `ast_pipeline/metadata.rs`: `extract_function_meta`, `extract_type_meta`, `extract_docstring`, `extract_decorators`
- All 14 `ast_*_extractor.rs` files: `extract_<lang>(source, rel_path, known_files)` → `Result<Vec<GraphEntity>, String>`
- All 11 `*_dep_extractor.rs` files: `extract_<ecosystem>_dependencies(project_root)` → `Result<Vec<GraphEntity>, String>`

## Conclusion

The graph module is verbatim-liftable. No NAPI types appear in any public
signature. The mechanical lift plan in
[`implementation-plan.md`](implementation-plan.md) is correct: change
`crate::graph::*` → `crate::*`, move files, update `include_str!` paths,
re-test.
