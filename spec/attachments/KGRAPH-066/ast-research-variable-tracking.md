# AST Research: Variable and Symbol Tracking

## Schema Location
- `codelet/napi/schemas/ast-code.pg` — current 4 node types (File, Module, Function, Type) + 1 external (Dependency) + 8 edge types
- `codelet/napi/schemas/ast-queries.gq` — all GQ queries; needs `all_variables`, `file_variables`, `variable_container`

## Existing Builder Pattern (helpers.rs)
All node types created via typed builder functions:
- `build_file_node(rel_path, file_slug, language, line_count, is_test)` → GraphEntity::Node "File"
- `build_function_node(file_slug, name, ...)` → GraphEntity::Node "Function", slug = `{file_slug}::{name}`
- `build_type_node(file_slug, name, ...)` → GraphEntity::Node "Type", slug = `{file_slug}::{name}`
- `build_dependency_node(name, ...)` → GraphEntity::Node "Dependency", slug = `dep::{name}`
- `build_contains_edge(file_slug, child_slug, edge_type)` → "Contains" or "ContainsType"

## Entity Type Routing (ast_dispatch.rs)
`dispatch_ast_search` routes entity_type to query:
- "Function" → "all_functions"
- "File" → "all_files"
- "Type" → "all_types"
- "Dependency" → "all_dependencies"
- None → all 4 types

Need to add: "Variable" → "all_variables" + include in None (all-types) fallback.

## dispatch_helpers.rs `format_graph_stats`
Used by `dispatch_ast_stats` — must include Variable count.

## Neighbor Queries
NEIGHBOR_QUERIES in ast_dispatch.rs needs:
- `file_variables` — File → Variable (outgoing ContainsVariable)
- `variable_container` — Variable ← File (incoming ContainsVariable)

## Per-Language Variable Extraction Patterns
Each extractor in `codelet/napi/src/ast_pipeline/` uses ast-grep patterns. Need to add variable patterns:

| Language | Module-level | Class-level | isConstant detection |
|----------|-------------|-------------|---------------------|
| TypeScript | `const/let/var x = ...` at top | `static x = ...` | `const` keyword |
| Python | `x = value` at top level | `self.x` in __init__ | ALL_CAPS convention |
| Rust | `const X: T = ...`, `static X: T = ...` | — | `const` keyword |
| Go | `var x = ...`, `const x = ...` at package level | — | `const` keyword or Exported (capital) |
| Java | `static` fields | instance fields | `final` keyword |
| C/C++ | `#define`, global vars | static members | `const` keyword, `#define` |
| C# | `static` fields | instance fields | `const`, `readonly` |
| Kotlin | `val`, `const val`, `var` top-level | class members | `const val` |
| Swift | `let`, `var` top-level | class members | `let` keyword |
| Scala | `val`, `var` top-level | class members | `val` keyword |
| Ruby | Constants (UPPER), `@@class_var` | — | ALL_CAPS |
| PHP | `define()`, `const`, `$var` | class constants | `const` keyword |

## Slug Convention for Variables
Following existing pattern: `{file_slug}::{name}` — same as Function and Type.
For class-scoped: `{file_slug}::{ClassName}.{name}` to avoid collisions.
