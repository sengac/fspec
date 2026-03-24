# KGRAPH-033: AST Extractor — C

## Overview

Add C language support to the AST extraction pipeline. C has no package manager, so this card only includes the AST extractor — no dependency extractor.

## Files to Create

### 1. `codelet/napi/src/graph/ast_pipeline/ast_c_extractor.rs`

**SupportLang variant:** `SupportLang::C`  
**Extensions:** `.c`, `.h`

#### Function Extraction Patterns

```rust
const C_FUNCTION_PATTERNS: &[&str] = &[
    // Standard function definitions (with body)
    "$RET $NAME($$$ARGS) { $$$BODY }",
    "static $RET $NAME($$$ARGS) { $$$BODY }",
    // Void return
    "void $NAME($$$ARGS) { $$$BODY }",
    "static void $NAME($$$ARGS) { $$$BODY }",
    // Pointer return
    "$RET *$NAME($$$ARGS) { $$$BODY }",
];
```

**Notes:**
- C has no classes, so all functions are top-level
- `is_public` = NOT `static` (static functions are file-scoped/private in C)
- `is_async` is always `false`
- Function declarations in `.h` files (prototypes without body) should be skipped — only definitions with `{ body }` count
- `param_count` from argument list; filter `void` as a parameter (C convention for no-args: `void foo(void)`)
- Variadic functions (`...`) count the named params only

#### Type Extraction Patterns

```rust
const C_TYPE_PATTERNS: &[(&str, &str)] = &[
    ("struct $NAME { $$$FIELDS }", "struct_kind"),
    ("typedef struct { $$$FIELDS } $NAME;", "struct_kind"),
    ("typedef struct $TAG { $$$FIELDS } $NAME;", "struct_kind"),
    ("enum $NAME { $$$VARIANTS }", "enum_kind"),
    ("typedef enum { $$$VARIANTS } $NAME;", "enum_kind"),
    ("typedef $BASE $NAME;", "type_alias"),
    ("union $NAME { $$$FIELDS }", "struct_kind"),  // unions modeled as struct_kind
];
```

**Notes:**
- `typedef` creates the usable name — extract the typedef name, not the struct tag
- C has no visibility modifiers for types — `is_public` is always `true` for non-static types
- Anonymous structs inside typedefs: extract the typedef name

#### Include Extraction

```rust
// #include directives
"#include \"$PATH\""       // project-local includes
"#include <$PATH>"         // system includes (optionally skip these)
```

**Approach:**
- `#include "foo.h"` → look for `foo.h` relative to the file's directory and project root
- `#include <stdlib.h>` → system headers; create Import edges with importPath but no target file resolution
- Create `Imports` edges (File → File) for project-local includes that resolve to known files
- Mark system includes with a property `isSystem: true`

**Note:** `#include` is a preprocessor directive, not a language statement. ast-grep with tree-sitter-c should still parse it as a `preproc_include` node. If ast-grep pattern matching doesn't handle `#include`, fall back to line-based regex scanning.

### 2. Pipeline Registration

```rust
// SUPPORTED_EXTENSIONS
"c", "h"

// extract_file() match
"c" | "h" => ast_c_extractor::extract_c(&source, &rel_path),
```

**Important:** `.h` files are ambiguous — they could be C or C++. When both C and C++ extractors are registered (KGRAPH-034), the `.h` extension dispatch needs a heuristic:
- If the file contains `class`, `namespace`, `template`, or `#include <iostream>` → route to C++
- Otherwise → route to C
- For this card, assume `.h` = C. The C++ card will refine the heuristic.

## Entity Summary

| Entity | Properties | Example |
|--------|-----------|---------|
| File | language=`"c"`, isTest from filename patterns | `src/parser.c` |
| Function | isPublic (not static), paramCount | `int parse_token(const char *input)` |
| Type | typeKind: struct_kind/enum_kind/type_alias | `typedef struct Node { ... } Node;` |

## Edges

| Edge | From → To |
|------|-----------|
| Contains | File → Function |
| ContainsType | File → Type |
| Imports | File → File (#include resolution) |

## C-Specific Considerations

- **Preprocessor macros:** `#define MACRO(x) ...` — not extracted as functions (they're preprocessor text substitutions)
- **Function pointers:** `void (*callback)(int)` — the parameter type, not a function definition
- **Forward declarations:** `struct Foo;` — skip these, only extract full definitions
- **Header guards:** `#ifndef HEADER_H` — ignored by the extractor (tree-sitter handles this)
- **Static inline:** `static inline int foo() { ... }` — isPublic = false

## Testing Strategy

1. Unit test `extract_c()` with sample `.c`/`.h` files containing functions, structs, enums, typedefs
2. Verify `#include` extraction creates proper Imports edges
3. Verify `static` functions are marked as non-public
4. Test typedef extraction picks up the typedef name, not the struct tag

## Estimated Complexity: 5 points
