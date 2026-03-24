# KGRAPH-040: AST Extractor — PHP (+ composer.json dependency extractor)

## Overview

Add PHP language support to the AST extraction pipeline. Includes an AST extractor for `.php` files and a `composer.json` dependency extractor.

## Files to Create

### 1. `codelet/napi/src/graph/ast_pipeline/ast_php_extractor.rs`

**SupportLang variant:** `SupportLang::Php`
**Extensions:** `.php`

#### Function Extraction Patterns

```rust
const PHP_FUNCTION_PATTERNS: &[(&str, bool)] = &[
    // Top-level functions
    ("function $NAME($$$ARGS) { $$$BODY }", false),
    // Class methods
    ("public function $NAME($$$ARGS) { $$$BODY }", true),
    ("public static function $NAME($$$ARGS) { $$$BODY }", true),
    ("protected function $NAME($$$ARGS) { $$$BODY }", false),
    ("private function $NAME($$$ARGS) { $$$BODY }", false),
    // Return type declarations (PHP 7+)
    ("public function $NAME($$$ARGS): $RET { $$$BODY }", true),
    ("function $NAME($$$ARGS): $RET { $$$BODY }", false),
    // Abstract methods
    ("abstract public function $NAME($$$ARGS): $RET;", true),
    ("abstract public function $NAME($$$ARGS);", true),
];
```

**Notes:**
- `is_public` from access modifier
- `is_async` always false (PHP doesn't have native async/await, uses generators/fibers)
- `qualifiedName` for methods: `ClassName::methodName`
- `__construct`, `__destruct` etc. are magic methods — extract with their names
- Arrow functions `fn($x) => $x * 2` are NOT extracted

#### Type Extraction Patterns

```rust
const PHP_TYPE_PATTERNS: &[(&str, &str, bool)] = &[
    ("class $NAME { $$$BODY }", "class", false),
    ("class $NAME extends $BASE { $$$BODY }", "class", false),
    ("class $NAME implements $$$IFACES { $$$BODY }", "class", false),
    ("class $NAME extends $BASE implements $$$IFACES { $$$BODY }", "class", false),
    ("abstract class $NAME { $$$BODY }", "class", false),
    ("final class $NAME { $$$BODY }", "class", false),
    ("interface $NAME { $$$BODY }", "interface", false),
    ("interface $NAME extends $$$BASES { $$$BODY }", "interface", false),
    ("trait $NAME { $$$BODY }", "trait_kind", false),
    ("enum $NAME { $$$BODY }", "enum_kind", false),         // PHP 8.1+
    ("enum $NAME: $TYPE { $$$BODY }", "enum_kind", false),  // backed enum
];
```

- `class Foo extends Bar implements Baz, Qux` → Extends + Implements edges
- PHP traits map to `trait_kind`
- PHP 8.1 enums supported

#### Import Extraction

```
"use $NAMESPACE;"
"use $NAMESPACE as $ALIAS;"
"use function $NAMESPACE\\$FUNC;"
"use const $NAMESPACE\\$CONST;"
```

Also handle:
```
"namespace $NAME;"
"namespace $NAME { $$$BODY }"
```

- `use` imports fully qualified class names
- Resolve namespace to PSR-4 directory structure: `App\Models\User` → `src/Models/User.php`
- PSR-4 autoload mapping from `composer.json` can help resolve paths

### 2. `codelet/napi/src/graph/ast_pipeline/composer_dep_extractor.rs`

#### composer.json Parser

- Read `composer.json` from project root
- Parse `require` and `require-dev` objects:
  ```json
  {
    "require": {
      "php": "^8.1",
      "laravel/framework": "^10.0",
      "guzzlehttp/guzzle": "^7.0"
    },
    "require-dev": {
      "phpunit/phpunit": "^10.0",
      "laravel/pint": "^1.0"
    }
  }
  ```
- Skip `php` and `ext-*` entries (runtime requirements, not packages)
- `require` → isDev = false; `require-dev` → isDev = true
- Dependency slug: `dep::<vendor/package>`
- Source: `"composer"`
- Uses `serde_json` (same as npm_dep_extractor)

### 3. Pipeline Registration

```rust
// SUPPORTED_EXTENSIONS
"php"

// extract_file()
"php" => ast_php_extractor::extract_php(&source, &rel_path),

// Dependencies
all_entities.extend(composer_dep_extractor::extract_composer_dependencies(&project_root)?);
```

## Edges

| Edge | From → To |
|------|-----------|
| Contains | File → Function |
| ContainsType | File → Type |
| Imports | File → File (use/namespace resolution) |
| Extends | Type → Type |
| Implements | Type → Type |
| DependsOn | File → Dependency (composer.json) |

## PHP-Specific Considerations

- **Test detection:** Files in `tests/` dir or extending `TestCase` or containing `@test` annotation
- **Namespace resolution:** PSR-4 mapping from composer.json `autoload` section
- **PHP opening tag:** All PHP files start with `<?php` — tree-sitter handles this
- **Mixed HTML/PHP:** Files with `?>` ... `<?php` — extract only the PHP parts
- **Traits:** `use SomeTrait;` inside a class is different from top-level `use`

## Estimated Complexity: 5 points
