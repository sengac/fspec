# AST Research: Cross-Language Edge Extraction Architecture

## Date: 2026-03-25

## Extractor Functions (all accept known_files: &HashSet<String>)

| Language | Function | File | Edges |
|----------|----------|------|-------|
| PHP | `extract_php()` | `ast_php_extractor.rs` | Imports, Calls, TypeRef |
| Python | `extract_python()` | `ast_python_extractor.rs` | Imports, Calls |
| Go | `extract_go()` | `ast_go_extractor.rs` | Imports, Calls |
| Rust | `extract_rust()` | `ast_rust_extractor.rs` | Imports, Calls, TypeRef |
| Java | `extract_java()` | `ast_java_extractor.rs` | Imports, Calls, TypeRef |
| C | `extract_c()` | `ast_c_extractor.rs` | Imports, Calls |
| C++ | `extract_cpp()` | `ast_cpp_extractor.rs` | Imports, Calls |
| C# | `extract_csharp()` | `ast_csharp_extractor.rs` | Imports, Calls, TypeRef |
| Ruby | `extract_ruby()` | `ast_ruby_extractor.rs` | Imports, Calls |
| Kotlin | `extract_kotlin()` | `ast_kotlin_extractor.rs` | Imports, Calls, TypeRef |
| Swift | `extract_swift()` | `ast_swift_extractor.rs` | Calls |
| Scala | `extract_scala()` | `ast_scala_extractor.rs` | Imports, Calls, TypeRef |

## Shared Helpers (edge_helpers.rs)

- `build_import_edge()` — Creates Imports edge + stub File node
- `build_calls_edge()` — Creates Calls edge between functions
- `build_typeref_edge()` — Creates TypeRef edge from function to type
- `extract_call_names_from_body()` — Extracts function call identifiers from body text
- `resolve_calls()` — Resolves call names to local functions or imports
- `resolve_type_refs()` — Resolves type references to local types or imports
- `extract_c_includes()` — Shared C/C++ #include processing

## Dispatch (mod.rs)

- `extract_file()` dispatches to language-specific extractors
- `walk_and_extract()` builds known_files set and passes it to all extractors

## Test Coverage

- 11 integration test files with 28 tests total
- All use shared `graph_test_helpers.rs` (find_edges, build_known_files, write_test_file)
- Each test uses real source code fixtures — no mocks
