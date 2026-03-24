# KGRAPH-036: AST Extractor — Ruby (+ Gemfile dependency extractor)

## Overview

Add Ruby language support to the AST extraction pipeline. Includes an AST extractor for `.rb` files and a Gemfile dependency extractor.

## Files to Create

### 1. `codelet/napi/src/graph/ast_pipeline/ast_ruby_extractor.rs`

**SupportLang variant:** `SupportLang::Ruby`
**Extensions:** `.rb`, `.gemspec`

#### Function Extraction Patterns

```rust
const RUBY_METHOD_PATTERNS: &[&str] = &[
    "def $NAME($$$ARGS) $$$BODY end",
    "def $NAME $$$BODY end",                    // no parens
    "def self.$NAME($$$ARGS) $$$BODY end",      // class method
    "def self.$NAME $$$BODY end",               // class method no parens
];
```

**Notes:**
- `is_public` default true; set false if preceded by `private` or `protected` keyword
- Ruby methods don't use `async` — `is_async` always false
- `self.method` = class method; instance methods are the default
- `qualifiedName`: `ClassName#method` (instance) or `ClassName.method` (class)
- Block parameters (`do |x|`) are not function definitions

#### Type Extraction Patterns

```rust
const RUBY_TYPE_PATTERNS: &[(&str, &str)] = &[
    ("class $NAME $$$BODY end", "class"),
    ("class $NAME < $BASE $$$BODY end", "class"),     // with inheritance
    ("module $NAME $$$BODY end", "module"),
];
```

- `class Foo < Bar` → `Extends` edge to Bar
- `include SomeModule` inside class → could generate `Implements`-like edge (optional)
- Modules used for namespacing and mixins

#### Import Extraction

```
"require '$PATH'"
"require \"$PATH\""
"require_relative '$PATH'"
"require_relative \"$PATH\""
```

- `require` loads gems or stdlib — resolve against project files if possible
- `require_relative` is always relative to current file — create Imports edge
- `require_relative '../models/user'` → resolve `.rb` extension

### 2. `codelet/napi/src/graph/ast_pipeline/gemfile_dep_extractor.rs`

#### Gemfile Parser

- Read `Gemfile` from project root
- Parse `gem` declarations via line scanning:
  ```ruby
  gem 'rails', '~> 7.0'
  gem 'rspec', group: :test
  gem 'pg', '~> 1.5', group: [:development, :test]
  ```
- Extract gem name and version constraint
- `isDev` when `group: :test` or `group: :development` or inside `group :test do ... end` block
- Dependency slug: `dep::<gem_name>`
- Source: `"gem"`

**Note:** Gemfile is Ruby DSL, not structured data. Use line-based regex:
```
gem\s+['"]([^'"]+)['"](?:\s*,\s*['"]([^'"]+)['"])?
```

### 3. Pipeline Registration

```rust
// SUPPORTED_EXTENSIONS
"rb", "gemspec"

// extract_file()
"rb" | "gemspec" => ast_ruby_extractor::extract_ruby(&source, &rel_path),

// Dependencies
all_entities.extend(gemfile_dep_extractor::extract_gemfile_dependencies(&project_root)?);
```

## Edges

| Edge | From → To |
|------|-----------|
| Contains | File → Function |
| ContainsType | File → Type |
| Imports | File → File (require/require_relative) |
| Extends | Type → Type (class < Base) |
| DependsOn | File → Dependency (Gemfile) |

## Ruby-Specific Considerations

- **Test detection:** Files in `spec/` or `test/` dirs, or `_spec.rb`/`_test.rb` suffix
- **Metaprogramming:** `define_method`, `method_missing` — not extractable from AST
- **attr_accessor:** Not extracted as functions
- **Blocks/Procs/Lambdas:** Not extracted as Function nodes

## Estimated Complexity: 5 points
