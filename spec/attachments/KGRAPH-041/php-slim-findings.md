# PHP Slim — Dead Code Detection Gap Analysis

## Summary

After rewriting the PHP extractor to use `KindMatcher` (KGRAPH-040), function extraction went from **3 → 734** on the Slim framework repo. However, dead code detection (`ast_dead_code`) returns **zero results** because the PHP extractor only emits `Contains` and `ContainsType` edges — it lacks the cross-reference edges (`Imports`, `Calls`, `TypeRef`) needed to build a reachability graph.

## Current State

```
Indexing results for php-slim (126 files, 734 functions, 124 types):

  Calls:        0   ← needed for dead code
  Contains:     734
  ContainsType: 124
  Imports:      0   ← needed for dead code
  TypeRef:      0   ← needed for dead code
```

## What's Missing

### 1. Imports Edges (File → File)

PHP uses `use` statements with fully-qualified namespace paths:

```php
// Slim/App.php
use Slim\Factory\ServerRequestCreatorFactory;
use Slim\Interfaces\CallableResolverInterface;
use Slim\Routing\RouteCollectorProxy;
use Slim\Routing\RouteResolver;
use Slim\Routing\RouteRunner;
use Psr\Http\Message\ResponseFactoryInterface;  // external — skip
```

**Approach:** Parse `use Slim\Foo\Bar` statements. Map namespace paths to file paths using PSR-4 convention (`Slim\Routing\RouteResolver` → `Slim/Routing/RouteResolver.php`). Only emit edges for files that exist in the project (skip external packages like `Psr\*`).

**AST node kind:** `namespace_use_declaration` contains `namespace_use_clause` children.

### 2. Calls Edges (Function → Function)

PHP functions/methods call other functions using several patterns:

```php
// Same-class method call via $this->
$this->middlewareDispatcher->add($middleware);
$this->getRouteResolver();

// Static method call
ServerRequestCreatorFactory::create();

// Free function call
strtoupper($request->getMethod());

// Constructor call
new RouteRunner($this->routeResolver, ...);
new MiddlewareDispatcher($routeRunner, ...);
```

**Approach for same-file calls:**
- Scan function bodies for `$this->methodName(` patterns
- Match against known methods in the same file
- Emit `Calls` edge from caller to callee

**Approach for cross-file calls:**
- `ClassName::method()` — resolve ClassName via `use` imports
- `new ClassName()` — resolve via `use` imports (→ `__construct`)
- Bare function calls (`strtoupper`) — typically builtins, skip unless imported

**Key difference from TypeScript:** PHP calls are heavily object-oriented. `$this->method()` is the most common call pattern. In TS, bare `functionName()` calls dominate.

### 3. TypeRef Edges (Function → Type)

PHP has rich type annotations:

```php
// Parameter types
public function __construct(
    ResponseFactoryInterface $responseFactory,
    ?ContainerInterface $container = null,
    ?CallableResolverInterface $callableResolver = null,
)

// Return types
public function getRouteResolver(): RouteResolverInterface

// Property types
protected RouteResolverInterface $routeResolver;

// Class relationships
class App extends RouteCollectorProxy implements RequestHandlerInterface
```

**Approach:**
- Parse parameter type hints from function signatures
- Parse return type annotations (`: TypeName`)
- Parse `extends` and `implements` clauses on class declarations
- Resolve type names via `use` import map (same as Imports resolution)
- Emit `TypeRef` edges for local types, `Extends`/`Implements` for inheritance

### 4. Extends/Implements Edges (Type → Type)

```php
class App extends RouteCollectorProxy implements RequestHandlerInterface
class ErrorHandler extends AbstractErrorRenderer
class HttpBadRequestException extends HttpException
```

Currently not extracted. These edges connect the type hierarchy and are essential for understanding which types are truly unused.

## Reference Implementation

The **TypeScript extractor** (`ast_ts_extractor.rs`) is the complete reference. Key functions:

| Function | What it extracts | Edge type |
|----------|-----------------|-----------|
| `extract_imports()` | `import { X } from './module'` | `Imports` (File → File) |
| `extract_calls()` | `functionName()` in function bodies | `Calls` (Function → Function) |
| `extract_type_refs()` | `: TypeName` in signatures | `TypeRef` (Function → Type) |
| `extract_call_names_from_body()` | Bare identifier calls (not methods) | Helper for Calls |
| `extract_type_names_from_signature()` | Type annotations after `:` | Helper for TypeRef |

## PHP-Specific AST Node Kinds

From tree-sitter-php, the relevant node kinds for extraction:

| What to find | AST node kind |
|-------------|---------------|
| `use Foo\Bar` | `namespace_use_declaration` → `namespace_use_clause` |
| `$this->method()` | `member_call_expression` |
| `ClassName::method()` | `scoped_call_expression` |
| `functionName()` | `function_call_expression` |
| `new ClassName()` | `object_creation_expression` |
| `: ReturnType` | `return_type` child of `method_declaration` |
| `TypeHint $param` | `type` child of `simple_parameter` |
| `extends Foo` | `base_clause` |
| `implements Bar` | `class_interface_clause` |

## Impact on Dead Code Detection

With all 3 edge types populated, `ast_dead_code` on php-slim would be able to:
- **Find orphan files** — PHP files never imported by any other file
- **Find uncalled methods** — methods never called via `$this->`, `static::`, or `new`
- **Find unreferenced types** — interfaces/traits never used in type hints or inheritance

## Scale Across All Languages

This same gap exists for ALL 12 non-TS extractors. Each language has its own import/call/type syntax:

| Language | Import syntax | Call syntax | Type syntax |
|----------|--------------|-------------|-------------|
| PHP | `use Foo\Bar` | `$this->m()`, `Foo::m()` | `: Type`, `extends`, `implements` |
| Python | `import`, `from x import y` | `func()`, `self.method()` | `: Type` (PEP 484) |
| Go | `import "pkg"` | `pkg.Func()`, `func()` | Structural typing |
| Java | `import pkg.Class` | `obj.method()`, `Class.method()` | `Type param`, `extends`, `implements` |
| Kotlin | `import pkg.func` | `func()`, `obj.method()` | `: Type`, `: Interface` |
| C# | `using Namespace` | `obj.Method()`, `Class.Method()` | `: Type`, `: Interface` |
| Ruby | `require 'file'` | `obj.method`, `method` | Duck typing (limited) |
| Swift | `import Module` | `func()`, `obj.method()` | `: Protocol`, `: Type` |
| Scala | `import pkg._` | `func()`, `obj.method()` | `: Type`, `extends`, `with` |
| Rust | `use crate::module` | `func()`, `self.method()` | `: Type`, `impl Trait` |
| C | `#include "file.h"` | `func()` | Structural (limited) |
| C++ | `#include`, `using` | `func()`, `obj.method()` | `: Type`, inheritance |
