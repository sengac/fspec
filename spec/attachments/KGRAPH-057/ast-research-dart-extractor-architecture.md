# AST Research — Dart Extractor Architecture

## Existing Extractor Pattern

All language extractors follow this structure (verified via AST search across 9 extractors):

1. **Entry function**: `extract_<lang>(source, rel_path, known_files) -> Result<Vec<GraphEntity>, String>`
2. **File node**: Created with `helpers::build_file_node()`
3. **Function extraction**: `extract_functions()` → returns `HashSet<String>` of names
4. **Type extraction**: `extract_types()` → returns `HashSet<String>` of type names
5. **Import extraction**: `extract_imports()` → returns `HashMap<String, (String, bool, String)>` import map
6. **Calls edges**: `extract_calls()` using `edge_helpers::extract_call_names_from_body()`
7. **TypeRef edges**: `extract_type_refs()` using `edge_helpers::resolve_type_refs()`

## DartLang Implementation (from KGRAPH-056)

Location: `codelet/tools/src/dart_lang.rs`

```rust
pub struct DartLang;
impl Language for DartLang { ... }   // kind_to_id, field_to_id, build_pattern
impl LanguageExt for DartLang { ... } // get_ts_language → tree_sitter_dart::LANGUAGE
```

Key facts:
- DartLang does NOT use expando_char ($ is valid in Dart identifiers)
- tree-sitter-dart v0.1.0 is the dependency
- Dart splits function_signature and function_body as sibling nodes at top-level

## Approach: KindMatcher (like Kotlin)

Since Dart top-level functions split into sibling nodes, pattern matching like Swift's
`func $NAME() { $$$BODY }` won't work. Instead, use **KindMatcher** like the Kotlin extractor.

Relevant Dart AST node kinds:
- Functions: `function_signature`, `method_signature`, `constructor_signature`, 
  `constant_constructor_signature`, `factory_constructor_signature`, 
  `getter_signature`, `setter_signature`, `operator_signature`
- Types: `class_declaration`, `enum_declaration`, `mixin_declaration`,
  `extension_declaration`, `extension_type_declaration`, `type_alias`, `mixin_application_class`
- Imports: `library_import`, `library_export`, `part_directive`

## Dependency: codelet-tools already exposes DartLang

The `codelet-tools` crate is already a dependency of `codelet-napi` (Cargo.toml line 27).
DartLang is pub-exported, so it can be reused directly:
```rust
use codelet_tools::dart_lang::DartLang;
```

tree-sitter-dart v0.1.0 still needs to be added to codelet-napi/Cargo.toml since
DartLang's `get_ts_language()` calls `tree_sitter_dart::LANGUAGE` which requires
the crate to be linked.

## Integration Points

1. **mod.rs**: Add "dart" to SUPPORTED_EXTENSIONS, add dispatch case in extract_file()
2. **ast_dispatch.rs**: Add pubspec_dep_extractor call in dispatch_ast_index()
3. **pubspec_dep_extractor.rs**: New file, similar pattern to swift_dep_extractor.rs
