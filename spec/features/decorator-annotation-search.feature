@done
@KGRAPH-068
Feature: Decorator and Annotation Search
  """
  This feature was largely implemented across KGRAPH-063 (metadata storage/extraction)
  and KGRAPH-067 (decorator/parameter query filters). Remaining gap: Scala and PHP
  decorator styles were mapped to None instead of AtSign/HashBracket.

  Cross-language decorator filter matching is covered by fulltext-content-search.feature.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Decorator extraction uses a data-driven DecoratorStyle enum per language
  #   2. AtSign languages (Python, TypeScript, JavaScript, Dart, Java, Kotlin, Scala, Swift) extract @name decorators; Rust and PHP extract #[name] as HashBracket; C# extracts [Name] as SquareBracket
  #   3. Languages without decorator syntax (Go, C, C++, Ruby) produce empty decorator strings
  #   4. Decorator filter strips leading @, #[, and trailing ] symbols for cross-language matching
  #   5. Parameter filter does contains matching against parameters property
  #
  # ========================================
  Background: User Story
    As an AI agent
    I want to find functions by decorator/annotation and by parameter name across all supported languages
    So that I can locate API endpoints, test functions, event handlers, and injection points

  Scenario: Python decorators extracted during indexing
    Given a Python file with functions decorated with @staticmethod and @override
    When the file is indexed via ast_index
    Then the function node's decorators property contains "@staticmethod, @override"

  Scenario: Scala annotations extracted as AtSign style
    Given a Scala file with a function annotated with @tailrec
    When the file is indexed via ast_index
    Then the function node's decorators property contains "@tailrec"

  Scenario: PHP 8 attributes extracted as HashBracket style
    Given a PHP file with a function attributed with #[Route('/api')]
    When the file is indexed via ast_index
    Then the function node's decorators property contains "#[Route('/api')]"

  Scenario: Rust attributes extracted as HashBracket style
    Given a Rust file with a function attributed with #[test]
    When the file is indexed via ast_index
    Then the function node's decorators property contains "#[test]"

  Scenario: C# attributes extracted as SquareBracket style
    Given a C# file with a function attributed with [HttpGet]
    When the file is indexed via ast_index
    Then the function node's decorators property contains "[HttpGet]"

  Scenario: Languages without decorators produce empty string
    Given a Go file with functions that have no decorator syntax
    When the file is indexed via ast_index
    Then the function node's decorators property is an empty string
