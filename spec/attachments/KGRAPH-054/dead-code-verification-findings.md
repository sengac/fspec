# Dead Code Verification Findings — AST Extractor Edge Quality Gaps

## Context

After completing KGRAPH-041 (Cross-Language Calls/Imports/TypeRef Edge Extraction), we verified `ast_dead_code` on real-world open-source repos cloned in `tmp/`. The edge extractors are emitting edges, but several quality gaps produce false positives and inflated orphan counts.

## Test Repos Used

| Language | Repo | Directory |
|----------|------|-----------|
| PHP | Slim Framework | `tmp/php-slim` |
| Python | Click | `tmp/python-click` |
| Go | Cobra | `tmp/go-cobra` |
| Java | Gson | `tmp/java-gson` |

## Edge Coverage Summary

| Language | Calls | Imports | TypeRef | Total | Quality |
|----------|-------|---------|---------|-------|---------|
| **PHP** | 151 | 251 | 124 | 1,384 | ✅ **High** — all 3 edge types populated |
| **Python** | 441 | **0** | **0** | 1,201 | ⚠️ **Low** — zero Imports/TypeRef |
| **Go** | 312 | **3** | **0** | 747 | ⚠️ **Low** — near-zero Imports/TypeRef |
| **Java** | 932 | **0** | 186 | 4,412 | ⚠️ **Medium** — zero Imports |

---

## Issue 1: Python — Zero Imports Edges (Imports=0)

### Problem
The Python edge extractor (`ast_python_extractor.rs`) emits **zero** `Imports` edges between File nodes. This means `ast_dead_code` reports **all 33 source files as orphans**, even though Python `import` statements clearly reference other modules.

### Evidence
```
ast_index on tmp/python-click:
  Calls: 441, Imports: 0, TypeRef: 0
```

All files reported as orphans:
- `docs/conf.py` — ✅ TRUE orphan (Sphinx config)
- `examples/inout/inout.py` — ✅ TRUE orphan (standalone example)
- `src/click/__init__.py` — ❌ FALSE orphan (imported by everything!)
- `src/click/core.py` — ❌ FALSE orphan (imported by `__init__.py`)

### Root Cause
The Python edge extractor likely emits Calls edges for function calls but does NOT resolve `import` statements (`import click`, `from click.core import Command`) into `Imports` edges between File nodes. The extractor needs to:
1. Parse `import X` and `from X import Y` statements
2. Resolve module names to file paths using Python's module resolution rules
3. Emit `Imports` edges from the importing File to the imported File

### Fix Required
In `codelet/napi/src/ast_python_extractor.rs`:
- Add tree-sitter queries for `import_statement` and `import_from_statement` nodes
- Resolve module paths to files using the `known_files` list
- Emit `Imports` edges from File→File

---

## Issue 2: Java — Zero Imports Edges (Imports=0)

### Problem
The Java edge extractor emits **zero** `Imports` edges despite Java having explicit `import` statements. This causes all files without TypeRef connections to appear as orphans.

### Evidence
```
ast_index on tmp/java-gson:
  Calls: 932, Imports: 0, TypeRef: 186
```

Orphan files reported:
- `metrics/src/.../BagOfPrimitivesDeserializationBenchmark.java` — ✅ TRUE orphan (standalone benchmark)
- `metrics/src/.../SerializationBenchmark.java` — ✅ TRUE orphan
- `metrics/src/.../CollectionsDeserializationBenchmark.java` — ✅ TRUE orphan
- `metrics/src/.../NonUploadingCaliperRunner.java` — ✅ TRUE orphan
- But files in `gson/src/main/java/com/google/gson/` are also showing as orphans despite being imported extensively

### Root Cause
The Java edge extractor emits Calls and TypeRef edges but does NOT parse `import` statements into File→File edges.

### Fix Required
In `codelet/napi/src/ast_java_extractor.rs`:
- Add tree-sitter queries for `import_declaration` nodes
- Resolve fully-qualified class names to file paths (e.g., `import com.google.gson.Gson` → `gson/src/main/java/com/google/gson/Gson.java`)
- Emit `Imports` edges from File→File

---

## Issue 3: Go — Near-Zero Imports Edges (Imports=3)

### Problem
Go has only **3** Imports edges despite having explicit `import` statements in every file. Additionally, Go's same-package model means files in the same package don't import each other, so all root-level files appear orphaned.

### Evidence
```
ast_index on tmp/go-cobra:
  Calls: 312, Imports: 3, TypeRef: 0
```

20 orphan files reported, including `command.go` (the **core file** of the entire library).

### Root Cause
1. The Go extractor IS emitting some Imports edges (3), but only for cross-package imports that happen to match a `known_files` entry
2. Go's same-package files (all `.go` files in the root of cobra) don't `import` each other — they're in the same package and share symbols implicitly
3. This means same-package Go files will ALWAYS appear orphaned in the current model

### Fix Required
In `codelet/napi/src/ast_go_extractor.rs`:
- Improve cross-package import resolution (the 3 edges suggest partial implementation)
- Consider adding **implicit same-package edges**: when multiple `.go` files share the same `package X` declaration, they should have bidirectional Imports edges to represent Go's implicit same-package visibility
- Alternative: Add a "package membership" concept where files in the same Go package are connected

---

## Issue 4: Go — Method-Body Calls to Package-Level Functions Not Captured

### Problem
Go functions called from within method receivers (e.g., `func (c *Command) Find(...)` calling `stripFlags()`) do NOT produce `Calls` edges. This causes legitimate functions to be reported as dead.

### Evidence (all FALSE POSITIVES):

| Function | File | Called From | Lines |
|----------|------|-------------|-------|
| `stripFlags` | `command.go` | `command.go:761`, `command.go:776` | Called from `Find()` method |
| `isFlagArg` | `command.go` | `command.go:844`, `completions.go:693` | Called from `ArgsFunction` method + cross-file |
| `commandNameMatches` | `command.go` | `command.go:801`, `command.go:1553` | Called from methods |
| `defaultUsageFunc` | `command.go` | `command.go:472` | Returned as function reference |

### Root Cause
The Go Calls edge extractor captures function calls at the package level but does NOT capture calls made from within method bodies (method receivers). In Go, methods are defined as `func (c *Command) MethodName()` — the extractor needs to also search for function calls WITHIN these method bodies.

### Fix Required
In `codelet/napi/src/ast_go_extractor.rs`:
- Extend the tree-sitter query to also capture `call_expression` nodes inside `method_declaration` bodies (not just `function_declaration` bodies)
- Ensure Calls edges are emitted from the calling function/method to the called function

---

## Issue 5: Go — Zero TypeRef Edges (TypeRef=0)

### Problem
Go emits **zero** TypeRef edges, so all type definitions (structs, interfaces) appear as unreferenced dead types.

### Evidence
```
ast_index on tmp/go-cobra: TypeRef: 0
```

`Command` struct — the CORE type of the entire cobra library — is reported as dead/unreferenced despite being used in virtually every file.

### Root Cause
The Go extractor does not emit TypeRef edges. In Go, type references appear as:
- Function parameters: `func Foo(c *Command)`
- Struct fields: `type Bar struct { cmd *Command }`
- Variable declarations: `var c Command`
- Type assertions: `x.(*Command)`

### Fix Required
In `codelet/napi/src/ast_go_extractor.rs`:
- Add tree-sitter queries to detect type name references in function signatures, struct fields, variable declarations, and type assertions
- Emit `TypeRef` edges from the containing function/type to the referenced type

---

## Issue 6: Python — Zero TypeRef Edges (TypeRef=0)

### Problem
Python emits zero TypeRef edges. Type hints (`def foo(x: str) -> int`) are not captured.

### Fix Required
In `codelet/napi/src/ast_python_extractor.rs`:
- Parse type annotations in function signatures and variable annotations
- Emit TypeRef edges for referenced types

---

## Summary of Fixes Needed

| # | Language | Issue | Severity | Effort |
|---|----------|-------|----------|--------|
| 1 | Python | Zero Imports edges | 🔴 High | Medium — need module path resolution |
| 2 | Java | Zero Imports edges | 🔴 High | Medium — need FQ class name resolution |
| 3 | Go | Near-zero Imports edges | 🔴 High | Medium-Hard — same-package implicit imports |
| 4 | Go | Method-body Calls missing | 🔴 High | Easy — extend tree-sitter query scope |
| 5 | Go | Zero TypeRef edges | 🟡 Medium | Medium — type reference detection |
| 6 | Python | Zero TypeRef edges | 🟡 Medium | Easy-Medium — type annotation parsing |

### Priority Order
1. **Issue 4** (Go method-body Calls) — easiest fix, highest accuracy improvement
2. **Issue 1** (Python Imports) — most impactful for Python dead code accuracy
3. **Issue 2** (Java Imports) — same pattern as Python
4. **Issue 3** (Go Imports / same-package) — requires design decision for Go's model
5. **Issue 5** (Go TypeRef) — improves type dead code detection
6. **Issue 6** (Python TypeRef) — nice-to-have for type tracking

---

## Verification Method

For each fix, re-run:
```
GraphSearch(ast_index, path: "tmp/<repo>")
GraphSearch(ast_dead_code, path: "tmp/<repo>")
```

Then verify:
1. Edge counts increase for the fixed edge type
2. False positive count decreases
3. True positives (genuinely dead code) remain detected
4. No regressions on existing Rust/TypeScript/PHP extractors

## PHP Reference (Gold Standard)

PHP-Slim serves as the reference implementation — it has all 3 edge types populated and dead code detection is highly accurate:
- 9 orphan files — ALL verified as true positives
- `HttpNotFoundException` and `HttpMethodNotAllowedException` correctly EXCLUDED (30+ references)
- Calls, Imports, and TypeRef all working correctly
