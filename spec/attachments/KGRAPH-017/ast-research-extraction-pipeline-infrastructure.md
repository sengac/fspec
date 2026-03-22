# AST Research: AST Extraction Pipeline — Existing Infrastructure

## Research Method
DeepSearch analysis of `codelet/tools/src/astgrep.rs`, `codelet/tools/src/astgrep_refactor.rs`, `codelet/napi/src/graph/extractors.rs`

## Existing AstGrep Infrastructure

### Tool Layer (codelet/tools/src/)
- `AstGrepTool` — read-only search, uses `ast_grep_core::Pattern` + `lang.ast_grep(&src).root().find_all(pattern)`
- `AstGrepRefactorTool` — match + extract/replace with transforms
- Both use `ignore::WalkBuilder` for `.gitignore`-aware file walking
- Both use `ast_grep_language::SupportLang` for 23 supported languages

### Key Patterns for Extraction

**TypeScript/JavaScript:**
- Functions: `function $NAME($$$ARGS) { $$$BODY }`
- Arrow functions: `($$$ARGS) => $BODY`
- Imports: `import $$$IMPORTS from $SOURCE`
- Types: `type $NAME = $VALUE`
- Interfaces: `interface $NAME { $$$BODY }`
- Classes: `class $NAME { $$$BODY }`
- Method calls: `$OBJ.$METHOD($$$ARGS)`
- Const declarations: `const $NAME = $VALUE`

**Rust:**
- Functions: `fn $NAME($$$ARGS) { $$$BODY }`
- Structs: `struct $NAME { $$$FIELDS }`
- Enums: `enum $NAME { $$$VARIANTS }`
- Traits: `trait $NAME { $$$BODY }`
- Impl blocks: `impl $NAME { $$$BODY }`
- Use statements: `use $$$PATH;`

### Graph Extractors (existing — different concern)
- `extractors.rs` produces `GraphEntity` from *tool call metadata* (not AST)
- `GraphEntity::Node` and `GraphEntity::Edge` are the target types
- `EntityQueue` provides batch buffering with threshold-based flush
- `merge::entities_to_jsonl()` converts to nanograph JSONL format

### Architecture Decision
The AST extraction pipeline should be a NEW module in `codelet/napi/src/graph/` that:
1. Uses `ast_grep_core` and `ast_grep_language` directly (same crates as AstGrepTool)
2. Uses `ignore::WalkBuilder` for file walking (same as AstGrepTool)
3. Produces `GraphEntity` values (same type as existing extractors)
4. Loads into AST graph via `registry::get_graph("ast-code")` + `load_entities()`
5. Each language extractor in its own file: `ast_ts_extractor.rs`, `ast_rust_extractor.rs`
6. Common trait `AstExtractor` for shared interface
