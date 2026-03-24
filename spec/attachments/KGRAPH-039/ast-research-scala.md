# KGRAPH-039: AST Extractor — Scala (+ sbt dependency extractor)

## Overview

Add Scala language support to the AST extraction pipeline. Includes an AST extractor for `.scala` files and an sbt dependency extractor for `build.sbt`.

## Files to Create

### 1. `codelet/napi/src/graph/ast_pipeline/ast_scala_extractor.rs`

**SupportLang variant:** `SupportLang::Scala`
**Extensions:** `.scala`, `.sc`

#### Function Extraction Patterns

```rust
const SCALA_FUNCTION_PATTERNS: &[&str] = &[
    "def $NAME($$$ARGS): $RET = { $$$BODY }",
    "def $NAME($$$ARGS) = { $$$BODY }",
    "def $NAME($$$ARGS): $RET = $EXPR",
    "def $NAME($$$ARGS) = $EXPR",
    "def $NAME: $RET = { $$$BODY }",         // parameterless def
    "def $NAME = { $$$BODY }",
    "override def $NAME($$$ARGS): $RET = { $$$BODY }",
    "private def $NAME($$$ARGS): $RET = { $$$BODY }",
    "protected def $NAME($$$ARGS): $RET = { $$$BODY }",
];
```

**Notes:**
- Scala has no `async` keyword natively (uses Futures/ZIO/Cats Effect) — `is_async` always false
- `is_public` = not `private` or `protected` (Scala default is public)
- `implicit def` — still a function, just with special resolution
- `val` and `var` are NOT functions
- Multiple parameter lists: `def foo(a: Int)(b: String)` — count all params across lists

#### Type Extraction Patterns

```rust
const SCALA_TYPE_PATTERNS: &[(&str, &str)] = &[
    ("class $NAME { $$$BODY }", "class"),
    ("class $NAME($$$PARAMS) { $$$BODY }", "class"),
    ("class $NAME extends $BASE { $$$BODY }", "class"),
    ("case class $NAME($$$PARAMS)", "class"),
    ("abstract class $NAME { $$$BODY }", "class"),
    ("trait $NAME { $$$BODY }", "trait_kind"),
    ("trait $NAME extends $BASE { $$$BODY }", "trait_kind"),
    ("object $NAME { $$$BODY }", "class"),
    ("case object $NAME { $$$BODY }", "class"),
    ("sealed trait $NAME { $$$BODY }", "trait_kind"),
    ("enum $NAME { $$$BODY }", "enum_kind"),         // Scala 3
];
```

- `class Foo extends Bar with Baz with Qux` → `Extends` to Bar, `Implements` to Baz, Qux
- `trait` maps to `trait_kind`
- `object` = singleton, modeled as class
- `case class` = data class, still typeKind `"class"`

#### Import Extraction

```
"import $PACKAGE"
"import $PACKAGE._"          // wildcard
"import $PACKAGE.{$$$NAMES}"  // selective
```

- Scala imports can be anywhere in the file (not just top)
- Wildcard imports: `import com.example._`
- Selective: `import com.example.{Foo, Bar}`
- Resolve to file paths where possible

### 2. `codelet/napi/src/graph/ast_pipeline/sbt_dep_extractor.rs`

#### build.sbt Parser

- Read `build.sbt` from project root
- Parse `libraryDependencies` via regex:
  ```scala
  libraryDependencies += "org.typelevel" %% "cats-core" % "2.9.0"
  libraryDependencies ++= Seq(
    "org.http4s" %% "http4s-core" % "0.23.18",
    "org.scalatest" %% "scalatest" % "3.2.15" % Test
  )
  ```
- `%%` = Scala-version-suffixed artifact (standard)
- `%` between group and artifact = Java dependency
- `% Test` or `% "test"` → isDev = true
- Dependency slug: `dep::<groupId>:<artifactId>`
- Source: `"sbt"`

**Note:** build.sbt is Scala DSL — use regex-based line scanning, not AST parsing.

### 3. Pipeline Registration

```rust
// SUPPORTED_EXTENSIONS
"scala", "sc"
// Note: .sbt files are NOT source code to extract AST from

// extract_file()
"scala" | "sc" => ast_scala_extractor::extract_scala(&source, &rel_path),

// Dependencies
all_entities.extend(sbt_dep_extractor::extract_sbt_dependencies(&project_root)?);
```

## Edges

| Edge | From → To |
|------|-----------|
| Contains | File → Function |
| ContainsType | File → Type |
| Imports | File → File |
| Extends | Type → Type |
| Implements | Type → Type (with Trait) |
| DependsOn | File → Dependency (build.sbt) |

## Scala-Specific Considerations

- **Test detection:** Files in `src/test/` or containing `extends AnyFunSuite`/`extends FlatSpec`
- **Implicits/Givens:** `implicit def`/`given` — extracted as normal functions
- **Type aliases:** `type Foo = Bar` — could extract as Type with type_alias kind
- **Package objects:** `package object foo { ... }` — extract contents normally
- **Scala 3 syntax:** `enum`, `given`, `extension` — newer patterns

## Estimated Complexity: 5 points
