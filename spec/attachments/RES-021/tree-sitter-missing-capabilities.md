# Tree-Sitter Capabilities Not Used in fspec AST Tool

**Analysis Date:** 2025-11-15
**Source:** AST analysis of tree-sitter source code using `fspec research --tool=ast`

## Executive Summary

The fspec AST research tool currently uses only basic tree-sitter features (manual tree traversal with hardcoded node type checks). A comprehensive analysis of tree-sitter's TypeScript implementation reveals **seven major capability categories** that could significantly enhance the AST tool's power, performance, and usability.

## Analysis Methodology

Used `fspec research --tool=ast --operation=list-functions --file <path>` on the following tree-sitter source files:
- `~/projects/tree-sitter/vendor/tree-sitter/lib/binding_web/src/parser.ts`
- `~/projects/tree-sitter/vendor/tree-sitter/lib/binding_web/src/node.ts`
- `~/projects/tree-sitter/vendor/tree-sitter/lib/binding_web/src/query.ts`
- `~/projects/tree-sitter/vendor/tree-sitter/lib/binding_web/src/tree.ts`
- `~/projects/tree-sitter/vendor/tree-sitter/lib/binding_web/src/tree_cursor.ts`

## Current fspec AST Usage

The AST utilities are currently used in two places:

1. **`fspec research --tool=ast`** (src/research-tools/ast.ts, src/utils/query-executor.ts)
   - Supported operations: `list-functions`, `find-class`, `list-classes`, `find-functions`, `list-imports`, `list-exports`, `find-exports`, `find-async-functions`, `find-identifiers`
   - Uses **manual tree traversal** (recursive `node.children` loops)
   - Hardcoded node type checks (e.g., `node.type === 'function_declaration'`)
   - Basic predicate filtering (name, pattern, min-params, export-type)
   - Supports 15 languages via language detection

2. **`fspec review <work-unit-id>`** (src/commands/review.ts, src/utils/ast-data-gatherer.ts)
   - Gathers structural data: function/class/import/export counts
   - Detects duplicate function names across files
   - Calls `fspec research --tool=ast` internally for each file
   - Used to generate code analysis prompts for AI review
   - Provides "CODE STRUCTURE DATA" in system-reminders

---

## 1. Tree-Sitter Query System (⭐ HIGHEST PRIORITY)

### Current State
fspec uses manual tree traversal with hardcoded node type checks:
```typescript
function traverse(n: Parser.SyntaxNode) {
  if (n.type === 'function_declaration' || n.type === 'function' || ...) {
    // Manual matching logic
  }
  for (const child of n.children) {
    traverse(child);
  }
}
```

### Available But Unused
**Query class with S-expression pattern matching:**
- `Query.matches(node, options)` - Find all matches of a query pattern
- `Query.captures(node, options)` - Capture specific nodes from patterns
- `Query.predicates` - Advanced filtering predicates:
  - `#eq?` / `#not-eq?` - Text equality checks
  - `#match?` / `#not-match?` - Regex pattern matching
  - `#any-of?` - Match against list of values
  - `#is?` / `#is-not?` - Property assertions
- `Query.disableCapture(name)` - Optimize by disabling captures
- `Query.disablePattern(index)` - Optimize by disabling patterns
- `Query.patternCount()` - Get number of patterns
- `Query.captureIndexForName(name)` - Lookup capture by name
- `Query.isPatternRooted(index)` - Check if pattern must match at root
- `Query.isPatternNonLocal(index)` - Check if pattern can match anywhere
- `Query.isPatternGuaranteedAtStep(byteIndex)` - Query step analysis
- `Query.startIndexForPattern(index)` / `Query.endIndexForPattern(index)` - Pattern boundaries
- Pattern properties: `setProperties`, `assertedProperties`, `refutedProperties`
- Capture quantifiers: Control how many times captures can match

### Example Query Usage
```scheme
; Find all async functions with more than 3 parameters
(function_declaration
  (async)
  name: (identifier) @func.name
  parameters: (formal_parameters
    (required_parameter)+) @params
  (#match? @func.name "^handle")
  (#eq? @params.length "3"))
```

### Benefits
- **Declarative**: Express patterns concisely instead of imperative traversal
- **Powerful**: Built-in predicates for filtering
- **Performant**: Optimized C implementation
- **Maintainable**: Query files separate from code

### Implementation Impact
- Add new operations: `--operation=query` with `--query-file=<path.scm>`
- Support inline queries: `--query="(function_declaration name: (identifier) @name)"`
- Return captures with metadata (pattern index, properties)

---

## 2. TreeCursor API (⭐ PERFORMANCE)

### Current State
Recursive traversal creating Node objects at each step:
```typescript
for (const child of n.children) {
  traverse(child);  // Creates Node wrapper for each child
}
```

### Available But Unused
**Stateful cursor for efficient tree walking:**
- `TreeCursor.copy()` - Clone cursor state
- `TreeCursor.gotoFirstChild()` / `gotoLastChild()` - Navigate to children
- `TreeCursor.gotoNextSibling()` / `gotoPreviousSibling()` - Navigate siblings
- `TreeCursor.gotoParent()` - Navigate up
- `TreeCursor.gotoDescendant(index)` - Jump to specific descendant by index
- `TreeCursor.gotoFirstChildForIndex(byteOffset)` - Navigate by byte offset
- `TreeCursor.gotoFirstChildForPosition(point)` - Navigate by row/column position
- `TreeCursor.reset(node)` / `resetTo(cursor)` - Reuse cursor instances
- `TreeCursor.currentNode` - Get current node
- `TreeCursor.currentDepth` - Get nesting level
- `TreeCursor.currentDescendantIndex` - Get position in tree
- `TreeCursor.currentFieldId` / `currentFieldName` - Field context
- `TreeCursor.nodeType` / `nodeTypeId` - Type without allocating Node
- `TreeCursor.nodeStateId` - Parse state
- `TreeCursor.nodeIsNamed` / `nodeIsMissing` - Node flags
- `TreeCursor.nodeText` - Get text without Node wrapper
- `TreeCursor.startPosition` / `endPosition` / `startIndex` / `endIndex` - Position info

### Benefits
- **Performance**: No Node allocation overhead during traversal
- **Memory**: Single cursor vs thousands of Node objects
- **Flexibility**: Jump to specific positions/offsets

### Implementation Impact
- Use cursors internally for better performance
- Add cursor-based operations for large file analysis
- Reduce memory footprint on large codebases

---

## 3. Advanced Node Navigation

### Current State
Basic parent/child/sibling navigation only:
```typescript
node.children
node.parent
node.firstChild / node.lastChild
node.nextSibling / node.previousSibling
```

### Available But Unused
- **`Node.closest(types)`** - Find nearest ancestor matching type(s) ⭐ NEW!
  ```typescript
  const funcDecl = node.closest(['function_declaration', 'arrow_function']);
  ```
- **`Node.childWithDescendant(descendant)`** - Get child containing a descendant
- **`Node.descendantsOfType(types, startPos, endPos)`** - Filter descendants by type and range
- **`Node.fieldNameForChild(index)`** / **`fieldNameForNamedChild(index)`** - Field introspection
- **`Node.childrenForFieldName(name)`** / **`childrenForFieldId(id)`** - Get all children for a field
- **`Node.parseState`** / **`nextParseState`** - Access GLR parse states
- **`Node.grammarType`** / **`grammarId`** - Distinguish between aliased and actual types
- **`Node.descendantCount`** - Get total descendants without iterating
- **`Node.descendantForIndex(start, end?)`** - Get node at byte offset
- **`Node.namedDescendantForIndex(start, end?)`** - Get named node at byte offset
- **`Node.descendantForPosition(start, end?)`** - Get node at row/column
- **`Node.namedDescendantForPosition(start, end?)`** - Get named node at row/column
- **`Node.firstChildForIndex(index)`** / **`firstNamedChildForIndex(index)`** - Get child extending beyond byte offset

### Benefits
- **Context-aware queries**: "Find the function I'm inside of"
- **Precise navigation**: Jump to exact byte/position
- **Grammar-agnostic**: Works with aliased types

### Implementation Impact
- Add `--operation=find-context` to find containing function/class/scope
- Add `--operation=find-at-position --row=X --column=Y` for IDE-like queries
- Support `--operation=descendants-of-type --types=function,class --start-line=X --end-line=Y`

---

## 4. Field-Based Access

### Current State
Hardcoded child searches:
```typescript
const nameNode = n.children.find(c => c.type === 'identifier');
```

### Available But Unused
- **`Node.childForFieldName(fieldName)`** - Get first child with field name
- **`Node.childForFieldId(fieldId)`** - Get child by numeric field ID
- **`Node.childrenForFieldName(fieldName)`** - Get all children for field (e.g., all parameters)
- **`Node.childrenForFieldId(fieldId)`** - Get all children by field ID
- **`Node.fieldNameForChild(index)`** - Reverse lookup: what field is this child?
- **`Node.fieldNameForNamedChild(index)`** - Field name for named child
- **`Language.fields`** - Array of all field names in grammar

### Benefits
- **Reliability**: Fields are stable across grammar versions
- **Clarity**: `node.childForFieldName('name')` vs `children.find(c => c.type === 'identifier')`
- **Completeness**: Get all values for repeating fields (parameters, arguments)

### Implementation Impact
- Add `--operation=find-by-field --field=name` to extract specific fields
- Use fields internally instead of type-based child searches
- Add field metadata to output: `{ node: ..., field: "name" }`

---

## 5. Tree Edit Tracking

### Current State
Full file re-parsing on every analysis.

### Available But Unused
- **`Tree.edit(edit)`** - Mark edited regions for incremental reparsing
- **`Tree.getChangedRanges(otherTree)`** - Diff two parse trees
- **`Tree.rootNodeWithOffset(bytes, extent)`** - Shift tree positions
- **`Node.edit(edit)`** - Update node positions after edits
- **`Node.hasChanges`** - Check if node was affected by edits
- **Edit object structure:**
  ```typescript
  {
    startIndex: number,
    oldEndIndex: number,
    newEndIndex: number,
    startPosition: Point,
    oldEndPosition: Point,
    newEndPosition: Point
  }
  ```

### Benefits
- **Performance**: Only reparse edited sections
- **Diffing**: Track what changed between versions
- **Incremental analysis**: Re-analyze only affected nodes

### Implementation Impact
- Low priority for one-shot analysis tool
- High priority if AST tool becomes persistent/incremental
- Could cache parse trees between runs

---

## 6. Parser Configuration

### Current State
Parse entire file with default settings:
```typescript
const tree = parser.parse(sourceCode);
```

### Available But Unused
- **`Parser.parse(input, oldTree, options)`** with options:
  - `includedRanges: Range[]` - Parse only specific byte ranges
  - `progressCallback: (bytes: number) => void` - Track progress
- **`Parser.getIncludedRanges()`** - Get configured ranges
- **`Parser.setTimeoutMicros(timeout)`** / **`getTimeoutMicros()`** - Timeout protection
- **`Parser.setLogger(callback)`** / **`getLogger()`** - Debug parsing issues
- **`Parser.reset()`** - Clear parser state between unrelated parses
- **`Parser.Language.nodeTypeInfo`** - Schema of node types, fields, children

### Benefits
- **Safety**: Timeout prevents infinite loops on malformed input
- **Targeted parsing**: Only parse function bodies, not entire file
- **Debugging**: Logger shows parse decisions

### Implementation Impact
- Add `--timeout=<ms>` flag for safety
- Add `--range=<start>:<end>` to parse subset
- Add `--debug` flag to enable logging

---

## 7. Language Introspection

### Current State
Hardcoded knowledge of specific node types per language.

### Available But Unused
- **`Language.fields`** - Array of all field names in grammar
- **`Language.types`** - Array of all node type names (indexed by typeId)
- **`Language.nodeTypeInfo`** - Full schema:
  ```typescript
  {
    type: string,
    named: boolean,
    fields?: { [name: string]: ChildNode },
    children?: ChildNode[],
    subtypes?: BaseNode[]
  }
  ```
- **`LookaheadIterator`** - Get valid next tokens for error recovery/autocomplete

### Benefits
- **Dynamic discovery**: Learn grammar capabilities at runtime
- **Generic operations**: Works with any language without hardcoding
- **Error recovery**: Suggest valid next tokens

### Implementation Impact
- Add `--operation=list-node-types` to show all types in grammar
- Add `--operation=list-fields` to show all fields
- Add `--operation=introspect-node --type=function_declaration` to show structure
- Make operations language-agnostic using introspection

---

## Benefits for `fspec review` Command

The missing tree-sitter capabilities would **significantly enhance** the `fspec review` command's code analysis:

### Current Review Limitations
- **Counts only** (functions, classes, imports, exports) - no semantic analysis
- **Name-based duplicate detection** - misses similar logic with different names
- **Manual traversal performance** - slow on large codebases
- **No complexity metrics** - can't identify God functions or deep nesting
- **No pattern matching** - can't find specific anti-patterns (e.g., unhandled errors)

### Potential Improvements with Missing Capabilities

1. **Query System** → Detect anti-patterns declaratively
   ```scheme
   ; Find functions with try-catch blocks that swallow errors
   (try_statement
     (catch_clause
       (statement_block) @empty-catch)
     (#match? @empty-catch "^\\{\\s*\\}$"))
   ```

2. **TreeCursor API** → 10x faster structural analysis on large files
   - Replace recursive traversal with stateful cursor
   - Reduce memory overhead from thousands of Node allocations
   - Enable real-time analysis during review

3. **Node.closest()** → Context-aware analysis
   ```typescript
   // Find if a function is inside a class or standalone
   const containingClass = node.closest(['class_declaration', 'class_definition']);
   if (containingClass) {
     // Method - check for proper this usage
   } else {
     // Standalone function - check for proper exports
   }
   ```

4. **descendantsOfType()** → Targeted complexity metrics
   ```typescript
   // Count nested if statements (cyclomatic complexity indicator)
   const nestedIfs = node.descendantsOfType(['if_statement']);
   if (nestedIfs.length > 5) {
     // Flag high complexity
   }
   ```

5. **Field-Based Access** → Grammar-agnostic analysis
   ```typescript
   // Works across all languages with "name" field
   const functionName = node.childForFieldName('name');
   // Instead of:
   const nameNode = node.children.find(c => c.type === 'identifier');
   ```

### Example: Enhanced Review Analysis

**Before (Current)**:
```
CODE STRUCTURE DATA
  Functions: 42
  Classes: 3
  Imports: 12
  Exports: 8
```

**After (With Missing Capabilities)**:
```
CODE STRUCTURE DATA
  Functions: 42 (12 with >5 parameters, 3 with cyclomatic complexity >10)
  Classes: 3 (1 with >15 methods - possible God object)
  Imports: 12
  Exports: 8

ANTI-PATTERNS DETECTED:
  - 5 empty catch blocks (silently swallowing errors)
  - 3 functions with nested depth >4 (hard to test)
  - 2 classes with circular dependencies

REFACTORING OPPORTUNITIES:
  - Functions with similar structure: processUser(), processOrder(), processPayment()
    (consider extracting common pattern)
```

---

## Recommendation: Priority Order for Implementation

### Phase 1: High Impact (v0.9.0)
1. ⭐ **Query System** (query.ts)
   - Most powerful missing feature
   - Enables declarative pattern matching
   - Add `--operation=query --query-file=<path.scm>`
   - Add `--operation=query --query="<inline>"`

2. ⭐ **Field-Based Access**
   - More reliable than type-based matching
   - Use internally for existing operations
   - Add `--operation=find-by-field --field=<name>`

3. ⭐ **closest() Method**
   - Add `--operation=find-context` to find containing function/class
   - Useful for IDE-like "where am I?" queries

### Phase 2: Performance (v0.10.0)
4. 🚀 **TreeCursor API**
   - Performance boost for large files
   - Use internally to reduce memory overhead
   - No new user-facing operations, just faster execution

5. 🚀 **Parser Configuration**
   - Add `--timeout=<ms>` for safety
   - Add `--range=<start>:<end>` for targeted parsing
   - Add `--debug` for troubleshooting

### Phase 3: Advanced (v0.11.0+)
6. 📊 **Language Introspection**
   - Add `--operation=list-node-types`
   - Add `--operation=list-fields`
   - Add `--operation=introspect-node --type=<name>`

7. 🔄 **Incremental Parsing**
   - Low priority for current one-shot analysis
   - High priority if tool becomes persistent
   - Implement only if performance issues arise

---

## API Examples: Before vs After

### Finding Async Functions (Current)
```bash
# Current: Hardcoded operation
fspec research --tool=ast --operation=find-async-functions --file=src/api.ts
```

### Finding Async Functions (With Queries)
```bash
# Phase 1: Query-based
fspec research --tool=ast --operation=query \
  --query="(function_declaration (async) name: (identifier) @name)" \
  --file=src/api.ts
```

### Finding Context (New Capability)
```bash
# Phase 1: Find containing function at line 42
fspec research --tool=ast --operation=find-context \
  --row=42 --column=10 \
  --context-type=function \
  --file=src/api.ts
```

### Language Introspection (New Capability)
```bash
# Phase 3: Discover grammar capabilities
fspec research --tool=ast --operation=list-node-types --file=src/api.ts
fspec research --tool=ast --operation=introspect-node --type=function_declaration --file=src/api.ts
```

---

## Breaking Changes

None expected. All enhancements are additive:
- Existing operations continue to work
- New operations are opt-in via `--operation` flag
- Internal refactors (TreeCursor, field-based access) are transparent

---

## Testing Strategy

1. **Query System**: Test all predicate types (#eq?, #match?, #any-of?)
2. **Field Access**: Verify against all 15 supported languages
3. **Context Finding**: Test nested function/class scenarios
4. **Performance**: Benchmark cursor vs recursive traversal
5. **Regression**: Ensure existing operations still work

---

## Implementation Estimate

- **Phase 1** (Query + Fields + Context): ~3-5 points (8-16 hours)
- **Phase 2** (Cursor + Parser Config): ~2-3 points (4-8 hours)
- **Phase 3** (Introspection + Incremental): ~3-5 points (8-16 hours)

**Total**: 8-13 points (~20-40 hours across 3 releases)

---

## References

- Tree-sitter documentation: https://tree-sitter.github.io/tree-sitter/
- Query syntax: https://tree-sitter.github.io/tree-sitter/using-parsers#pattern-matching-with-queries
- Node.js bindings: https://github.com/tree-sitter/node-tree-sitter
- Source analysis: Used `fspec research --tool=ast` on tree-sitter TypeScript implementation files
