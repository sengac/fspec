@KGRAPH-025 @KGRAPH-030 @KGRAPH-031 @KGRAPH-032 @KGRAPH-033 @KGRAPH-034 @KGRAPH-035 @KGRAPH-036 @KGRAPH-037 @KGRAPH-038 @KGRAPH-039 @KGRAPH-040 @knowledge-graph
Feature: Multi-Language AST Extraction
  As a developer using fspec on a polyglot codebase
  I want the AST extraction pipeline to support Python, Go, Java, C, C++, C#, Ruby, Kotlin, Swift, Scala, and PHP
  So that code structure graphs are built for any language in my project

  """
  Architecture:
  Each language has a dedicated ast_<lang>_extractor.rs in codelet/napi/src/graph/ast_pipeline/.
  All extractors follow the same pattern: parse source with SupportLang::<Lang>, extract Function
  and Type nodes using ast-grep patterns, and return Vec<GraphEntity>.
  Dependency extractors parse language-specific manifest files (requirements.txt, go.mod, pom.xml,
  build.gradle, Gemfile, composer.json, Package.swift, build.sbt, .csproj).
  Pipeline registration in mod.rs maps file extensions to extractors.
  """

  Scenario: Extract Python functions and classes from .py files
    Given a Python file "src/auth/login.py" with def, async def, and class declarations
    When the Python extractor parses the file
    Then Function nodes should be created for each def and async def
    And Type nodes should be created for each class with typeKind "class"
    And Contains and ContainsType edges should link File to children
    And isPublic should be false for functions starting with underscore

  Scenario: Extract Python dependencies from requirements.txt and pyproject.toml
    Given a project with requirements.txt listing packages with version constraints
    And a pyproject.toml with project.dependencies and optional-dependencies
    When the pip dependency extractor runs
    Then Dependency nodes should be created with source "pip"
    And DependsOn edges should link manifest files to dependencies

  Scenario: Extract Go functions and types from .go files
    Given a Go file "internal/handler.go" with func, method receivers, and struct/interface
    When the Go extractor parses the file
    Then Function nodes should be created with isPublic based on capitalization
    And Type nodes should be created for structs and interfaces
    And test files ending in _test.go should have isTest set to true

  Scenario: Extract Go dependencies from go.mod
    Given a project with go.mod listing require blocks
    When the go.mod dependency extractor runs
    Then Dependency nodes should be created with source "go"

  Scenario: Extract Java methods and types from .java files
    Given a Java file with public/private methods and class/interface/enum declarations
    When the Java extractor parses the file
    Then Function nodes should be created with isPublic from access modifiers
    And Type nodes should be created for classes, interfaces, and enums

  Scenario: Extract Java dependencies from pom.xml and build.gradle
    Given a project with pom.xml dependencies and build.gradle implementation blocks
    When the Java dependency extractors run
    Then Dependency nodes should be created with source "maven" or "gradle"

  Scenario: Extract C functions and types from .c and .h files
    Given a C file with function definitions, structs, enums, and typedefs
    When the C extractor parses the file
    Then Function nodes should be created with isPublic false for static functions
    And Type nodes should be created for structs, enums, and typedefs

  Scenario: Extract C++ functions and types from .cpp files
    Given a C++ file with classes, methods, namespaces, and templates
    When the C++ extractor parses the file
    Then Function nodes should be created for standalone and class methods
    And Type nodes should be created for classes, structs, enums, and namespaces

  Scenario: Disambiguate .h files between C and C++
    Given a .h file containing C++ keywords like class or namespace
    When the pipeline processes the .h file
    Then the C++ extractor should be used instead of C

  Scenario: Extract C# methods and types from .cs files
    Given a C# file with public/private methods and class/interface/struct/enum
    When the C# extractor parses the file
    Then Function nodes should be created with access modifier visibility
    And Type nodes should be created for all C# type declarations

  Scenario: Extract C# dependencies from .csproj files
    Given a project with .csproj PackageReference elements
    When the .csproj dependency extractor runs
    Then Dependency nodes should be created with source "nuget"

  Scenario: Extract Ruby methods and types from .rb files
    Given a Ruby file with def, def self., class, and module declarations
    When the Ruby extractor parses the file
    Then Function nodes and Type nodes should be created
    And spec files should have isTest set to true

  Scenario: Extract Ruby dependencies from Gemfile
    Given a project with Gemfile listing gems with version constraints
    When the Gemfile dependency extractor runs
    Then Dependency nodes should be created with source "gem"

  Scenario: Extract Kotlin functions and types from .kt files
    Given a Kotlin file with fun, suspend fun, and class/interface/object/enum
    When the Kotlin extractor parses the file
    Then Function nodes should be created with isAsync for suspend functions
    And Type nodes should be created for all Kotlin type declarations

  Scenario: Extract Swift functions and types from .swift files
    Given a Swift file with func, async func, and class/struct/protocol/enum
    When the Swift extractor parses the file
    Then Function nodes and Type nodes should be created
    And protocols should have typeKind "trait_kind"

  Scenario: Extract Swift dependencies from Package.swift
    Given a project with Package.swift listing .package dependencies
    When the Swift dependency extractor runs
    Then Dependency nodes should be created with source "spm"

  Scenario: Extract Scala functions and types from .scala files
    Given a Scala file with def, class, trait, object, and case class
    When the Scala extractor parses the file
    Then Function nodes and Type nodes should be created
    And traits should have typeKind "trait_kind"

  Scenario: Extract Scala dependencies from build.sbt
    Given a project with build.sbt listing libraryDependencies
    When the sbt dependency extractor runs
    Then Dependency nodes should be created with source "sbt"

  Scenario: Extract PHP functions and types from .php files
    Given a PHP file with function, class, interface, and trait declarations
    When the PHP extractor parses the file
    Then Function nodes and Type nodes should be created

  Scenario: Extract PHP dependencies from composer.json
    Given a project with composer.json require and require-dev sections
    When the composer dependency extractor runs
    Then Dependency nodes should be created with source "composer"

  Scenario: Walk project discovers all supported language files
    Given a project directory with files in Python, Go, Java, C, C++, C#, Ruby, Kotlin, Swift, Scala, and PHP
    When walk_and_extract processes the project
    Then File nodes should be created for all supported extensions
    And no constraint violations should occur when loading into the graph

  Scenario: Missing dependency files return empty results
    Given a project without any dependency manifest files
    When all dependency extractors run
    Then each should return an empty list without errors
