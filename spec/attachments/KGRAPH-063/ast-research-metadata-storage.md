# KGRAPH-063 AST Research: Source Code and Metadata Storage

## Research Date: 2026-03-27

## Current Schema (ast-code.pg)

### Function Node
```
node Function {
    slug: String @key
    name: String
    qualifiedName: String?
    isAsync: Bool?
    isPublic: Bool?
    paramCount: I32?
    lineStart: I32?
    lineEnd: I32?
    cyclomaticComplexity: I32?
}
```

### Type Node
```
node Type {
    slug: String @key
    name: String
    typeKind: enum(struct_kind, class, interface, enum_kind, trait_kind, type_alias, extension)
    isGeneric: Bool?
    isPublic: Bool?
    fieldCount: I32?
}
```

## Current build_function_node() — helpers.rs:43-72

Takes 8 params: file_slug, name, is_async, is_public, param_count, line_start, line_end, cyclomatic_complexity.
Returns GraphEntity::Node with 9 properties (slug derived).

## Current build_type_node() — helpers.rs:89-106

Takes 4 params: file_slug, name, type_kind, is_public.
Returns GraphEntity::Node with 4 properties. **No lineStart/lineEnd on Type nodes.**

## TypeScript Extractor Pattern — ast_ts_extractor.rs:95-139

```rust
fn extract_functions(root, file_slug, entities) -> HashSet<String> {
    for pattern in TS_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();        // full source text available
            let name = extract_function_name(&matched_text);
            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_async = matched_text.contains("async ");
            let is_public = ...;
            let param_count = helpers::count_params(&matched_text);
            let cc = complexity::calculate(&matched_text, "typescript");

            entities.push(helpers::build_function_node(
                file_slug, &name, is_async, is_public, param_count,
                start_pos.line() as i32 + 1, end_pos.line() as i32 + 1, cc,
            ));
        }
    }
}
```

Key observation: `node.text()` gives us the **full source text** of each matched function. We already have this data — we just don't store it.

## Extract Types Pattern — ast_ts_extractor.rs:144-179

```rust
fn extract_types(root, file_slug, entities) -> HashSet<String> {
    for (pattern, type_kind) in TS_TYPE_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();       // full source text available
            let name = extract_type_name(&matched_text, type_kind);
            let is_public = ...;

            // Note: NO start_pos/end_pos captured for types currently
            entities.push(helpers::build_type_node(
                file_slug, &name, type_kind, is_public,
            ));
        }
    }
}
```

Key gap: Types don't extract start_pos/end_pos, but ast-grep provides them via `node.start_pos()` / `node.end_pos()`.

## Complexity Module Pattern (DRY reference) — complexity.rs

```rust
pub fn calculate(source: &str, language: &str) -> i32 {
    let config = config_for_language(language);
    let stripped = strip_for_complexity(source, config.comment_style);
    1 + count_decision_points(&stripped, config)
}
```

Uses data-driven configs (`ComplexityConfig` per language) with a `config_for_language()` dispatch. Same pattern should be used for docstring/decorator extraction.

## Query Changes Needed (ast-queries.gq)

### all_functions — currently returns:
```
$fn.slug, $fn.name, $fn.qualifiedName, $fn.isAsync, $fn.isPublic, $fn.paramCount, $fn.lineStart, $fn.lineEnd, $fn.cyclomaticComplexity
```
Need to add: `$fn.parameters, $fn.source, $fn.docstring, $fn.decorators, $fn.language`

### all_types — currently returns:
```
$t.slug, $t.name, $t.typeKind, $t.isPublic
```
Need to add: `$t.lineStart, $t.lineEnd, $t.source, $t.docstring, $t.decorators, $t.language`

## Files to Change

1. `codelet/napi/schemas/ast-code.pg` — schema
2. `codelet/napi/schemas/ast-queries.gq` — queries
3. `codelet/napi/src/graph/ast_pipeline/helpers.rs` — build helpers
4. `codelet/napi/src/graph/ast_pipeline/metadata.rs` — **NEW** shared metadata extraction
5. `codelet/napi/src/graph/ast_pipeline/mod.rs` — module registration
6. `codelet/napi/src/graph/ast_pipeline/ast_ts_extractor.rs` — TS
7. `codelet/napi/src/graph/ast_pipeline/ast_rust_extractor.rs` — Rust
8. `codelet/napi/src/graph/ast_pipeline/ast_python_extractor.rs` — Python
9. `codelet/napi/src/graph/ast_pipeline/ast_go_extractor.rs` — Go
10. `codelet/napi/src/graph/ast_pipeline/ast_java_extractor.rs` — Java
11. `codelet/napi/src/graph/ast_pipeline/ast_c_extractor.rs` — C
12. `codelet/napi/src/graph/ast_pipeline/ast_cpp_extractor.rs` — C++
13. `codelet/napi/src/graph/ast_pipeline/ast_csharp_extractor.rs` — C#
14. `codelet/napi/src/graph/ast_pipeline/ast_ruby_extractor.rs` — Ruby
15. `codelet/napi/src/graph/ast_pipeline/ast_kotlin_extractor.rs` — Kotlin
16. `codelet/napi/src/graph/ast_pipeline/ast_swift_extractor.rs` — Swift
17. `codelet/napi/src/graph/ast_pipeline/ast_scala_extractor.rs` — Scala
18. `codelet/napi/src/graph/ast_pipeline/ast_php_extractor.rs` — PHP
19. `codelet/napi/src/graph/ast_pipeline/ast_dart_extractor.rs` — Dart
20. `codelet/napi/src/graph/dispatch_helpers.rs` — searchable fields
