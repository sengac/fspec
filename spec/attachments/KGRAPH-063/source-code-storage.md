# KGRAPH-063: Source Code and Metadata Storage in Graph Nodes

## Problem

Our Function and Type nodes store only structural metadata. When an agent finds a function via the graph, it must make a separate `Read` tool call to see the actual code. CGC stores source code directly in graph nodes, cutting tool calls in half.

## Current State

### Function Node (ast-code.pg schema)
```
slug, name, qualifiedName, isAsync, isPublic, paramCount, lineStart, lineEnd, cyclomaticComplexity
```
`build_function_node()` takes 8 params and populates all 9 properties (slug derived).

### Type Node (ast-code.pg schema)
```
slug, name, typeKind, isGeneric (never populated), isPublic, fieldCount (never populated)
```
`build_type_node()` takes 4 params (file_slug, name, type_kind, is_public). No lineStart/lineEnd on Types.

## CGC Reference

### Properties stored per Function — `graph_builder.py`
- `name`, `line_number`, `end_line`, `args` (list of param names), `source` (full body),
  `docstring`, `decorators` (list), `cyclomatic_complexity`, `context`, `context_type`,
  `class_context`, `lang`, `is_dependency`

### Properties stored per Class — `graph_builder.py`
- `name`, `line_number`, `end_line`, `bases`, `source`, `docstring`, `decorators`, `lang`, `is_dependency`

### Key CGC design decisions
- `source` and `docstring` gated by `INDEX_SOURCE=true` config flag
- `SET n += $props` merges entire dict onto Neo4j node (schemaless)
- Parameters stored BOTH as property (list) on node AND separate Parameter child nodes
- Each language parser returns consistent schema with language-specific extras

## What We Add

### Schema changes (ast-code.pg)

**Function node — 5 new properties:**
```
parameters: String?      // comma-separated: "self, name, age"
source: String?          // function body, capped at 100 lines / 4KB
docstring: String?       // extracted doc comment
decorators: String?      // comma-separated: "@staticmethod, @override"
language: String?        // "typescript", "rust", "python", etc.
```

**Type node — 6 new properties:**
```
lineStart: I32?          // was missing from Type (only on Function)
lineEnd: I32?            // was missing from Type (only on Function)
source: String?          // type body, capped
docstring: String?
decorators: String?
language: String?
```

### DRY Architecture — metadata.rs

Create `codelet/napi/src/graph/ast_pipeline/metadata.rs` with:

1. **`extract_source(full_text, max_lines=100, max_bytes=4096) -> (String, bool)`**
   Returns (capped_source, was_truncated). Single entry point for all extractors.

2. **`extract_docstring(text_before_entity, language) -> String`**
   Data-driven per-language config (same pattern as complexity.rs):
   - TS/JS: `/** ... */` (JSDoc)
   - Rust: `///` lines or `//!` (rustdoc)
   - Python: first triple-quoted string in body
   - Java/Kotlin/Scala: `/** ... */` (Javadoc)
   - C#: `/// <summary>` (XML doc)
   - Go: `//` lines immediately before declaration
   - Ruby: `#` lines before `def`/`class`
   - PHP: `/** ... */` (PHPDoc)
   - Dart: `///` (DartDoc)
   - C/C++: `/** ... */` or `///`
   - Swift: `///` or `/** ... */`

3. **`extract_decorators(text_before_entity, language) -> String`**
   - Python/TS/Dart: `@decorator` syntax
   - Rust: `#[attr]` or `#[derive(...)]`
   - Java/Kotlin: `@Annotation`
   - C#: `[Attribute]`
   - Swift: `@objc`, `@available`
   - Go/C/C++/Ruby/Scala/PHP: empty (no decorator syntax or not standard)

4. **`extract_parameters(signature_text, language) -> String`**
   Extracts parameter names only (no types), comma-separated.
   Filters out `self`/`cls` (Python), `&self`/`&mut self` (Rust), receiver (Go).

### Signature changes

**`build_function_node()` — 8 → 13 params:**
```rust
pub fn build_function_node(
    file_slug: &str,
    name: &str,
    is_async: bool,
    is_public: bool,
    param_count: i32,
    line_start: i32,
    line_end: i32,
    cyclomatic_complexity: i32,
    // NEW:
    parameters: &str,       // comma-separated param names
    source: &str,           // capped source text
    docstring: &str,        // extracted doc comment
    decorators: &str,       // comma-separated decorators
    language: &str,         // language identifier
) -> GraphEntity
```

**`build_type_node()` — 4 → 10 params:**
```rust
pub fn build_type_node(
    file_slug: &str,
    name: &str,
    type_kind: &str,
    is_public: bool,
    // NEW:
    line_start: i32,        // was missing from Types
    line_end: i32,          // was missing from Types
    source: &str,
    docstring: &str,
    decorators: &str,
    language: &str,
) -> GraphEntity
```

### Query changes (ast-queries.gq)

Update `all_functions` to include: `$fn.parameters, $fn.source, $fn.docstring, $fn.decorators, $fn.language`
Update `all_types` to include: `$t.lineStart, $t.lineEnd, $t.source, $t.docstring, $t.decorators, $t.language`
Update `uncalled_functions` and `unreferenced_types` similarly.

### Searchable fields update (dispatch_helpers.rs)

Add `"source"`, `"docstring"`, `"parameters"`, `"decorators"` to `AST_SEARCHABLE_FIELDS`.

### Files to modify

| File | Change |
|------|--------|
| `codelet/napi/schemas/ast-code.pg` | Add new properties to Function and Type |
| `codelet/napi/schemas/ast-queries.gq` | Return new properties in queries |
| `codelet/napi/src/graph/ast_pipeline/helpers.rs` | Extend build_function_node/build_type_node |
| `codelet/napi/src/graph/ast_pipeline/metadata.rs` | **NEW** — extract_source, extract_docstring, extract_decorators, extract_parameters |
| `codelet/napi/src/graph/ast_pipeline/mod.rs` | Register metadata module |
| `codelet/napi/src/graph/ast_pipeline/ast_*.rs` (×14) | Pass new fields to build helpers |
| `codelet/napi/src/graph/dispatch_helpers.rs` | Update AST_SEARCHABLE_FIELDS |
