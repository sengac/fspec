# KGRAPH-063: Source Code and Metadata Storage in Graph Nodes

## Problem

Our Function and Type nodes store only structural metadata (`name`, `slug`, `path`, `qualifiedName`, `visibility`). When an agent finds a function via the graph, it must make a separate `Read` tool call to see the actual code. CGC stores source code directly in graph nodes, cutting tool calls in half.

## CGC Reference Implementation

### Properties stored per Function — `graph_builder.py` lines 312–348, 388–450

```python
session.run("""
    MERGE (f:Function {name: $name, path: $path, line_number: $line_number})
    SET f.end_line = $end_line,
        f.args = $args,
        f.source = $source,
        f.docstring = $docstring,
        f.decorators = $decorators,
        f.cyclomatic_complexity = $cyclomatic_complexity,
        f.is_dependency = $is_dependency,
        f.lang = $lang,
        f.context = $context
""", ...)
```

### What each language parser extracts — e.g. `languages/python.py`

Each language-specific parser returns a dict per function containing:
- `name` — function/method name
- `line_number` — start line
- `end_line` — end line (critical for knowing function boundaries)
- `args` — parameter list as array of strings
- `source` — **full source code** of the function body
- `docstring` — extracted docstring/JSDoc/rustdoc
- `decorators` — list of decorator/attribute names
- `cyclomatic_complexity` — complexity score
- `context` — "class" or "module" (scope indicator)
- `lang` — language identifier

### Properties stored per Class — `graph_builder.py` lines 450–490

```python
session.run("""
    MERGE (c:Class {name: $name, path: $path, line_number: $line_number})
    SET c.end_line = $end_line,
        c.bases = $bases,
        c.source = $source,
        c.docstring = $docstring,
        c.decorators = $decorators,
        c.is_dependency = $is_dependency,
        c.lang = $lang
""", ...)
```

### How source is extracted — `languages/typescript.py` (representative)

```python
def _extract_source(self, node):
    """Extract source text from a tree-sitter node."""
    return node.text.decode('utf-8') if node.text else ''
```

The full text of the AST node (function/class body) is stored directly.

## What We Need to Implement

### Extend nanograph schema

Current Function node:
```
node Function {
  @key slug: String
  name: String
  path: String
  qualifiedName: String
  visibility: String
}
```

Proposed Function node:
```
node Function {
  @key slug: String
  name: String
  path: String
  qualifiedName: String
  visibility: String
  startLine: Int           // NEW
  endLine: Int             // NEW
  parameters: String       // NEW — JSON array or comma-separated
  source: String           // NEW — first N lines of source (capped)
  docstring: String        // NEW — extracted doc comment
  decorators: String       // NEW — JSON array of decorator names
  language: String         // NEW — "typescript", "rust", etc.
}
```

Similarly for Type nodes.

### Source extraction during AST walk

We use ast-grep for extraction. During the walk, for each matched function/type:
1. Read the matched node's text span from the source file
2. Cap at ~100 lines or 4KB to prevent graph bloat
3. Extract leading doc comments (JSDoc, rustdoc, Python docstrings)
4. Extract parameter names from function signatures
5. Extract decorator/attribute lists

### Storage size considerations

Storing full source for every function could bloat the graph significantly:
- A project with 5,000 functions averaging 30 lines × 40 chars = ~6MB
- With 100-line cap: manageable
- **Recommendation**: Store first 50 lines + a `truncated: bool` flag
- Alternatively, store only the **signature** + docstring (much smaller)

### Impact on other cards

- **KGRAPH-067 (Full-Text Search)** depends on this — needs `source` and `docstring` properties to search over
- **KGRAPH-068 (Decorator Search)** depends on `decorators` property
- **KGRAPH-062 (Complexity)** needs `startLine`/`endLine` for AST range

### Files to modify

| File | Change |
|------|--------|
| `codelet/napi/src/graph/` | Update AST schema with new properties |
| `codelet/napi/src/ast_pipeline/` | Extract source, docstring, params, decorators per language |
| `codelet/napi/src/graph/graph_entities.rs` | Add new fields to GraphEntity serialization |
| `codelet/napi/src/graph/dispatch_helpers.rs` | Include new fields in search results |

### Effort estimate

**Low-Medium** — Schema change is straightforward. Source extraction is mostly reading byte ranges from the already-parsed file. Docstring extraction requires language-specific logic (JSDoc vs rustdoc vs Python docstrings). Parameter extraction is already partially done in some extractors.
