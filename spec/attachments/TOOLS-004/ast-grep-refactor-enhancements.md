# AST-Grep Refactor Tool Enhancements - Research & Design

## Overview

This document captures the research findings from analyzing the ast-grep source code and the design decisions made for enhancing the `astgrep_refactor` tool (TOOLS-003).

**Source Research**: Cloned ast-grep from `https://github.com/ast-grep/ast-grep` and analyzed the Rust source code in depth.

---

## Problem Statement

When Claude previously used the refactor tool, it thought it had to pass the **entire code block** as the pattern parameter, not understanding that patterns are **partial structural matches** with wildcards.

Additionally, the tool lacked:
- Rename/transform capabilities (case conversion, regex replace)
- Batch mode for multiple matches
- Preview/dry-run mode

---

## Research Findings from ast-grep Source

### 1. Meta-Variable Syntax (from `crates/core/src/meta_var.rs`)

| Pattern | Meaning | Example |
|---------|---------|---------|
| `$NAME` | Single AST node capture (named) | `fn $NAME()` matches any function |
| `$$NAME` | Single AST node capture (unnamed) | Structural matching |
| `$_` | Dropped single-node wildcard | `fn $_($$$_)` - ignore name |
| `$$$` | Anonymous multi-node ellipsis | `{ $$$ }` matches any block |
| `$$$ARGS` | Captured multi-node ellipsis | `fn test($$$ARGS)` captures all params |

**Key Constraint**: Meta-variable names must be UPPERCASE letters, underscores, or digits (after first char). Lowercase like `$name` is NOT valid.

### 2. Transform System (from `crates/config/src/transform/`)

The `Trans` enum in `trans.rs` provides three transform types:

#### Substring Transform
Extract a substring from captured variable with Python-style indexing:
```yaml
substring:
  source: $NAME
  startChar: 0    # optional, defaults to 0
  endChar: -1     # optional, defaults to end. Negative = from end
```

#### Replace Transform
Regex-based find/replace on captured text:
```yaml
replace:
  source: $NAME
  replace: "_id$"   # regex pattern
  by: ""            # replacement string
```

#### Convert Transform
Case conversion with optional word separators:
```yaml
convert:
  source: $NAME
  toCase: camelCase
  separatedBy: [underscore, caseChange]  # optional
```

**Supported Cases** (from `string_case.rs`):
- `lowerCase` - all lowercase
- `upperCase` - ALL UPPERCASE
- `capitalize` - First letter uppercase
- `camelCase` - firstWordLower
- `snakeCase` - words_with_underscores
- `kebabCase` - words-with-dashes
- `pascalCase` - FirstWordUpper

**Separator Options**:
- `caseChange` - split on case transitions (camelCase → camel, Case)
- `underscore` - split on `_`
- `dash` - split on `-`
- `dot` - split on `.`
- `slash` - split on `/`
- `space` - split on spaces

### 3. Transform Chaining

Transforms can reference other transforms by dependency:
```yaml
transforms:
  STRIPPED:
    replace:
      source: $NAME
      replace: "_suffix$"
      by: ""
  FINAL:
    convert:
      source: $STRIPPED  # references STRIPPED output
      toCase: camelCase
```

The system automatically orders transforms by dependency (topological sort).

### 4. Indentation Handling (from `crates/core/src/replacer/indent.rs`)

The replacer system has sophisticated indentation handling:
- De-indents matched code to remove source indentation
- Re-indents replacement to match target context
- Preserves relative indentation within multi-line blocks

This is already built-in and works automatically.

### 5. Fixer Capabilities (from `crates/config/src/fixer.rs`)

Additional capabilities available:
- `expandStart`/`expandEnd` - expand match range to include adjacent nodes
- Multiple fix suggestions with titles

**Not implementing in this story** - these are more advanced features.

---

## Design Decisions

### Decision 1: Transforms Only Apply to Replace Mode

**Question**: Should extract mode (move code to target file) support transforms?

**Decision**: No. Transforms only apply to replace mode.

**Rationale**:
- Extract is about **moving** code (reorganization)
- Transform is about **changing** code (renaming)
- These are conceptually different operations
- If extraction + rename needed, do two operations
- Keeps extract mode simple

### Decision 2: Batch Mode Only for Replace Mode

**Question**: Should batch mode work with extract mode?

**Decision**: No. Batch mode only applies to replace mode.

**Rationale**:
- Batch replace is the killer use case (rename 50 call sites at once)
- Batch extract is less compelling - extraction is usually deliberate, one-at-a-time
- Current append behavior supports multiple extracts via repeated calls
- Reduces implementation complexity

### Decision 3: Fail Fast on Transform Errors

**Question**: If a transform fails (e.g., invalid regex), should the operation fail or skip?

**Decision**: Fail the whole operation with a clear error.

**Rationale**:
- Silent failures are dangerous
- Agent might think rename succeeded when it didn't
- Especially critical in batch mode (50 broken call sites)
- Fail fast, fail loud - fix and retry

### Decision 4: Document All Separator Options

**Question**: Document all separators or just common ones?

**Decision**: Document all with brief descriptions.

**Rationale**:
- Incomplete docs are frustrating
- Implementation already supports all options
- Complete information is clearer than partial

---

## Tool Description Improvements

The current tool description is minimal. The enhanced description should include:

### Pattern Syntax Section
```
PATTERN SYNTAX:
Patterns match PARTIAL code structure, not exact strings.
Use meta-variables as wildcards:

  $NAME       - Captures single AST node (e.g., function name, variable)
  $$NAME      - Same but unnamed (structural matching only)
  $_          - Matches but discards (don't capture)
  $$$         - Matches zero or more nodes (anonymous)
  $$$ARGS     - Captures zero or more nodes (e.g., all function args)

Meta-variable names must be UPPERCASE: $NAME, $ARGS, $BODY (not $name)

EXAMPLES:
  fn $NAME($$$ARGS)           - matches any function
  console.log($MSG)           - matches console.log calls
  if ($COND) { $$$BODY }      - matches if statements
  import { $$$ITEMS } from $M - matches ES imports
```

### Transform Syntax Section
```
TRANSFORMS (replace mode only):
Apply transformations to captured variables before replacement.

  substring:
    source: $NAME
    startChar: 0      # optional, default 0
    endChar: -1       # optional, negative = from end

  replace:
    source: $NAME
    replace: "regex"  # pattern to find
    by: "text"        # replacement

  convert:
    source: $NAME
    toCase: camelCase  # lowerCase|upperCase|capitalize|camelCase|snakeCase|kebabCase|pascalCase
    separatedBy: [underscore, caseChange]  # optional: caseChange|underscore|dash|dot|slash|space

Transforms can chain: use $TRANSFORMED_VAR as source for another transform.
```

---

## API Design

### New Parameters

```typescript
interface AstGrepRefactorArgs {
  pattern: string;          // AST pattern with meta-variables
  language: string;         // rust, typescript, etc.
  source_file: string;      // file to refactor

  // Mode selection (mutually exclusive)
  target_file?: string;     // extract mode: move to this file
  replacement?: string;     // replace mode: replace with this template

  // New parameters
  transforms?: {            // replace mode only
    [varName: string]: Transform;
  };
  batch?: boolean;          // replace mode only, default false
  preview?: boolean;        // dry-run mode, default false
}

type Transform =
  | { substring: { source: string; startChar?: number; endChar?: number } }
  | { replace: { source: string; replace: string; by: string } }
  | { convert: { source: string; toCase: CaseType; separatedBy?: Separator[] } };

type CaseType = 'lowerCase' | 'upperCase' | 'capitalize' | 'camelCase' | 'snakeCase' | 'kebabCase' | 'pascalCase';
type Separator = 'caseChange' | 'underscore' | 'dash' | 'dot' | 'slash' | 'space';
```

### Result Structure

```typescript
interface AstGrepRefactorResult {
  success: boolean;
  mode: 'extract' | 'replace';

  // For single match (batch: false)
  moved_code?: string;
  original_code?: string;
  replacement_code?: string;
  match_location?: { line: number; column: number };

  // For batch mode
  matches_count?: number;
  matches?: Array<{
    location: { file: string; line: number; column: number };
    original: string;
    replacement: string;
  }>;

  // For preview mode
  preview?: boolean;  // true if no changes made

  source_file: string;
  target_file?: string;  // extract mode only
}
```

---

## Example Use Cases

### 1. Rename Function (Simple)
```json
{
  "pattern": "fn old_name($$$ARGS) { $$$BODY }",
  "language": "rust",
  "source_file": "src/lib.rs",
  "replacement": "fn new_name($$$ARGS) { $$$BODY }"
}
```

### 2. Rename with Case Conversion
```json
{
  "pattern": "fn $NAME($$$ARGS)",
  "language": "rust",
  "source_file": "src/lib.rs",
  "transforms": {
    "NEW_NAME": {
      "convert": {
        "source": "$NAME",
        "toCase": "camelCase"
      }
    }
  },
  "replacement": "fn $NEW_NAME($$$ARGS)"
}
```

### 3. Batch Rename All Call Sites
```json
{
  "pattern": "old_function($$$ARGS)",
  "language": "typescript",
  "source_file": "src/app.ts",
  "replacement": "new_function($$$ARGS)",
  "batch": true
}
```

### 4. Preview Before Applying
```json
{
  "pattern": "console.log($MSG)",
  "language": "typescript",
  "source_file": "src/app.ts",
  "replacement": "logger.info($MSG)",
  "batch": true,
  "preview": true
}
```

### 5. Chained Transforms
```json
{
  "pattern": "fn $NAME($$$ARGS)",
  "language": "rust",
  "source_file": "src/lib.rs",
  "transforms": {
    "STRIPPED": {
      "replace": {
        "source": "$NAME",
        "replace": "_impl$",
        "by": ""
      }
    },
    "FINAL": {
      "convert": {
        "source": "$STRIPPED",
        "toCase": "pascalCase"
      }
    }
  },
  "replacement": "fn $FINAL($$$ARGS)"
}
```

---

## Implementation Notes

### Transform Dependency Resolution

When transforms reference each other, resolve in dependency order:
1. Parse all transform sources to find dependencies
2. Topological sort to get execution order
3. Execute in order, building up transformed values
4. Error if cyclic dependency detected

### Batch Mode Safety

- Always preview first (recommend in docs)
- Return full list of changes in result
- Atomic: either all changes succeed or none
- Clear error if any match fails to transform

### Error Messages

Provide actionable error messages:
```
Transform error in 'NEW_NAME': Invalid regex in replace transform: unclosed group at position 5
Pattern matched 0 nodes. Check pattern syntax - remember patterns are partial matches, not exact code.
Meta-variable '$name' is invalid - names must be UPPERCASE (e.g., $NAME)
```

---

## Related Work

- **TOOLS-003**: Original AST refactor tool implementation (extract + replace modes)
- **ast-grep source**: https://github.com/ast-grep/ast-grep
- **ast-grep docs**: https://ast-grep.github.io/
