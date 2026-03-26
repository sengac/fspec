# KGRAPH-062: Cyclomatic Complexity Analysis

## Problem

Our AST graph stores Function nodes with `name`, `slug`, `path`, `qualifiedName`, `visibility` — no complexity metrics. Agents cannot answer "which functions should I refactor first?" or "find the most complex functions in the codebase."

## CGC Reference Implementation

### Complexity stored during indexing — `graph_builder.py` lines 312–348

Every Function node gets a `cyclomatic_complexity` property set during the tree-sitter parse phase. Each language parser calculates this by counting decision points in the AST.

### Schema constraint — `graph_builder.py` line 160

```python
session.run("CREATE CONSTRAINT function_unique IF NOT EXISTS "
            "FOR (f:Function) REQUIRE (f.name, f.path, f.line_number) IS UNIQUE")
```

Function nodes include: `name`, `path`, `line_number`, `end_line`, `args`, `cyclomatic_complexity`, `decorators`, `lang`, `source`, `docstring`, `is_dependency`

### Query: get complexity of a specific function — `code_finder.py` lines 960–985

```python
def get_cyclomatic_complexity(self, function_name, path=None, repo_path=None):
    query = f"""
        MATCH (f:Function {{name: $function_name}})
        WHERE (f.path ENDS WITH $path OR f.path = $path) {repo_filter}
        RETURN f.name as function_name, f.cyclomatic_complexity as complexity,
               f.path as path, f.line_number as line_number
    """
```

### Query: find most complex functions — `code_finder.py` lines 987–999

```python
def find_most_complex_functions(self, limit=10, repo_path=None):
    query = f"""
        MATCH (f:Function)
        WHERE f.cyclomatic_complexity IS NOT NULL AND f.is_dependency = false {repo_filter}
        RETURN f.name as function_name, f.path as path, 
               f.cyclomatic_complexity as complexity, f.line_number as line_number
        ORDER BY f.cyclomatic_complexity DESC
        LIMIT $limit
    """
```

### MCP exposure — `tool_definitions.py` lines 94–117

Two dedicated MCP tools:
- `calculate_cyclomatic_complexity` — single function lookup
- `find_most_complex_functions` — top-N across codebase

### Python parser example — `languages/python.py`

CGC counts these AST nodes as decision points:
- `if_statement`, `elif_clause`
- `for_statement`, `while_statement`
- `except_clause`, `with_statement`
- `and` / `or` boolean operators
- `conditional_expression` (ternary)
- `match_statement` / `case_clause`
- `assert_statement`
- `list_comprehension`, `set_comprehension`, `dictionary_comprehension`, `generator_expression`

Base complexity = 1 + count of decision points.

## What We Need to Implement

### Cyclomatic Complexity Calculation

We already parse the full AST via ast-grep during extraction. We need to add a complexity counter that walks the AST and counts decision points per function.

**Language-specific decision point patterns:**

| Language | Decision Points |
|----------|----------------|
| TypeScript/JavaScript | `if`, `else if`, `for`, `while`, `do`, `switch case`, `catch`, `&&`, `\|\|`, `??`, `?.`, ternary |
| Rust | `if`, `else if`, `match` arm, `for`, `while`, `loop`, `&&`, `\|\|`, `?` operator |
| Python | `if`, `elif`, `for`, `while`, `except`, `and`, `or`, ternary, comprehensions |
| Go | `if`, `for`, `switch case`, `select case`, `&&`, `\|\|` |
| Java | `if`, `else if`, `for`, `while`, `do`, `switch case`, `catch`, `&&`, `\|\|`, ternary |
| C/C++ | `if`, `else if`, `for`, `while`, `do`, `switch case`, `&&`, `\|\|`, `?:` |

### Schema change — nanograph AST-code schema

Add `cyclomaticComplexity` property to Function node type:

```
node Function {
  @key slug: String
  name: String
  path: String
  qualifiedName: String
  visibility: String
  cyclomaticComplexity: Int  // NEW
}
```

### New GraphSearch action — `ast_complexity`

```rust
AstComplexity {
    limit: Option<u32>,      // default: 20
    min_threshold: Option<u32>, // only return functions above this
    path: Option<String>,    // glob filter
}
```

Returns functions sorted by descending complexity.

### Alternative: Enrich `ast_search` results

Instead of a dedicated action, add `cyclomaticComplexity` to the properties returned by `ast_search` when `entity_type = Function`. This is simpler and avoids tool proliferation.

### Files to modify

| File | Change |
|------|--------|
| `codelet/napi/src/ast_pipeline/` | Add complexity counting to each language extractor |
| `codelet/napi/src/graph/` | Update AST schema to include `cyclomaticComplexity` |
| `codelet/tools/src/graph_search/types.rs` | Add action variant (if dedicated) |
| `codelet/napi/src/graph_search_handler.rs` | Add dispatch + query |

### Effort estimate

**Low** — We already walk the full AST for extraction. Adding a complexity counter is a ~20-line function per language that counts specific node types. The schema change is a single property addition. The query is trivial (ORDER BY complexity DESC).
