# AST Research: Full-Text Content Search — KGRAPH-067

## Research Summary

### Current Search Architecture

`dispatch_ast_search` in `ast_dispatch.rs` is a **client-side brute-force filter**:
1. Loads ALL entities of each type from nanograph (`all_functions`, `all_files`, etc.)
2. Calls `matches_fields(item, query, AST_SEARCHABLE_FIELDS)` — case-insensitive substring on 8 fields
3. Applies optional glob path filter
4. Returns up to `max_results` (default 20)

### Current `AST_SEARCHABLE_FIELDS` (dispatch_helpers.rs:59-61)

```rust
pub const AST_SEARCHABLE_FIELDS: &[&str] = &[
    "name", "slug", "path", "qualifiedName", "source", "docstring", "parameters", "decorators",
];
```

**Problem**: Always searches ALL 8 fields — no way to scope to name-only or content-only.

### Current `dispatch_ast_search` Signature (ast_dispatch.rs:132-138)

```rust
pub async fn dispatch_ast_search(
    db: &GraphDatabase,
    query: &str,
    entity_type: Option<&str>,
    limit: Option<usize>,
    path_pattern: Option<&str>,
) -> String
```

### `graph_search_handler.rs` Dispatch Pattern (lines 52-59)

```rust
GraphSearchAction::AstSearch { query, entity_type, limit, path } => {
    let db = match get_graph_or_err(graph::registry::AST_CODE_GRAPH, "ast_search").await {
        Ok(db) => db,
        Err(err_json) => return err_json,
    };
    graph::ast_dispatch::dispatch_ast_search(
        &db, &query, entity_type.as_deref(), limit, path.as_deref(),
    ).await
}
```

### Files to Modify

1. **types.rs** — Add `search_mode`, `decorator`, `parameter` to `AstSearch` variant
2. **graph_search_handler.rs** — Destructure + forward new fields
3. **dispatch_helpers.rs** — Add per-mode field list constants (`AST_NAME_FIELDS`, `AST_CONTENT_FIELDS`)
4. **ast_dispatch.rs** — Accept new params, select field list by mode, apply decorator/parameter post-filters
5. **mod.rs** — Update tool definition JSON schema

### Implementation Approach

**search_mode** — Use different field lists, not different matching functions:
- `"name"` → `["name", "slug", "path", "qualifiedName"]` (default)
- `"content"` → `["source", "docstring"]`
- `"all"` → all 8 fields (current behavior)

**decorator filter** — Post-match predicate: case-insensitive contains on `decorators` field, stripping `@`/`#[`/`]` for cross-language matching.

**parameter filter** — Post-match predicate: case-insensitive contains on `parameters` field.

Both filters work as AND constraints with the query.

### Backward Compatibility

All new fields are `Option<T>` — existing tool calls work unchanged. Default `search_mode` is `"name"` which matches the pre-KGRAPH-063 behavior (when source/docstring didn't exist).
