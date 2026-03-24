# KGRAPH-038: AST Extractor — Swift (+ Package.swift dependency extractor)

## Overview

Add Swift language support to the AST extraction pipeline. Includes an AST extractor for `.swift` files and a `Package.swift` dependency extractor for Swift Package Manager.

## Files to Create

### 1. `codelet/napi/src/graph/ast_pipeline/ast_swift_extractor.rs`

**SupportLang variant:** `SupportLang::Swift`
**Extensions:** `.swift`

#### Function Extraction Patterns

```rust
const SWIFT_FUNCTION_PATTERNS: &[(&str, bool)] = &[
    ("func $NAME($$$ARGS) { $$$BODY }", false),
    ("func $NAME($$$ARGS) -> $RET { $$$BODY }", false),
    ("func $NAME($$$ARGS) throws { $$$BODY }", false),
    ("func $NAME($$$ARGS) throws -> $RET { $$$BODY }", false),
    ("func $NAME($$$ARGS) async { $$$BODY }", false),
    ("func $NAME($$$ARGS) async -> $RET { $$$BODY }", false),
    ("func $NAME($$$ARGS) async throws -> $RET { $$$BODY }", false),
    ("public func $NAME($$$ARGS) -> $RET { $$$BODY }", true),
    ("private func $NAME($$$ARGS) -> $RET { $$$BODY }", false),
    ("internal func $NAME($$$ARGS) -> $RET { $$$BODY }", false),
    ("static func $NAME($$$ARGS) -> $RET { $$$BODY }", false),
    ("class func $NAME($$$ARGS) -> $RET { $$$BODY }", false),
    ("override func $NAME($$$ARGS) -> $RET { $$$BODY }", false),
];
```

**Notes:**
- `is_async` from `async` keyword
- `is_public` from access modifier (`public`, `open` = public; `private`, `fileprivate`, `internal` = not public; default = `internal`)
- `class func` = type method (like static but overridable)
- `throws`/`async throws` are part of the signature, not separate patterns — may need regex fallback

#### Type Extraction Patterns

```rust
const SWIFT_TYPE_PATTERNS: &[(&str, &str, bool)] = &[
    ("class $NAME { $$$BODY }", "class", false),
    ("class $NAME: $$$BASES { $$$BODY }", "class", false),
    ("struct $NAME { $$$BODY }", "struct_kind", false),
    ("struct $NAME: $$$PROTOS { $$$BODY }", "struct_kind", false),
    ("protocol $NAME { $$$BODY }", "trait_kind", false),   // protocols = traits
    ("enum $NAME { $$$BODY }", "enum_kind", false),
    ("enum $NAME: $BASE { $$$BODY }", "enum_kind", false),
    ("actor $NAME { $$$BODY }", "class", false),            // Swift concurrency actors
];
```

- `class Foo: Bar, SomeProtocol` → `Extends` to Bar, `Implements` to SomeProtocol
- Protocols mapped to `trait_kind` (same as Rust traits)
- `actor` modeled as class (it's a reference type with isolation)

#### Import Extraction

```
"import $MODULE"
"import class $MODULE.$TYPE"
"import func $MODULE.$FUNC"
```

- Swift imports are module-level: `import UIKit`, `import Foundation`
- For project-internal imports, no file-level resolution (Swift uses module system)
- Create Imports edges with importPath property

### 2. `codelet/napi/src/graph/ast_pipeline/swift_dep_extractor.rs`

#### Package.swift Parser

- Read `Package.swift` from project root
- Parse `.package()` declarations via regex:
  ```swift
  .package(url: "https://github.com/vapor/vapor.git", from: "4.0.0"),
  .package(url: "https://github.com/apple/swift-log.git", .upToNextMajor(from: "1.0.0")),
  .package(name: "MyLibrary", path: "../MyLibrary"),
  ```
- Extract package name from URL (last path component minus `.git`)
- Extract version from `from:` parameter
- Dependency slug: `dep::<package_name>`
- Source: `"spm"` (Swift Package Manager)
- All dependencies are production (SPM doesn't have dev-only concept in same way)

### 3. Pipeline Registration

```rust
// SUPPORTED_EXTENSIONS
"swift"

// extract_file()
"swift" => ast_swift_extractor::extract_swift(&source, &rel_path),

// Dependencies
all_entities.extend(swift_dep_extractor::extract_swift_dependencies(&project_root)?);
```

## Edges

| Edge | From → To |
|------|-----------|
| Contains | File → Function |
| ContainsType | File → Type |
| Imports | File → File (module-level) |
| Extends | Type → Type |
| Implements | Type → Type (protocol conformance) |
| DependsOn | File → Dependency (Package.swift) |

## Swift-Specific Considerations

- **Test detection:** Files in `Tests/` directory or containing `XCTestCase`
- **Extensions:** `extension Foo: SomeProtocol { ... }` — could create Implements edges
- **Computed properties:** Not extracted as functions
- **Property wrappers:** `@Published var x` — not extracted

## Estimated Complexity: 5 points
