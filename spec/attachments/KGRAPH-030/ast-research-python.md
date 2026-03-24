# KGRAPH-030: AST Extractor — Python

## Overview

Add Python language support to the AST extraction pipeline. This includes an AST extractor for Python source files and a dependency extractor for `requirements.txt` and `pyproject.toml`.

## Files to Create

### 1. `codelet/napi/src/graph/ast_pipeline/ast_python_extractor.rs`

**SupportLang variant:** `SupportLang::Python`  
**Extensions:** `.py`, `.pyi`

#### Function Extraction Patterns

```rust
const PYTHON_FUNCTION_PATTERNS: &[(&str, bool)] = &[
    // Top-level and method definitions
    ("def $NAME($$$ARGS): $$$BODY", false),
    ("async def $NAME($$$ARGS): $$$BODY", false),
    // Decorated functions need parent-check for @staticmethod, @classmethod
];
```

**Notes:**
- Python indentation-based scoping means ast-grep parses indented blocks as function body
- `is_public` is determined by name convention: names starting with `_` are private, `__` are name-mangled
- `is_async` from `async def` keyword
- `param_count` must filter out `self` and `cls` parameters (similar to Rust's `count_params_rust`)
- Nested functions (closures inside functions) should be extracted as separate Function nodes

#### Type Extraction Patterns

```rust
const PYTHON_TYPE_PATTERNS: &[(&str, &str)] = &[
    ("class $NAME: $$$BODY", "class"),
    ("class $NAME($$$BASES): $$$BODY", "class"),  // with inheritance
];
```

**Notes:**
- `type_kind` is always `"class"` for Python
- Extract `Extends` edges from base classes in `class Foo(Bar, Baz):`
- Dataclasses (`@dataclass`) are still classes — no special handling needed
- `is_public` follows underscore convention

#### Import Extraction

```rust
// Patterns for import statements
"import $MODULE"
"from $MODULE import $$$NAMES"
"from $MODULE import ($$$NAMES)"
```

**Approach:** Parse import statements to create `Imports` edges (File → File). Python imports are module-path based (`from auth.models import User`), so we resolve dotted paths to file paths:
- `import auth.models` → look for `auth/models.py` or `auth/models/__init__.py`
- `from . import utils` → relative import from same package

For the initial version, create Imports edges with the raw `importPath` property and only resolve to actual files if they exist in `known_files`.

### 2. `codelet/napi/src/graph/ast_pipeline/python_dep_extractor.rs`

#### requirements.txt Parser

- Read `requirements.txt` from project root
- Parse each non-comment, non-empty line
- Handle version specifiers: `package==1.0`, `package>=1.0,<2.0`, `package`
- Strip extras: `package[extra1,extra2]>=1.0` → package name is `package`
- Ignore `-r other-file.txt` include directives (don't follow)
- Create `Dependency` node (source: `"pip"`) + `DependsOn` edge

#### pyproject.toml Parser

- Read `pyproject.toml` if it exists
- Parse `[project.dependencies]` array (PEP 621 format)
- Parse `[project.optional-dependencies]` as dev dependencies
- Also check `[tool.poetry.dependencies]` for Poetry projects
- Uses the `toml` crate (already in Cargo.toml for Cargo extractor)
- Create `Dependency` node (source: `"pip"`) + `DependsOn` edge

### 3. Pipeline Registration (in `mod.rs`)

```rust
// Add to SUPPORTED_EXTENSIONS
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "ts", "tsx", "js", "jsx", "mjs", "mts", "rs",
    "py", "pyi",  // NEW
];

// Add to extract_file() match
"py" | "pyi" => ast_python_extractor::extract_python(&source, &rel_path),

// Add to populate_ast_graph() dependency chain
all_entities.extend(python_dep_extractor::extract_python_dependencies(&project_root)?);
```

## Entity Summary

| Entity | Properties | Example |
|--------|-----------|---------|
| File (`.py`) | language=`"python"`, isTest from `test_` prefix or `tests/` dir | `src/auth/models.py` |
| Function | isAsync, isPublic (no underscore prefix), paramCount (excl. self/cls) | `def authenticate(user, password)` |
| Type | typeKind=`"class"`, isPublic | `class UserModel(Base)` |
| Dependency | source=`"pip"`, isDev, version | `dep::flask` |

## Edges

| Edge | From → To | Notes |
|------|-----------|-------|
| Contains | File → Function | |
| ContainsType | File → Type | |
| Imports | File → File | Resolve dotted module paths to file paths |
| DependsOn | File → Dependency | requirements.txt/pyproject.toml → pip packages |
| Extends | Type → Type | `class Foo(Bar)` — only if Bar's file is in the project |

## Testing Strategy

1. Unit test `extract_python()` with sample `.py` source containing functions, classes, imports
2. Unit test `extract_python_dependencies()` with sample `requirements.txt` and `pyproject.toml`
3. Integration test with a small Python project directory
4. Verify deduplication handles Python's `__init__.py` barrel pattern

## Estimated Complexity: 5 points
