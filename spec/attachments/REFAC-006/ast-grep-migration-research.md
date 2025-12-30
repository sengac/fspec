# AST-Grep Migration Research: fspec Research Tool

## Executive Summary

This document provides deep research for migrating fspec's `fspec research --tool=ast` command from the current TypeScript tree-sitter implementation to the native Rust ast-grep implementation in `codelet/tools/src/astgrep.rs`.

**Key Benefits:**
1. **Eliminate 17 tree-sitter npm dependencies** - Reduces package complexity and native binary compilation issues
2. **Unified codebase** - Single AST implementation shared between codelet CLI and fspec
3. **Advanced refactoring capabilities** - ast-grep's pattern-based extraction enables moving code out of large files like AgentModal.tsx (3558 lines)
4. **Better performance** - Native Rust implementation vs Node.js tree-sitter bindings

**Breaking Change:** This is NOT backwards compatible with the current implementation.

---

## Current Implementation Analysis

### TypeScript Tree-Sitter Stack

**Location:** `src/research-tools/ast.ts`

**Dependencies (17 packages):**
```json
"@sengac/tree-sitter": "^0.25.15",
"@sengac/tree-sitter-bash": "^0.25.15",
"@sengac/tree-sitter-c": "^0.25.15",
"@sengac/tree-sitter-c-sharp": "^0.25.15",
"@sengac/tree-sitter-cpp": "^0.25.15",
"@sengac/tree-sitter-dart": "^1.1.6",
"@sengac/tree-sitter-go": "^0.25.15",
"@sengac/tree-sitter-java": "^0.25.15",
"@sengac/tree-sitter-javascript": "^0.25.15",
"@sengac/tree-sitter-json": "^0.25.15",
"@sengac/tree-sitter-kotlin": "^0.4.6",
"@sengac/tree-sitter-php": "^0.25.15",
"@sengac/tree-sitter-python": "^0.25.15",
"@sengac/tree-sitter-ruby": "^0.25.15",
"@sengac/tree-sitter-rust": "^0.25.15",
"@sengac/tree-sitter-swift": "^0.25.15",
"@sengac/tree-sitter-typescript": "^0.25.15"
```

**Current API:**
```bash
fspec research --tool=ast --operation=list-functions --file=src/auth.ts
fspec research --tool=ast --operation=find-class --name=AuthController --file=src/auth.ts
fspec research --tool=ast --query-file=queries/custom.scm --file=src/utils.ts
```

**Key Files:**
- `src/research-tools/ast.ts` - Main tool implementation
- `src/utils/query-executor.ts` - Tree-sitter query execution
- `src/utils/language-loader.ts` - Lazy parser loading
- `src/utils/ast-queries/` - Predefined .scm query files

**Limitations:**
1. Operations are predefined (list-functions, find-class, etc.) - not flexible
2. Requires .scm files for custom queries - steep learning curve
3. Cannot easily extract/refactor code blocks
4. Native module compilation issues on some platforms

---

### Rust ast-grep Implementation (Target)

**Location:** `codelet/tools/src/astgrep.rs`

**Dependencies (Cargo.toml):**
```toml
ast-grep-core = "0.40.0"
ast-grep-language = "0.40.0"
```

**Current Capabilities:**
```rust
// Pattern-based search (already implemented)
let ast_grep = lang.ast_grep(&source);
let matches = ast_grep.root().find_all(pattern);
```

**Supported Languages (27):**
TypeScript, TSX, JavaScript, Rust, Python, Go, Java, C, C++, C#, Ruby, Kotlin, Swift, Scala, PHP, Bash, HTML, CSS, JSON, YAML, Lua, Elixir, Haskell, and more.

**Pattern Syntax:**
- `$VAR` - Captures one AST node
- `$_` - Matches one node (no capture)
- `$$$` - Matches zero or more nodes (anonymous)
- `$$$ARGS` - Matches zero or more nodes with capture

---

## Migration Architecture

### Option A: NAPI Bridge (Recommended)

Expose ast-grep through the existing NAPI module (`codelet-napi`):

```typescript
// New NAPI exports in codelet/napi/index.d.ts
export declare function astGrepSearch(
  pattern: string,
  language: string,
  paths: string[],
  options?: AstGrepOptions
): Promise<AstGrepResult[]>;

export declare function astGrepExtract(
  pattern: string,
  language: string,
  file: string,
  options?: ExtractOptions
): Promise<ExtractedCode>;

export interface AstGrepResult {
  file: string;
  line: number;
  column: number;
  text: string;
  captures: Record<string, string>;
}

export interface ExtractedCode {
  code: string;
  imports: string[];
  exports: string[];
  dependencies: string[];
}
```

**Implementation in Rust:**
```rust
// codelet/napi/src/astgrep.rs (new file)
use ast_grep_language::{LanguageExt, SupportLang};
use napi_derive::napi;

#[napi]
pub async fn ast_grep_search(
    pattern: String,
    language: String,
    paths: Vec<String>,
    options: Option<AstGrepOptions>,
) -> Result<Vec<AstGrepResult>, napi::Error> {
    // Delegate to existing codelet-tools implementation
    let tool = AstGrepTool::new();
    let args = build_args(pattern, language, paths);
    let result = tool.execute(args).await?;
    parse_results(result)
}
```

### Option B: Direct CLI Invocation (Fallback)

If NAPI integration is complex, invoke codelet binary:

```typescript
// Fallback approach using codelet CLI
import { spawn } from 'child_process';

async function astGrepSearch(pattern: string, language: string, paths: string[]): Promise<AstGrepResult[]> {
  const result = await execCodelet(['astgrep', '--pattern', pattern, '--language', language, ...paths]);
  return JSON.parse(result);
}
```

---

## New API Design (Breaking Change)

### Command-Line Interface

**Old (REMOVED):**
```bash
fspec research --tool=ast --operation=list-functions --file=src/auth.ts
```

**New:**
```bash
# Pattern-based search (primary interface)
fspec research --tool=ast --pattern="function $NAME($$$ARGS)" --lang=typescript --path=src/

# Find specific constructs
fspec research --tool=ast --pattern="async function $NAME($$$ARGS)" --lang=typescript

# Extract code blocks for refactoring
fspec research --tool=ast --extract --pattern="const $NAME = ($$$ARGS) => $BODY" --lang=typescript --file=src/tui/components/AgentModal.tsx

# Find React components
fspec research --tool=ast --pattern="function $COMPONENT($PROPS): JSX.Element" --lang=tsx --path=src/tui/

# Find Rust structs
fspec research --tool=ast --pattern="struct $NAME { $$$FIELDS }" --lang=rust --path=codelet/
```

### Core Operations

| Old Operation | New Pattern Equivalent |
|---------------|------------------------|
| `list-functions` | `function $NAME($$$ARGS)` or `const $NAME = ($$$) => $_` |
| `find-class` | `class $NAME { $$$BODY }` |
| `list-imports` | `import $$$IMPORTS from "$SOURCE"` |
| `list-exports` | `export $$$EXPORTS` |
| `find-async-functions` | `async function $NAME($$$ARGS)` |

### New Capabilities (Not Possible Before)

**1. Extract and Refactor Large Files**

The killer feature for refactoring AgentModal.tsx (3558 lines):

```bash
# Find all function components that could be extracted
fspec research --tool=ast --extract \
  --pattern="const $COMPONENT: React.FC<$PROPS> = ($ARGS) => { $$$BODY }" \
  --lang=tsx \
  --file=src/tui/components/AgentModal.tsx

# Find all useCallback hooks (candidates for extraction)
fspec research --tool=ast --extract \
  --pattern="const $NAME = useCallback(($$$ARGS) => { $$$BODY }, [$$$DEPS])" \
  --lang=tsx \
  --file=src/tui/components/AgentModal.tsx

# Find all custom hooks
fspec research --tool=ast \
  --pattern="const { $$$DESTRUCTURE } = use$HOOK($$$ARGS)" \
  --lang=tsx \
  --file=src/tui/components/AgentModal.tsx
```

**2. Semantic Code Analysis**

```bash
# Find all components receiving specific props
fspec research --tool=ast \
  --pattern="<$COMPONENT isActive={$_} $$$ATTRS />" \
  --lang=tsx

# Find all error handling patterns
fspec research --tool=ast \
  --pattern="try { $$$TRY } catch ($ERR) { $$$CATCH }" \
  --lang=typescript

# Find all async/await with error handling
fspec research --tool=ast \
  --pattern="try { await $ASYNC_CALL } catch ($_) { $$$HANDLER }" \
  --lang=typescript
```

**3. Dependency Analysis**

```bash
# Find all imports from a specific package
fspec research --tool=ast \
  --pattern="import { $$$IMPORTS } from 'ink'" \
  --lang=typescript

# Find all React hook usages
fspec research --tool=ast \
  --pattern="use$HOOK($$$ARGS)" \
  --lang=tsx
```

---

## AgentModal.tsx Refactoring Use Case

### Current Problem

`src/tui/components/AgentModal.tsx` is 3558 lines - a "god component" that handles:
- Model selection UI
- Session management
- Message rendering
- Input handling
- Provider configuration
- Thinking level configuration
- History navigation
- And more...

### Extraction Strategy Using ast-grep

**Step 1: Identify Extractable Components**

```bash
# Find all sub-components defined inline
fspec research --tool=ast --extract \
  --pattern="const $COMPONENT: React.FC<$_> = ($_) => { $$$BODY }" \
  --lang=tsx \
  --file=src/tui/components/AgentModal.tsx \
  --output-format=json
```

**Step 2: Find Hook Dependencies**

```bash
# Find all useState hooks to understand state shape
fspec research --tool=ast \
  --pattern="const [$STATE, $SETTER] = useState<$TYPE>($_)" \
  --lang=tsx \
  --file=src/tui/components/AgentModal.tsx
```

**Step 3: Extract Component with Dependencies**

```bash
# Extract SafeTextInput component (lines 148-210)
fspec research --tool=ast --extract \
  --pattern="const SafeTextInput: React.FC<{ $$$PROPS }> = ({ $$$DESTRUCTURE }) => { $$$BODY }" \
  --lang=tsx \
  --file=src/tui/components/AgentModal.tsx \
  --include-imports \
  --include-types
```

Expected output:
```typescript
// Extracted from AgentModal.tsx
// Imports needed:
import React from 'react';
import { useInput } from 'ink';

// Types needed:
interface SafeTextInputCallbacks {
  onHistoryPrev?: () => void;
  onHistoryNext?: () => void;
}

// Component:
const SafeTextInput: React.FC<{
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  placeholder?: string;
  isActive?: boolean;
} & SafeTextInputCallbacks> = ({
  value,
  onChange,
  onSubmit,
  placeholder = '',
  isActive = true,
  onHistoryPrev,
  onHistoryNext,
}) => {
  // ... body
};
```

### Helper Types for Extraction

```typescript
// Types to add to NAPI bindings
interface ExtractedCode {
  // The extracted code block
  code: string;
  
  // Line range in original file
  startLine: number;
  endLine: number;
  
  // Required imports (detected from usage)
  imports: ImportStatement[];
  
  // Type definitions needed
  types: TypeDefinition[];
  
  // Other symbols referenced but not defined
  externalDependencies: string[];
  
  // Suggested filename based on component name
  suggestedFilename: string;
}

interface ImportStatement {
  source: string;
  namedImports: string[];
  defaultImport?: string;
}

interface TypeDefinition {
  name: string;
  definition: string;
}
```

---

## Implementation Plan

### Phase 1: NAPI Bindings (Core)

1. **Create `codelet/napi/src/astgrep.rs`**
   - Expose `ast_grep_search()` function
   - Expose `ast_grep_extract()` function for code extraction
   - Reuse existing `codelet-tools` AstGrepTool logic

2. **Update `codelet/napi/src/lib.rs`**
   - Add astgrep module exports
   - Register NAPI functions

3. **Regenerate TypeScript definitions**
   - Run `napi build` to update `index.d.ts`

### Phase 2: TypeScript Integration

1. **Create new `src/research-tools/ast-grep.ts`**
   - Import from `@sengac/codelet-napi`
   - Implement ResearchTool interface
   - Handle pattern parsing and result formatting

2. **Update `src/research-tools/registry.ts`**
   - Replace 'ast' tool registration with new implementation

3. **Remove old implementation**
   - Delete `src/research-tools/ast.ts`
   - Delete `src/utils/query-executor.ts`
   - Delete `src/utils/language-loader.ts`
   - Delete `src/utils/ast-queries/` directory

### Phase 3: Dependency Cleanup

1. **Remove tree-sitter dependencies from package.json**
   - All 17 `@sengac/tree-sitter-*` packages

2. **Update tests**
   - Migrate existing AST tool tests to new pattern syntax
   - Add new tests for extraction capabilities

### Phase 4: Documentation

1. **Update help system**
   - New `--help` output with pattern examples
   - Add playground link: https://ast-grep.github.io/playground.html

2. **Update FOUNDATION.md**
   - Document new AST research capabilities
   - Add refactoring workflow examples

---

## Risk Assessment

### Breaking Changes

| Change | Impact | Mitigation |
|--------|--------|------------|
| `--operation` flag removed | All existing AST commands break | Provide migration guide with pattern equivalents |
| `.scm` query files no longer supported | Custom queries need rewriting | ast-grep patterns are simpler; document migration |
| Different output format | Scripts parsing output break | Provide `--output-format=legacy` option |

### Technical Risks

1. **NAPI binary size increase**
   - ast-grep bundled parsers add ~10MB
   - Mitigation: Already acceptable for codelet

2. **Pattern complexity**
   - Some tree-sitter queries may not have direct ast-grep equivalents
   - Mitigation: Document limitations; most common operations supported

3. **Multi-line pattern limitations**
   - ast-grep doesn't support patterns spanning multiple statements
   - Mitigation: Use multiple patterns or grep fallback

---

## Success Metrics

1. **Dependency reduction**: 17 → 0 tree-sitter npm packages
2. **Performance**: 2x faster for large file analysis (native Rust)
3. **New capability**: Code extraction for refactoring
4. **Test coverage**: 100% of new AST tool code tested

---

## Appendix: Pattern Examples

### TypeScript/JavaScript

```bash
# Find all exports
fspec research --tool=ast --pattern="export const $NAME = $_" --lang=typescript

# Find React components
fspec research --tool=ast --pattern="function $NAME($PROPS): JSX.Element { $$$BODY }" --lang=tsx

# Find async arrow functions
fspec research --tool=ast --pattern="const $NAME = async ($$$ARGS) => $_" --lang=typescript

# Find useEffect with specific dependency
fspec research --tool=ast --pattern="useEffect(() => { $$$BODY }, [$$$DEPS])" --lang=tsx
```

### Rust

```bash
# Find all public functions
fspec research --tool=ast --pattern="pub fn $NAME($$$ARGS) -> $RET { $$$BODY }" --lang=rust

# Find impl blocks
fspec research --tool=ast --pattern="impl $TRAIT for $TYPE { $$$BODY }" --lang=rust

# Find async functions
fspec research --tool=ast --pattern="async fn $NAME($$$ARGS) -> $RET { $$$BODY }" --lang=rust

# Find derive macros
fspec research --tool=ast --pattern="#[derive($$$DERIVES)]" --lang=rust
```

### Python

```bash
# Find class definitions
fspec research --tool=ast --pattern="class $NAME($$$BASES): $$$BODY" --lang=python

# Find decorated functions
fspec research --tool=ast --pattern="@$DECORATOR def $NAME($$$ARGS): $$$BODY" --lang=python

# Find async functions
fspec research --tool=ast --pattern="async def $NAME($$$ARGS): $$$BODY" --lang=python
```

---

## References

- [ast-grep Documentation](https://ast-grep.github.io/)
- [ast-grep Playground](https://ast-grep.github.io/playground.html)
- [ast-grep Rust Crate](https://crates.io/crates/ast-grep-core)
- [codelet AstGrepTool Implementation](codelet/tools/src/astgrep.rs)
- [Current tree-sitter AST tool](src/research-tools/ast.ts)
