# KGRAPH-068: Decorator and Annotation Search

## Problem

Our AST extractors don't capture decorators, annotations, or attributes on functions and types. Agents can't answer "find all API endpoints" (= functions with `@app.route`, `@Get()`, `@RequestMapping`), "find all test functions" (= `@test`, `@Test`), or "find all event handlers" (= `@EventHandler`, `@on`).

## CGC Reference Implementation

### Decorators stored during parsing — `graph_builder.py` ~line 400

Function nodes include a `decorators` list property:
```python
SET f.decorators = $decorators   # e.g., ["@app.route", "@login_required"]
```

### Python decorator extraction — `languages/python.py`

```python
# Extract decorators from function/class definition nodes
decorators = []
for child in node.children:
    if child.type == 'decorator':
        decorator_text = child.text.decode('utf-8').strip()
        decorators.append(decorator_text)
```

### TypeScript decorator extraction — `languages/typescript.py`

Extracts: `@Component`, `@Injectable`, `@Get()`, etc. from TypeScript/Angular/NestJS code.

### Find by decorator — `code_finder.py` lines 257–281

```python
def find_functions_by_decorator(self, decorator_name, path=None, repo_path=None):
    """Find functions that have a specific decorator applied."""
    query = f"""
        MATCH (f:Function)
        WHERE $decorator_name IN f.decorators {repo_filter}
        RETURN f.name AS function_name, f.path AS path, 
               f.line_number AS line_number, f.docstring AS docstring,
               f.decorators AS decorators
        ORDER BY f.path, f.line_number
        LIMIT 20
    """
```

### Find by argument — `code_finder.py` lines 231–255

```python
def find_functions_by_argument(self, argument_name, path=None, repo_path=None):
    """Find functions that take a specific argument name."""
    query = f"""
        MATCH (f:Function)-[:HAS_PARAMETER]->(p:Parameter)
        WHERE p.name = $argument_name {repo_filter}
        RETURN f.name AS function_name, f.path AS path, f.line_number AS line_number
        LIMIT 20
    """
```

**Note**: CGC uses a separate `Parameter` node type with `HAS_PARAMETER` edges. This is more granular than storing params as a string property.

### MCP integration — `code_finder.py` lines 849–861

```python
elif query_type == "find_functions_by_argument":
    results = self.find_functions_by_argument(target, context, repo_path=repo_path)
    return { "summary": f"Found {len(results)} functions that take '{target}' as an argument" }

elif query_type == "find_functions_by_decorator":
    results = self.find_functions_by_decorator(target, context, repo_path=repo_path)
    return { "summary": f"Found {len(results)} functions decorated with '{target}'" }
```

## What We Need to Implement

### Schema extension

Add `decorators` property to Function and Type nodes:

```
node Function {
  @key slug: String
  name: String
  ...
  decorators: String    // JSON array: '["@app.route", "@login_required"]'
}

node Type {
  @key slug: String
  name: String
  ...
  decorators: String    // JSON array: '["@Component", "@Injectable"]'
}
```

### Extraction per language

| Language | Decorator Syntax | AST Node Type |
|----------|-----------------|---------------|
| Python | `@decorator` | `decorator` |
| TypeScript | `@Decorator()` | `decorator` |
| Java | `@Annotation` | `annotation`, `marker_annotation` |
| Kotlin | `@Annotation` | `annotation` |
| Rust | `#[attribute]` | `attribute_item` |
| C# | `[Attribute]` | `attribute_list` |
| Go | N/A (no decorators) | — |
| Swift | `@attribute` | `attribute` |
| PHP | `#[Attribute]` (PHP 8+) | `attribute_list` |
| Ruby | N/A (no decorators, but method wrappers) | — |

### New GraphSearch capability

Option A: Extend `ast_search` with a `decorator` filter:

```rust
AstSearch {
    query: String,
    entity_type: Option<EntityType>,
    decorator: Option<String>,  // NEW: filter by decorator name
    limit: Option<u32>,
    path: Option<String>,
}
```

Option B: Dedicated `ast_by_decorator` action:

```rust
AstByDecorator {
    decorator: String,       // e.g., "@app.route" or "Test"
    entity_type: Option<EntityType>, // Function or Type
    path: Option<String>,
    limit: Option<u32>,
}
```

### Query implementation

After loading all functions, filter client-side:

```rust
fn matches_decorator(entity: &Entity, decorator: &str) -> bool {
    if let Some(decorators_json) = entity.get("decorators") {
        if let Ok(decorators) = serde_json::from_str::<Vec<String>>(decorators_json) {
            return decorators.iter().any(|d| {
                d.contains(decorator) || d.trim_start_matches('@').contains(decorator)
            });
        }
    }
    false
}
```

### Parameter search (optional enhancement)

CGC's `find_functions_by_argument` uses a separate `Parameter` node type. We could either:
1. Store parameters as a JSON array property (simpler)
2. Create Parameter nodes with HAS_PARAMETER edges (more powerful, but heavier)

**Recommendation**: Start with property-based storage (JSON array string), add Parameter nodes later if needed.

### Files to modify

| File | Change |
|------|--------|
| `codelet/napi/src/ast_pipeline/` | Extract decorators in each language extractor |
| `codelet/napi/src/graph/` | Add `decorators` property to schema |
| `codelet/tools/src/graph_search/types.rs` | Add decorator filter to AstSearch |
| `codelet/napi/src/graph/dispatch_helpers.rs` | Add decorator matching logic |

### Effort estimate

**Low** — The extraction is straightforward for most languages (decorators are always direct children of function/class definition nodes in the AST). Schema change is a single property. Query is client-side filtering.

### High-value examples

| Decorator | Meaning | Framework |
|-----------|---------|-----------|
| `@app.route` | HTTP endpoint | Flask |
| `@Get`, `@Post` | HTTP endpoint | NestJS |
| `@RequestMapping` | HTTP endpoint | Spring |
| `@test`, `@Test` | Test function | Vitest, JUnit |
| `@Component` | UI component | Angular |
| `#[test]` | Test function | Rust |
| `#[derive(...)]` | Trait derivation | Rust |
| `@property` | Getter | Python |
| `@staticmethod` | Static method | Python |
