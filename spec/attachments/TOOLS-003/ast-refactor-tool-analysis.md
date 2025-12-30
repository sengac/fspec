# AST Code Refactor Tool for Codelet - Technical Analysis

## Executive Summary

This document analyzes the existing AST search and refactoring infrastructure to inform the implementation of a new `AstGrepRefactorTool` for the codelet tools layer. The refactoring capability already exists in the NAPI bindings but is not exposed as a standalone tool in the codelet agent tool system.

---

## 1. Current Architecture Overview

### Architecture Flow
```
fspec research --tool=ast [options]
    ↓
/src/commands/research.ts (registerResearchCommand)
    ↓
/src/research-tools/registry.ts (getResearchTool)
    ↓
/src/research-tools/ast.ts (tool.execute)
    ↓
@sengac/codelet-napi (astGrepSearch or astGrepRefactor)
    ↓
/codelet/napi/src/astgrep.rs (NAPI functions)
    ↓
ast-grep-core & ast-grep-language (Rust crates)
    ↓
AST matching engine
```

### Key Finding: Gap in Tool Layer

| Layer | Search | Refactor |
|-------|--------|----------|
| NAPI Bindings | ✅ `astGrepSearch()` | ✅ `astGrepRefactor()` |
| TypeScript Research Tool | ✅ `--pattern` mode | ✅ `--refactor` mode |
| **Codelet Tools (rig::tool::Tool)** | ✅ `AstGrepTool` | ❌ **MISSING** |

The goal is to add `AstGrepRefactorTool` to the codelet tools layer.

---

## 2. Existing Implementation Analysis

### 2.1 NAPI Refactor Function

**File**: `/codelet/napi/src/astgrep.rs` (lines 185-277)

```rust
#[napi]
pub async fn ast_grep_refactor(
    pattern: String,
    language: String,
    source_file: String,
    target_file: String,
) -> napi::Result<AstGrepRefactorResult>
```

**Algorithm**:
1. Parse language and validate
2. Read source file asynchronously
3. Find all matches using AST pattern
4. **Validate exactly 1 match** (critical constraint)
   - Error if 0 matches: "No matches found"
   - Error if multiple matches: Shows all match locations
5. Extract matched text and byte range
6. Remove matched code from source
7. Clean up resulting blank lines
8. Write updated source file
9. Write matched code to target file

**Return Type**:
```rust
pub struct AstGrepRefactorResult {
    pub success: bool,
    pub moved_code: String,
    pub source_file: String,
    pub target_file: String,
}
```

### 2.2 TypeScript Research Tool Interface

**File**: `/src/research-tools/ast.ts` (lines 16-100)

```typescript
async execute(args: string[]): Promise<string> {
  // Parses: --pattern, --lang, --path, --refactor, --source, --target

  if (isRefactorMode) {
    const result = await astGrepRefactor(pattern, lang, sourceFile, targetFile);
    return `Refactor successful...`;
  } else {
    const results = await astGrepSearch(pattern, lang, paths);
    return formatSearchResults(results);
  }
}
```

### 2.3 Existing AstGrepTool (Search Only)

**File**: `/codelet/tools/src/astgrep.rs` (lines 1-397)

**Tool Definition**:
```rust
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AstGrepArgs {
    pub pattern: String,
    pub language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}
```

**Key Implementation Features**:
- Implements `rig::tool::Tool` trait
- Async file reads with `tokio::fs::read_to_string`
- Panic safety via `std::panic::catch_unwind`
- `.gitignore` aware using `ignore` crate
- Multiline pattern validation
- Output truncation for large results

---

## 3. Proposed Implementation

### 3.1 New Tool Structure

**File**: `/codelet/tools/src/astgrep_refactor.rs` (new file)

```rust
use ast_grep_core::{language::TSLanguage, AstGrep, Pattern};
use ast_grep_language::SupportLang;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use rig::tool::Tool;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AstGrepRefactorArgs {
    /// AST pattern to match (ast-grep syntax)
    pub pattern: String,
    /// Programming language
    pub language: String,
    /// Source file to extract code from
    pub source_file: String,
    /// Target file to write extracted code to
    pub target_file: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AstGrepRefactorResult {
    pub success: bool,
    pub moved_code: String,
    pub source_file: String,
    pub target_file: String,
    pub error: Option<String>,
}

pub struct AstGrepRefactorTool;

impl Tool for AstGrepRefactorTool {
    const NAME: &'static str = "astgrep_refactor";
    type Error = ToolError;
    type Args = AstGrepRefactorArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        // Tool metadata for LLM
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Implementation
    }
}
```

### 3.2 Core Algorithm

```rust
async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    // 1. Parse and validate language
    let lang = parse_language(&args.language)?;

    // 2. Read source file
    let source_content = tokio::fs::read_to_string(&args.source_file).await?;

    // 3. Parse AST and find matches
    let grep = AstGrep::new(&source_content, lang);
    let pattern = Pattern::new(&args.pattern, lang);
    let matches: Vec<_> = grep.root().find_all(&pattern).collect();

    // 4. Validate exactly one match
    match matches.len() {
        0 => return Err(ToolError::NoMatches),
        1 => { /* proceed */ },
        n => return Err(ToolError::MultipleMatches(n, format_locations(&matches))),
    }

    // 5. Extract and remove code
    let match_node = &matches[0];
    let matched_text = match_node.text().to_string();
    let byte_range = match_node.range();

    let mut new_source = source_content.clone();
    new_source.replace_range(byte_range, "");
    new_source = cleanup_blank_lines(new_source);

    // 6. Write files
    tokio::fs::write(&args.source_file, &new_source).await?;
    tokio::fs::write(&args.target_file, &matched_text).await?;

    // 7. Return result
    Ok(serde_json::to_string(&AstGrepRefactorResult {
        success: true,
        moved_code: matched_text,
        source_file: args.source_file,
        target_file: args.target_file,
        error: None,
    })?)
}
```

### 3.3 Integration Points

**Update `/codelet/tools/src/lib.rs`**:
```rust
mod astgrep_refactor;
pub use astgrep_refactor::AstGrepRefactorTool;
```

---

## 4. Language Support Matrix

Both search and refactor support 23 languages:

| Language | Extension(s) |
|----------|--------------|
| TypeScript | `.ts` |
| TSX | `.tsx` |
| JavaScript | `.js`, `.mjs`, `.cjs`, `.jsx` |
| Rust | `.rs` |
| Python | `.py`, `.pyi` |
| Go | `.go` |
| Java | `.java` |
| C | `.c`, `.h` |
| C++ | `.cpp`, `.hpp`, `.cc`, `.hh`, `.cxx`, `.hxx` |
| C# | `.cs` |
| Ruby | `.rb` |
| Kotlin | `.kt`, `.kts` |
| Swift | `.swift` |
| Scala | `.scala` |
| PHP | `.php` |
| Bash | `.sh`, `.bash` |
| HTML | `.html`, `.htm` |
| CSS | `.css` |
| JSON | `.json` |
| YAML | `.yaml`, `.yml` |
| Lua | `.lua` |
| Elixir | `.ex`, `.exs` |
| Haskell | `.hs` |

---

## 5. Pattern Syntax Reference

### Wildcards
- `$NAME` - Single AST node wildcard
- `$$$ARGS` - Multiple nodes (zero or more)

### Example Patterns
```
# Find function
fn $NAME($_)

# Find async function with body
async function $NAME($$$ARGS) { $$$BODY }

# Find class
class $NAME { $$$MEMBERS }

# Find import
import $BINDING from "$MODULE"

# Find JSX component
<$COMPONENT $$$PROPS />
```

---

## 6. Error Handling

### Critical Validations
1. **Language validation**: Must be a supported language
2. **Pattern validation**: Must be valid ast-grep syntax
3. **Single match constraint**: Exactly one match required
4. **File existence**: Source file must exist
5. **Write permissions**: Target directory must be writable

### Error Messages (from NAPI implementation)
```
"No matches found for pattern: {pattern}"
"Multiple matches found ({count}). Refactor requires exactly one match:\n{locations}"
```

---

## 7. Test Scenarios

### Happy Path
1. Pattern matches exactly one code block
2. Code is extracted from source
3. Code is written to target
4. Source file is cleaned up

### Error Cases
1. Pattern matches zero nodes → Error with pattern details
2. Pattern matches multiple nodes → Error with locations
3. Invalid language → Error with supported languages
4. Source file not found → File not found error
5. Target directory not writable → Permission error

### Edge Cases
1. Empty source file
2. Pattern at start/end of file
3. Nested pattern matches
4. Unicode content
5. Very large files

---

## 8. Key Files Reference

| File | Lines | Purpose |
|------|-------|---------|
| `codelet/napi/src/astgrep.rs` | 185-277 | NAPI refactor implementation (reference) |
| `codelet/tools/src/astgrep.rs` | 1-397 | Search tool implementation (template) |
| `codelet/tools/src/lib.rs` | 1-85 | Tool exports (needs update) |
| `src/research-tools/ast.ts` | 1-252 | TypeScript wrapper (reference) |
| `codelet/napi/index.d.ts` | 246-256 | TypeScript types (reference) |

---

## 9. Implementation Checklist

- [ ] Create `/codelet/tools/src/astgrep_refactor.rs`
- [ ] Define `AstGrepRefactorArgs` struct with JSON schema
- [ ] Define `AstGrepRefactorResult` struct
- [ ] Implement `AstGrepRefactorTool` struct
- [ ] Implement `rig::tool::Tool` trait
- [ ] Add language parsing (reuse from astgrep.rs)
- [ ] Implement single-match validation
- [ ] Implement code extraction logic
- [ ] Implement blank line cleanup
- [ ] Add comprehensive error handling
- [ ] Update `lib.rs` exports
- [ ] Write unit tests
- [ ] Write integration tests
- [ ] Update documentation

---

## 10. Open Questions

1. **Should refactor support replace-in-place?** Current design only extracts to new file. Could support `--replace <replacement>` mode.

2. **Should target file be created or appended?** Current NAPI implementation overwrites. Consider append mode for collecting multiple extractions.

3. **Should we support dry-run mode?** Show what would be changed without modifying files.

4. **Backup files?** Should we create `.bak` files before modifying source?

---

## 11. Related Work Units

- `TOOLS-001`: Research Tool Migration (completed)
- `NAPI-*`: NAPI-RS native bindings work units (completed)
- This story: Add standalone refactor tool to codelet
