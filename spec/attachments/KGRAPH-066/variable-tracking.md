# KGRAPH-066: Variable and Symbol Tracking

## Problem

Our AST graph has no concept of variables or constants. It indexes `File`, `Module`, `Function`, `Type`, and `Dependency` — but not the symbols that live inside functions or at module scope. Agents can't answer "where is `API_KEY` set?" or "what functions modify `user_count`?"

## CGC Reference Implementation

### Variable as node type — `graph_builder.py` line 165

```python
session.run("CREATE CONSTRAINT variable_unique IF NOT EXISTS "
            "FOR (v:Variable) REQUIRE (v.name, v.path, v.line_number) IS UNIQUE")
```

### Variable properties stored — `graph_builder.py` ~lines 490–530

Each Variable node has:
- `name` — variable name
- `path` — file path (absolute)
- `line_number` — definition line
- `value` — initial value (as string, if extractable)
- `context` — scope: "function", "class", or "module"
- `is_dependency` — whether from a dependency

### Variable extraction — `languages/python.py` (representative)

Python parser extracts:
- Module-level assignments (`x = 5`)
- Class-level attributes (`self.x = 5` in `__init__`)
- Constants (ALL_CAPS names)
- Type-annotated variables
- Variable docstrings (Python 3.12+)

### Find variables by name — `code_finder.py` lines 86–98

```python
def find_by_variable_name(self, search_term, repo_path=None):
    result = session.run(f"""
        MATCH (v:Variable)
        WHERE v.name CONTAINS $search_term 
              {"AND v.path STARTS WITH $repo_path" if repo_path else ""}
        RETURN v.name as name, v.path as path, v.line_number as line_number,
               v.value as value, v.context as context, v.is_dependency as is_dependency
        ORDER BY v.is_dependency ASC, v.name
        LIMIT 20
    """, search_term=search_term)
```

### Who modifies a variable — `code_finder.py` lines 417–447

```python
def who_modifies_variable(self, variable_name, repo_path=None):
    result = session.run(f"""
        MATCH (var:Variable {{name: $variable_name}})
        MATCH (container)-[:CONTAINS]->(var)
        WHERE (container:Function OR container:Class OR container:File)
        RETURN DISTINCT
            CASE WHEN container:Function THEN container.name ... END as container_name,
            CASE WHEN container:Function THEN 'function' ... END as container_type,
            var.line_number as variable_line_number,
            var.value as variable_value,
            var.context as variable_context
    """)
```

### Variable scope analysis — `code_finder.py` lines 761–821

```python
def find_variable_usage_scope(self, variable_name, path=None, repo_path=None):
    """Find the scope and usage patterns of a variable."""
    result = session.run(f"""
        MATCH (var:Variable {{name: $variable_name}})
        OPTIONAL MATCH (container)-[:CONTAINS]->(var)
        RETURN DISTINCT
            var.name as variable_name,
            var.value as variable_value,
            var.line_number as line_number,
            var.context as context,
            CASE WHEN container:Function THEN 'function'
                 WHEN container:Class THEN 'class'
                 ELSE 'module' END as scope_type,
            CASE WHEN container:Function THEN container.name
                 WHEN container:Class THEN container.name
                 ELSE 'module_level' END as scope_name
    """)
    return { "variable_name": variable_name, "instances": result.data() }
```

### MCP integration — `code_finder.py` lines 863–868

```python
elif query_type in ["who_modifies", "modifies", "mutations", "changes", "variable_usage"]:
    results = self.who_modifies_variable(target, repo_path=repo_path)
    return { "summary": f"Found {len(results)} containers that hold variable '{target}'" }
```

## What We Need to Implement

### New node type in nanograph schema

```
node Variable {
  @key slug: String         // e.g., "file__src_config_ts__API_KEY"
  name: String              // e.g., "API_KEY"
  path: String              // file path
  line: Int                 // definition line
  value: String             // initial value (optional, capped length)
  scope: String             // "module", "class", "function"
  scopeName: String         // containing class/function name, or ""
  isConstant: Boolean       // true for const/final/ALL_CAPS
}
```

### New edge types

```
edge ContainsVariable {
  from: File -> Variable     // or Function -> Variable, Type -> Variable
}

edge Assigns {
  from: Function -> Variable  // which functions assign to this variable
}
```

### Extraction changes per language

Each extractor needs to identify:

| Language | Module-level | Class-level | Constants |
|----------|-------------|-------------|-----------|
| TypeScript | `const/let/var x = ...` at top level | `static x = ...` in class | `const` keyword |
| Rust | `static`, `const`, `lazy_static!` | Struct fields | `const` keyword |
| Python | Top-level assignments | `self.x` in `__init__` | ALL_CAPS convention |
| Go | `var x`, package-level | Struct fields | Exported (capitalized) |
| Java | `static` fields | Instance fields | `final` keyword |

### New GraphSearch actions

Option A: Extend `ast_search` with `entity_type: Variable`
Option B: Dedicated `ast_variables` action

```rust
AstVariables {
    query: String,           // search by name
    scope: Option<String>,   // "module", "class", "function"
    constants_only: Option<bool>,
    path: Option<String>,    // glob filter
    limit: Option<u32>,
}
```

### Files to modify

| File | Change |
|------|--------|
| `codelet/napi/src/graph/` | Add Variable node type + edges to schema |
| `codelet/napi/src/ast_pipeline/` | Add variable extraction to each language extractor |
| `codelet/tools/src/graph_search/types.rs` | Add Variable to entity_type enum or new action |
| `codelet/napi/src/graph_search_handler.rs` | Add dispatch + queries |

### Effort estimate

**High** — This requires:
1. Schema extension (new node + edges)
2. Variable extraction logic in all 14 language extractors
3. New queries for search + scope analysis
4. Testing across languages

The extraction is the hard part — identifying "meaningful" variables (not loop vars or temp vars) requires language-specific heuristics.

### Scoping recommendation

Start with **module-level constants and exported variables only** (the highest-value, lowest-noise subset). Expand to class-level and function-level in a follow-up.
