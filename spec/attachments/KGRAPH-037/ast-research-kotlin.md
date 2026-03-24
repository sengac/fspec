# KGRAPH-037: AST Extractor — Kotlin

## Overview

Add Kotlin language support to the AST extraction pipeline. Depends on KGRAPH-032 (Java) for the shared Gradle dependency extractor (`build.gradle.kts` uses the same dependency syntax as `build.gradle`).

## Files to Create

### 1. `codelet/napi/src/graph/ast_pipeline/ast_kotlin_extractor.rs`

**SupportLang variant:** `SupportLang::Kotlin`
**Extensions:** `.kt`, `.kts`

#### Function Extraction Patterns

```rust
const KOTLIN_FUNCTION_PATTERNS: &[(&str, bool)] = &[
    ("fun $NAME($$$ARGS): $RET { $$$BODY }", false),
    ("fun $NAME($$$ARGS) { $$$BODY }", false),
    ("fun $NAME($$$ARGS): $RET = $EXPR", false),          // expression body
    ("suspend fun $NAME($$$ARGS): $RET { $$$BODY }", false),
    ("suspend fun $NAME($$$ARGS) { $$$BODY }", false),
    ("private fun $NAME($$$ARGS): $RET { $$$BODY }", false),
    ("internal fun $NAME($$$ARGS): $RET { $$$BODY }", false),
    ("override fun $NAME($$$ARGS): $RET { $$$BODY }", false),
];
```

**Notes:**
- `is_async` from `suspend` keyword (Kotlin coroutines)
- `is_public` = not `private` or `internal` (Kotlin default is public)
- Extension functions: `fun String.toSlug(): String` — name = `toSlug`, qualifiedName = `String.toSlug`
- `override` modifier indicates interface/superclass implementation

#### Type Extraction Patterns

```rust
const KOTLIN_TYPE_PATTERNS: &[(&str, &str, bool)] = &[
    ("class $NAME { $$$BODY }", "class", false),
    ("class $NAME($$$PARAMS) { $$$BODY }", "class", false),        // primary constructor
    ("class $NAME : $$$BASES { $$$BODY }", "class", false),
    ("data class $NAME($$$PARAMS)", "class", false),
    ("sealed class $NAME { $$$BODY }", "class", false),
    ("interface $NAME { $$$BODY }", "interface", false),
    ("object $NAME { $$$BODY }", "class", false),                   // singleton
    ("enum class $NAME { $$$BODY }", "enum_kind", false),
    ("abstract class $NAME { $$$BODY }", "class", false),
];
```

- `class Foo : Bar(), Baz` → `Extends` to Bar, `Implements` to Baz (interfaces have no `()`)
- `data class` and `sealed class` are still `typeKind: "class"`
- `object` declarations = singleton objects, modeled as class
- Companion objects: `companion object { ... }` — skip or extract as nested type

#### Import Extraction

```
"import $PACKAGE"
"import $PACKAGE as $ALIAS"
```

- Kotlin imports similar to Java — fully qualified paths
- Resolve to files by converting dots to slashes + `.kt` extension
- `import` with `as` alias — store importPath as the original, not the alias

### 2. Gradle Dependency Extractor (Shared from KGRAPH-032)

The `gradle_dep_extractor.rs` from KGRAPH-032 already handles both `build.gradle` and `build.gradle.kts`. Kotlin projects use `.kts` variant with identical dependency syntax. No additional dep extractor needed.

### 3. Pipeline Registration

```rust
// SUPPORTED_EXTENSIONS
"kt", "kts"

// extract_file()
"kt" | "kts" => ast_kotlin_extractor::extract_kotlin(&source, &rel_path),

// No new dependency extractor — gradle_dep_extractor already handles .kts
```

## Edges

| Edge | From → To |
|------|-----------|
| Contains | File → Function |
| ContainsType | File → Type |
| Imports | File → File |
| Extends | Type → Type |
| Implements | Type → Type |

## Kotlin-Specific Considerations

- **Top-level functions:** Kotlin allows functions outside classes — extracted normally
- **Extension functions:** `fun Type.name()` — qualifiedName includes receiver type
- **Coroutines:** `suspend` → isAsync = true
- **Default parameters:** `fun foo(x: Int = 0)` — count all params including defaulted
- **Test detection:** Files in `src/test/` or containing `@Test` annotation

## Estimated Complexity: 3 points

Lower than other cards because: (1) Gradle dep extractor is shared from Java card, (2) Kotlin syntax is cleaner and more regular than Java.
