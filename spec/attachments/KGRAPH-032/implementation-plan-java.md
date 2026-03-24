# KGRAPH-032: AST Extractor — Java (+ Maven/Gradle dependency extractor)

## Overview

Add Java language support to the AST extraction pipeline. Includes an AST extractor for Java source, plus Maven (pom.xml) and Gradle (build.gradle) dependency extractors. The Gradle extractor will be shared with Kotlin (KGRAPH-037).

## Files to Create

### 1. `codelet/napi/src/graph/ast_pipeline/ast_java_extractor.rs`

**SupportLang variant:** `SupportLang::Java`  
**Extensions:** `.java`

#### Function (Method) Extraction Patterns

```rust
const JAVA_METHOD_PATTERNS: &[(&str, bool)] = &[
    // Public methods
    ("public $RET $NAME($$$ARGS) { $$$BODY }", true),
    ("public static $RET $NAME($$$ARGS) { $$$BODY }", true),
    ("public synchronized $RET $NAME($$$ARGS) { $$$BODY }", true),
    // Protected methods
    ("protected $RET $NAME($$$ARGS) { $$$BODY }", false),
    // Private methods
    ("private $RET $NAME($$$ARGS) { $$$BODY }", false),
    // Package-private (no modifier)
    ("$RET $NAME($$$ARGS) { $$$BODY }", false),
    // Abstract methods (no body)
    ("public abstract $RET $NAME($$$ARGS);", true),
    ("abstract $RET $NAME($$$ARGS);", false),
];
```

**Notes:**
- Java has no top-level functions — everything is a method inside a class
- `is_public` from access modifier keyword
- `is_async` is always `false` (Java uses threads, not async/await)
- `qualifiedName`: `ClassName.methodName`
- Constructor: pattern `public $NAME($$$ARGS) { $$$BODY }` where `$NAME` matches the class name
- Annotations (`@Override`, `@Test`) are not extracted but `@Test` could mark test methods

#### Type Extraction Patterns

```rust
const JAVA_TYPE_PATTERNS: &[(&str, &str, bool)] = &[
    ("public class $NAME { $$$BODY }", "class", true),
    ("class $NAME { $$$BODY }", "class", false),
    ("public class $NAME extends $BASE { $$$BODY }", "class", true),
    ("public class $NAME implements $$$IFACES { $$$BODY }", "class", true),
    ("public interface $NAME { $$$BODY }", "interface", true),
    ("interface $NAME { $$$BODY }", "interface", false),
    ("public enum $NAME { $$$BODY }", "enum_kind", true),
    ("enum $NAME { $$$BODY }", "enum_kind", false),
    ("public abstract class $NAME { $$$BODY }", "class", true),
    ("public record $NAME($$$FIELDS) { $$$BODY }", "class", true),  // Java 16+ records
];
```

**Notes:**
- Extract `Extends` edges from `extends BaseClass`
- Extract `Implements` edges from `implements Interface1, Interface2`
- One public class per file (Java convention) — file slug maps naturally

#### Import Extraction

```rust
// Java import patterns
"import $PACKAGE;"
"import static $PACKAGE;"
```

**Approach:**
- Java imports are fully qualified class paths: `import com.example.auth.UserService;`
- Resolve to file paths by converting dots to slashes: `com/example/auth/UserService.java`
- Wildcard imports (`import com.example.auth.*`) create edges to the package directory
- `import static` sets `isTypeOnly: false` (it imports methods)
- Regular imports are type imports: `isTypeOnly: true`

### 2. `codelet/napi/src/graph/ast_pipeline/maven_dep_extractor.rs`

#### pom.xml Parser

- Read `pom.xml` from project root
- Parse XML `<dependencies>` section (use a simple XML parser or regex for this structured format)
- Extract `<groupId>`, `<artifactId>`, `<version>`, `<scope>`
- `isDev` when `<scope>test</scope>`
- Dependency slug: `dep::<groupId>:<artifactId>`
- Create `Dependency` node (source: `"maven"`) + `DependsOn` edge
- **Multi-module:** Check for `<modules>` and process child pom.xml files

**XML Parsing:** Use the `quick-xml` crate (lightweight, already common in Rust ecosystem) or simple regex-based extraction since pom.xml structure is very predictable.

### 3. `codelet/napi/src/graph/ast_pipeline/gradle_dep_extractor.rs`

#### build.gradle / build.gradle.kts Parser

- Read `build.gradle` or `build.gradle.kts` from project root
- Parse dependency declarations via regex/text scanning:
  ```groovy
  implementation 'com.google.code.gson:gson:2.10'
  implementation("com.google.code.gson:gson:2.10")
  testImplementation 'junit:junit:4.13'
  api 'com.example:library:1.0'
  compileOnly 'javax.servlet:javax.servlet-api:4.0.1'
  ```
- Map configuration to isDev:
  - `testImplementation`, `testCompileOnly`, `testRuntimeOnly` → isDev = true
  - `implementation`, `api`, `compileOnly`, `runtimeOnly` → isDev = false
- Dependency slug: `dep::<group>:<artifact>`
- Create `Dependency` node (source: `"gradle"`) + `DependsOn` edge
- **Multi-project:** Check `settings.gradle(.kts)` for `include` directives

**Note:** This extractor is shared with Kotlin (KGRAPH-037). The `.kts` variant uses identical dependency syntax.

### 4. Pipeline Registration

```rust
// SUPPORTED_EXTENSIONS
"java"

// extract_file() match
"java" => ast_java_extractor::extract_java(&source, &rel_path),

// Dependencies
all_entities.extend(maven_dep_extractor::extract_maven_dependencies(&project_root)?);
all_entities.extend(gradle_dep_extractor::extract_gradle_dependencies(&project_root)?);
```

## Entity Summary

| Entity | Properties | Example |
|--------|-----------|---------|
| File | language=`"java"`, isTest from `src/test/` path | `src/main/java/com/example/App.java` |
| Function | isPublic, paramCount, qualifiedName=`Class.method` | `public void handleRequest(Request req)` |
| Type | typeKind: class/interface/enum_kind | `public class UserService` |
| Dependency | source=`"maven"`/`"gradle"`, isDev | `dep::com.google.gson:gson` |

## Edges

| Edge | From → To |
|------|-----------|
| Contains | File → Function |
| ContainsType | File → Type |
| Imports | File → File (resolved from FQ class name) |
| Extends | Type → Type (`extends`) |
| Implements | Type → Type (`implements`) |
| DependsOn | File → Dependency |

## Java-Specific Considerations

- **Test detection:** `src/test/` directory pattern or `@Test` annotation
- **Generics:** `class Foo<T extends Bar>` — extract the base constraint for `Extends` edge
- **Inner classes:** `class Outer { class Inner {} }` — Inner gets its own Type node with qualified slug
- **Annotations:** Not extracted as entities but inform `isTest` detection

## Testing Strategy

1. Unit test `extract_java()` with classes, interfaces, enums, methods
2. Unit test `extract_maven_dependencies()` with sample `pom.xml`
3. Unit test `extract_gradle_dependencies()` with both `.gradle` and `.gradle.kts`
4. Verify `implements`/`extends` edge creation

## Estimated Complexity: 5 points

The dual dependency extractor (Maven + Gradle) adds scope compared to single-format languages.
