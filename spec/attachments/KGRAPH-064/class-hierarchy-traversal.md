# KGRAPH-064: Class Hierarchy and Inheritance Traversal

## Problem

We have `Implements` and `Extends` edge types in our AST schema, and `ast_neighbors` shows direct parents/children. But there's no dedicated action to get the full inheritance tree, list methods of a class, or find all overrides of a method across subclasses. These are the basic questions agents ask about OOP codebases.

## CGC Reference Implementation

### find_class_hierarchy — `code_finder.py` lines 449–510

Returns parents, children, AND methods in a single call:

```python
def find_class_hierarchy(self, class_name, path=None, repo_path=None):
    # Query 1: Find parent classes
    parents_query = f"""
        {match_clause}
        MATCH (child)-[:INHERITS]->(parent:Class)
        RETURN DISTINCT parent.name, parent.path, parent.line_number, parent.docstring
    """
    
    # Query 2: Find child classes (subclasses)
    children_query = f"""
        {match_clause}
        MATCH (grandchild:Class)-[:INHERITS]->(child)
        RETURN DISTINCT grandchild.name, grandchild.path, grandchild.line_number
    """
    
    # Query 3: Find methods contained in this class
    methods_query = f"""
        {match_clause}
        MATCH (child)-[:CONTAINS]->(method:Function)
        RETURN DISTINCT method.name, method.path, method.line_number, method.args, method.docstring
    """
    
    return {
        "class_name": class_name,
        "parent_classes": parents_result.data(),
        "child_classes": children_result.data(),
        "methods": methods_result.data()
    }
```

### find_function_overrides — `code_finder.py` lines 512–533

Finds all implementations of a method name across different classes:

```python
def find_function_overrides(self, function_name, repo_path=None):
    result = session.run(f"""
        MATCH (class:Class)-[:CONTAINS]->(func:Function {{name: $function_name}})
        RETURN DISTINCT class.name, class.path, func.name, func.line_number,
               func.args, func.docstring
        ORDER BY class_name
        LIMIT 20
    """, function_name=function_name)
```

### MCP integration — `code_finder.py` lines 870–882

```python
elif query_type in ["class_hierarchy", "inheritance", "extends"]:
    results = self.find_class_hierarchy(target, context, repo_path=repo_path)
    return {
        "summary": f"Class '{target}' has {len(results['parent_classes'])} parents, "
                   f"{len(results['child_classes'])} children, "
                   f"and {len(results['methods'])} methods"
    }

elif query_type in ["overrides", "implementations", "polymorphism"]:
    results = self.find_function_overrides(target, repo_path=repo_path)
    return {
        "summary": f"Found {len(results)} implementations of function '{target}'"
    }
```

## What We Need to Implement

### New GraphSearch action — `ast_hierarchy`

```rust
AstHierarchy {
    node_id: String,           // type slug
    include_methods: Option<bool>,  // default: true
    include_overrides: Option<bool>, // default: false
    depth: Option<u32>,        // max traversal depth up/down, default: 3
}
```

Returns:
```json
{
  "type": { "name": "BaseHandler", "slug": "...", "path": "..." },
  "parents": [{ "name": "Object", "depth": 1, "via": "Extends" }],
  "children": [
    { "name": "HttpHandler", "depth": 1, "via": "Extends" },
    { "name": "WebSocketHandler", "depth": 1, "via": "Extends" }
  ],
  "methods": [
    { "name": "handle", "slug": "...", "path": "..." },
    { "name": "validate", "slug": "...", "path": "..." }
  ]
}
```

### New nanograph queries needed

```
// Parents: traverse Extends/Implements edges upward
match {
  $child: Type { slug: $slug }
  ($child)-[: Extends]->($parent: Type)
}
return $parent

// Children: traverse Extends/Implements edges downward  
match {
  $parent: Type { slug: $slug }
  ($child: Type)-[: Extends]->($parent)
}
return $child

// Methods: traverse Contains edges from File to Function, filtered by type
// This requires our extraction to create Contains edges from Type→Function
// OR we match on qualifiedName prefix
```

### Gap: Type→Function containment

CGC has `(Class)-[:CONTAINS]->(Function)` edges. Do we? Our current schema has:
- `(File)-[Contains]->(Function)` 
- `(File)-[ContainsType]->(Type)`

We may NOT have `(Type)-[Contains]->(Function)` for methods. If not, we need either:
1. Add a `ContainsMethod` edge from Type to Function during extraction
2. Use `qualifiedName` matching (e.g., `MyClass.myMethod` → functions where qualifiedName starts with type name)

### New action: `ast_overrides` (optional, could fold into ast_hierarchy)

```rust
AstOverrides {
    method_name: String,   // e.g., "render"
    limit: Option<u32>,
}
```

Finds all Function nodes with the same `name` that belong to different types in the same inheritance tree.

### Files to modify

| File | Change |
|------|--------|
| `codelet/tools/src/graph_search/types.rs` | Add `AstHierarchy` (and optionally `AstOverrides`) |
| `codelet/napi/src/graph_search_handler.rs` | Add dispatch |
| `codelet/napi/src/graph/` | Add nanograph queries for hierarchy traversal |
| `codelet/napi/src/ast_pipeline/` | Possibly add Type→Function containment edges |

### Effort estimate

**Medium** — The queries are straightforward if we have the right edge types. The main risk is whether our extraction pipeline creates Type→Function containment edges for methods.
