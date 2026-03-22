# AST Research: Dependency Extraction Infrastructure

## Existing AST Pipeline Infrastructure

### `ast_pipeline/helpers.rs` — Shared Helper Functions
- `slugify_path(path)` → slug key format
- `build_file_node(rel_path, file_slug, language, line_count, is_test)` → File GraphEntity
- `build_function_node(...)` → Function GraphEntity  
- `build_contains_edge(file_slug, child_slug, edge_type)` → Edge GraphEntity
- `build_type_node(...)` → Type GraphEntity
- `count_params(text)` → i32
- `count_params_rust(text)` → i32

### `ast_pipeline/mod.rs` — Pipeline Coordinator
- `extract_file(file_path, project_root)` — dispatches to language extractors
- `walk_and_extract(project_root)` — walks directory, collects all entities
- `SUPPORTED_EXTENSIONS` — ts, tsx, js, jsx, mjs, mts, rs
- `SKIP_DIRS` — node_modules, target, dist, .git, .fspec, __pycache__

### GraphEntity Types (from `extractors.rs`)
```rust
pub enum GraphEntity {
    Node { node_type: String, slug: String, properties: Map<String, Value> },
    Edge { edge_type: String, from_slug: String, to_slug: String, properties: Map<String, Value> },
}
```

### AST Code Schema (`schemas/ast-code.pg`)
Already defines:
- `node Dependency { slug @key, name, version?, isDev?, source? }`
- `edge DependsOn: File -> Dependency {}`

## Approach

### New Files Needed
1. `ast_pipeline/npm_dep_extractor.rs` — Parse package.json
2. `ast_pipeline/cargo_dep_extractor.rs` — Parse Cargo.toml

### Reusable from helpers.rs
- `slugify_path()` — not needed for deps (use `dep::` prefix)
- `build_file_node()` — needed to create File nodes for package.json/Cargo.toml
- `build_contains_edge()` — not directly, but DependsOn edge builder can follow same pattern

### New helper needed in helpers.rs
- `build_dependency_node(name, version, is_dev, source)` → GraphEntity
- `build_depends_on_edge(file_slug, dep_slug)` → GraphEntity

### Dependencies in Cargo.toml (codelet/napi)
- `serde` / `serde_json` already available for package.json parsing
- `toml` NOT in napi/Cargo.toml — available in workspace root. Need to add to napi.

### Integration Points
- `mod.rs walk_and_extract()` should be extended to also extract dependency files
- OR a separate `extract_dependencies(project_root)` function is cleaner
