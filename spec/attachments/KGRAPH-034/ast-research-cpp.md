# KGRAPH-034: AST Extractor — C++

## Overview

Add C++ language support to the AST extraction pipeline. Depends on KGRAPH-033 (C) for shared `#include` resolution patterns. No dependency extractor needed (C++ uses CMake/vcpkg/Conan which are too complex for initial support).

## Files to Create

### 1. `codelet/napi/src/graph/ast_pipeline/ast_cpp_extractor.rs`

**SupportLang variant:** `SupportLang::Cpp`
**Extensions:** `.cpp`, `.cc`, `.cxx`, `.hpp`, `.h` (when C++ detected)

#### Function Extraction Patterns

```rust
const CPP_FUNCTION_PATTERNS: &[(&str, bool)] = &[
    ("$RET $NAME($$$ARGS) { $$$BODY }", false),
    ("static $RET $NAME($$$ARGS) { $$$BODY }", false),
    ("$RET $NS::$NAME($$$ARGS) { $$$BODY }", false),
    ("virtual $RET $NAME($$$ARGS) { $$$BODY }", false),
    ("template <$$$TPARAMS> $RET $NAME($$$ARGS) { $$$BODY }", false),
];
```

**Notes:**
- Reuses `is_public` logic: `static` = private, namespace-level = public
- Methods inside class bodies inherit class visibility sections (`public:`/`private:`/`protected:`)
- `qualifiedName` for methods: `ClassName::methodName`
- Constructors/destructors: name = class name / `~ClassName`
- Operator overloads: name = `operator+` etc.

#### Type Extraction Patterns

```rust
const CPP_TYPE_PATTERNS: &[(&str, &str, bool)] = &[
    ("class $NAME { $$$BODY }", "class", false),
    ("class $NAME : $$$BASES { $$$BODY }", "class", false),
    ("struct $NAME { $$$BODY }", "struct_kind", false),
    ("enum $NAME { $$$VARIANTS }", "enum_kind", false),
    ("enum class $NAME { $$$VARIANTS }", "enum_kind", false),
    ("namespace $NAME { $$$BODY }", "namespace", false),
    ("template <$$$TPARAMS> class $NAME { $$$BODY }", "class", false),
];
```

- `class Foo : public Bar, private Baz` → `Extends` edges to Bar and Baz
- Namespaces modeled as Type with typeKind `"namespace"`

#### Include Extraction

Reuse the same `#include` pattern from C extractor. Factor the include-resolution logic into `helpers.rs` as a shared function.

### 2. `.h` Header Disambiguation

Update `extract_file()` in `mod.rs` with heuristic to route `.h` files:

```rust
fn source_is_cpp(source: &str) -> bool {
    source.contains("class ") || source.contains("namespace ")
        || source.contains("template") || source.contains("std::")
        || source.contains("public:") || source.contains("private:")
}
```

### 3. Pipeline Registration

```rust
// SUPPORTED_EXTENSIONS
"cpp", "cc", "cxx", "hpp"

// extract_file()
"cpp" | "cc" | "cxx" | "hpp" => ast_cpp_extractor::extract_cpp(&source, &rel_path),
// "h" → uses heuristic to choose C vs C++ extractor
```

## Edges

| Edge | From → To |
|------|-----------|
| Contains | File → Function |
| ContainsType | File → Type |
| Imports | File → File (#include) |
| Extends | Type → Type (class inheritance) |

## Estimated Complexity: 5 points
