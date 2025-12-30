# AST Research: Codebase Analysis for REFAC-006

## Current TypeScript Implementation (to be replaced)

**File:** `src/research-tools/ast.ts`

Functions:
- `execute()` - Main method that parses args and runs tree-sitter queries
- `parseArg()` - Helper to parse CLI arguments
- `getHelpConfig()` - Returns help configuration for the tool
- `detectLanguage()` - Maps file extensions to language names

**Dependencies:**
- `src/utils/query-executor.ts` - Executes tree-sitter queries
- `src/utils/language-loader.ts` - Lazy loads tree-sitter parsers
- `src/utils/ast-queries/` - Predefined .scm query files

## Existing Rust Implementation (to extend)

**File:** `codelet/tools/src/astgrep.rs`

Functions:
- `new()` - Constructor
- `parse_language()` - Parses language string to SupportLang enum
- `get_extensions()` - Maps SupportLang to file extensions
- `search_file()` - Async function that searches a single file for pattern matches
- `execute()` - Main method that handles search across files/directories
- `definition()` - Returns rig tool definition for LLM integration
- `call()` - rig::tool::Tool implementation

**Key Patterns Found:**
```rust
// Pattern matching using ast-grep
let ast_grep = lang.ast_grep(&source);
let root = ast_grep.root();
let matches = root.find_all(pattern_owned.as_str());
```

## NAPI Bindings Structure

**File:** `codelet/napi/src/lib.rs`

Current exports include:
- `CodeletSession` - Main session class
- `persistenceStoreMessageEnvelope` - Persistence functions
- `getThinkingConfig` - Thinking level configuration
- `modelsListAll`, `modelsRefreshCache` - Model management

**New exports to add:**
- `astGrepSearch(pattern, language, paths)` - Search function
- `astGrepRefactor(pattern, language, sourceFile, targetFile)` - Refactor function

## Files to Delete

1. `src/research-tools/ast.ts` - Replace with NAPI wrapper
2. `src/utils/query-executor.ts` - No longer needed
3. `src/utils/language-loader.ts` - No longer needed  
4. `src/utils/ast-queries/` directory - No longer needed

## package.json Dependencies to Remove

17 tree-sitter packages:
- @sengac/tree-sitter
- @sengac/tree-sitter-bash
- @sengac/tree-sitter-c
- @sengac/tree-sitter-c-sharp
- @sengac/tree-sitter-cpp
- @sengac/tree-sitter-dart
- @sengac/tree-sitter-go
- @sengac/tree-sitter-java
- @sengac/tree-sitter-javascript
- @sengac/tree-sitter-json
- @sengac/tree-sitter-kotlin
- @sengac/tree-sitter-php
- @sengac/tree-sitter-python
- @sengac/tree-sitter-ruby
- @sengac/tree-sitter-rust
- @sengac/tree-sitter-swift
- @sengac/tree-sitter-typescript
