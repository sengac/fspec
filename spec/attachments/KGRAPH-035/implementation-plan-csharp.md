# KGRAPH-035: AST Extractor — C# (+ .csproj dependency extractor)

## Overview

Add C# language support to the AST extraction pipeline. Includes an AST extractor for `.cs` files and a `.csproj` dependency extractor for NuGet package references.

## Files to Create

### 1. `codelet/napi/src/graph/ast_pipeline/ast_csharp_extractor.rs`

**SupportLang variant:** `SupportLang::CSharp`
**Extensions:** `.cs`

#### Function (Method) Extraction Patterns

```rust
const CSHARP_METHOD_PATTERNS: &[(&str, bool)] = &[
    ("public $RET $NAME($$$ARGS) { $$$BODY }", true),
    ("public static $RET $NAME($$$ARGS) { $$$BODY }", true),
    ("public async $RET $NAME($$$ARGS) { $$$BODY }", true),
    ("private $RET $NAME($$$ARGS) { $$$BODY }", false),
    ("protected $RET $NAME($$$ARGS) { $$$BODY }", false),
    ("internal $RET $NAME($$$ARGS) { $$$BODY }", false),
    ("$RET $NAME($$$ARGS) { $$$BODY }", false),
    // Expression-bodied members
    ("public $RET $NAME($$$ARGS) => $EXPR;", true),
];
```

**Notes:**
- `is_async` from `async` keyword
- `is_public` from access modifier
- `qualifiedName`: `ClassName.MethodName`
- Properties with getters/setters are NOT extracted as functions
- Lambda expressions are NOT extracted

#### Type Extraction Patterns

```rust
const CSHARP_TYPE_PATTERNS: &[(&str, &str, bool)] = &[
    ("public class $NAME { $$$BODY }", "class", true),
    ("public class $NAME : $$$BASES { $$$BODY }", "class", true),
    ("class $NAME { $$$BODY }", "class", false),
    ("public interface $NAME { $$$BODY }", "interface", true),
    ("interface $NAME { $$$BODY }", "interface", false),
    ("public struct $NAME { $$$BODY }", "struct_kind", true),
    ("struct $NAME { $$$BODY }", "struct_kind", false),
    ("public enum $NAME { $$$BODY }", "enum_kind", true),
    ("enum $NAME { $$$BODY }", "enum_kind", false),
    ("public record $NAME($$$FIELDS);", "class", true),
    ("public abstract class $NAME { $$$BODY }", "class", true),
    ("public sealed class $NAME { $$$BODY }", "class", true),
];
```

- `Extends` edges from `: BaseClass`
- `Implements` edges from `: IInterface` (convention: interface names start with `I`)
- `partial class` — may appear in multiple files; both get Type nodes, dedup handles it

#### Import Extraction

```
"using $NAMESPACE;"
"using static $NAMESPACE;"
"using $ALIAS = $NAMESPACE;"
```

- `using` directives are namespace-level, not file-level
- Map namespace to directory structure for Imports edges where possible
- `using static` imports static members

### 2. `codelet/napi/src/graph/ast_pipeline/csproj_dep_extractor.rs`

#### .csproj Parser

- Find `*.csproj` files in project root (may have multiple for solution)
- Parse XML `<PackageReference>` elements:
  ```xml
  <PackageReference Include="Newtonsoft.Json" Version="13.0.1" />
  ```
- Extract `Include` (package name) and `Version`
- `isDev` when inside `<ItemGroup Condition="...Test...">` or has `PrivateAssets="All"`
- Dependency slug: `dep::<PackageName>`
- Source: `"nuget"`
- Also check for `<ProjectReference>` for internal project dependencies

### 3. Pipeline Registration

```rust
// SUPPORTED_EXTENSIONS
"cs"

// extract_file()
"cs" => ast_csharp_extractor::extract_csharp(&source, &rel_path),

// Dependencies
all_entities.extend(csproj_dep_extractor::extract_csproj_dependencies(&project_root)?);
```

## Edges

| Edge | From → To |
|------|-----------|
| Contains | File → Function |
| ContainsType | File → Type |
| Imports | File → File (using directive resolution) |
| Extends | Type → Type |
| Implements | Type → Type (interfaces) |
| DependsOn | File → Dependency (.csproj → NuGet) |

## C#-Specific Considerations

- **Namespaces:** `namespace Foo.Bar { ... }` or file-scoped `namespace Foo.Bar;` (C# 10+)
- **Partial classes:** Same type in multiple files — handled by dedup
- **Test detection:** Files in `*.Tests` project or containing `[TestMethod]`/`[Fact]`/`[Test]`
- **Records:** `record Person(string Name)` — extracted as class

## Estimated Complexity: 5 points
