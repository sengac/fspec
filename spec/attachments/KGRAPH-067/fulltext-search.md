# KGRAPH-067: Full-Text and Content Search Within Graph

## Dependency

**Requires KGRAPH-063** (Source Code and Metadata Storage) — ✅ DONE. Source, docstring, parameters, decorators are now stored on Function/Type nodes.

## Current State

`ast_search` already has `AST_SEARCHABLE_FIELDS` that includes all 8 fields:
```rust
pub const AST_SEARCHABLE_FIELDS: &[&str] = &[
    "name", "slug", "path", "qualifiedName", "source", "docstring", "parameters", "decorators",
];
```

**Problem**: This is always-on — searching for "User" matches function names, file paths, AND source code. Agents can't control what they're searching.

## CGC Reference Implementation

### Multi-strategy search — `code_finder.py` lines 181–229

CGC has separate search methods with distinct relevance scores:
- `find_by_function_name()` — score 0.9 (name match)
- `find_by_class_name()` — score 0.8 (name match)
- `find_by_variable_name()` — score 0.7 (name match)
- `find_by_content()` — score 0.6 (source/docstring match)

Results are ranked and combined in `find_related_code()`.

### Dedicated filters — `code_finder.py` lines 231–281

- `find_functions_by_decorator(decorator_name)` — exact match on decorators list
- `find_functions_by_argument(argument_name)` — match via HAS_PARAMETER edges

## Implementation Plan

### 1. Add `search_mode` parameter to `AstSearch` (types.rs)

```rust
AstSearch {
    query: String,
    entity_type: Option<String>,
    limit: Option<usize>,
    path: Option<String>,
    search_mode: Option<String>,  // NEW: "name" (default), "content", "all"
    decorator: Option<String>,     // NEW: filter by decorator
    parameter: Option<String>,     // NEW: filter by parameter name
}
```

### 2. Define field lists per search mode (dispatch_helpers.rs)

```rust
pub const AST_NAME_FIELDS: &[&str] = &["name", "slug", "path", "qualifiedName"];
pub const AST_CONTENT_FIELDS: &[&str] = &["source", "docstring"];
pub const AST_ALL_FIELDS: &[&str] = &[
    "name", "slug", "path", "qualifiedName", "source", "docstring", "parameters", "decorators",
];
```

### 3. Add decorator/parameter filter predicates (ast_dispatch.rs)

```rust
fn matches_decorator(item: &Value, decorator: &str) -> bool {
    // Case-insensitive, strip leading @/# for matching
}

fn matches_parameter(item: &Value, parameter: &str) -> bool {
    // Case-insensitive contains on comma-separated parameter names
}
```

### 4. Update tool definition (mod.rs)

Add `search_mode`, `decorator`, `parameter` to JSON schema properties.

### Files to modify

| File | Change |
|------|--------|
| `codelet/tools/src/graph_search/types.rs` | Add search_mode, decorator, parameter fields to AstSearch |
| `codelet/napi/src/graph/dispatch_helpers.rs` | Add per-mode field lists, decorator/parameter matchers |
| `codelet/napi/src/graph/ast_dispatch.rs` | Use search_mode to select fields, apply filters |
| `codelet/napi/src/graph_search_handler.rs` | Pass new params through to dispatch |
| `codelet/tools/src/graph_search/mod.rs` | Update tool definition JSON schema |

### Effort estimate

**5 points** — Straightforward extension of existing search infrastructure. No new modules needed.
