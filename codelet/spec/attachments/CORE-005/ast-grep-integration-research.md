# AstGrep Native Rust Integration Research

## Overview

This document contains research findings for implementing the AstGrep tool in codelet using the native `ast-grep` Rust crates as a library (not shelling out to binary).

## Source Analysis

### TypeScript Codelet Implementation (Reference)

The TypeScript codelet uses `@ast-grep/cli` npm package which shells out to the `sg` binary:

```typescript
// From codelet/src/agent/tools.ts
const sgBinary = path.join(process.cwd(), 'node_modules', '.bin', 'sg');
const command = `${sgBinary} -p '${escapedPattern}' -l ${language} ${pathsArg} --json=compact 2>&1`;
const output = execSync(command, { encoding: 'utf-8', maxBuffer: 10 * 1024 * 1024 });
```

### Native Rust Crates (Our Approach)

The `ast-grep` project is actually written in Rust. We can use it directly as a library:

**Crates to use:**
- `ast-grep-core = "0.40.0"` - Core pattern matching and AST traversal
- `ast-grep-language = "0.40.0"` - 27 supported languages with tree-sitter grammars

## API Usage Guide

### 1. Creating an AstGrep Instance

```rust
use ast_grep_language::SupportLang;

// Parse a string
let lang = SupportLang::TypeScript;  // or JavaScript, Rust, Python, etc.
let source = "const x = 5;";
let ast_grep = lang.ast_grep(source);  // Returns AstGrep<StrDoc<SupportLang>>
```

### 2. Searching for Matches

```rust
// Find single match
let root = lang.ast_grep(source);
let match_result = root.root().find("const $A = $B");  // Returns Option<NodeMatch>

// Find all matches
let matches: Vec<_> = root.root().find_all("const $A = $B").collect();
```

### 3. Extracting Information from Match Results

```rust
if let Some(match_) = root.root().find("const $A = $B") {
    // Access as Node (NodeMatch dereferences to Node)
    let node: &Node = &*match_;

    // Position information (0-based)
    let line = node.start_pos().line();
    let col = node.start_pos().column(&*match_);
    let range = node.range();  // byte offset range

    // Content information
    let matched_text = node.text();
    let kind = node.kind();  // AST node kind (e.g., "variable_declaration")

    // Captured variables from pattern
    let env = match_.get_env();
    if let Some(var_a) = env.get_match("A") {
        println!("$A captured: {}", var_a.text());
    }
}
```

## Pattern Syntax

### Meta Variables

| Syntax | Description |
|--------|-------------|
| `$A`, `$VAR`, `$ANYTHING` | Captures one AST node |
| `$_` | Matches one node but doesn't capture |
| `$$$` | Matches zero or more nodes (anonymous) |
| `$$$ARGS` | Matches zero or more nodes and captures as `ARGS` |

### Example Patterns

```rust
// Match function calls with any arguments
"foo($$$ARGS)"

// Match variable declarations
"let $NAME = $VALUE"

// Match specific structure
"if ($CONDITION) { $$$BODY }"

// Match Rust functions with return type
"fn $NAME($$$ARGS) -> $RET { $$$BODY }"
```

## Supported Languages (SupportLang Enum)

```rust
pub enum SupportLang {
    Bash, C, Cpp, CSharp, Css, Elixir, Go, Haskell, Hcl, Html,
    Java, JavaScript, Json, Kotlin, Lua, Nix, Php, Python, Ruby,
    Rust, Scala, Solidity, Swift, Tsx, TypeScript, Yaml,
}
```

### Parsing from Strings

```rust
let lang: SupportLang = "ts".parse().unwrap();      // TypeScript
let lang: SupportLang = "javascript".parse().unwrap();
let lang: SupportLang = "py".parse().unwrap();      // Python
let lang: SupportLang = "rust".parse().unwrap();
```

### Auto-detecting from File Extension

```rust
use std::path::Path;
let path = Path::new("file.ts");
if let Some(lang) = SupportLang::from_path(path) {
    // lang is TypeScript
}
```

## Directory Walking Strategy

Use the `ignore` crate (already a dependency in codelet) for gitignore-aware walking:

```rust
use ignore::WalkBuilder;

let walker = WalkBuilder::new(".")
    .hidden(false)
    .git_ignore(true)
    .build();

for result in walker.flatten() {
    if let Some(lang) = SupportLang::from_path(result.path()) {
        let source = std::fs::read_to_string(result.path())?;
        let root = lang.ast_grep(&source);
        let matches: Vec<_> = root.root().find_all(pattern).collect();
        // Process matches...
    }
}
```

## Output Format

Following the TypeScript implementation, results should be in `file:line:column:text` format:

```
src/agent/tools.ts:310:1:export function executeAstGrep(
src/agent/tools.ts:446:1:export function executeGrep(params: GrepParams): string {
```

Convert 0-based line/column from ast-grep to 1-based for output:
- Line: `node.start_pos().line() + 1`
- Column: `node.start_pos().column(&*match_) + 1`

## Error Handling

Invalid patterns should return helpful error messages:

```
Error: Invalid AST pattern syntax. Pattern: "function {{{"
Details: The pattern could not be parsed as valid typescript syntax.

Pattern syntax guide:
- Use $VAR for single AST node (e.g., 'function $NAME()')
- Use $$$VAR for multiple nodes (e.g., 'function $NAME($$$ARGS)')
- Pattern must be valid typescript syntax
- Meta-variables must start with $ and uppercase letter

Try the ast-grep playground: https://ast-grep.github.io/playground.html
```

## Implementation Notes

1. **Native integration** - No subprocess spawning, direct Rust API calls
2. **Memory efficiency** - Nodes are borrowed from root document
3. **Performance** - Uses tree-sitter's incremental parsing
4. **Consistent truncation** - Apply same OUTPUT_LIMITS as other tools (30000 chars, 2000 per line)

## Dependencies to Add

```toml
[dependencies]
ast-grep-core = "0.40.0"
ast-grep-language = "0.40.0"
```

Note: tree-sitter grammars are bundled in ast-grep-language with the "builtin-parser" feature.

## Key Files in ast-grep-lib Repository

- `/home/rquast/projects/ast-grep-lib/crates/core/src/matcher/pattern.rs` - Pattern API
- `/home/rquast/projects/ast-grep-lib/crates/core/src/node.rs` - Node traversal methods
- `/home/rquast/projects/ast-grep-lib/crates/language/src/lib.rs` - Language support
