# Python & Java ast_index Crash — Imported Classes Create Dangling Function Slug References

## Summary

`ast_index` crashes on real-world Python and Java repos. The edge extraction code (added in KGRAPH-054) creates `Calls` edges referencing imported names as Function nodes, but when the imported name is a **class** (Type node), the graph loader fails with "node not found by @key".

## Reproduction

```
GraphSearch(ast_index, path: "tmp/python-click")
→ ERROR: Failed to load entities into graph: node not found by @key: Function.slug=src-click-parser-py::_OptionParser

GraphSearch(ast_index, path: "tmp/java-gson")
→ ERROR: Failed to load entities into graph: node not found by @key: Type.slug=gson-src-main-java-com-google-gson-JsonIOException-java::JsonIOException
```

## Root Cause Analysis

### The Call Chain

1. Python: `from click.parser import _OptionParser` → `import_map["_OptionParser"] = ("src-click-parser-py", true, "_OptionParser")`
2. Later, `_OptionParser(ctx)` appears in a function body → `extract_call_names_from_body` captures `_OptionParser` as a callee name
3. `resolve_calls` (edge_helpers.rs:228-234) looks up `_OptionParser` in `import_map`, finds it, and creates a `Calls` edge:
   - `from_slug`: `src-click-core-py::make_parser`
   - `to_slug`: `src-click-parser-py::_OptionParser` ← **treated as a Function**
4. But `_OptionParser` is a **class** (Type node with slug `src-click-parser-py::_OptionParser`), not a Function
5. The graph loader tries to resolve the `to_slug` as a Function node, fails, and aborts the entire index operation

### Why Unit Tests Pass

The test fixtures in `ast_edge_quality_test.rs` only use scenarios where imported names are functions. Example:
```python
# Test fixture
from click.core import Command  # Command is tested as a TYPE in TypeRef tests
```
But no test covers the case where an imported **class** is called as a constructor — which is extremely common in real Python and Java code.

### Affected Code Paths

- `codelet/napi/src/graph/ast_pipeline/edge_helpers.rs:211-237` — `resolve_calls()` function
- `codelet/napi/src/graph/ast_pipeline/ast_python_extractor.rs:282-322` — `extract_calls()`
- `codelet/napi/src/graph/ast_pipeline/ast_java_extractor.rs:265-310` — `extract_calls()`

## Evidence from Real Repos

### Python (click)

```python
# src/click/core.py:35
from .parser import _OptionParser

# src/click/core.py:1083 — constructor call
parser = _OptionParser(ctx)
```

`_OptionParser` is defined as `class _OptionParser:` at parser.py:220. The import_map resolves it correctly, but `resolve_calls` emits a Calls edge to `src-click-parser-py::_OptionParser` which doesn't exist as a Function node.

### Java (gson)

```java
// ReflectionHelper.java:19
import com.google.gson.JsonIOException;

// ReflectionHelper.java:71
throw new JsonIOException("...")
```

`JsonIOException` is a class in `JsonIOException.java`. The import resolver finds it, but the edge creates a reference to a Type slug that doesn't match.

## Fix Approach

The core issue is that `resolve_calls` doesn't distinguish between functions and classes in the import_map. Several approaches:

### Option A: Check both Function and Type slugs
Before emitting a Calls edge, check whether the target slug exists as a Function or Type. If it's a Type, either:
- Skip the Calls edge (constructors are typically not "calls" in the dead code sense)
- Emit a TypeRef edge instead (more accurate — it IS a type reference)

### Option B: Store entity kind in import_map
Extend the import_map tuple to include whether the imported name is a function or type. Then `resolve_calls` can decide what edge type to emit.

### Option C: Make the graph loader tolerant of missing nodes
Instead of aborting on missing nodes, skip the dangling edge. This is less precise but prevents crashes.

### Recommended: Option A (with TypeRef fallback)
- Most correct semantically: `_OptionParser(ctx)` IS a type reference + constructor call
- Doesn't require changing the import_map schema
- Prevents the crash AND improves edge quality

## Verification Matrix

After fix, re-index these repos and confirm:
1. `tmp/python-click` — indexes successfully, Imports > 0, no crash
2. `tmp/java-gson` — indexes successfully, Imports > 0, no crash
3. `tmp/go-cobra` — still works (regression check)
4. `tmp/php-slim` — still works (gold standard regression check)
5. Dead code results on each should be meaningful (not all-orphans)

## Languages NOT Affected

Go edge extraction uses a different code path (same-package implicit imports, method receiver parsing) that doesn't go through `resolve_calls` for cross-file references the same way. All other languages (Ruby, Kotlin, C#, C, C++, Rust, Scala, Swift, TypeScript) index successfully.
