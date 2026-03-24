# KGRAPH-031: AST Extractor — Go

## Overview

Add Go language support to the AST extraction pipeline. This includes an AST extractor for Go source files and a dependency extractor for `go.mod`.

## Files to Create

### 1. `codelet/napi/src/graph/ast_pipeline/ast_go_extractor.rs`

**SupportLang variant:** `SupportLang::Go`  
**Extensions:** `.go`

#### Function Extraction Patterns

```rust
const GO_FUNCTION_PATTERNS: &[&str] = &[
    // Standalone functions
    "func $NAME($$$ARGS) { $$$BODY }",
    "func $NAME($$$ARGS) $RET { $$$BODY }",
    "func $NAME($$$ARGS) ($$$RETS) { $$$BODY }",
    // Method receivers — capture receiver but extract name
    "func ($RECV $TYPE) $NAME($$$ARGS) { $$$BODY }",
    "func ($RECV $TYPE) $NAME($$$ARGS) $RET { $$$BODY }",
    "func ($RECV $TYPE) $NAME($$$ARGS) ($$$RETS) { $$$BODY }",
    "func ($RECV *$TYPE) $NAME($$$ARGS) { $$$BODY }",
    "func ($RECV *$TYPE) $NAME($$$ARGS) $RET { $$$BODY }",
    "func ($RECV *$TYPE) $NAME($$$ARGS) ($$$RETS) { $$$BODY }",
];
```

**Notes:**
- Go has no `async` keyword — all functions are synchronous (goroutines are at call site)
- `is_public` is determined by first letter capitalization: `Uppercase` = exported, `lowercase` = unexported
- `param_count` from args; receiver parameter is NOT counted
- Method receivers create an implicit relationship to their struct type — generate `qualifiedName` as `TypeName.MethodName`

#### Type Extraction Patterns

```rust
const GO_TYPE_PATTERNS: &[(&str, &str)] = &[
    ("type $NAME struct { $$$FIELDS }", "struct_kind"),
    ("type $NAME interface { $$$METHODS }", "interface"),
    ("type $NAME = $BASE", "type_alias"),       // type alias
    ("type $NAME $BASE", "type_alias"),          // type definition
];
```

**Notes:**
- Go interfaces are implicit (no `implements` keyword), so no `Implements` edges from AST alone
- Embedded structs (`type Foo struct { Bar }`) could generate `Extends` edges

#### Import Extraction

```rust
// Go import patterns
"import \"$PATH\""
"import ($$$IMPORTS)"     // grouped imports
```

**Approach:**
- Go imports are package paths, not file paths: `"github.com/user/repo/pkg"`
- For project-internal imports, resolve against the module path from `go.mod`
- External imports don't map to local files — skip or create stub File nodes
- Create `Imports` edges with `importPath` property

### 2. `codelet/napi/src/graph/ast_pipeline/go_dep_extractor.rs`

#### go.mod Parser

- Read `go.mod` from project root
- Parse the `module` directive (project's own module path)
- Parse `require` block:
  ```
  require (
      github.com/gin-gonic/gin v1.9.0
      golang.org/x/crypto v0.14.0
  )
  ```
- Parse individual `require` directives
- Detect `// indirect` comments to mark as dev/indirect dependencies
- Create `Dependency` node (source: `"go"`) + `DependsOn` edge

**go.sum is NOT parsed** — it's a lockfile, not a dependency declaration.

### 3. Pipeline Registration (in `mod.rs`)

```rust
// Add to SUPPORTED_EXTENSIONS
"go"

// Add to extract_file() match
"go" => ast_go_extractor::extract_go(&source, &rel_path),

// Add to dependency chain
all_entities.extend(go_dep_extractor::extract_go_dependencies(&project_root)?);
```

## Entity Summary

| Entity | Properties | Example |
|--------|-----------|---------|
| File (`.go`) | language=`"go"`, isTest from `_test.go` suffix | `internal/auth/handler.go` |
| Function | isPublic (capitalized), paramCount, qualifiedName for methods | `func (s *Server) HandleLogin(w, r)` |
| Type | typeKind: struct_kind/interface/type_alias | `type Config struct { ... }` |
| Dependency | source=`"go"`, version, isDev (indirect) | `dep::github.com/gin-gonic/gin` |

## Edges

| Edge | From → To | Notes |
|------|-----------|-------|
| Contains | File → Function | |
| ContainsType | File → Type | |
| Imports | File → File | Resolve internal package paths via go.mod module prefix |
| DependsOn | File → Dependency | go.mod → Go modules |

## Go-Specific Considerations

- **Test files:** `_test.go` suffix — set `isTest = true`
- **Build tags:** Ignore `//go:build` constraints; extract all files
- **Generated files:** Skip files starting with `// Code generated` comment (optional)
- **Package declaration:** Could be used to group files, but not needed for initial version

## Testing Strategy

1. Unit test `extract_go()` with sample `.go` files containing funcs, methods, structs, interfaces
2. Unit test `extract_go_dependencies()` with sample `go.mod`
3. Verify method receivers correctly set qualifiedName
4. Verify exported/unexported detection via capitalization

## Estimated Complexity: 5 points
