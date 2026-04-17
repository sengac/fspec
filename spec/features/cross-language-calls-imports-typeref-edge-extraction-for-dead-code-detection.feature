@KGRAPH-041
Feature: Cross-Language Calls/Imports/TypeRef Edge Extraction for Dead Code Detection
  """
  Parent story for edge extraction across 12 non-TypeScript languages.
  Each language extractor emits Imports, Calls, and TypeRef edges.
  Children: KGRAPH-042 (PHP), KGRAPH-043 (Python), KGRAPH-044 (Go),
  KGRAPH-045 (Rust), KGRAPH-046 (Java), KGRAPH-047 (C/C++),
  KGRAPH-048 (C#), KGRAPH-049 (Ruby), KGRAPH-050 (Kotlin),
  KGRAPH-051 (Swift), KGRAPH-052 (Scala), KGRAPH-053 (Infrastructure).
  """

  Background: User Story
    As a developer
    I want to run ast_dead_code on any language repo and get meaningful results
    So that I can find orphan files, uncalled functions, and unreferenced types across all 13 supported languages, not just TypeScript

  Scenario: All language extractors emit edge relationships for dead code analysis
    Given updated extractors for PHP, Python, Go, Rust, Java, C, C++, C#, Ruby, Kotlin, Swift, and Scala
    And each extractor accepts a `known_files` parameter for import resolution
    When each extractor processes source files with imports, function calls, and type annotations
    Then Imports edges are emitted for project-local imports in each language
    And Calls edges are emitted for same-file function calls in each language
    And TypeRef edges are emitted for type annotations where the language supports them
    And external package imports do not generate any edges
